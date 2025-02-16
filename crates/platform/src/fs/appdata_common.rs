use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::get_engine_config;

pub fn get_path_in_appdata(relative_path: &str) -> Result<PathBuf> {
    let appdata_dir = get_engine_config().appdata_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to get appdata directory for app '{}'",
            get_engine_config().app_name
        )
    })?;

    let path = appdata_dir.join(path_clean::clean(relative_path));

    Ok(path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub last_modified: u64,
}
