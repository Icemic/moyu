//! Engine manifest fetching, `.tar.zst` extraction, and version switching.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::Utc;

use crate::config::{meta_file, platform_dir};
use crate::model::engine::{
    ActiveEngine, DownloadedVersion, EngineConfig, EngineMeta, PlatformDownload, VersionsJson,
};
use crate::model::tasks::{TaskEvent, TaskId};
use crate::service::{EventSink, download_to_file_verified, http_get_bytes};

pub fn try_detect_platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("windows-amd64"),
        ("linux", "x86_64") => Some("linux-amd64"),
        ("linux", "aarch64") => Some("linux-aarch64"),
        ("macos", "x86_64") => Some("macos-amd64"),
        ("macos", "aarch64") => Some("macos-aarch64"),
        _ => None,
    }
}

pub fn detect_platform() -> Result<&'static str> {
    try_detect_platform().ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported platform: {}/{}\nSupported platforms:\n  windows/x86_64 -> windows-amd64\n  linux/x86_64 -> linux-amd64\n  linux/aarch64 -> linux-aarch64\n  macos/x86_64 -> macos-amd64\n  macos/aarch64 -> macos-aarch64",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })
}

pub fn native_executable_name(target: &str) -> Option<&'static str> {
    match target {
        "windows-amd64" => Some("moyu.exe"),
        "linux-amd64" | "linux-aarch64" | "macos-amd64" | "macos-aarch64" => Some("moyu"),
        _ => None,
    }
}

