mod backend;
mod present_mode;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "web"))]
use std::env;
use url::Url;

pub use self::backend::RenderingBackend;
pub use self::present_mode::RenderingPresentMode;

static HAI_ENV: OnceCell<HaiConfig> = OnceCell::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HaiConfig {
    pub entry: String,
    pub show_fps: bool,
    pub present_mode: RenderingPresentMode,
    pub backend: RenderingBackend,
    /// see https://docs.rs/wgpu/latest/wgpu/type.SurfaceConfiguration.html#structfield.desired_maximum_frame_latency
    pub desired_maximum_frame_latency: u32,
    pub font_file: String,
}

impl Default for HaiConfig {
    fn default() -> Self {
        Self {
            entry: env::current_dir().unwrap().to_str().unwrap().to_string(),
            show_fps: false,
            present_mode: RenderingPresentMode::default(),
            backend: RenderingBackend::default(),
            desired_maximum_frame_latency: 2,
            font_file: "assets/fonts/default.otf".to_string(),
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
            println!("Failed to read config: {}", error);
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
        Url::parse(entry_dir).unwrap()
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
