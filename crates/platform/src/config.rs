mod backend;
mod logical_size;
mod present_mode;

use csscolorparser::Color;
use once_cell::sync::OnceCell;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

/// How multi-resolution asset variants (`abc@2x.png`) are selected.
///
/// Written in the configuration file as `"auto"`, `"off"`, or a scale such as
/// `2`. Serde cannot derive one enum from that mix of shapes, so both
/// directions are implemented by hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MultiResAssetsMode {
    /// Pick a variant scale matching the current display ratio.
    Auto,
    /// Always use the base asset.
    Off,
    /// Use the variant with exactly this scale, falling back to the base asset
    /// when the variant file is missing.
    Fixed(f32),
}

impl Serialize for MultiResAssetsMode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Auto => serializer.serialize_str("auto"),
            Self::Off => serializer.serialize_str("off"),
            Self::Fixed(scale) => serializer.serialize_f32(*scale),
        }
    }
}

impl<'de> Deserialize<'de> for MultiResAssetsMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ModeVisitor;

        impl Visitor<'_> for ModeVisitor {
            type Value = MultiResAssetsMode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("\"auto\", \"off\", or a scale such as 2")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match value {
                    "auto" => Ok(MultiResAssetsMode::Auto),
                    "off" => Ok(MultiResAssetsMode::Off),
                    other => Err(E::unknown_variant(other, &["auto", "off"])),
                }
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Ok(MultiResAssetsMode::Fixed(value as f32))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(MultiResAssetsMode::Fixed(value as f32))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(MultiResAssetsMode::Fixed(value as f32))
            }
        }

        deserializer.deserialize_any(ModeVisitor)
    }
}

/// Order in which candidate variant scales are tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MultiResAssetPolicy {
    /// Prefer the smallest candidate scale that is not smaller than the display
    /// ratio, so assets are never upscaled when a larger variant exists.
    Quality,
    /// Prefer the candidate scale closest to the display ratio in log space.
    Balanced,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SteamConfig {
    pub app_id: u32,
    pub required: bool,
    pub restart_through_client: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FontFile {
    Path(String),
    Sources(Vec<FontSourceConfig>),
}

impl Default for FontFile {
    fn default() -> Self {
        Self::Path("fonts/default.otf".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FontSourceConfig {
    Path(String),
    Source {
        path: String,
        alias: Option<String>,
        kind: Option<FontSourceKind>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontSourceKind {
    Cjk,
    Western,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MoyuConfig {
    pub entry: Option<String>,
    pub entry_filename: String,
    pub app_name: String,
    pub autorun: AutorunMode,
    pub font_file: FontFile,
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
    pub multi_res_assets: MultiResAssetsMode,
    /// Variant scales to try besides the base asset. Empty means base only.
    pub multi_res_asset_scales: Vec<f32>,
    pub multi_res_asset_policy: MultiResAssetPolicy,
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
            font_file: FontFile::default(),
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
            multi_res_assets: MultiResAssetsMode::Off,
            multi_res_asset_scales: vec![1.5, 2.0],
            multi_res_asset_policy: MultiResAssetPolicy::Quality,
            enable_gamepads: false,
            skip_splash: false,
            steam: None,
            params: String::new(),
        }
    }
}

impl MoyuConfig {
    /// Drop multi-resolution asset settings that cannot be used, so consumers
    /// can rely on the values returned by [`get_engine_config`].
    ///
    /// `1` is rejected as a candidate scale because the base asset already
    /// covers it and is never probed.
    fn normalize_multi_res_assets(&mut self) {
        self.multi_res_asset_scales.retain(|scale| {
            let valid = *scale > 0.0 && *scale != 1.0;
            if !valid {
                log::warn!("ignoring invalid multiResAssetScales entry {scale}");
            }
            valid
        });

        if let MultiResAssetsMode::Fixed(scale) = self.multi_res_assets {
            if scale <= 0.0 {
                log::warn!("invalid multiResAssets value {scale}, falling back to 'auto'");
                self.multi_res_assets = MultiResAssetsMode::Auto;
            }
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

    // On Android the host application can pass the entry through the launch intent,
    // which is how a dynamically generated entry (for example a local preview
    // server) is loaded without repackaging the APK.
    #[cfg(android)]
    let mut entry = crate::platform::intent_entry().unwrap_or_else(|| {
        log::info!("no entry provided through the launch intent, defaulting to ./index.json");
        "./index.json".to_string()
    });

    #[cfg(ios)]
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

                config.normalize_multi_res_assets();
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
    let mut config: MoyuConfig = config.into_serde().unwrap_or_default();
    config.normalize_multi_res_assets();
    MOYU_ENV.set(config).unwrap();
}

pub fn get_engine_config() -> &'static MoyuConfig {
    MOYU_ENV.get().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mode is written as a string or as a number, which serde cannot
    /// derive; this pins the format the configuration file uses.
    #[test]
    fn parses_and_writes_multi_res_assets_mode() {
        for (json, expected) in [
            ("\"auto\"", MultiResAssetsMode::Auto),
            ("\"off\"", MultiResAssetsMode::Off),
            ("2.0", MultiResAssetsMode::Fixed(2.0)),
            ("1.5", MultiResAssetsMode::Fixed(1.5)),
        ] {
            assert_eq!(
                serde_json::from_str::<MultiResAssetsMode>(json).unwrap(),
                expected,
                "parsing {json}"
            );
            assert_eq!(
                serde_json::to_string(&expected).unwrap(),
                json,
                "writing {json}"
            );
        }

        // An integer is accepted too, and written back as a float.
        assert_eq!(
            serde_json::from_str::<MultiResAssetsMode>("2").unwrap(),
            MultiResAssetsMode::Fixed(2.0)
        );
    }
}
