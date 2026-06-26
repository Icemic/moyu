use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use wasm_bindgen_futures::JsFuture;
use web_sys::FileSystemRemoveOptions;
use web_sys::js_sys::{Function, Promise, Reflect};
use web_sys::wasm_bindgen::JsValue;

use super::{FileEntry, get_path_in_appdata};

const LOCAL_STORAGE_PREFIX: &str = "moyu:appdata:";

fn opfs_root_promise() -> Result<Promise> {
    use web_sys::wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("Failed to get window object"))?;
    let navigator = window.navigator();
    let storage = Reflect::get(navigator.as_ref(), &JsValue::from_str("storage"))
        .map_err(|err| anyhow::anyhow!("Failed to get navigator.storage: {:?}", err))?;
    let get_directory = Reflect::get(&storage, &JsValue::from_str("getDirectory"))
        .map_err(|err| anyhow::anyhow!("Failed to get storage.getDirectory: {:?}", err))?;

    if !get_directory.is_function() {
        return Err(anyhow::anyhow!("OPFS is not supported"));
    }

    let get_directory = get_directory.dyn_into::<Function>().map_err(|err| {
        anyhow::anyhow!("Failed to cast storage.getDirectory to Function: {:?}", err)
    })?;
    let promise = get_directory
        .call0(&storage)
        .map_err(|err| anyhow::anyhow!("Failed to call storage.getDirectory: {:?}", err))?
        .dyn_into::<Promise>()
        .map_err(|err| {
            anyhow::anyhow!("storage.getDirectory did not return a Promise: {:?}", err)
        })?;

    Ok(promise)
}

fn opfs_supported() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };

    let navigator = window.navigator();
    let Ok(storage) = Reflect::get(navigator.as_ref(), &JsValue::from_str("storage")) else {
        return false;
    };

    if storage.is_undefined() || storage.is_null() {
        return false;
    }

    Reflect::get(&storage, &JsValue::from_str("getDirectory"))
        .map(|get_directory| get_directory.is_function())
        .unwrap_or(false)
}

fn local_storage() -> Result<web_sys::Storage> {
    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("Failed to get window object"))?;
    window
        .local_storage()
        .map_err(|err| anyhow::anyhow!("Failed to access localStorage: {:?}", err))?
        .ok_or_else(|| anyhow::anyhow!("localStorage is not available"))
}

fn local_storage_key(path: &Path) -> String {
    format!(
        "{}{}",
        LOCAL_STORAGE_PREFIX,
        path.to_string_lossy().replace('\\', "/")
    )
}

fn local_storage_file_key(relative_path: &str) -> Result<String> {
    Ok(local_storage_key(&get_path_in_appdata(relative_path)?))
}

fn read_from_local_storage(relative_path: &str) -> Result<Option<Vec<u8>>> {
    let key = local_storage_file_key(relative_path)?;
    let storage = match local_storage() {
        Ok(storage) => storage,
        Err(err) => {
            log::warn!("Failed to access localStorage appdata fallback: {}", err);
            return Ok(None);
        }
    };

    let Some(data) = storage
        .get_item(&key)
        .map_err(|err| anyhow::anyhow!("Failed to read localStorage item: {:?}", err))?
    else {
        return Ok(None);
    };

    Ok(Some(BASE64_STANDARD.decode(data)?))
}

fn write_to_local_storage(relative_path: &str, data: Vec<u8>) -> Result<()> {
    let key = local_storage_file_key(relative_path)?;
    let storage = local_storage()?;
    storage
        .set_item(&key, &BASE64_STANDARD.encode(data))
        .map_err(|err| anyhow::anyhow!("Failed to write localStorage item: {:?}", err))
}

