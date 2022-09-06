use hai_pal::url::Url;
use log::error;
use std::process::exit;
use tokio::fs;

pub async fn read_code_local(filename: &Url) -> std::string::String {
    match fs::read_to_string(filename.to_file_path().unwrap()).await {
        Ok(data) => data,
        Err(err) => {
            // force quit if a module cannot be loaded
            error!(
                "cannot load module, something went wrong at reading file '{}' ({}).",
                filename.to_string(),
                err.to_string()
            );
            exit(-1);
        }
    }
}

#[cfg(not(feature = "remote"))]
pub async fn read_code_remote(_: &Url) -> std::string::String {
    unimplemented!(
        "loading module from remote server is not support unless 'remote' feature enabled."
    );
}

#[cfg(feature = "remote")]
pub async fn read_code_remote(url: &PathBuf) -> std::string::String {
    use log::debug;

    let client = reqwest::Client::new();
    let code = client
        .get(url)
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
        )
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    // print only the first 255 characters
    debug!(
        "content downloaded from: {}\n{}",
        url,
        &code[..(255.min(code.len()))]
    );
    code
}
