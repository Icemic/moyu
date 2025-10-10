use std::path::PathBuf;

use url::Url;

use crate::config::get_engine_config;

pub fn entry_dir() -> Url {
    let entry_dir = get_engine_config().entry.as_ref().unwrap();
    parse_entry_dir(entry_dir)
}

pub fn assets_dir() -> Url {
    entry_dir().join("assets/").unwrap()
}

pub fn appdata_dir() -> Option<PathBuf> {
    let app_name = &get_engine_config().app_name;
    #[cfg(desktop)]
    {
        if let Some(mut appdata_dir) = dirs::data_dir() {
            appdata_dir.push(app_name);
            // equals to `mkdir -p`
            std::fs::create_dir_all(&appdata_dir).unwrap();
            return Some(appdata_dir);
        };
    }

    #[cfg(web)]
    {
        use std::str::FromStr;
        return Some(PathBuf::from_str("moyu").unwrap().join(app_name));
    }

    #[cfg(android)]
    {
        use crate::platform::get_android_app;
        if let Some(external_data_path) = get_android_app().external_data_path() {
            let appdata_dir = external_data_path.join("files");
            std::fs::create_dir_all(&appdata_dir).unwrap();
            return Some(appdata_dir);
        }
    }

    #[cfg(ios)]
    unimplemented!("appdata_dir is to be implemented in ios platform.");

    None
}

pub(crate) fn parse_entry_dir(entry_dir: &String) -> Url {
    if entry_dir.starts_with("http://")
        || entry_dir.starts_with("https://")
        || entry_dir.starts_with("file://")
    {
        return Url::parse(entry_dir).unwrap();
    }

    #[cfg(target_os = "android")]
    if !entry_dir.contains("://") {
        let local_path = Url::parse("file:///android_asset/").unwrap();
        return local_path.join(entry_dir).unwrap();
    }

    #[cfg(all(native, not(target_os = "android")))]
    if !entry_dir.contains("://") {
        let local_path = std::env::current_dir().unwrap();
        let local_path = local_path.join(entry_dir);
        if local_path.is_dir() {
            return Url::from_directory_path(&local_path).unwrap();
        } else {
            return Url::from_file_path(&local_path).unwrap();
        }
    }

    #[cfg(web)]
    if !entry_dir.contains("://") {
        let local_path = web_sys::window().unwrap().location().href().unwrap();
        return Url::parse(&local_path).unwrap().join(entry_dir).unwrap();
    }

    unimplemented!("unsupported entry '{}'.", entry_dir);
}