fn readdir_from_local_storage(
    relative_path: &str,
    pattern: Option<String>,
) -> Result<Vec<FileEntry>> {
    let path = get_path_in_appdata(relative_path)?;
    let prefix = format!("{}/", local_storage_key(&path).trim_end_matches('/'));
    let storage = match local_storage() {
        Ok(storage) => storage,
        Err(err) => {
            log::warn!("Failed to access localStorage appdata fallback: {}", err);
            return Ok(Vec::new());
        }
    };

    let mut entries = HashMap::<String, FileEntry>::new();
    let len = storage
        .length()
        .map_err(|err| anyhow::anyhow!("Failed to get localStorage length: {:?}", err))?;

    for index in 0..len {
        let Some(key) = storage
            .key(index)
            .map_err(|err| anyhow::anyhow!("Failed to get localStorage key: {:?}", err))?
        else {
            continue;
        };

        if !key.starts_with(&prefix) {
            continue;
        }

        let rest = &key[prefix.len()..];
        if rest.is_empty() {
            continue;
        }

        let (name, is_dir) = match rest.split_once('/') {
            Some((name, _)) => (name.to_string(), true),
            None => (rest.to_string(), false),
        };

        if let Some(pattern) = pattern.as_ref()
            && !fast_glob::glob_match(pattern, &name)
        {
            continue;
        }

        if is_dir {
            entries.entry(name.clone()).or_insert(FileEntry {
                name,
                is_dir: true,
                size: 0,
                last_modified: 0,
            });
            continue;
        }

        let size = storage
            .get_item(&key)
            .map_err(|err| anyhow::anyhow!("Failed to read localStorage item: {:?}", err))?
            .and_then(|data| BASE64_STANDARD.decode(data).ok())
            .map(|data| data.len() as u64)
            .unwrap_or(0);

        entries.insert(
            name.clone(),
            FileEntry {
                name,
                is_dir: false,
                size,
                last_modified: 0,
            },
        );
    }

    let mut entries = entries.into_values().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn remove_from_local_storage(relative_path: &str) -> Result<()> {
    let key = local_storage_file_key(relative_path)?;
    let prefix = format!("{}/", key.trim_end_matches('/'));
    let storage = local_storage()?;
    let mut keys = vec![key];
    let len = storage
        .length()
        .map_err(|err| anyhow::anyhow!("Failed to get localStorage length: {:?}", err))?;

    for index in 0..len {
        if let Some(key) = storage
            .key(index)
            .map_err(|err| anyhow::anyhow!("Failed to get localStorage key: {:?}", err))?
            && key.starts_with(&prefix)
        {
            keys.push(key);
        }
    }

    for key in keys {
        storage
            .remove_item(&key)
            .map_err(|err| anyhow::anyhow!("Failed to remove localStorage item: {:?}", err))?;
    }

    Ok(())
}

async fn get_dir_from_appdata(
    clean_path: Option<&std::path::Path>,
    create: bool,
) -> Result<web_sys::FileSystemDirectoryHandle> {
    use std::path::Component;

    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_futures::wasm_bindgen::JsCast;
    use web_sys::{FileSystemDirectoryHandle, FileSystemGetDirectoryOptions};

    let opfs_root = JsFuture::from(opfs_root_promise()?)
        .await
        .map_err(|err| anyhow::anyhow!("Failed to get storage directory: {:?}", err))?
        .dyn_into::<web_sys::FileSystemDirectoryHandle>()
        .map_err(|err| anyhow::anyhow!("Failed to cast to FileSystemDirectoryHandle: {:?}", err))?;

    if let Some(clean_path) = clean_path {
        let mut current_dir = opfs_root;
        let options = FileSystemGetDirectoryOptions::new();
        options.set_create(create);

        for component in clean_path.components() {
            let Component::Normal(component) = component else {
                return Err(anyhow::anyhow!("Invalid path component"));
            };

            let component = component.to_string_lossy();

            current_dir =
                JsFuture::from(current_dir.get_directory_handle_with_options(&component, &options))
                    .await
                    .map_err(|err| anyhow::anyhow!("Failed to get directory handle: {:?}", err))?
                    .dyn_into::<FileSystemDirectoryHandle>()
                    .map_err(|err| {
                        anyhow::anyhow!("Failed to cast to FileSystemDirectoryHandle: {:?}", err)
                    })?;
        }

        Ok(current_dir)
    } else {
        Ok(opfs_root)
    }
}

async fn get_file_from_appdata(
    clean_path: &PathBuf,
    create_parent: bool,
    create_file: bool,
) -> Result<Option<web_sys::FileSystemFileHandle>> {
    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_futures::wasm_bindgen::JsCast;
    use web_sys::{FileSystemFileHandle, FileSystemGetFileOptions};

    let dir = get_dir_from_appdata(clean_path.parent(), create_parent).await?;

    let options = FileSystemGetFileOptions::new();
    options.set_create(create_file);

    let filename = clean_path.file_name().unwrap().to_string_lossy();

    let file = match JsFuture::from(dir.get_file_handle_with_options(&filename, &options)).await {
        Ok(file) => file
            .dyn_into::<FileSystemFileHandle>()
            .map_err(|err| anyhow::anyhow!("Failed to cast to FileSystemFileHandle: {:?}", err))?,
        Err(err) => {
            let error = err
                .dyn_into::<web_sys::DomException>()
                .map(|e| e.name())
                .unwrap_or_else(|_| "unknown".to_string());

            if error == "NotFoundError" {
                return Ok(None);
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to get file from appdata: {}",
                    error
                ));
            }
        }
    };

    Ok(Some(file))
}

/// Read a file from the appdata directory.
/// However, on Web there's no real filesystem, so it is simulated by
/// [OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system).
/// For example, path `foo/bar` will be stored in the path `moyu/<app_name>/foo/bar`.
pub async fn read_from_appdata(relative_path: &str) -> Result<Option<Vec<u8>>> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::File;
    use web_sys::wasm_bindgen::JsCast;

    if !opfs_supported() {
        return read_from_local_storage(relative_path);
    }

    // create parent directory if it doesn't exist
    let Some(file) =
        get_file_from_appdata(&get_path_in_appdata(relative_path)?, true, false).await?
    else {
        return Ok(None);
    };

    let file = JsFuture::from(file.get_file())
        .await
        .map_err(|err| anyhow::anyhow!("Failed to get file from FileSystemFileHandle: {:?}", err))?
        .dyn_into::<File>()
        .map_err(|err| anyhow::anyhow!("Failed to cast to File: {:?}", err))?;

    let data = JsFuture::from(file.array_buffer())
        .await
        .map_err(|err| anyhow::anyhow!("Failed to get array buffer from File: {:?}", err))?;

    let data = web_sys::js_sys::Uint8Array::new(&data);

    Ok(Some(data.to_vec()))
}

