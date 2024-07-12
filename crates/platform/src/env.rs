mod backend;
mod present_mode;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use url::Url;

pub use self::backend::RenderingBackend;
pub use self::present_mode::RenderingPresentMode;

static HAI_ENV: OnceCell<HaiConfig> = OnceCell::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowState {
    Idle,
    Maximized,
    Minimized,
    Fullscreen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HaiConfig {
    #[cfg(not(feature = "web"))]
    pub entry: String,
    pub font_file: String,
    pub window_title: String,
    pub window_state: WindowState,
    pub window_resizable: bool,
    pub surface_size: (u32, u32),
    pub stage_size: (u32, u32),
    pub present_mode: RenderingPresentMode,
    pub backend: RenderingBackend,
    /// see https://docs.rs/wgpu/latest/wgpu/type.SurfaceConfiguration.html#structfield.desired_maximum_frame_latency
    pub desired_maximum_frame_latency: u32,

    pub show_fps: bool,
}

impl Default for HaiConfig {
    fn default() -> Self {
        Self {
            #[cfg(not(feature = "web"))]
            entry: std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            font_file: "fonts/default.subset.otf".to_string(),
            window_title: "Hai no engine".to_string(),
            window_state: WindowState::Idle,
            window_resizable: false,
            surface_size: (1280, 720),
            stage_size: (1280, 720),
            present_mode: RenderingPresentMode::default(),
            backend: RenderingBackend::default(),
            desired_maximum_frame_latency: 2,
            show_fps: false,
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
            panic!("Failed to read config: {}", error);
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
    let entry_dir = "http://localhost:8080/demo.js";

    if entry_dir.starts_with("http://")
        || entry_dir.starts_with("https://")
        || entry_dir.starts_with("file://")
    {
        return Url::parse(entry_dir).unwrap();
    }

    #[cfg(not(feature = "web"))]
    if !entry_dir.contains("://") {
        #[cfg(not(feature = "web"))]
        return get_entry_dir_local(entry_dir);
    }

    unimplemented!("unsupported entry '{}'.", entry_dir);
}

#[cfg(not(feature = "web"))]
fn get_entry_dir_local(entry_dir: &String) -> Url {
    let local_path = std::env::current_dir().unwrap();
    let local_path = local_path.join(entry_dir);
    if local_path.is_file() {
        Url::from_file_path(&local_path).unwrap()
    } else {
        Url::from_directory_path(&local_path).unwrap()
    }
}
