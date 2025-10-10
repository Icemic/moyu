use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::dir::appdata_dir;

pub fn get_path_in_appdata(relative_path: &str) -> Result<PathBuf> {
    let path = appdata_dir().join(path_clean::clean(relative_path));

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
