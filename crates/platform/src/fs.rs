use std::io::Cursor;
#[cfg(target_os = "android")]
use std::io::Read;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
#[cfg(native)]
use tokio::fs;
use url::Url;

use crate::config::get_engine_config;
#[cfg(target_os = "android")]
use crate::platform::get_android_app;

/// Open a file from a URL, returns a `Vec<u8>`
pub async fn read(url: &Url) -> Result<Vec<u8>> {
    // support reading files from android assets
    #[cfg(target_os = "android")]
    if url.scheme() == "file" && url.to_string().starts_with("file:///android_asset/") {
        let asset_path = url.to_string().replace("file:///android_asset/", "");
        let asset_manager = get_android_app().asset_manager();
        let mut asset = asset_manager
            .open(&std::ffi::CString::new(asset_path)?)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
        let mut buf = Vec::new();

        asset.read_to_end(&mut buf)?;

        return Ok(buf);
    };

    #[cfg(native)]
    if url.scheme() == "file" {
        return match fs::read(url.to_file_path().unwrap()).await {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow::anyhow!(err)),
        };
    };

    if url.scheme() == "http" || url.scheme() == "https" {
        let client = reqwest::Client::new();
        let code = client
            .get(url.clone())
            .header("accept", "*/*")
            .header("accept-encoding", "gzip, deflate")
            .header(
                "accept-language",
                "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7,zh-TW;q=0.6",
            )
            .header("cache-control", "no-cache")
            .header("dnt", "1")
            .header("pragma", "no-cache")
            .header("upgrade-insecure-requests", "1")
            .header(
                "sec-ch-ua",
                "\" Not;A Brand\";v=\"99\", \"Google Chrome\";v=\"97\", \"Chromium\";v=\"97\"",
            )
            .header(
                "user-agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                        Chrome/97.0.4692.71 Safari/537.36",
            )
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        return Ok(code.to_vec());
    };

    Err(anyhow::format_err!("Unsupported scheme '{}'", url.scheme()))
}

/// Open a file from a URL, returns a `Cursor<Vec<u8>>` which is [`Read`](std::io::Read) + [`Seek`](std::io::Seek)
pub async fn open(url: &Url) -> Result<Cursor<Vec<u8>>> {
    Ok(Cursor::new(read(url).await?))
}

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

/// Read a file from the appdata directory.
/// On Windows, the appdata directory is the `%APPDATA%/<app_name>` directory.
/// On Linux, the appdata directory is the `~/.local/share/<app_name>` directory.
/// On macOS, the appdata directory is the `~/Library/Application Support/<app_name>` directory.
/// On Android, the appdata directory is the `/Android/data/<package_name>/files` directory.
///
#[cfg(native)]
pub async fn read_from_appdata(relative_path: &str) -> Result<Vec<u8>> {
    let path = get_path_in_appdata(relative_path)?;

    match tokio::fs::read(&path).await {
        Ok(data) => Ok(data),
        Err(err) => Err(anyhow::anyhow!("Error reading from appdata: {:?}", err)),
    }
}

#[cfg(web)]
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

#[cfg(web)]
async fn get_file_from_appdata(
    clean_path: &PathBuf,
    create: bool,
) -> Result<web_sys::FileSystemFileHandle> {
    use wasm_bindgen_futures::wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{FileSystemFileHandle, FileSystemGetFileOptions};

    let dir = get_dir_from_appdata(clean_path.parent(), create).await?;

    let options = FileSystemGetFileOptions::new();
    options.set_create(create);

    let filename = clean_path.file_name().unwrap().to_string_lossy();

    let file = JsFuture::from(dir.get_file_handle_with_options(&filename, &options))
        .await
        .map_err(|err| anyhow::anyhow!("Failed to get file handle: {:?}", err))?
        .dyn_into::<FileSystemFileHandle>()
        .map_err(|err| anyhow::anyhow!("Failed to cast to FileSystemFileHandle: {:?}", err))?;

    Ok(file)
}

/// Read a file from the appdata directory.
/// However, on Web there's no real filesystem, so it is simulated by
/// [OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system).
/// For example, path `foo/bar` will be stored in the path `doufu/<app_name>/foo/bar`.
#[cfg(web)]
pub async fn read_from_appdata(relative_path: &str) -> Result<Vec<u8>> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::File;

    let file = get_file_from_appdata(&get_path_in_appdata(relative_path)?, false).await?;

    let file = JsFuture::from(file.get_file())
        .await
        .map_err(|err| anyhow::anyhow!("Failed to get file from FileSystemFileHandle: {:?}", err))?
        .dyn_into::<File>()
        .map_err(|err| anyhow::anyhow!("Failed to cast to File: {:?}", err))?;

    let data = JsFuture::from(file.array_buffer())
        .await
        .map_err(|err| anyhow::anyhow!("Failed to get array buffer from File: {:?}", err))?;

    let data = web_sys::js_sys::Uint8Array::new(&data);

    Ok(data.to_vec())
}

/// Write a file to the appdata directory.
/// See [`read_from_appdata`](crate::fs::read_from_appdata) for more information.
#[cfg(native)]
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

/// Write a file to the appdata directory.
/// See [`read_from_appdata`](crate::fs::read_from_appdata) for more information.
#[cfg(web)]
pub async fn write_to_appdata(relative_path: &str, data: Vec<u8>) -> Result<()> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::wasm_bindgen::JsCast;

    let file = get_file_from_appdata(&get_path_in_appdata(relative_path)?, true).await?;

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

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub last_modified: u64,
}

/// Read a directory from the appdata directory.
#[cfg(native)]
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

/// Read a directory from the appdata directory.
#[cfg(web)]
pub async fn readdir_from_appdata(relative_path: &str) -> Result<Vec<FileEntry>> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::{FileSystemHandle, FileSystemHandleKind};

    let path = get_path_in_appdata(relative_path)?;

    let dir = get_dir_from_appdata(Some(&path), false).await?;

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
