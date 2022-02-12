use hai_module_compiler::ScriptType;
use log::error;
use std::{fs, path::PathBuf, process::exit};

use super::ModuleType;
use crate::v8::utils::try_find_file;

pub fn resolve_module(specifier: &str, referrer_name: &str) -> (ModuleType, std::string::String) {
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

pub fn read_code_local(filename: &std::string::String) -> std::string::String {
    match fs::read_to_string(filename) {
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

pub fn read_code_remote(url: &std::string::String) -> std::string::String {
    // TODO: use Client to set headers
    // let result = reqwest::blocking::get(url).unwrap();
    // let code = result.text().unwrap();
    // // print only the first 255 characters
    // debug!(
    //     "content downloaded from: {}\n{}",
    //     url,
    //     &code[..(255.min(code.len()))]
    // );
    // code
    todo!("")
}
