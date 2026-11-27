//! Project discovery, manifest loading, and multi-resolution asset preflight.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::model::project::{
    FrameworkSummary, MoyuConfigSummary, MultiResMode, MultiResReport, ProjectEvent,
    ProjectIndexSummary, VariantFile,
};
use crate::service::ServiceReporter;

const BASE_SCALE: f64 = 1.0;

/// Walk upward from `start_dir` (or current working directory) until `index.json` is found.
pub fn find_project_root(start_dir: Option<&Path>) -> Option<PathBuf> {
    let mut dir = match start_dir {
        Some(p) => fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()),
        None => std::env::current_dir().ok()?,
    };

    loop {
        if dir.join("index.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn require_project_root(start_dir: Option<&Path>) -> Result<PathBuf> {
    find_project_root(start_dir).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not locate a Moyu project (no index.json found).\nMake sure you are running this command inside a Moyu project directory."
        )
    })
}

#[allow(dead_code)]
pub fn load_index_summary(project_root: &Path) -> Result<ProjectIndexSummary> {
    let path = project_root.join("index.json");
    if !path.is_file() {
        bail!("index.json not found in {}", project_root.display());
    }
    let text = fs::read_to_string(&path)?;
    let parsed: ProjectIndexSummary = serde_json::from_str(&text).unwrap_or_default();
    Ok(parsed)
}

