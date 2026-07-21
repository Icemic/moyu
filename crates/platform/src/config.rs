mod backend;
mod logical_size;
mod present_mode;

use csscolorparser::Color;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dir::parse_entry_dir;
use crate::platform::show_fatal_error_and_exit;

pub use self::backend::RenderingBackend;
use self::logical_size::MoyuLogicalSize;
pub use self::present_mode::RenderingPresentMode;

static MOYU_ENV: OnceCell<MoyuConfig> = OnceCell::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(TS)]
#[ts(export, optional_fields)]
pub enum WindowState {
    Idle,
    Maximized,
    Minimized,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutorunMode {
    All,
    NativeOnly,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SteamConfig {
    pub app_id: u32,
    pub required: bool,
    pub restart_through_client: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MoyuConfig {
    pub entry: Option<String>,
    pub entry_filename: String,
    pub app_name: String,
    pub autorun: AutorunMode,
    pub font_file: String,
    pub window_title: String,
    pub window_state: WindowState,
    pub window_resizable: bool,
    pub initial_surface_size: MoyuLogicalSize,
    pub stage_size: MoyuLogicalSize,
    pub present_mode: RenderingPresentMode,
    pub backend: RenderingBackend,
    /// see https://docs.rs/wgpu/latest/wgpu/type.SurfaceConfiguration.html#structfield.desired_maximum_frame_latency
    pub desired_maximum_frame_latency: u32,
    pub background_color: Color,
    #[serde(rename = "showFPS")]
    pub show_fps: bool,
    #[serde(rename = "enableMSAA")]
    pub enable_msaa: bool,
    pub enable_mipmaps: bool,
    pub enable_gamepads: bool,
    pub skip_splash: bool,
    pub steam: Option<SteamConfig>,
    /// Custom parameters that can be accessed in the engine.
    /// The content is not interpreted by the platform, it's just passed to the engine as-is.
    pub params: String,
}

impl Default for MoyuConfig {
    fn default() -> Self {
        Self {
            entry: None,
            entry_filename: "index.js".to_string(),
            app_name: "moyu".to_string(),
            autorun: AutorunMode::All,
            font_file: "fonts/default.otf".to_string(),
            window_title: "moyu".to_string(),
            window_state: WindowState::Idle,
            window_resizable: false,
            initial_surface_size: "1280x720".parse().unwrap(),
            stage_size: "1280x720".parse().unwrap(),
            present_mode: RenderingPresentMode::default(),
            backend: RenderingBackend::default(),
            desired_maximum_frame_latency: 2,
            background_color: Color::from_html("transparent").unwrap(),
            show_fps: false,
            enable_msaa: false,
            enable_mipmaps: false,
            enable_gamepads: false,
            skip_splash: false,
            steam: None,
            params: String::new(),
        }
    }
}

pub async fn setup() {
    #[cfg(desktop)]
    let mut args = pico_args::Arguments::from_env();

    #[cfg(desktop)]
    let mut entry = {
        args.opt_value_from_str("--entry")
            .unwrap()
            .unwrap_or_else(|| {
                log::info!("No --entry argument provided, defaulting to ./index.json");
                "./index.json".to_string()
            })
    };

    #[cfg(mobile)]
    let mut entry = "./index.json".to_string();

    #[cfg(web)]
    let mut entry = web_sys::window()
        .unwrap()
        .get("__moyu_entry")
        .map(|v| v.as_string().unwrap())
        .unwrap_or("./index.json".to_string());

    loop {
        let entry_dir = parse_entry_dir(&entry);
        log::info!("loading entry file: {}", entry_dir);
        match crate::fs::read(&entry_dir).await {
            Ok(content) => {
                let mut config = match serde_json::from_slice::<MoyuConfig>(&content) {
                    Ok(content) => content,
                    Err(error) => {
                        log::error!("error when parsing config: {:?}", error);
                        panic!("Failed to parse entry file: {}", error);
                    }
                };

                if let Some(_entry) = &config.entry {
                    if entry.as_str() != _entry.as_str() {
                        log::info!("redirecting entry file to: {}", _entry);
                        entry = _entry.clone();
                        continue;
                    }
                }

                config.entry = Some(entry);

                #[cfg(desktop)]
                if let Some(params) = args.opt_value_from_str("--params").unwrap() {
                    config.params = params;
                }

                #[cfg(web)]
                if let Some(params) = web_sys::window()
                    .unwrap()
                    .get("__moyu_params")
                    .map(|v| v.as_string().unwrap())
                {
                    config.params = params;
                }

                MOYU_ENV.set(config).unwrap();
                break;
            }
            Err(err) => {
                log::error!("Config file ({entry_dir}) cannot be loaded: {err:?}");
                show_fatal_error_and_exit(&format!(
                    "Failed to load configuration: {err:?}\nPlease check your configuration file."
                ));
            }
        }
    }
}

#[cfg(web)]
pub fn setup_with_wasm_config(config: wasm_bindgen::JsValue) {
    let config: MoyuConfig = config.into_serde().unwrap_or_default();
    MOYU_ENV.set(config).unwrap();
}

pub fn get_engine_config() -> &'static MoyuConfig {
    MOYU_ENV.get().unwrap()
}
