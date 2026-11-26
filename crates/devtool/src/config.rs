//! Path conventions and persistent tool configuration for `moyu-devtool`.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CDN_URL: &str = "https://cdn.momoyu.ink/releases/versions.json";
pub const DEFAULT_CHANNEL: &str = "stable";
pub const DEV_SERVER_PORT: u16 = 6020;
pub const DEFAULT_WEB_PORT: u16 = 6320;
pub const DEFAULT_ATTACH_PORT: u16 = 6321;

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevtoolGlobalConfig {
    #[serde(default)]
    pub recent_projects: Vec<PathBuf>,
    #[serde(default)]
    pub managed_node_version: Option<String>,
}

/// Global user directory (`~/.moyu`).
pub fn user_moyu_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine user home directory"))?;
    Ok(home.join(".moyu"))
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
