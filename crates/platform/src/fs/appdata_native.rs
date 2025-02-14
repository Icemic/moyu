use anyhow::Result;

use crate::config::get_engine_config;

use super::{get_path_in_appdata, FileEntry};

/// Read a file from the appdata directory.
/// On Windows, the appdata directory is the `%APPDATA%/<app_name>` directory.
/// On Linux, the appdata directory is the `~/.local/share/<app_name>` directory.
/// On macOS, the appdata directory is the `~/Library/Application Support/<app_name>` directory.
/// On Android, the appdata directory is the `/Android/data/<package_name>/files` directory.
///
pub async fn read_from_appdata(relative_path: &str) -> Result<Vec<u8>> {
    let path = get_path_in_appdata(relative_path)?;

    match tokio::fs::read(&path).await {
        Ok(data) => Ok(data),
        Err(err) => Err(anyhow::anyhow!("Error reading from appdata: {:?}", err)),
    }
}

/// Write a file to the appdata directory.
/// See [`read_from_appdata`](crate::fs::read_from_appdata) for more information.
pub async fn write_to_appdata(relative_path: &str, data: Vec<u8>) -> Result<()> {
    let appdata_dir = get_engine_config()
        .appdata_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get appdata directory"))?;

    let path = appdata_dir.join(path_clean::clean(relative_path));

    tokio::fs::write(&path, &data)
        .await
        .map_err(|err| anyhow::anyhow!("Failed to write appdata {}: {}", path.display(), err))?;

    Ok(())
}

/// Read a directory from the appdata directory.
pub async fn readdir_from_appdata(relative_path: &str) -> Result<Vec<FileEntry>> {
    let path = get_path_in_appdata(relative_path)?;

    let mut entries = tokio::fs::read_dir(&path)
        .await
        .map_err(|err| anyhow::anyhow!("Failed to read directory {}: {}", path.display(), err))?;

    let mut arr = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let last_modified = metadata.modified()?;
        let last_modified = last_modified
            .duration_since(std::time::SystemTime::UNIX_EPOCH)?
            .as_millis() as u64;

        arr.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            last_modified,
        });
    }

    Ok(arr)
}
