use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "web"))]
use std::env;
use url::Url;

static HAI_ENV: OnceCell<HaiConfig> = OnceCell::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaiConfig {
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default = "default_vsync")]
    pub vsync: bool,
}

fn default_entry() -> String {
    env::current_dir().unwrap().to_str().unwrap().to_string()
}

fn default_vsync() -> bool {
    true
}

impl Default for HaiConfig {
    fn default() -> Self {
        Self {
            entry: default_entry(),
            vsync: default_vsync(),
        }
    }
}

#[cfg(not(feature = "web"))]
pub fn setup() {
    use dotenv::dotenv;
    // load custom env from .env file
    dotenv().ok();

    let env = match envy::prefixed("HAI_").from_env::<HaiConfig>() {
        Ok(config) => config,
        Err(error) => {
            println!("Failed to read config: {}", error.to_string());
            HaiConfig::default()
        }
    };

    HAI_ENV.set(env).unwrap();
}

#[cfg(feature = "web")]
pub fn setup() {
    HAI_ENV.set(HaiConfig::default()).unwrap();
}

pub fn get_hai_env() -> &'static HaiConfig {
    HAI_ENV.get().unwrap()
}

pub fn entry_dir() -> Url {
    #[cfg(not(feature = "web"))]
    let entry_dir = &get_hai_env().entry;
    #[cfg(feature = "web")]
    let entry_dir = "http://localhost:3020/examples/bunnyMark/index.ts".to_string();

    if entry_dir.starts_with("http://")
        || entry_dir.starts_with("https://")
        || entry_dir.starts_with("file://")
    {
        Url::parse(&entry_dir).unwrap()
    } else if cfg!(not(feature = "web")) && !entry_dir.contains("://") {
        #[cfg(not(feature = "web"))]
        get_entry_dir_local(entry_dir)
    } else {
        unimplemented!("unsupported entry '{}'.", entry_dir);
    }
}

#[cfg(not(feature = "web"))]
fn get_entry_dir_local(entry_dir: &String) -> Url {
    let local_path = env::current_dir().unwrap();
    let local_path = local_path.join(entry_dir);
    if local_path.is_file() {
        Url::from_file_path(&local_path).unwrap()
    } else {
        Url::from_directory_path(&local_path).unwrap()
    }
}
