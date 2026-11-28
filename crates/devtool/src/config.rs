//! Path conventions and persistent tool configuration for `moyu-devtool`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CDN_URL: &str = "https://cdn.momoyu.ink/releases/versions.json";
pub const DEFAULT_CHANNEL: &str = "stable";
pub const DEV_SERVER_PORT: u16 = 6020;
pub const DEFAULT_WEB_PORT: u16 = 6320;
pub const DEFAULT_ATTACH_PORT: u16 = 6321;

const MAX_RECENT_PROJECTS: usize = 10;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
    SolarizedLight,
    OneDark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalePreference {
    #[default]
    ZhCn,
    EnUs,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevtoolGlobalConfig {
    #[serde(default)]
    pub recent_projects: Vec<PathBuf>,
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub locale: LocalePreference,
    #[serde(default)]
    pub managed_node_version: Option<String>,
    #[serde(default)]
    pub preferred_editor: Option<String>,
}

impl DevtoolGlobalConfig {
    /// Insert `path` at the front of `recent_projects`, deduplicating and capping length.
    #[allow(dead_code)]
    pub fn record_recent_project(&mut self, path: PathBuf) {
        self.recent_projects.retain(|p| p != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }

    /// Remove `path` from `recent_projects`. Returns `true` if an entry was removed.
    #[allow(dead_code)]
    pub fn remove_recent_project(&mut self, path: &Path) -> bool {
        let before = self.recent_projects.len();
        self.recent_projects.retain(|p| p != path);
        self.recent_projects.len() != before
    }
}

/// Global user directory (`~/.moyu`).
pub fn user_moyu_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine user home directory"))?;
    Ok(home.join(".moyu"))
}

pub fn devtool_config_path() -> Result<PathBuf> {
    Ok(user_moyu_dir()?.join("devtool.json"))
}

#[allow(dead_code)]
pub fn load_global_config() -> DevtoolGlobalConfig {
    let Ok(path) = devtool_config_path() else {
        return DevtoolGlobalConfig::default();
    };
    if !path.is_file() {
        return DevtoolGlobalConfig::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn save_global_config(config: &DevtoolGlobalConfig) -> Result<()> {
    let path = devtool_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)? + "\n";
    fs::write(path, json)?;
    Ok(())
}

pub fn managed_node_root() -> Result<PathBuf> {
    Ok(user_moyu_dir()?.join("node"))
}

pub fn engine_dir(project_root: &Path) -> PathBuf {
    project_root.join(".moyu").join("engine")
}

pub fn meta_file(project_root: &Path) -> PathBuf {
    engine_dir(project_root).join("meta.json")
}

pub fn assert_safe_segment<'a>(value: &'a str, label: &str) -> Result<&'a str> {
    let valid = !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-'));
    if !valid {
        bail!(
            "Invalid {label} \"{value}\". Engine path segments may only contain letters, numbers, dot, underscore, plus, and hyphen."
        );
    }
    Ok(value)
}

pub fn version_dir(project_root: &Path, version: &str) -> Result<PathBuf> {
    Ok(engine_dir(project_root).join(assert_safe_segment(version, "version")?))
}

pub fn platform_dir(project_root: &Path, version: &str, platform: &str) -> Result<PathBuf> {
    Ok(version_dir(project_root, version)?.join(assert_safe_segment(platform, "platform")?))
}
