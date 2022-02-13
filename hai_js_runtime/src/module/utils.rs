use hai_module_compiler::ScriptType;
use log::{debug, error};
use std::{path::PathBuf, process::exit};
use tokio::fs;

use super::types::ModuleType;
use crate::utils::try_find_file;

/// not follow completely https://html.spec.whatwg.org/multipage/webappapis.html#resolve-a-module-specifier
pub fn resolve_module_specifier(
    specifier: &str,
    referrer_name: &str,
) -> (ModuleType, std::string::String) {
    if specifier.starts_with(".") {
        let path = PathBuf::from(referrer_name).with_file_name("");

        if let Some((filename, ext)) =
            try_find_file(&path, specifier, vec!["ts", "tsx", "mjs", "jsx", "js"])
        {
            let script_type;

            if ext == "ts" || ext == "tsx" {
                script_type = ScriptType::Typescript;
            } else {
                script_type = ScriptType::Javascript;
            }

            return (
                ModuleType::Local(script_type),
                filename.to_str().unwrap().to_string(),
            );
        }

        return (ModuleType::None, "".to_string());
    }

    // treat others as remote modules (just like modules in `node_modules` for nodejs)
    let mut path = std::string::String::new();

    // specifier with a 'http://' or 'https://' will be used as-is,
    // otherwise it will be treated as plain remote package name,
    // then a default remote package cdn prefix will be added.
    if specifier.starts_with("http://") || specifier.starts_with("https://") {
        path.push_str(specifier);
    } else {
        path.push_str("https://esm.sh/");
        path.push_str(specifier);
        if path.contains('?') {
            path.push_str("&target=es2022");
        } else {
            path.push_str("?target=es2022");
        }
    }

    return (ModuleType::Remote, path);
}

pub async fn read_code_local(filename: &std::string::String) -> std::string::String {
    match fs::read_to_string(filename).await {
        Ok(data) => data,
        Err(err) => {
            // force quit if a module cannot be loaded
            error!(
                "[module] cannot load module, something went wrong at reading file '{}' ({}).",
                filename,
                err.to_string()
            );
            exit(-1);
        }
    }
}

pub async fn read_code_remote(url: &std::string::String) -> std::string::String {
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
