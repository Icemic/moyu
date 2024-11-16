mod backend;
mod logical_size;
mod present_mode;

use logical_size::HaiLogicalSize;
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
#[serde(default, rename_all = "camelCase")]
pub struct HaiConfig {
    pub entry: Option<String>,
    pub entry_filename: String,
    pub font_file: String,
    pub window_title: String,
    pub window_state: WindowState,
    pub window_resizable: bool,
    pub surface_size: HaiLogicalSize,
    pub stage_size: HaiLogicalSize,
    pub present_mode: RenderingPresentMode,
    pub backend: RenderingBackend,
    /// see https://docs.rs/wgpu/latest/wgpu/type.SurfaceConfiguration.html#structfield.desired_maximum_frame_latency
    pub desired_maximum_frame_latency: u32,
    #[serde(rename = "showFPS")]
    pub show_fps: bool,
}

impl Default for HaiConfig {
    fn default() -> Self {
        Self {
            entry: None,
            entry_filename: "index.js".to_string(),
            font_file: "fonts/default.otf".to_string(),
            window_title: "Doufu".to_string(),
            window_state: WindowState::Idle,
            window_resizable: false,
            surface_size: "1280x720".parse().unwrap(),
            stage_size: "1280x720".parse().unwrap(),
            present_mode: RenderingPresentMode::default(),
            backend: RenderingBackend::default(),
            desired_maximum_frame_latency: 2,
            show_fps: false,
        }
    }
}

pub async fn setup() {
    #[cfg(not(feature = "web"))]
    let mut entry = "./index.json".to_string();

    #[cfg(feature = "web")]
    let mut entry = web_sys::window()
        .unwrap()
        .get("__hai_entry")
        .map(|v| v.as_string().unwrap())
        .unwrap_or("./index.json".to_string());

    loop {
        let entry_dir = parse_entry_dir(&entry);
        log::info!("loading entry file: {}", entry_dir);
        match crate::fs::read(&entry_dir).await {
            Ok(content) => {
                let mut config = match serde_json::from_slice::<HaiConfig>(&content) {
                    Ok(content) => content,
                    Err(error) => {
                        panic!("Failed to parse entry file: {}", error);
                    }
                };

                if let Some(_entry) = &config.entry {
                    if entry.as_str() != _entry.as_str() {
                        println!("redirecting entry file to: {}", _entry);
                        entry = _entry.clone();
                        continue;
                    }
                }

                config.entry = Some(entry);

                HAI_ENV.set(config).unwrap();
                break;
            }
            Err(_) => {
                println!("config file cannot be loaded, using default value.");
                HAI_ENV.set(HaiConfig::default()).unwrap();
                break;
            }
        }
    }
}

pub fn get_engine_config() -> &'static HaiConfig {
    HAI_ENV.get().unwrap()
}

fn parse_entry_dir(entry_dir: &String) -> Url {
    if entry_dir.starts_with("http://")
        || entry_dir.starts_with("https://")
        || entry_dir.starts_with("file://")
    {
        return Url::parse(entry_dir).unwrap();
    }

    #[cfg(not(feature = "web"))]
    if !entry_dir.contains("://") {
        let local_path = std::env::current_dir().unwrap();
        let local_path = local_path.join(entry_dir);
        if local_path.is_file() {
            return Url::from_file_path(&local_path).unwrap();
        } else {
            return Url::from_directory_path(&local_path).unwrap();
        }
    }

    #[cfg(feature = "web")]
    if !entry_dir.contains("://") {
        let local_path = web_sys::window().unwrap().location().href().unwrap();
        return Url::parse(&local_path).unwrap().join(entry_dir).unwrap();
    }

    unimplemented!("unsupported entry '{}'.", entry_dir);
}

pub fn entry_dir() -> Url {
    let entry_dir = get_engine_config().entry.as_ref().unwrap();
    parse_entry_dir(entry_dir)
}
