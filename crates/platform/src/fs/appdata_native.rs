use anyhow::Result;

use super::{FileEntry, get_path_in_appdata};

/// Read a file from the appdata directory.
/// On Windows, the appdata directory is the `%APPDATA%/<app_name>` directory.
/// On Linux, the appdata directory is the `~/.local/share/<app_name>` directory.
/// On macOS, the appdata directory is the `~/Library/Application Support/<app_name>` directory.
/// On Android, the appdata directory is the `/Android/data/<package_name>/files` directory.
///
pub async fn read_from_appdata(relative_path: &str) -> Result<Option<Vec<u8>>> {
    let path = get_path_in_appdata(relative_path)?;

    // create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|err| {
            anyhow::anyhow!("Failed to create directory {}: {}", parent.display(), err)
        })?;
    }

    match tokio::fs::read(&path).await {
        Ok(data) => Ok(Some(data)),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(anyhow::anyhow!(
                    "Failed to read appdata {}: {}",
                    path.display(),
                    err
                ))
            }
        }
    }
}

/// Write a file to the appdata directory.
/// See [`read_from_appdata`](crate::fs::read_from_appdata) for more information.
pub async fn write_to_appdata(relative_path: &str, data: Vec<u8>) -> Result<()> {
    let path = get_path_in_appdata(relative_path)?;

    // create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|err| {
            anyhow::anyhow!("Failed to create directory {}: {}", parent.display(), err)
        })?;
    }

    tokio::fs::write(&path, &data)
        .await
        .map_err(|err| anyhow::anyhow!("Failed to write appdata {}: {}", path.display(), err))?;

    Ok(())
}

/// Read a directory from the appdata directory.
pub async fn readdir_from_appdata(
    relative_path: &str,
    pattern: Option<String>,
) -> Result<Vec<FileEntry>> {
    let path = get_path_in_appdata(relative_path)?;

    // create the directory if it doesn't exist
    tokio::fs::create_dir_all(&path)
        .await
        .map_err(|err| anyhow::anyhow!("Failed to create directory {}: {}", path.display(), err))?;

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

        let name = entry.file_name().to_string_lossy().to_string();

        if let Some(pattern) = pattern.as_ref() {
            if !fast_glob::glob_match(pattern, &name) {
                continue; // Skip entries that do not match the pattern
            }
        }

        arr.push(FileEntry {
            name,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            last_modified,
        });
    }

    Ok(arr)
}

/// Remove a file or directory from the appdata directory.
pub async fn remove_from_appdata(relative_path: &str) -> Result<()> {
    let path = get_path_in_appdata(relative_path)?;

    if path.is_dir() {
        tokio::fs::remove_dir_all(&path)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to remove {}: {}", path.display(), err))?;
    } else {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to remove {}: {}", path.display(), err))?;
    }

    Ok(())
}
