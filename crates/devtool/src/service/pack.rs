//! Game and framework distribution packager (`pack`).

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;

use crate::config::{meta_file, platform_dir};
use crate::service::android::{
    ANDROID_FORMATS, ANDROID_PLATFORM, PackAndroidOptions, is_android_format, pack_android,
};
use crate::service::engine::{load_meta, native_executable_name};
use crate::service::node::{generate_json_schema, run_project_build};
use crate::service::project::{check_multi_res_assets, print_multi_res_warnings};
use crate::service::video_decoder::{ensure_video_decoder_library, supports_video_decoder};
use crate::service::{copy_dir_all, format_bytes};

const WEB_PLATFORM: &str = "web-universal";
const FRAMEWORK_ARCHIVE_NAME: &str = "framework.zip";
const GAME_ARCHIVE_NAME: &str = "game.zip";

#[derive(Debug, Clone)]
pub struct PackOptions {
    pub target: Option<String>,
    pub framework: bool,
    pub compress: bool,
    pub output: Option<PathBuf>,
    pub android_format: Option<String>,
    pub include_video_decoder: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AssetPatternConfig {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameworkConfig {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    author: Option<String>,
    email: Option<String>,
    version: Option<String>,
    assets: Option<AssetPatternConfig>,
    constructible: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameworkMeta {
    name: String,
    title: String,
    description: String,
    author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    constructible: Option<bool>,
}

pub async fn pack_project(project_root: &Path, options: PackOptions) -> Result<()> {
    let report = check_multi_res_assets(project_root)?;
    print_multi_res_warnings(&report);

    let framework_mode = options.framework;
    let compress = if framework_mode { true } else { options.compress };
    let tmp_pack_dir = project_root.join(".moyu").join("tmp-pack");
    let archive_name = if framework_mode {
        FRAMEWORK_ARCHIVE_NAME
    } else {
        GAME_ARCHIVE_NAME
    };

    let date_string = Utc::now().format("%Y%m%d%H%M%S%3f").to_string();
    let output_dir = match &options.output {
        Some(out) => project_root.join(out),
        None => project_root
            .join(".moyu")
            .join("release")
            .join(&date_string),
    };

    let mut active_version = None;
    let mut target_path = None;
    let mut is_web = false;
    let mut is_android = false;

    if !framework_mode {
        let meta = load_meta(&meta_file(project_root));
        let active = meta.and_then(|m| m.active).ok_or_else(|| {
            anyhow::anyhow!("No active engine version. Run \"moyu-devtool download\" first.")
        })?;
        let target = options.target.as_deref().ok_or_else(|| {
            anyhow::anyhow!("Target platform is required unless --framework is used.")
        })?;

        is_web = target == WEB_PLATFORM;
        is_android = target == ANDROID_PLATFORM;

        if !is_web && !is_android && native_executable_name(target).is_none() {
            bail!(
                "Unsupported target: \"{target}\".\nSupported targets: windows-amd64, linux-amd64, linux-aarch64, macos-amd64, macos-aarch64, {WEB_PLATFORM}, {ANDROID_PLATFORM}"
            );
        }
        if is_android && compress {
            bail!("--compress is not supported for Android targets.");
        }
        if is_android && !is_android_format(options.android_format.as_deref()) {
            bail!(
                "Invalid --android-format. Supported values: {}",
                ANDROID_FORMATS.join(", ")
            );
        }

        let dir = platform_dir(project_root, &active.version, target)?;
        if !dir.exists() {
            bail!(
                "Platform \"{target}\" not downloaded for version {}.\nRun \"moyu-devtool download\" to download it.",
                active.version
            );
        }

        active_version = Some(active.version);
        target_path = Some(dir);
    }

    if framework_mode {
        eprintln!("info: Mode: framework");
    } else {
        eprintln!("info: Target: {}", options.target.as_deref().unwrap_or(""));
        eprintln!(
            "info: Engine version: {}",
            active_version.as_deref().unwrap_or("")
        );
    }
    eprintln!("info: Compress: {compress}");
    if is_android {
        eprintln!(
            "info: Android format: {}",
            options.android_format.as_deref().unwrap_or("")
        );
    }
    if options.include_video_decoder {
        eprintln!("info: Video playback plugin: included");
    }
    eprintln!("info: Output: {}", output_dir.display());

    eprintln!("info: Building project...");
    run_project_build(project_root).await?;
    eprintln!("success: Build complete.");

    if tmp_pack_dir.exists() {
        eprintln!("info: Cleaning existing tmp-pack directory...");
        fs::remove_dir_all(&tmp_pack_dir)?;
    }
    fs::create_dir_all(&tmp_pack_dir)?;

    let res = async {
        if framework_mode {
            let fw_cfg = load_framework_config(project_root)?;
            copy_framework_assets(project_root, &tmp_pack_dir, fw_cfg.assets.as_ref())?;
            copy_index_json(project_root, &tmp_pack_dir)?;
            copy_bundle_js(project_root, &tmp_pack_dir)?;
            regenerate_schemas(project_root).await?;
            copy_commands_schema(project_root, &tmp_pack_dir)?;
            copy_ui_schema(project_root, &tmp_pack_dir)?;
            copy_index_html(project_root, &tmp_pack_dir)?;
            copy_index_preview_html(project_root, &tmp_pack_dir)?;
            let fw_meta = build_framework_meta(&fw_cfg)?;
            if fw_meta.constructible == Some(true) {
                copy_construct_files(project_root, &tmp_pack_dir)?;
            }
            write_framework_meta(&tmp_pack_dir, &fw_meta)?;
        } else {
            copy_assets(project_root, &tmp_pack_dir)?;
            copy_index_json(project_root, &tmp_pack_dir)?;
            copy_bundle_js(project_root, &tmp_pack_dir)?;

            let target = options.target.as_deref().unwrap();
            let engine_dir = target_path.as_deref().unwrap();
            let video_lib = if options.include_video_decoder && supports_video_decoder(target) {
                Some(ensure_video_decoder_library(project_root, target).await?)
            } else {
                None
            };

            if is_android {
                pack_android(PackAndroidOptions {
                    project_root,
                    engine_dir,
                    runtime_package_dir: &tmp_pack_dir,
                    output_dir: &output_dir,
                    format: options.android_format.as_deref().unwrap_or(""),
                    video_decoder_library: video_lib.as_deref(),
                })
                .await?;
            } else if is_web {
                copy_web_engine(project_root, engine_dir, &tmp_pack_dir)?;
            } else {
                copy_native_engine(target, engine_dir, &tmp_pack_dir)?;
                if let Some(lib) = &video_lib
                    && let Some(name) = lib.file_name()
                {
                    fs::copy(lib, tmp_pack_dir.join(name))?;
                }
            }
        }

        if !is_android {
            if compress {
                create_zip(&tmp_pack_dir, &output_dir, archive_name).await?;
            } else {
                let game_path = output_dir.join("game");
                eprintln!("info: Copying files to: {}", game_path.display());
                copy_dir_all(&tmp_pack_dir, &game_path)?;
            }
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;

    eprintln!("info: Cleaning up tmp-pack...");
    let _ = fs::remove_dir_all(&tmp_pack_dir);
    res?;
    eprintln!("success: Pack complete!");
    Ok(())
}

fn copy_bundle_js(project_root: &Path, tmp_pack_dir: &Path) -> Result<()> {
    let mut entry_filename = "index.js".to_string();
    let index_json = project_root.join("index.json");
    if index_json.is_file()
        && let Ok(text) = fs::read_to_string(&index_json)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(f) = val.get("entryFilename").and_then(|v| v.as_str())
        && !f.is_empty()
    {
        entry_filename = f
            .trim_start_matches("./")
            .trim_start_matches(".\\")
            .to_string();
    }

    let src_path = project_root.join("dist").join(&entry_filename);
    if !src_path.is_file() {
        bail!("Bundle not found: {}", src_path.display());
    }
    let name = Path::new(&entry_filename)
        .file_name()
        .context("Invalid entryFilename")?;
    eprintln!("info: Copying bundle: {}", name.to_string_lossy());
    fs::copy(&src_path, tmp_pack_dir.join(name))?;
    Ok(())
}

fn copy_assets(project_root: &Path, tmp_pack_dir: &Path) -> Result<()> {
    let assets_dir = project_root.join("assets");
    if !assets_dir.is_dir() {
        bail!("Assets directory not found.");
    }
    eprintln!("info: Copying assets...");
    copy_dir_all(&assets_dir, &tmp_pack_dir.join("assets"))?;
    Ok(())
}

fn copy_framework_assets(
    project_root: &Path,
    tmp_pack_dir: &Path,
    patterns: Option<&AssetPatternConfig>,
) -> Result<()> {
    let Some(patterns) = patterns else {
        return copy_assets(project_root, tmp_pack_dir);
    };

    let list: Vec<&str> = match patterns {
        AssetPatternConfig::Single(s) => vec![s.as_str()],
        AssetPatternConfig::Multiple(v) => v.iter().map(String::as_str).collect(),
    };
    let assets_root = project_root.join("assets");
    eprintln!("info: Copying assets...");

    for pattern in list {
        let has_glob = pattern.contains(['*', '?', '[', ']', '{', '}']);
        if !has_glob {
            let src = assets_root.join(pattern);
            if !src.exists() {
                eprintln!("warn: Asset path not found: {pattern}");
                continue;
            }
            let dst = tmp_pack_dir.join("assets").join(pattern);
            if src.is_dir() {
                copy_dir_all(&src, &dst)?;
            } else {
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(src, dst)?;
            }
            continue;
        }

        // Walk assets and match each relative POSIX path using `fast_glob::glob_match`.
        if assets_root.is_dir() {
            for entry in walkdir::WalkDir::new(&assets_root) {
                let entry = entry?;
                if entry.file_type().is_dir() {
                    continue;
                }
                let rel = entry
                    .path()
                    .strip_prefix(&assets_root)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                if fast_glob::glob_match(pattern, &rel) {
                    let dst = tmp_pack_dir
                        .join("assets")
                        .join(entry.path().strip_prefix(&assets_root)?);
                    if let Some(parent) = dst.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(entry.path(), dst)?;
                }
            }
        }
    }
    Ok(())
}

fn copy_index_json(project_root: &Path, tmp_pack_dir: &Path) -> Result<()> {
    let src = project_root.join("index.json");
    if !src.is_file() {
        bail!("index.json not found.");
    }
    eprintln!("info: Copying index.json...");
    fs::copy(src, tmp_pack_dir.join("index.json"))?;
    Ok(())
}

fn copy_index_html(project_root: &Path, tmp_pack_dir: &Path) -> Result<()> {
    let src = project_root.join("index.html");
    if !src.is_file() {
        bail!("index.html not found in project root.");
    }
    eprintln!("info: Copying index.html...");
    fs::copy(src, tmp_pack_dir.join("index.html"))?;
    Ok(())
}

fn copy_index_preview_html(project_root: &Path, tmp_pack_dir: &Path) -> Result<()> {
    let src = project_root.join("index-preview.html");
    if src.is_file() {
        eprintln!("info: Copying index-preview.html...");
        fs::copy(src, tmp_pack_dir.join("index-preview.html"))?;
    }
    Ok(())
}

async fn regenerate_schemas(project_root: &Path) -> Result<()> {
    let commands_input = project_root.join("src/commands/commands.ts");
    if commands_input.is_file() {
        generate_json_schema(
            project_root,
            &commands_input,
            &project_root.join("commands.schema.json"),
            "ScenarioCommandSchema",
        )
        .await?;
    } else {
        eprintln!("warn: src/commands/commands.ts not found, skipping commands schema generation.");
    }

    let ui_input = project_root.join("src/data/ui.ts");
    if ui_input.is_file() {
        generate_json_schema(
            project_root,
            &ui_input,
            &project_root.join("ui.schema.json"),
            "GameUiSchema",
        )
        .await?;
    }
    Ok(())
}

fn copy_commands_schema(project_root: &Path, tmp_pack_dir: &Path) -> Result<()> {
    let src = project_root.join("commands.schema.json");
    if !src.is_file() {
        bail!("commands.schema.json not found. Run \"moyu-devtool schema\" to generate it first.");
    }
    eprintln!("info: Copying commands.schema.json...");
    fs::copy(src, tmp_pack_dir.join("commands.schema.json"))?;
    Ok(())
}

fn copy_ui_schema(project_root: &Path, tmp_pack_dir: &Path) -> Result<()> {
    let src = project_root.join("ui.schema.json");
    if src.is_file() {
        eprintln!("info: Copying ui.schema.json...");
        fs::copy(src, tmp_pack_dir.join("ui.schema.json"))?;
    }
    Ok(())
}

fn copy_native_engine(target: &str, target_path: &Path, tmp_pack_dir: &Path) -> Result<()> {
    let exe_name = native_executable_name(target)
        .ok_or_else(|| anyhow::anyhow!("Unsupported native target: {target}"))?;
    let exe_path = target_path.join(exe_name);
    if !exe_path.is_file() {
        bail!(
            "Engine executable not found: {}\nRun \"moyu-devtool download\" to download engine files.",
            exe_path.display()
        );
    }
    eprintln!("info: Copying native engine executable: {exe_name}");
    fs::copy(exe_path, tmp_pack_dir.join(exe_name))?;
    Ok(())
}

fn copy_web_engine(project_root: &Path, web_path: &Path, tmp_pack_dir: &Path) -> Result<()> {
    if !web_path.is_dir() {
        bail!("Web engine directory not found. Run \"moyu-devtool download\" to download web-universal platform.");
    }
    eprintln!("info: Copying web engine files...");
    copy_dir_all(web_path, tmp_pack_dir)?;
    copy_index_html(project_root, tmp_pack_dir)?;
    Ok(())
}

fn copy_construct_files(project_root: &Path, tmp_pack_dir: &Path) -> Result<()> {
    for filename in ["builtins.json", "events.json"] {
        let src = project_root.join(filename);
        if !src.is_file() {
            bail!("Construct file not found: {filename}");
        }
        eprintln!("info: Copying Construct file: {filename}");
        fs::copy(src, tmp_pack_dir.join(filename))?;
    }
    Ok(())
}

fn load_framework_config(project_root: &Path) -> Result<FrameworkConfig> {
    let path = project_root.join("framework.json");
    if !path.is_file() {
        bail!("framework.json not found in project root. This file is required for --framework mode.");
    }
    let text = fs::read_to_string(&path)?;
    serde_json::from_str(&text).context("Failed to parse framework.json.")
}

fn build_framework_meta(config: &FrameworkConfig) -> Result<FrameworkMeta> {
    let require_non_empty = |val: Option<&String>, field: &str| -> Result<String> {
        let s = val.map(|s| s.trim()).unwrap_or("");
        if s.is_empty() {
            bail!("Missing required \"{field}\" field in framework.json.");
        }
        Ok(s.to_string())
    };

    Ok(FrameworkMeta {
        name: require_non_empty(config.name.as_ref(), "name")?,
        title: require_non_empty(config.title.as_ref(), "title")?,
        description: require_non_empty(config.description.as_ref(), "description")?,
        author: require_non_empty(config.author.as_ref(), "author")?,
        email: config
            .email
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string),
        version: require_non_empty(config.version.as_ref(), "version")?,
        constructible: config.constructible,
    })
}

fn write_framework_meta(tmp_pack_dir: &Path, meta: &FrameworkMeta) -> Result<()> {
    eprintln!("info: Writing framework meta.json...");
    fs::write(
        tmp_pack_dir.join("meta.json"),
        serde_json::to_string_pretty(meta)? + "\n",
    )?;
    Ok(())
}

async fn create_zip(tmp_pack_dir: &Path, output_dir: &Path, archive_name: &str) -> Result<()> {
    let src_dir = tmp_pack_dir.to_path_buf();
    let out_dir = output_dir.to_path_buf();
    let archive = archive_name.to_string();

    tokio::task::spawn_blocking(move || -> Result<()> {
        fs::create_dir_all(&out_dir)?;
        let zip_path = out_dir.join(&archive);
        eprintln!("info: Creating zip archive: {}", zip_path.display());

        let file = File::create(&zip_path)?;
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(6));

        for entry in walkdir::WalkDir::new(&src_dir) {
            let entry = entry?;
            let rel = entry
                .path()
                .strip_prefix(&src_dir)?
                .to_string_lossy()
                .replace('\\', "/");
            if rel.is_empty() {
                continue;
            }
            if entry.file_type().is_dir() {
                writer.add_directory(format!("{rel}/"), options)?;
            } else {
                writer.start_file(&rel, options)?;
                let mut f = File::open(entry.path())?;
                std::io::copy(&mut f, &mut writer)?;
            }
        }
        writer.finish()?;

        let size = fs::metadata(&zip_path)?.len();
        eprintln!(
            "success: Archive created: {} ({})",
            zip_path.display(),
            format_bytes(size)
        );
        Ok(())
    })
    .await?
}
