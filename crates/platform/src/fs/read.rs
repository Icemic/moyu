use std::io::{Cursor, Read};
use std::path::PathBuf;

use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use url::Url;
use zip::result::ZipError;

use crate::dir::assets_dir;
use crate::fs::read_from_appdata;
#[cfg(target_os = "android")]
use crate::platform::get_android_app;

/// Open a file from a URL, returns a `Vec<u8>`
pub async fn read(url: &Url) -> Result<Vec<u8>> {
    match url.scheme() {
        #[cfg(native)]
        "file" => read_by_file(url).await,
        "http" | "https" => read_by_http(url).await,
        "assets" => {
            let url = assets_dir().join(url.path().trim_start_matches('/'))?;
            Box::pin(read(&url)).await
        }
        "saves" => read_by_saves(url).await,
        "appdata" => read_by_appdata(url).await,
        "data" => read_by_data_url(url),
        _ => Err(anyhow::format_err!("Unsupported scheme '{}'", url.scheme())),
    }
}

/// Open a file from a URL, returns a `Vec<u8>`
#[cfg(native)]
pub async fn read_by_file(url: &Url) -> Result<Vec<u8>> {
    match tokio::fs::read(url.to_file_path().unwrap()).await {
        Ok(v) => Ok(v),
        Err(err) => Err(anyhow::anyhow!(err)),
    }
}

/// Open a file from a URL, returns a `Vec<u8>`
#[cfg(target_os = "android")]
pub async fn read_from_file(url: &Url) -> Result<Vec<u8>> {
    if url.to_string().starts_with("file:///android_asset/") {
        let asset_path = url.to_string().replace("file:///android_asset/", "");
        let asset_manager = get_android_app().asset_manager();
        let mut asset = asset_manager
            .open(&std::ffi::CString::new(asset_path)?)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
        let mut buf = Vec::new();

        asset.read_to_end(&mut buf)?;

        return Ok(buf);
    }

    return Err(anyhow::anyhow!(
        "You can only read from 'file:///android_asset/' on Android."
    ));
}

/// Open a file from a URL, returns a `Vec<u8>`
pub async fn read_by_http(url: &Url) -> Result<Vec<u8>> {
    let mut request = ehttp::Request::get(url);
    let headers = &mut request.headers;
    headers.insert("accept", "*/*");
    headers.insert("accept-encoding", "gzip, deflate");
    headers.insert(
        "accept-language",
        "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7,zh-TW;q=0.6",
    );
    headers.insert("cache-control", "no-cache");
    headers.insert("dnt", "1");
    headers.insert("pragma", "no-cache");
    headers.insert("upgrade-insecure-requests", "1");
    headers.insert(
        "sec-ch-ua",
        "\" Not;A Brand\";v=\"99\", \"Google Chrome\";v=\"97\", \"Chromium\";v=\"97\"",
    );
    headers.insert(
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                    Chrome/97.0.4692.71 Safari/537.36",
    );

    let data = match ehttp::fetch_async(request).await {
        Ok(res) => {
            if res.ok {
                res.bytes
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to fetch url ({}): {} {}",
                    url,
                    res.status,
                    res.status_text
                ));
            }
        }
        Err(err) => {
            return Err(anyhow::anyhow!(
                "Failed to fetch url ({}): {}",
                url,
                err.to_string()
            ));
        }
    };

    Ok(data)
}

pub async fn read_by_saves(url: &Url) -> Result<Vec<u8>> {
    let name = url
        .fragment()
        .ok_or_else(|| anyhow::format_err!("Missing fragment in saves URL: {}", url.to_string()))?;

    let save_dir = PathBuf::from("saves");
    let save_file_path = save_dir.join(url.path().trim_start_matches('/'));

    let data = read_from_appdata(&save_file_path.to_string_lossy())
        .await
        .map(|v| {
            if let Some(v) = v {
                Ok(v)
            } else {
                Err(anyhow::format_err!("File not found: {}", url.to_string()))
            }
        })
        .flatten()?;

    let mut zip = zip::ZipArchive::new(Cursor::new(data))?;

    match zip.by_name(name) {
        Ok(file_data) => {
            Ok(Cursor::new(file_data.bytes().collect::<Result<Vec<_>, _>>()?).into_inner())
        }
        Err(ZipError::FileNotFound) => {
            return Err(anyhow::format_err!(
                "File not found in save archive: {}",
                url.to_string(),
            ));
        }
        Err(err) => {
            log::error!("Error reading save file {}: {}", url.to_string(), err);
            return Err(anyhow::format_err!(
                "Error reading save file: {}",
                url.to_string(),
            ));
        }
    }
}

pub async fn read_by_appdata(url: &Url) -> Result<Vec<u8>> {
    read_from_appdata(url.path().trim_start_matches('/'))
        .await
        .map(|v| {
            if let Some(v) = v {
                Ok(v)
            } else {
                Err(anyhow::format_err!("File not found: {}", url.to_string()))
            }
        })
        .flatten()
}

pub fn read_by_data_url(url: &Url) -> Result<Vec<u8>> {
    let data = url.path();
    let comma_index = data
        .find(',')
        .ok_or_else(|| anyhow::format_err!("Invalid data URL: missing comma separator"))?;
    let (metadata, data) = data.split_at(comma_index);
    let is_base64 = metadata.ends_with(";base64");
    let data = &data[1..]; // Skip the comma
    if is_base64 {
        let decoded_data = BASE64_STANDARD.decode(data)?;
        Ok(decoded_data)
    } else {
        // TODO: Handle percent-decoding if necessary
        Ok(data.as_bytes().to_vec())
    }
}
