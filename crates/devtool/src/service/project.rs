//! Project discovery, manifest loading, and multi-resolution asset preflight.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::model::project::{
    FrameworkSummary, MoyuConfigSummary, MultiResMode, MultiResReport, ProjectEvent,
    ProjectIndexSummary, VariantFile,
};
use crate::service::ServiceReporter;

const BASE_SCALE: f64 = 1.0;

/// Walk upward from `start_dir` (or current working directory) until `index.json` is found.
pub fn find_project_root(start_dir: Option<&Path>) -> Option<PathBuf> {
    let mut dir = match start_dir {
        Some(p) => fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()),
        None => std::env::current_dir().ok()?,
    };

    loop {
        if dir.join("index.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn require_project_root(start_dir: Option<&Path>) -> Result<PathBuf> {
    find_project_root(start_dir).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not locate a Moyu project (no index.json found).\nMake sure you are running this command inside a Moyu project directory."
        )
    })
}

#[allow(dead_code)]
pub fn load_index_summary(project_root: &Path) -> Result<ProjectIndexSummary> {
    let path = project_root.join("index.json");
    if !path.is_file() {
        bail!("index.json not found in {}", project_root.display());
    }
    let text = fs::read_to_string(&path)?;
    let parsed: ProjectIndexSummary = serde_json::from_str(&text).unwrap_or_default();
    Ok(parsed)
}