pub fn load_config(project_root: &Path) -> EngineConfig {
    let defaults = EngineConfig::default();
    let config_path = project_root.join("moyu.json");
    if !config_path.is_file() {
        return defaults;
    }

    let Ok(text) = fs::read_to_string(&config_path) else {
        return defaults;
    };
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) else {
        eprintln!("warn: Failed to parse moyu.json - using default engine config.");
        return defaults;
    };

    let engine = raw.get("engine");
    EngineConfig {
        cdn_url: engine
            .and_then(|e| e.get("cdnUrl"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .unwrap_or(defaults.cdn_url),
        version: engine
            .and_then(|e| e.get("version"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        channel: engine
            .and_then(|e| e.get("channel"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .unwrap_or(defaults.channel),
    }
}

pub fn load_meta(meta_path: &Path) -> Option<EngineMeta> {
    if !meta_path.is_file() {
        return None;
    }
    let text = fs::read_to_string(meta_path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_meta(meta_path: &Path, meta: &EngineMeta) -> Result<()> {
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(meta)? + "\n";
    fs::write(meta_path, json)?;
    Ok(())
}

pub async fn fetch_versions_json(cdn_url: &str) -> Result<VersionsJson> {
    let bytes = http_get_bytes(cdn_url).await?;
    let data: VersionsJson = serde_json::from_slice(&bytes)
        .with_context(|| format!("Failed to parse versions.json from {cdn_url}"))?;
    if data.schema_version != 2 {
        bail!(
            "Unsupported versions.json schema_version: {} (expected 2). Please update moyu-devtool.",
            data.schema_version
        );
    }
    Ok(data)
}

pub async fn download_and_extract(
    url: &str,
    dest_dir: &Path,
    expected_sha256: &str,
    task_id: Option<TaskId>,
    tx: Option<&EventSink<TaskEvent>>,
) -> Result<()> {
    let tmp_name = format!("moyu-engine-{}.tar.zst", uuid::Uuid::new_v4());
    let tmp_file = std::env::temp_dir().join(tmp_name);

    let res = async {
        download_to_file_verified(url, &tmp_file, expected_sha256, task_id, tx).await?;

        if let (Some(id), Some(sender)) = (task_id, tx) {
            let _ = sender.try_send(TaskEvent::Stage {
                id,
                stage: format!("Extracting to {}", dest_dir.display()),
            });
        }

        let archive_path = tmp_file.clone();
        let dest = dest_dir.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<()> {
            if dest.exists() {
                fs::remove_dir_all(&dest)?;
            }
            fs::create_dir_all(&dest)?;

            let file = File::open(&archive_path)?;
            let zstd_decoder = zstd::stream::read::Decoder::new(file)?;
            let mut archive = tar::Archive::new(zstd_decoder);
            archive.unpack(&dest)?;
            Ok(())
        })
        .await??;

        Ok(())
    }
    .await;

    let _ = fs::remove_file(&tmp_file);
    res
}

pub async fn download_platforms(
    project_root: &Path,
    channel_name: &str,
    version_name: &str,
    manifest: &VersionsJson,
    platforms: &[String],
    activate: bool,
    tx: Option<&EventSink<TaskEvent>>,
) -> Result<EngineMeta> {
    let channel = manifest
        .channels
        .get(channel_name)
        .ok_or_else(|| anyhow::anyhow!("Channel \"{channel_name}\" not found."))?;
    let entry = channel
        .versions
        .get(version_name)
        .ok_or_else(|| anyhow::anyhow!("Version \"{version_name}\" not found."))?;

    let meta_path = meta_file(project_root);
    let mut meta = load_meta(&meta_path).unwrap_or_default();

    let existing_platforms = meta
        .downloads
        .get(version_name)
        .map(|d| d.platforms.clone())
        .unwrap_or_default();

    let mut to_download = Vec::new();
    for p in platforms {
        let remote = entry
            .assets
            .get(p)
            .ok_or_else(|| anyhow::anyhow!("No asset for platform \"{p}\" in {version_name}."))?;
        let up_to_date = existing_platforms
            .get(p)
            .is_some_and(|local| local.sha256 == remote.sha256);
        if !up_to_date {
            to_download.push((p.clone(), remote.clone()));
        }
    }

    let mut handles = Vec::new();
    for (platform, asset) in &to_download {
        let dest_dir = platform_dir(project_root, version_name, platform)?;
        let url = asset.url.clone();
        let sha = asset.sha256.clone();
        let platform_label = platform.clone();
        let tx_clone = tx.cloned();
        handles.push(tokio::spawn(async move {
            download_and_extract(&url, &dest_dir, &sha, None, tx_clone.as_ref())
                .await
                .with_context(|| format!("Failed downloading {platform_label}"))
        }));
    }

    for handle in handles {
        handle.await??;
    }

    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let downloaded_set: BTreeSet<&str> = to_download.iter().map(|(p, _)| p.as_str()).collect();
    let mut updated_platforms: BTreeMap<String, PlatformDownload> = existing_platforms;

    for p in platforms {
        let remote = &entry.assets[p];
        let downloaded_at = if downloaded_set.contains(p.as_str()) {
            now.clone()
        } else {
            updated_platforms
                .get(p)
                .map(|old| old.downloaded_at.clone())
                .unwrap_or_else(|| now.clone())
        };
        updated_platforms.insert(
            p.clone(),
            PlatformDownload {
                sha256: remote.sha256.clone(),
                downloaded_at,
            },
        );
    }

    meta.downloads.insert(
        version_name.to_string(),
        DownloadedVersion {
            channel: channel_name.to_string(),
            published_at: entry.published_at.clone(),
            platforms: updated_platforms,
        },
    );

    if activate || meta.active.is_none() {
        meta.active = Some(ActiveEngine {
            version: version_name.to_string(),
            channel: channel_name.to_string(),
        });
    }

    save_meta(&meta_path, &meta)?;
    Ok(meta)
}

/// Resolve target platforms for `update`: current native + `web-universal` + active version's platforms.
pub fn resolve_update_platforms(meta: &EngineMeta) -> Result<Vec<String>> {
    let mut set = BTreeSet::new();
    set.insert(detect_platform()?.to_string());
    set.insert("web-universal".to_string());
    if let Some(active) = &meta.active
        && let Some(dl) = meta.downloads.get(&active.version)
    {
        for p in dl.platforms.keys() {
            set.insert(p.clone());
        }
    }
    Ok(set.into_iter().collect())
}

pub fn switch_active_version(project_root: &Path, version: &str) -> Result<EngineMeta> {
    let meta_path = meta_file(project_root);
    let mut meta = load_meta(&meta_path).ok_or_else(|| {
        anyhow::anyhow!("No engine versions downloaded yet. Run \"moyu-devtool download\" first.")
    })?;

    let dl = meta
        .downloads
        .get(version)
        .ok_or_else(|| anyhow::anyhow!("Version \"{version}\" is not downloaded."))?;

    meta.active = Some(ActiveEngine {
        version: version.to_string(),
        channel: dl.channel.clone(),
    });
    save_meta(&meta_path, &meta)?;
    Ok(meta)
}
