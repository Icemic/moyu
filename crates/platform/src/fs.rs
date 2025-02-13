use std::io::Cursor;
#[cfg(target_os = "android")]
use std::io::Read;

use anyhow::Result;
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

/// Read a file from the appdata directory.
/// On Windows, the appdata directory is the `%APPDATA%/<app_name>` directory.
/// On Linux, the appdata directory is the `~/.local/share/<app_name>` directory.
/// On macOS, the appdata directory is the `~/Library/Application Support/<app_name>` directory.
/// On Android, the appdata directory is the `/Android/data/<package_name>/files` directory.
///
#[cfg(native)]
pub fn read_from_appdata(relative_path: &str) -> Result<Option<Vec<u8>>> {
    let appdata_dir = get_engine_config().appdata_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to get appdata directory for app '{}'",
            get_engine_config().app_name
        )
    })?;

    let path = appdata_dir.join(relative_path);

    match std::fs::read(&path) {
        Ok(data) => Ok(Some(data)),
        Err(err) => Err(anyhow::anyhow!("Error reading from appdata: {:?}", err)),
    }
}

/// Read a file from the appdata directory.
/// However, on Web there's no real filesystem, so it is simulated by
/// [localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage).
/// For example, path `foo/bar` will be stored in the key `DOUFU_FS/<app_name>/foo/bar`.
/// All data is stored as base64 encoded strings.
#[cfg(web)]
pub fn read_from_appdata(relative_path: &str) -> Result<Option<Vec<u8>>> {
    use base64ct::{Base64Url, Encoding};

    let key = format!(
        "DOUFU_FS/{}/{}",
        get_engine_config().app_name,
        relative_path
    );

    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("Failed to get window object"))?;
    let local_storage = window
        .local_storage()
        .map_err(|err| anyhow::anyhow!("Failed to get local storage: {:?}", err))?
        .ok_or_else(|| anyhow::anyhow!("Failed to get local storage object"))?;

    let data = local_storage
        .get_item(&key)
        .map_err(|err| anyhow::anyhow!("Failed to get item from local storage: {:?}", err))?
        .map(|data| {
            Base64Url::decode_vec(&data)
                .map_err(|err| anyhow::anyhow!("Failed to decode base64: {:?}", err))
        })
        .transpose()?;

    Ok(data)
}

/// Write a file to the appdata directory.
/// See [`read_from_appdata`](crate::fs::read_from_appdata) for more information.
#[cfg(native)]
pub fn write_to_appdata(relative_path: &str, data: &[u8]) -> Result<()> {
    let appdata_dir = get_engine_config()
        .appdata_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get appdata directory"))?;

    let path = appdata_dir.join(relative_path);

    std::fs::write(&path, data)
        .map_err(|err| anyhow::anyhow!("Failed to write appdata {}: {}", path.display(), err))?;

    Ok(())
}

/// Write a file to the appdata directory.
/// See [`read_from_appdata`](crate::fs::read_from_appdata) for more information.
#[cfg(web)]
pub fn write_to_appdata(relative_path: &str, data: &[u8]) -> Result<()> {
    use base64ct::{Base64Url, Encoding};

    let key = format!(
        "DOUFU_FS/{}/{}",
        get_engine_config().app_name,
        relative_path
    );

    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("Failed to get window object"))?;
    let local_storage = window
        .local_storage()
        .map_err(|err| anyhow::anyhow!("Failed to get local storage: {:?}", err))?
        .ok_or_else(|| anyhow::anyhow!("Failed to get local storage object"))?;

    let data = Base64Url::encode_string(data);

    local_storage
        .set_item(&key, &data)
        .map_err(|err| anyhow::anyhow!("Failed to set item in local storage: {:?}", err))?;

    Ok(())
}