#[allow(dead_code)]
pub fn load_moyu_summary(project_root: &Path) -> Option<MoyuConfigSummary> {
    let text = fs::read_to_string(project_root.join("moyu.json")).ok()?;
    let raw: serde_json::Value = serde_json::from_str(&text).ok()?;
    let engine = raw.get("engine");
    let android = raw.get("android");
    Some(MoyuConfigSummary {
        engine_channel: engine
            .and_then(|e| e.get("channel"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        engine_cdn_url: engine
            .and_then(|e| e.get("cdnUrl"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        android_app_name: android
            .and_then(|a| a.get("appName"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        android_application_id: android
            .and_then(|a| a.get("applicationId"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        android_version_name: android
            .and_then(|a| a.get("versionName"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        android_version_code: android
            .and_then(|a| a.get("versionCode"))
            .and_then(|v| v.as_u64())
            .and_then(|n| u32::try_from(n).ok()),
    })
}

#[allow(dead_code)]
pub fn load_framework_summary(project_root: &Path) -> Option<FrameworkSummary> {
    let text = fs::read_to_string(project_root.join("framework.json")).ok()?;
    serde_json::from_str(&text).ok()
}

/// One font source entry in `index.json`'s `fontFile`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FontEntryForm {
    pub path: String,
    pub alias: String,
    /// `""` (auto) | `"cjk"` | `"western"`
    pub kind: String,
}

impl Default for FontEntryForm {
    fn default() -> Self {
        Self {
            path: "fonts/default.otf".to_string(),
            alias: String::new(),
            kind: String::new(),
        }
    }
}

/// One custom cursor entry in `index.json`'s `cursor`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CursorEntryForm {
    /// Cursor state name (e.g. `default`, `pointer`, `text`, `wait`).
    pub state: String,
    /// Relative asset path to cursor image.
    pub src: String,
    /// Optional hotspot X in image pixels.
    pub hotspot_x: String,
    /// Optional hotspot Y in image pixels.
    pub hotspot_y: String,
}

impl Default for CursorEntryForm {
    fn default() -> Self {
        Self {
            state: "default".to_string(),
            src: String::new(),
            hotspot_x: String::new(),
            hotspot_y: String::new(),
        }
    }
}

/// Complete editable representation of `index.json` (engine runtime configuration).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RuntimeConfigForm {
    // Window & basics
    pub window_title: String,
    pub app_name: String,
    pub initial_surface_size: String,
    pub stage_size: String,
    pub window_state: String,
    pub window_resizable: bool,
    pub background_color: String,
    pub fonts: Vec<FontEntryForm>,
    // Multi-resolution assets
    pub multi_res_mode: String, // "off" | "auto" | "fixed"
    pub multi_res_fixed_scale: String,
    pub multi_res_scales: String,
    pub multi_res_policy: String, // "quality" | "balanced"
    // Features & platform
    pub enable_msaa: bool,
    pub enable_mipmaps: bool,
    pub enable_gamepads: bool,
    pub skip_splash: bool,
    pub show_fps: bool,
    pub steam_enabled: bool,
    pub steam_app_id: String,
    pub steam_required: bool,
    pub steam_restart_through_client: bool,
    pub cursor_enabled: bool,
    pub cursors: Vec<CursorEntryForm>,
    // Advanced rendering & entry
    pub entry_filename: String,
    pub entry: String,
    pub autorun: String,      // "all" | "native-only"
    pub backend: String,      // "auto" | "vulkan" | "metal" | "dx12" | "webgpu" | "gles"
    pub present_mode: String, // "recommended" | "autovsync" | "autonovsync"
    pub desired_maximum_frame_latency: String,
    pub params: String,
}

impl Default for RuntimeConfigForm {
    fn default() -> Self {
        Self {
            window_title: "moyu".to_string(),
            app_name: "moyu".to_string(),
            initial_surface_size: "1280x720".to_string(),
            stage_size: "1280x720".to_string(),
            window_state: "idle".to_string(),
            window_resizable: false,
            background_color: "transparent".to_string(),
            fonts: vec![FontEntryForm::default()],
            multi_res_mode: "off".to_string(),
            multi_res_fixed_scale: "2".to_string(),
            multi_res_scales: "1.5, 2".to_string(),
            multi_res_policy: "quality".to_string(),
            enable_msaa: false,
            enable_mipmaps: false,
            enable_gamepads: false,
            skip_splash: false,
            show_fps: false,
            steam_enabled: false,
            steam_app_id: String::new(),
            steam_required: false,
            steam_restart_through_client: false,
            cursor_enabled: false,
            cursors: Vec::new(),
            entry_filename: "index.js".to_string(),
            entry: String::new(),
            autorun: "all".to_string(),
            backend: "auto".to_string(),
            present_mode: "recommended".to_string(),
            desired_maximum_frame_latency: "2".to_string(),
            params: String::new(),
        }
    }
}

/// Editable representation of `moyu.json`'s `engine` section.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EngineProjectConfigForm {
    pub channel: String,
    pub cdn_url: String,
    pub version: String,
}

impl Default for EngineProjectConfigForm {
    fn default() -> Self {
        Self {
            channel: crate::config::DEFAULT_CHANNEL.to_string(),
            cdn_url: String::new(),
            version: String::new(),
        }
    }
}

#[allow(dead_code)]
pub fn load_runtime_config(project_root: &Path) -> Result<RuntimeConfigForm> {
    let index_path = project_root.join("index.json");
    if !index_path.is_file() {
        bail!("index.json not found in {}", project_root.display());
    }
    let text = fs::read_to_string(&index_path)?;
    let val: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse index.json: {e}"))?;
    let Some(obj) = val.as_object() else {
        bail!("index.json root must be a JSON object.");
    };

    let mut form = RuntimeConfigForm::default();
    if let Some(v) = obj.get("windowTitle").and_then(|v| v.as_str()) {
        form.window_title = v.to_string();
    }
    if let Some(v) = obj.get("appName").and_then(|v| v.as_str()) {
        form.app_name = v.to_string();
    }
    if let Some(v) = obj.get("initialSurfaceSize").and_then(|v| v.as_str()) {
        form.initial_surface_size = v.to_string();
    }
    if let Some(v) = obj.get("stageSize").and_then(|v| v.as_str()) {
        form.stage_size = v.to_string();
    }
    if let Some(v) = obj.get("windowState").and_then(|v| v.as_str()) {
        form.window_state = v.to_ascii_lowercase();
    }
    if let Some(v) = obj.get("windowResizable").and_then(|v| v.as_bool()) {
        form.window_resizable = v;
    }
    if let Some(v) = obj.get("backgroundColor").and_then(|v| v.as_str()) {
        form.background_color = v.to_string();
    }
    if let Some(ff) = obj.get("fontFile") {
        if let Some(s) = ff.as_str() {
            form.fonts = vec![FontEntryForm {
                path: s.to_string(),
                alias: String::new(),
                kind: String::new(),
            }];
        } else if let Some(arr) = ff.as_array() {
            let mut entries = Vec::new();
            for item in arr {
                if let Some(s) = item.as_str() {
                    entries.push(FontEntryForm {
                        path: s.to_string(),
                        alias: String::new(),
                        kind: String::new(),
                    });
                } else if let Some(o) = item.as_object() {
                    entries.push(FontEntryForm {
                        path: o
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        alias: o
                            .get("alias")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        kind: o
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_ascii_lowercase(),
                    });
                }
            }
            if !entries.is_empty() {
                form.fonts = entries;
            }
        }
    }
    if let Some(mr) = obj.get("multiResAssets") {
        if let Some(s) = mr.as_str() {
            form.multi_res_mode = if s == "auto" {
                "auto".to_string()
            } else {
                "off".to_string()
            };
        } else if let Some(n) = mr.as_f64() {
            form.multi_res_mode = "fixed".to_string();
            form.multi_res_fixed_scale = format!("{n}");
        }
    }
    if let Some(arr) = obj.get("multiResAssetScales").and_then(|v| v.as_array()) {
        let joined: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_f64())
            .map(|n| format!("{n}"))
            .collect();
        form.multi_res_scales = joined.join(", ");
    }
    if let Some(v) = obj.get("multiResAssetPolicy").and_then(|v| v.as_str()) {
        form.multi_res_policy = v.to_ascii_lowercase();
    }
    if let Some(v) = obj.get("enableMSAA").and_then(|v| v.as_bool()) {
        form.enable_msaa = v;
    }
    if let Some(v) = obj.get("enableMipmaps").and_then(|v| v.as_bool()) {
        form.enable_mipmaps = v;
    }
    if let Some(v) = obj.get("enableGamepads").and_then(|v| v.as_bool()) {
        form.enable_gamepads = v;
    }
    if let Some(v) = obj.get("skipSplash").and_then(|v| v.as_bool()) {
        form.skip_splash = v;
    }
    if let Some(v) = obj.get("showFPS").and_then(|v| v.as_bool()) {
        form.show_fps = v;
    }
    if let Some(steam) = obj.get("steam").and_then(|v| v.as_object()) {
        form.steam_enabled = true;
        if let Some(id) = steam.get("appId").and_then(|v| v.as_u64()) {
            form.steam_app_id = id.to_string();
        }
        if let Some(req) = steam.get("required").and_then(|v| v.as_bool()) {
            form.steam_required = req;
        }
        if let Some(rst) = steam.get("restartThroughClient").and_then(|v| v.as_bool()) {
            form.steam_restart_through_client = rst;
        }
    }
    if let Some(cur) = obj.get("cursor")
        && !cur.is_null()
    {
        if let Some(s) = cur.as_str() {
            form.cursor_enabled = true;
            form.cursors = vec![CursorEntryForm {
                state: "default".to_string(),
                src: s.to_string(),
                hotspot_x: String::new(),
                hotspot_y: String::new(),
            }];
        } else if let Some(map) = cur.as_object() {
            let mut entries = Vec::new();
            for (state, val) in map {
                if let Some(s) = val.as_str() {
                    entries.push(CursorEntryForm {
                        state: state.clone(),
                        src: s.to_string(),
                        hotspot_x: String::new(),
                        hotspot_y: String::new(),
                    });
                } else if let Some(o) = val.as_object() {
                    let src = o
                        .get("src")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let (hx, hy) = o
                        .get("hotspot")
                        .and_then(|v| v.as_array())
                        .filter(|a| a.len() == 2)
                        .map(|a| {
                            (
                                a[0].as_u64().map(|n| n.to_string()).unwrap_or_default(),
                                a[1].as_u64().map(|n| n.to_string()).unwrap_or_default(),
                            )
                        })
                        .unwrap_or_default();
                    entries.push(CursorEntryForm {
                        state: state.clone(),
                        src,
                        hotspot_x: hx,
                        hotspot_y: hy,
                    });
                }
            }
            if !entries.is_empty() {
                // Keep `default` first for a predictable UI order.
                entries.sort_by(|a, b| {
                    (a.state != "default")
                        .cmp(&(b.state != "default"))
                        .then_with(|| a.state.cmp(&b.state))
                });
                form.cursor_enabled = true;
                form.cursors = entries;
            }
        }
    }
    if let Some(v) = obj.get("entryFilename").and_then(|v| v.as_str()) {
        form.entry_filename = v.to_string();
    }
    if let Some(v) = obj.get("entry").and_then(|v| v.as_str()) {
        form.entry = v.to_string();
    }
    if let Some(v) = obj.get("autorun").and_then(|v| v.as_str()) {
        form.autorun = v.to_ascii_lowercase();
    }
    if let Some(v) = obj.get("backend").and_then(|v| v.as_str()) {
        form.backend = v.to_ascii_lowercase();
    }
    if let Some(v) = obj.get("presentMode").and_then(|v| v.as_str()) {
        form.present_mode = v.to_ascii_lowercase();
    }
    if let Some(v) = obj
        .get("desiredMaximumFrameLatency")
        .and_then(|v| v.as_u64())
    {
        form.desired_maximum_frame_latency = v.to_string();
    }
    if let Some(v) = obj.get("params").and_then(|v| v.as_str()) {
        form.params = v.to_string();
    }

    Ok(form)
}

#[allow(dead_code)]
pub fn validate_size_string(value: &str, label: &str) -> Result<()> {
    let trimmed = value.trim();
    let Some((w_str, h_str)) = trimmed.split_once('x').or_else(|| trimmed.split_once('X')) else {
        bail!("{label} must be formatted as <width>x<height> (e.g. 1280x720).");
    };
    let w: u32 = w_str
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("{label} width must be a positive integer."))?;
    let h: u32 = h_str
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("{label} height must be a positive integer."))?;
    if w == 0 || h == 0 {
        bail!("{label} dimensions must be greater than zero.");
    }
    Ok(())
}

/// Write `key` into `obj` only if the key already existed in the file or its value differs from
/// the engine's default — preventing untouched defaults from polluting user config files.
fn upsert_if_changed_or_present(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    val: serde_json::Value,
    default_val: serde_json::Value,
) {
    if obj.contains_key(key) || val != default_val {
        obj.insert(key.to_string(), val);
    }
}

/// Save `index.json` runtime configuration while preserving `$schema` and any unrelated keys.
#[allow(dead_code)]
pub fn save_runtime_config(project_root: &Path, form: &RuntimeConfigForm) -> Result<()> {
    let window_title = form.window_title.trim();
    let app_name = form.app_name.trim();
    let initial_surface_size = form.initial_surface_size.trim();
    let stage_size = form.stage_size.trim();
    let entry_filename = form.entry_filename.trim();
    let bg_color = form.background_color.trim();

    if window_title.is_empty() {
        bail!("Window title cannot be empty.");
    }
    if app_name.is_empty() {
        bail!("App name cannot be empty.");
    }
    if entry_filename.is_empty() {
        bail!("Entry script cannot be empty.");
    }
    validate_size_string(initial_surface_size, "Window size")?;
    validate_size_string(stage_size, "Stage size")?;

    let latency: u32 = form
        .desired_maximum_frame_latency
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Frame latency must be a positive integer (e.g. 2)."))?;
    if latency == 0 {
        bail!("Frame latency must be at least 1.");
    }

    let non_empty_fonts: Vec<&FontEntryForm> = form
        .fonts
        .iter()
        .filter(|f| !f.path.trim().is_empty())
        .collect();
    if non_empty_fonts.is_empty() {
        bail!("At least one font file path is required.");
    }
    let font_val: serde_json::Value = if non_empty_fonts.len() == 1
        && non_empty_fonts[0].alias.trim().is_empty()
        && non_empty_fonts[0].kind.trim().is_empty()
    {
        serde_json::Value::String(non_empty_fonts[0].path.trim().to_string())
    } else {
        let mut arr = Vec::with_capacity(non_empty_fonts.len());
        for f in non_empty_fonts {
            let path = f.path.trim();
            let alias = f.alias.trim();
            let kind = f.kind.trim();
            if alias.is_empty() && kind.is_empty() {
                arr.push(serde_json::Value::String(path.to_string()));
            } else {
                let mut entry = serde_json::Map::new();
                entry.insert("path".into(), serde_json::json!(path));
                if !alias.is_empty() {
                    entry.insert("alias".into(), serde_json::json!(alias));
                }
                if matches!(kind, "cjk" | "western") {
                    entry.insert("kind".into(), serde_json::json!(kind));
                }
                arr.push(serde_json::Value::Object(entry));
            }
        }
        serde_json::Value::Array(arr)
    };

    let multi_res_val = match form.multi_res_mode.as_str() {
        "auto" => serde_json::Value::String("auto".to_string()),
        "fixed" => {
            let n: f64 =
                form.multi_res_fixed_scale.trim().parse().map_err(|_| {
                    anyhow::anyhow!("Fixed multi-res scale must be a positive number.")
                })?;
            if n <= 0.0 {
                bail!("Fixed multi-res scale must be greater than zero.");
            }
            serde_json::json!(n)
        }
        _ => serde_json::Value::String("off".to_string()),
    };

    let mut scales = Vec::new();
    for part in form.multi_res_scales.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        let n: f64 = t
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid multiResAssetScales number: \"{t}\"."))?;
        if n <= 0.0 || (n - 1.0).abs() < f64::EPSILON {
            bail!("multiResAssetScales entries must be positive numbers other than 1.");
        }
        scales.push(serde_json::json!(n));
    }

    let steam_val = if form.steam_enabled {
        let app_id: u32 = form
            .steam_app_id
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Steam App ID must be a valid integer."))?;
        Some(serde_json::json!({
            "appId": app_id,
            "required": form.steam_required,
            "restartThroughClient": form.steam_restart_through_client,
        }))
    } else {
        None
    };

    let cursor_val: Option<serde_json::Value> = if !form.cursor_enabled {
        None
    } else {
        let active: Vec<&CursorEntryForm> = form
            .cursors
            .iter()
            .filter(|c| !c.src.trim().is_empty())
            .collect();
        if active.is_empty() {
            None
        } else if active.len() == 1
            && active[0].state.trim() == "default"
            && active[0].hotspot_x.trim().is_empty()
            && active[0].hotspot_y.trim().is_empty()
        {
            Some(serde_json::Value::String(active[0].src.trim().to_string()))
        } else {
            let mut map = serde_json::Map::new();
            for c in active {
                let state = c.state.trim();
                if state.is_empty() {
                    bail!("Cursor state name cannot be empty (e.g. default, pointer).");
                }
                let src = c.src.trim();
                let hx_str = c.hotspot_x.trim();
                let hy_str = c.hotspot_y.trim();
                if hx_str.is_empty() && hy_str.is_empty() {
                    map.insert(state.to_string(), serde_json::json!(src));
                } else {
                    let hx: u16 = if hx_str.is_empty() {
                        0
                    } else {
                        hx_str.parse().map_err(|_| {
                            anyhow::anyhow!("Cursor \"{state}\" hotspot X must be an integer >= 0.")
                        })?
                    };
                    let hy: u16 = if hy_str.is_empty() {
                        0
                    } else {
                        hy_str.parse().map_err(|_| {
                            anyhow::anyhow!("Cursor \"{state}\" hotspot Y must be an integer >= 0.")
                        })?
                    };
                    map.insert(
                        state.to_string(),
                        serde_json::json!({
                            "src": src,
                            "hotspot": [hx, hy],
                        }),
                    );
                }
            }
            Some(serde_json::Value::Object(map))
        }
    };

    let index_path = project_root.join("index.json");
    if !index_path.is_file() {
        bail!("index.json not found in {}", project_root.display());
    }
    let index_text = fs::read_to_string(&index_path)?;
    let mut index_val: serde_json::Value = serde_json::from_str(&index_text)
        .map_err(|e| anyhow::anyhow!("Failed to parse index.json: {e}"))?;
    let obj = index_val
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("index.json root must be a JSON object."))?;

    let defaults = RuntimeConfigForm::default();
    let effective_bg = if bg_color.is_empty() {
        "transparent"
    } else {
        bg_color
    };

    upsert_if_changed_or_present(
        obj,
        "windowTitle",
        serde_json::json!(window_title),
        serde_json::json!(defaults.window_title),
    );
    upsert_if_changed_or_present(
        obj,
        "appName",
        serde_json::json!(app_name),
        serde_json::json!(defaults.app_name),
    );
    upsert_if_changed_or_present(
        obj,
        "initialSurfaceSize",
        serde_json::json!(initial_surface_size),
        serde_json::json!(defaults.initial_surface_size),
    );
    upsert_if_changed_or_present(
        obj,
        "stageSize",
        serde_json::json!(stage_size),
        serde_json::json!(defaults.stage_size),
    );
    upsert_if_changed_or_present(
        obj,
        "windowState",
        serde_json::json!(form.window_state),
        serde_json::json!(defaults.window_state),
    );
    upsert_if_changed_or_present(
        obj,
        "windowResizable",
        serde_json::json!(form.window_resizable),
        serde_json::json!(defaults.window_resizable),
    );
    upsert_if_changed_or_present(
        obj,
        "backgroundColor",
        serde_json::json!(effective_bg),
        serde_json::json!(defaults.background_color),
    );
    upsert_if_changed_or_present(
        obj,
        "fontFile",
        font_val,
        serde_json::json!("fonts/default.otf"),
    );

    upsert_if_changed_or_present(
        obj,
        "multiResAssets",
        multi_res_val,
        serde_json::json!("off"),
    );
    upsert_if_changed_or_present(
        obj,
        "multiResAssetScales",
        serde_json::Value::Array(scales),
        serde_json::json!([1.5, 2.0]),
    );
    upsert_if_changed_or_present(
        obj,
        "multiResAssetPolicy",
        serde_json::json!(form.multi_res_policy),
        serde_json::json!(defaults.multi_res_policy),
    );

    upsert_if_changed_or_present(
        obj,
        "enableMSAA",
        serde_json::json!(form.enable_msaa),
        serde_json::json!(defaults.enable_msaa),
    );
    upsert_if_changed_or_present(
        obj,
        "enableMipmaps",
        serde_json::json!(form.enable_mipmaps),
        serde_json::json!(defaults.enable_mipmaps),
    );
    upsert_if_changed_or_present(
        obj,
        "enableGamepads",
        serde_json::json!(form.enable_gamepads),
        serde_json::json!(defaults.enable_gamepads),
    );
    upsert_if_changed_or_present(
        obj,
        "skipSplash",
        serde_json::json!(form.skip_splash),
        serde_json::json!(defaults.skip_splash),
    );
    upsert_if_changed_or_present(
        obj,
        "showFPS",
        serde_json::json!(form.show_fps),
        serde_json::json!(defaults.show_fps),
    );

    if let Some(steam) = steam_val {
        obj.insert("steam".into(), steam);
    } else {
        obj.remove("steam");
    }
    if let Some(cursor) = cursor_val {
        obj.insert("cursor".into(), cursor);
    } else {
        obj.remove("cursor");
    }

    upsert_if_changed_or_present(
        obj,
        "entryFilename",
        serde_json::json!(entry_filename),
        serde_json::json!(defaults.entry_filename),
    );
    let entry_redir = form.entry.trim();
    if entry_redir.is_empty() {
        obj.remove("entry");
    } else {
        obj.insert("entry".into(), serde_json::json!(entry_redir));
    }
    upsert_if_changed_or_present(
        obj,
        "autorun",
        serde_json::json!(form.autorun),
        serde_json::json!(defaults.autorun),
    );
    upsert_if_changed_or_present(
        obj,
        "backend",
        serde_json::json!(form.backend),
        serde_json::json!(defaults.backend),
    );
    upsert_if_changed_or_present(
        obj,
        "presentMode",
        serde_json::json!(form.present_mode),
        serde_json::json!(defaults.present_mode),
    );
    upsert_if_changed_or_present(
        obj,
        "desiredMaximumFrameLatency",
        serde_json::json!(latency),
        serde_json::json!(2u32),
    );
    let params = form.params.trim();
    if params.is_empty() {
        obj.remove("params");
    } else {
        obj.insert("params".into(), serde_json::json!(params));
    }

    fs::write(
        &index_path,
        serde_json::to_string_pretty(&index_val)? + "\n",
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn load_engine_project_config(project_root: &Path) -> EngineProjectConfigForm {
    let mut form = EngineProjectConfigForm::default();
    let Ok(text) = fs::read_to_string(project_root.join("moyu.json")) else {
        return form;
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) else {
        return form;
    };
    if let Some(engine) = val.get("engine").and_then(|v| v.as_object()) {
        if let Some(v) = engine.get("channel").and_then(|v| v.as_str()) {
            form.channel = v.to_string();
        }
        if let Some(v) = engine.get("cdnUrl").and_then(|v| v.as_str()) {
            form.cdn_url = v.to_string();
        }
        if let Some(v) = engine.get("version").and_then(|v| v.as_str()) {
            form.version = v.to_string();
        }
    }
    form
}

/// Save `moyu.json`'s `engine` section while preserving `android` and all unrelated keys.
#[allow(dead_code)]
pub fn save_engine_project_config(
    project_root: &Path,
    form: &EngineProjectConfigForm,
) -> Result<()> {
    let channel = form.channel.trim();
    if channel.is_empty() {
        bail!("Engine channel cannot be empty.");
    }
    let moyu_path = project_root.join("moyu.json");
    let mut moyu_val: serde_json::Value = if moyu_path.is_file() {
        let text = fs::read_to_string(&moyu_path)?;
        serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse moyu.json: {e}"))?
    } else {
        serde_json::json!({})
    };
    let moyu_obj = moyu_val
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("moyu.json root must be a JSON object."))?;
    let engine_entry = moyu_obj
        .entry("engine".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !engine_entry.is_object() {
        *engine_entry = serde_json::json!({});
    }
    let engine_obj = engine_entry.as_object_mut().unwrap();
    engine_obj.insert("channel".into(), serde_json::json!(channel));
    let cdn = form.cdn_url.trim();
    if cdn.is_empty() {
        engine_obj.remove("cdnUrl");
    } else {
        engine_obj.insert("cdnUrl".into(), serde_json::json!(cdn));
    }
    let ver = form.version.trim();
    if ver.is_empty() {
        engine_obj.remove("version");
    } else {
        engine_obj.insert("version".into(), serde_json::json!(ver));
    }

    fs::write(&moyu_path, serde_json::to_string_pretty(&moyu_val)? + "\n")?;
    Ok(())
}

#[allow(dead_code)]
pub fn load_project_opened_event(project_root: &Path) -> Result<ProjectEvent> {
    let index = load_index_summary(project_root)?;
    let moyu = load_moyu_summary(project_root);
    let framework = load_framework_summary(project_root);
    Ok(ProjectEvent::Opened {
        root: project_root.to_path_buf(),
        index,
        moyu,
        framework,
    })
}

/// Mirror `crates/resource/src/variant.rs` and `packages/cli/src/utils/multi-res-assets.ts`.
pub fn check_multi_res_assets(project_root: &Path) -> Result<MultiResReport> {
    let assets_dir = project_root.join("assets");
    let variants = scan_variants(&assets_dir)?;
    if variants.is_empty() {
        return Ok(MultiResReport::default());
    }

    let (mode, scales) = read_multi_res_config(project_root);
    let mut report = MultiResReport {
        mode: Some(mode.clone()),
        ..Default::default()
    };

    match &mode {
        MultiResMode::Off => {
            report.all_unused_when_off = variants.len();
        }
        MultiResMode::Auto | MultiResMode::Forced(_) => {
            let used: BTreeSet<u64> = match &mode {
                MultiResMode::Auto => scales.iter().map(|s| s.to_bits()).collect(),
                MultiResMode::Forced(s) => [s.to_bits()].into_iter().collect(),
                MultiResMode::Off => unreachable!(),
            };

            let mut grouped: BTreeMap<u64, (f64, Vec<String>)> = BTreeMap::new();
            for v in &variants {
                if !used.contains(&v.scale.to_bits()) {
                    let entry = grouped
                        .entry(v.scale.to_bits())
                        .or_insert_with(|| (v.scale, Vec::new()));
                    entry.1.push(v.path.clone());
                }
            }
            report.unused_by_scale = grouped.into_values().collect();

            if matches!(mode, MultiResMode::Auto) {
                let present: BTreeSet<u64> = variants.iter().map(|v| v.scale.to_bits()).collect();
                for s in &scales {
                    if !present.contains(&s.to_bits()) {
                        report.undeclared_scales.push(*s);
                    }
                }
            }
        }
    }

    for v in &variants {
        if !assets_dir.join(&v.base).exists() {
            report.missing_base.push((v.path.clone(), v.base.clone()));
        }
    }

    Ok(report)
}

pub fn report_multi_res_warnings(report: &MultiResReport, reporter: &ServiceReporter) {
    if report.all_unused_when_off > 0 {
        let n = report.all_unused_when_off;
        reporter.warn(format!(
            "Found {n} variant file{}, but multiResAssets is \"off\", so only the base assets are used.\nSet \"multiResAssets\": \"auto\" in index.json to use them.",
            if n == 1 { "" } else { "s" }
        ));
    }
    for (scale, files) in &report.unused_by_scale {
        let hint = if (*scale - BASE_SCALE).abs() < f64::EPSILON {
            "the base asset already covers scale 1, so it is never used".to_string()
        } else if matches!(report.mode, Some(MultiResMode::Auto)) {
            "add it to multiResAssetScales in index.json to use them".to_string()
        } else if let Some(MultiResMode::Forced(m)) = report.mode {
            format!("multiResAssets forces scale {m}")
        } else {
            String::new()
        };
        let count = files.len();
        let example = files.first().map(String::as_str).unwrap_or("");
        reporter.warn(format!(
            "Scale {scale} is not tried by the engine ({count} file{}, e.g. {example}): {hint}",
            if count == 1 { "" } else { "s" }
        ));
    }
    if !report.undeclared_scales.is_empty() {
        let joined: Vec<String> = report
            .undeclared_scales
            .iter()
            .map(ToString::to_string)
            .collect();
        let pronoun = if joined.len() == 1 { "it" } else { "them" };
        reporter.info(format!(
            "multiResAssetScales declares {}, but no variant file uses {pronoun}. Every asset will make one extra failed read for each.",
            joined.join(", ")
        ));
    }
    if !report.missing_base.is_empty() {
        let n = report.missing_base.len();
        let pronoun = if n == 1 { "it" } else { "them" };
        let lines: Vec<String> = report
            .missing_base
            .iter()
            .map(|(p, b)| format!("  {p} (expected {b})"))
            .collect();
        reporter.warn(format!(
            "No base asset for {n} variant file{}, so nothing can reference {pronoun}:\n{}",
            if n == 1 { "" } else { "s" },
            lines.join("\n")
        ));
    }
}

fn scan_variants(assets_dir: &Path) -> Result<Vec<VariantFile>> {
    if !assets_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in walkdir::WalkDir::new(assets_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy();
        let Some(scale) = parse_variant_scale(&file_name) else {
            continue;
        };
        let rel = entry
            .path()
            .strip_prefix(assets_dir)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        let base_dir = rel.rfind('/').map(|idx| &rel[..=idx]).unwrap_or("");
        let base = format!("{base_dir}{}", base_name(&file_name));
        found.push(VariantFile {
            path: rel,
            scale,
            base,
        });
    }
    Ok(found)
}

fn read_multi_res_config(project_root: &Path) -> (MultiResMode, Vec<f64>) {
    let index_path = project_root.join("index.json");
    let Ok(text) = fs::read_to_string(index_path) else {
        return (MultiResMode::Off, Vec::new());
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (MultiResMode::Off, Vec::new());
    };

    let mode = match val.get("multiResAssets") {
        Some(v) if v.is_number() => v
            .as_f64()
            .map(MultiResMode::Forced)
            .unwrap_or(MultiResMode::Off),
        Some(v) if v.as_str() == Some("auto") => MultiResMode::Auto,
        _ => MultiResMode::Off,
    };

    let scales: Vec<f64> = match val.get("multiResAssetScales").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_f64())
            .filter(|s| *s > 0.0 && (*s - BASE_SCALE).abs() > f64::EPSILON)
            .collect(),
        None => vec![1.5, 2.0],
    };

    (mode, scales)
}

fn parse_variant_scale(file_name: &str) -> Option<f64> {
    let dot = file_name.rfind('.');
    let stem = dot.map(|i| &file_name[..i]).unwrap_or(file_name);
    let at = stem.rfind('@')?;
    let suffix = &stem[at + 1..];
    let digits = suffix
        .strip_suffix('x')
        .or_else(|| suffix.strip_suffix('X'))?;
    if digits.is_empty() {
        return None;
    }
    // Accept `<digits>` or `<digits>.<digits>` matching the JS regex `/^\d+(\.\d+)?$/`.
    let mut parts = digits.split('.');
    let whole = parts.next()?;
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if let Some(frac) = parts.next()
        && (frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) || parts.next().is_some())
    {
        return None;
    }
    let scale: f64 = digits.parse().ok()?;
    (scale > 0.0).then_some(scale)
}

fn base_name(file_name: &str) -> String {
    let dot = file_name.rfind('.');
    let stem = dot.map(|i| &file_name[..i]).unwrap_or(file_name);
    let ext = dot.map(|i| &file_name[i..]).unwrap_or("");
    let at = stem.rfind('@').unwrap_or(stem.len());
    format!("{}{}", &stem[..at], ext)
}
