#[cfg(target_os = "android")]
use std::io::Read;

use anyhow::Result;
use url::Url;

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
        return match tokio::fs::read(url.to_file_path().unwrap()).await {
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
