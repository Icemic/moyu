use std::path::PathBuf;

use anyhow::Result;
use wasm_bindgen_futures::JsFuture;
use web_sys::FileSystemRemoveOptions;

use super::{get_path_in_appdata, FileEntry};

async fn get_dir_from_appdata(
    clean_path: Option<&std::path::Path>,
    create: bool,
) -> Result<web_sys::FileSystemDirectoryHandle> {
    use std::path::Component;

    use wasm_bindgen_futures::wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{FileSystemDirectoryHandle, FileSystemGetDirectoryOptions};

    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("Failed to get window object"))?;
    let navigator = window.navigator();
    let opfs_root = JsFuture::from(navigator.storage().get_directory())
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
    use wasm_bindgen_futures::wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
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
/// For example, path `foo/bar` will be stored in the path `doufu/<app_name>/foo/bar`.
pub async fn read_from_appdata(relative_path: &str) -> Result<Option<Vec<u8>>> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::File;

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
pub async fn readdir_from_appdata(relative_path: &str) -> Result<Vec<FileEntry>> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::{FileSystemHandle, FileSystemHandleKind};

    let path = get_path_in_appdata(relative_path)?;

    // create the directory if it doesn't exist
    let dir = get_dir_from_appdata(Some(&path), true).await?;

    let mut arr = Vec::new();
    let entries = dir.values();
    loop {
        let promise = entries
            .next()
            .map_err(|err| anyhow::anyhow!("Failed to get next entry: {:?}", err))?;

        let item = JsFuture::from(promise)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to get next entry: {:?}", err))?;

        if item.is_undefined() {
            break;
        }

        let item = item
            .dyn_into::<FileSystemHandle>()
            .map_err(|err| anyhow::anyhow!("Failed to cast to FileSystemHandle: {:?}", err))?;

        let name = item.name();
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