/// Write a file to the appdata directory.
/// See [`read_from_appdata`](crate::fs::read_from_appdata) for more information.
pub async fn write_to_appdata(relative_path: &str, data: Vec<u8>) -> Result<()> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::wasm_bindgen::JsCast;

    if !opfs_supported() {
        return write_to_local_storage(relative_path, data);
    }

    let Some(file) =
        get_file_from_appdata(&get_path_in_appdata(relative_path)?, true, true).await?
    else {
        return Err(anyhow::anyhow!(
            "Failed to get file handle, this should not happen"
        ));
    };

    let stream = JsFuture::from(file.create_writable())
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "Failed to create writable stream from FileSystemFileHandle: {:?}",
                err
            )
        })?
        .dyn_into::<web_sys::FileSystemWritableFileStream>()
        .map_err(|err| {
            anyhow::anyhow!("Failed to cast to FileSystemWritableFileStream: {:?}", err)
        })?;

    let promise = stream
        .write_with_u8_array(&data)
        .map_err(|err| anyhow::anyhow!("Failed to write to stream: {:?}", err))?;

    JsFuture::from(promise)
        .await
        .map_err(|err| anyhow::anyhow!("Failed to write to stream: {:?}", err))?;

    JsFuture::from(stream.close())
        .await
        .map_err(|err| anyhow::anyhow!("Failed to write to stream: {:?}", err))?;

    Ok(())
}

/// Read a directory from the appdata directory.
pub async fn readdir_from_appdata(
    relative_path: &str,
    pattern: Option<String>,
) -> Result<Vec<FileEntry>> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::{FileSystemHandle, FileSystemHandleKind};

    if !opfs_supported() {
        return readdir_from_local_storage(relative_path, pattern);
    }

    let path = get_path_in_appdata(relative_path)?;

    // create the directory if it doesn't exist
    let dir = get_dir_from_appdata(Some(&path), true).await?;

    let mut arr = Vec::new();
    let entries = dir.values();

    let property_value = JsValue::from_str("value");

    loop {
        let promise = entries
            .next()
            .map_err(|err| anyhow::anyhow!("Failed to get next entry: {:?}", err))?;

        let item = JsFuture::from(promise)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to get next entry: {:?}", err))?;

        let item = Reflect::get(&item, &property_value).map_err(|err| {
            anyhow::anyhow!("Failed to get 'value' property from entry: {:?}", err)
        })?;

        if item.is_undefined() {
            break;
        }

        let item = item
            .dyn_into::<FileSystemHandle>()
            .map_err(|err| anyhow::anyhow!("Failed to cast to FileSystemHandle: {:?}", err))?;

        let name = item.name();

        if let Some(pattern) = pattern.as_ref() {
            if !fast_glob::glob_match(pattern, &name) {
                continue; // Skip entries that do not match the pattern
            }
        }

        let is_dir = item.kind() == FileSystemHandleKind::Directory;

        if !is_dir {
            let file = item
                .dyn_into::<web_sys::FileSystemFileHandle>()
                .map_err(|err| {
                    anyhow::anyhow!("Failed to cast to FileSystemFileHandle: {:?}", err)
                })?;
            let file = JsFuture::from(file.get_file())
                .await
                .map_err(|err| {
                    anyhow::anyhow!("Failed to get file from FileSystemFileHandle: {:?}", err)
                })?
                .dyn_into::<web_sys::File>()
                .map_err(|err| anyhow::anyhow!("Failed to cast to File: {:?}", err))?;
            let last_modified = file.last_modified() as u64;
            let size = file.size() as u64;

            arr.push(FileEntry {
                name,
                is_dir,
                size,
                last_modified,
            });
        } else {
            arr.push(FileEntry {
                name,
                is_dir,
                size: 0,
                last_modified: 0,
            });
        }
    }

    Ok(arr)
}

/// Remove a file or directory from the appdata directory.
pub async fn remove_from_appdata(relative_path: &str) -> Result<()> {
    if !opfs_supported() {
        return remove_from_local_storage(relative_path);
    }

    let path = get_path_in_appdata(relative_path)?;

    let dir = get_dir_from_appdata(path.parent(), false).await?;

    let options = FileSystemRemoveOptions::new();
    options.set_recursive(true);

    let filename = path.file_name().unwrap().to_string_lossy();

    JsFuture::from(dir.remove_entry_with_options(&filename, &options))
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "Failed to remove entry from FileSystemDirectoryHandle: {:?}",
                err
            )
        })?;

    Ok(())
}
