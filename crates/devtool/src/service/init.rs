//! Project creation service (`init`): downloads the latest framework template and installs dependencies.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::model::engine::EngineMeta;
use crate::model::tasks::{TaskKind, TaskStatus};
use crate::service::engine::{
    download_platforms, fetch_versions_json, load_config, resolve_update_platforms,
};
use crate::service::node::{detect_node, install_managed_node, run_project_install};
use crate::service::{ServiceReporter, copy_dir_all, download_to_file};

pub const FRAMEWORK_MAIN_ZIP_URL: &str =
    "https://github.com/DeepSpaceMill/framework/archive/refs/heads/main.zip";

#[derive(Debug, Clone, Default)]
pub struct CreateProjectResult {
    /// Set when managed Node.js LTS was automatically installed during project creation.
    #[allow(dead_code)]
    pub managed_node_version: Option<String>,
}

pub fn validate_project_name(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    let valid = !bytes.is_empty()
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        });
    if !valid {
        bail!("Invalid project name. Use lowercase letters, numbers, hyphens, and dots.");
    }
    Ok(())
}

/// Download the latest `main` archive from `DeepSpaceMill/framework`, extract it into `target_dir`,
/// ensure Node.js is available (installing managed Node LTS if needed), and install `node_modules`.
pub async fn create_project(
    name: &str,
    target_dir: &Path,
    reporter: &ServiceReporter,
) -> Result<CreateProjectResult> {
    reporter.started(TaskKind::CreateProject, format!("Create project {name}"));
    let res = create_project_inner(name, target_dir, reporter).await;
    match &res {
        Ok(_) => reporter.finished(TaskStatus::Completed),
        Err(_) if reporter.cancel.is_cancelled() => reporter.finished(TaskStatus::Cancelled),
        Err(e) => reporter.finished(TaskStatus::Failed(format!("{e:#}"))),
    }
    res
}

async fn create_project_inner(
    name: &str,
    target_dir: &Path,
    reporter: &ServiceReporter,
) -> Result<CreateProjectResult> {
    reporter.cancel.check()?;
    validate_project_name(name)?;
    if target_dir.exists() {
        bail!("Directory \"{}\" already exists.", target_dir.display());
    }

    let tmp_zip = std::env::temp_dir().join(format!("moyu-framework-{}.zip", uuid::Uuid::new_v4()));

    reporter.stage("Downloading latest Moyu framework...");
    reporter.info(format!("Downloading {FRAMEWORK_MAIN_ZIP_URL} ..."));
    let dl_res = download_to_file(FRAMEWORK_MAIN_ZIP_URL, &tmp_zip, reporter).await;
    if let Err(err) = dl_res {
        let _ = fs::remove_file(&tmp_zip);
        return Err(err);
    }

    reporter.cancel.check()?;
    reporter.stage("Extracting framework archive...");

    let archive_path = tmp_zip.clone();
    let dest = target_dir.to_path_buf();
    let project_name = name.to_string();

    let extract_res = tokio::task::spawn_blocking(move || -> Result<()> {
        let tmp_extract =
            std::env::temp_dir().join(format!("moyu-framework-extract-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp_extract)?;

        let inner_res = (|| -> Result<()> {
            let file = fs::File::open(&archive_path)
                .with_context(|| format!("Failed to open {}", archive_path.display()))?;
            let mut zip = zip::ZipArchive::new(file).context("Invalid framework ZIP archive")?;
            zip.extract(&tmp_extract)
                .context("Failed to extract framework ZIP archive")?;

            let source_root =
                find_single_root_dir(&tmp_extract).unwrap_or_else(|| tmp_extract.clone());
            copy_dir_all(&source_root, &dest)?;
            patch_project_metadata(&dest, &project_name);
            Ok(())
        })();

        let _ = fs::remove_dir_all(&tmp_extract);
        if inner_res.is_err() {
            let _ = fs::remove_dir_all(&dest);
        }
        inner_res
    })
    .await?;

    let _ = fs::remove_file(&tmp_zip);
    extract_res?;

    reporter.cancel.check()?;

    // Ensure Node.js is available; automatically install managed Node LTS if missing.
    let mut managed_node_version = None;
    let has_node = tokio::task::spawn_blocking(|| detect_node().is_some()).await?;
    if !has_node {
        reporter.stage("Node.js not found; installing managed Node.js LTS...");
        reporter.info("Node.js was not found. Installing managed Node.js LTS automatically...");
        let ver = install_managed_node(&reporter.child()).await?;
        managed_node_version = Some(ver);
    }

    reporter.cancel.check()?;
    reporter.stage("Installing project dependencies...");
    reporter.info("Running package manager install...");
    run_project_install(target_dir, &reporter.child()).await?;

    reporter.cancel.check()?;
    let engine_cfg = load_config(target_dir);
    reporter.stage(format!(
        "Fetching latest engine ({})...",
        engine_cfg.channel
    ));
    reporter.info(format!(
        "Fetching engine manifest from {} ...",
        engine_cfg.cdn_url
    ));
    let manifest = fetch_versions_json(&engine_cfg.cdn_url).await?;
    let latest_ver = manifest
        .channels
        .get(&engine_cfg.channel)
        .ok_or_else(|| anyhow::anyhow!("Engine channel \"{}\" not found.", engine_cfg.channel))?
        .latest
        .clone();
    let platforms = resolve_update_platforms(&EngineMeta::default())?;
    reporter.stage(format!("Downloading latest engine {latest_ver}..."));
    download_platforms(
        target_dir,
        &engine_cfg.channel,
        &latest_ver,
        &manifest,
        &platforms,
        true,
        &reporter.child(),
    )
    .await?;

    reporter.success(format!(
        "Created project \"{name}\" at {}",
        target_dir.display()
    ));
    Ok(CreateProjectResult {
        managed_node_version,
    })
}

/// If `dir` contains a single top-level directory (as GitHub archive ZIPs do), return it.
fn find_single_root_dir(dir: &Path) -> Option<PathBuf> {
    let entries: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .collect();
    if entries.len() == 1 && entries[0].is_dir() {
        Some(entries[0].clone())
    } else {
        None
    }
}

/// Update `package.json` name and `index.json` windowTitle when present, preserving all other fields.
fn patch_project_metadata(project_root: &Path, name: &str) {
    let pkg_path = project_root.join("package.json");
    if let Ok(text) = fs::read_to_string(&pkg_path)
        && let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(obj) = val.as_object_mut()
    {
        obj.insert(
            "name".to_string(),
            serde_json::Value::String(name.to_string()),
        );
        if let Ok(pretty) = serde_json::to_string_pretty(&val) {
            let _ = fs::write(&pkg_path, format!("{pretty}\n"));
        }
    }

    let index_path = project_root.join("index.json");
    if let Ok(text) = fs::read_to_string(&index_path)
        && let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(obj) = val.as_object_mut()
    {
        obj.insert(
            "windowTitle".to_string(),
            serde_json::Value::String(name.to_string()),
        );
        if let Ok(pretty) = serde_json::to_string_pretty(&val) {
            let _ = fs::write(&index_path, format!("{pretty}\n"));
        }
    }
}
