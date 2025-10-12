#[cfg(target_os = "android")]
use std::io::Read;

use anyhow::Result;
use url::Url;

#[cfg(target_os = "android")]
use crate::platform::get_android_app;

/// Open a file from a URL, returns a `Vec<u8>`
pub async fn read(url: &Url) -> Result<Vec<u8>> {
    if url.scheme() == "file" {
        read_from_file(url).await
    } else if url.scheme() == "http" || url.scheme() == "https" {
        read_from_http(url).await
    } else {
        Err(anyhow::format_err!("Unsupported scheme '{}'", url.scheme()))
    }
}

/// Open a file from a URL, returns a `Vec<u8>`
#[cfg(native)]
pub async fn read_from_file(url: &Url) -> Result<Vec<u8>> {
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

    return Err("You can only read from 'file:///android_asset/' on Android.".into());
}

/// Open a file from a URL, returns a `Vec<u8>`
pub async fn read_from_http(url: &Url) -> Result<Vec<u8>> {
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
