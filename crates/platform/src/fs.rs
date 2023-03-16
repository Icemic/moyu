use anyhow::Result;
#[cfg(not(feature = "web"))]
use tokio::fs;
use url::Url;

pub async fn read(url: &Url) -> Result<Vec<u8>> {
    #[cfg(not(feature = "web"))]
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
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36"
        ).send().await?.bytes().await?;

        return Ok(code.to_vec());
    };

    return Err(anyhow::format_err!("Unsupported scheme '{}'", url.scheme()));
}