#[allow(dead_code)]
pub fn load_moyu_summary(project_root: &Path) -> Option<MoyuConfigSummary> {
    let text = fs::read_to_string(project_root.join("moyu.json")).ok()?;
    let raw: serde_json::Value = serde_json::from_str(&text).ok()?;
    let engine = raw.get("engine");
    let android = raw.get("android");
    Some(MoyuConfigSummary {
        engine_channel: engine
            .and_then(|e| e.get("channel"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        engine_cdn_url: engine
            .and_then(|e| e.get("cdnUrl"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        android_app_name: android
            .and_then(|a| a.get("appName"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        android_application_id: android
            .and_then(|a| a.get("applicationId"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        android_version_name: android
            .and_then(|a| a.get("versionName"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        android_version_code: android
            .and_then(|a| a.get("versionCode"))
            .and_then(|v| v.as_u64())
            .and_then(|n| u32::try_from(n).ok()),
    })
}

#[allow(dead_code)]
pub fn load_framework_summary(project_root: &Path) -> Option<FrameworkSummary> {
    let text = fs::read_to_string(project_root.join("framework.json")).ok()?;
    serde_json::from_str(&text).ok()
}

#[allow(dead_code)]
pub fn load_project_opened_event(project_root: &Path) -> Result<ProjectEvent> {
    let index = load_index_summary(project_root)?;
    let moyu = load_moyu_summary(project_root);
    let framework = load_framework_summary(project_root);
    Ok(ProjectEvent::Opened {
        root: project_root.to_path_buf(),
        index,
        moyu,
        framework,
    })
}

/// Mirror `crates/resource/src/variant.rs` and `packages/cli/src/utils/multi-res-assets.ts`.
pub fn check_multi_res_assets(project_root: &Path) -> Result<MultiResReport> {
    let assets_dir = project_root.join("assets");
    let variants = scan_variants(&assets_dir)?;
    if variants.is_empty() {
        return Ok(MultiResReport::default());
    }

    let (mode, scales) = read_multi_res_config(project_root);
    let mut report = MultiResReport {
        mode: Some(mode.clone()),
        ..Default::default()
    };

    match &mode {
        MultiResMode::Off => {
            report.all_unused_when_off = variants.len();
        }
        MultiResMode::Auto | MultiResMode::Forced(_) => {
            let used: BTreeSet<u64> = match &mode {
                MultiResMode::Auto => scales.iter().map(|s| s.to_bits()).collect(),
                MultiResMode::Forced(s) => [s.to_bits()].into_iter().collect(),
                MultiResMode::Off => unreachable!(),
            };

            let mut grouped: BTreeMap<u64, (f64, Vec<String>)> = BTreeMap::new();
            for v in &variants {
                if !used.contains(&v.scale.to_bits()) {
                    let entry = grouped
                        .entry(v.scale.to_bits())
                        .or_insert_with(|| (v.scale, Vec::new()));
                    entry.1.push(v.path.clone());
                }
            }
            report.unused_by_scale = grouped.into_values().collect();

            if matches!(mode, MultiResMode::Auto) {
                let present: BTreeSet<u64> = variants.iter().map(|v| v.scale.to_bits()).collect();
                for s in &scales {
                    if !present.contains(&s.to_bits()) {
                        report.undeclared_scales.push(*s);
                    }
                }
            }
        }
    }

    for v in &variants {
        if !assets_dir.join(&v.base).exists() {
            report.missing_base.push((v.path.clone(), v.base.clone()));
        }
    }

    Ok(report)
}

pub fn report_multi_res_warnings(report: &MultiResReport, reporter: &ServiceReporter) {
    if report.all_unused_when_off > 0 {
        let n = report.all_unused_when_off;
        reporter.warn(format!(
            "Found {n} variant file{}, but multiResAssets is \"off\", so only the base assets are used.\nSet \"multiResAssets\": \"auto\" in index.json to use them.",
            if n == 1 { "" } else { "s" }
        ));
    }
    for (scale, files) in &report.unused_by_scale {
        let hint = if (*scale - BASE_SCALE).abs() < f64::EPSILON {
            "the base asset already covers scale 1, so it is never used".to_string()
        } else if matches!(report.mode, Some(MultiResMode::Auto)) {
            "add it to multiResAssetScales in index.json to use them".to_string()
        } else if let Some(MultiResMode::Forced(m)) = report.mode {
            format!("multiResAssets forces scale {m}")
        } else {
            String::new()
        };
        let count = files.len();
        let example = files.first().map(String::as_str).unwrap_or("");
        reporter.warn(format!(
            "Scale {scale} is not tried by the engine ({count} file{}, e.g. {example}): {hint}",
            if count == 1 { "" } else { "s" }
        ));
    }
    if !report.undeclared_scales.is_empty() {
        let joined: Vec<String> = report
            .undeclared_scales
            .iter()
            .map(ToString::to_string)
            .collect();
        let pronoun = if joined.len() == 1 { "it" } else { "them" };
        reporter.info(format!(
            "multiResAssetScales declares {}, but no variant file uses {pronoun}. Every asset will make one extra failed read for each.",
            joined.join(", ")
        ));
    }
    if !report.missing_base.is_empty() {
        let n = report.missing_base.len();
        let pronoun = if n == 1 { "it" } else { "them" };
        let lines: Vec<String> = report
            .missing_base
            .iter()
            .map(|(p, b)| format!("  {p} (expected {b})"))
            .collect();
        reporter.warn(format!(
            "No base asset for {n} variant file{}, so nothing can reference {pronoun}:\n{}",
            if n == 1 { "" } else { "s" },
            lines.join("\n")
        ));
    }
}

fn scan_variants(assets_dir: &Path) -> Result<Vec<VariantFile>> {
    if !assets_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in walkdir::WalkDir::new(assets_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy();
        let Some(scale) = parse_variant_scale(&file_name) else {
            continue;
        };
        let rel = entry
            .path()
            .strip_prefix(assets_dir)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        let base_dir = rel
            .rfind('/')
            .map(|idx| &rel[..=idx])
            .unwrap_or("");
        let base = format!("{base_dir}{}", base_name(&file_name));
        found.push(VariantFile {
            path: rel,
            scale,
            base,
        });
    }
    Ok(found)
}

fn read_multi_res_config(project_root: &Path) -> (MultiResMode, Vec<f64>) {
    let index_path = project_root.join("index.json");
    let Ok(text) = fs::read_to_string(index_path) else {
        return (MultiResMode::Off, Vec::new());
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (MultiResMode::Off, Vec::new());
    };

    let mode = match val.get("multiResAssets") {
        Some(v) if v.is_number() => v
            .as_f64()
            .map(MultiResMode::Forced)
            .unwrap_or(MultiResMode::Off),
        Some(v) if v.as_str() == Some("auto") => MultiResMode::Auto,
        _ => MultiResMode::Off,
    };

    let scales: Vec<f64> = match val.get("multiResAssetScales").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_f64())
            .filter(|s| *s > 0.0 && (*s - BASE_SCALE).abs() > f64::EPSILON)
            .collect(),
        None => vec![1.5, 2.0],
    };

    (mode, scales)
}

fn parse_variant_scale(file_name: &str) -> Option<f64> {
    let dot = file_name.rfind('.');
    let stem = dot.map(|i| &file_name[..i]).unwrap_or(file_name);
    let at = stem.rfind('@')?;
    let suffix = &stem[at + 1..];
    let digits = suffix
        .strip_suffix('x')
        .or_else(|| suffix.strip_suffix('X'))?;
    if digits.is_empty() {
        return None;
    }
    // Accept `<digits>` or `<digits>.<digits>` matching the JS regex `/^\d+(\.\d+)?$/`.
    let mut parts = digits.split('.');
    let whole = parts.next()?;
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if let Some(frac) = parts.next() {
        if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) || parts.next().is_some() {
            return None;
        }
    }
    let scale: f64 = digits.parse().ok()?;
    (scale > 0.0).then_some(scale)
}

fn base_name(file_name: &str) -> String {
    let dot = file_name.rfind('.');
    let stem = dot.map(|i| &file_name[..i]).unwrap_or(file_name);
    let ext = dot.map(|i| &file_name[i..]).unwrap_or("");
    let at = stem.rfind('@').unwrap_or(stem.len());
    format!("{}{}", &stem[..at], ext)
}
