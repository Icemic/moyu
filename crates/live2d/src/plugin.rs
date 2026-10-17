//! Shared library client and the engine plugin that drives it.
//!
//! The plugin binary is loaded once per process and pinned to the thread that
//! creates it — the script thread, where the engine polls plugins. Every call
//! below happens on that thread.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::thread::ThreadId;

use moyu_core::traits::{Plugin, PluginBaseTrait};
use moyu_live2d_abi::command::ParameterPair;
use moyu_live2d_abi::error;
use moyu_live2d_abi::frame::FrameDesc;
use moyu_live2d_abi::init::PluginInit;
use moyu_live2d_abi::model::ModelDesc;
use moyu_live2d_abi::query::{
    CanvasInfo, EventDesc, ExpressionInfo, HitAreaInfo, MotionInfo, ParameterInfo, TextureInfo,
};
use moyu_live2d_abi::types::Span;
use moyu_live2d_abi::version;
use moyu_macros::Plugin as PluginDerive;

use crate::job;

/// A call that failed, keeping the contract code the plugin reported.
#[derive(Debug, Clone)]
pub struct PluginError {
    /// `moyu_live2d_abi::error` code.
    pub code: i32,
    /// Message the plugin left in its error buffer, when it left one.
    pub message: String,
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code {})", self.message, self.code)
    }
}

impl std::error::Error for PluginError {}

/// Reports a plugin failure together with the code it returned.
pub type PluginResult<T> = std::result::Result<T, PluginError>;

// ---- entry point signatures ----

type AbiVersionFn = extern "C" fn() -> u32;
type CreateFn = unsafe extern "C" fn(*const PluginInit, *mut *mut c_void) -> i32;
type DestroyFn = unsafe extern "C" fn(*mut c_void) -> i32;
type PluginLastErrorFn = extern "C" fn() -> Span;
type LoadModelFn = unsafe extern "C" fn(*mut c_void, *const ModelDesc, *mut u32) -> i32;
type UnloadModelFn = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type UpdateFn = unsafe extern "C" fn(*mut c_void, u32, f32) -> i32;
type GetFrameFn = unsafe extern "C" fn(*mut c_void, u32, *mut FrameDesc) -> i32;
type SetOptionsFn = unsafe extern "C" fn(*mut c_void, u32, u32) -> i32;
type SetLookTargetFn = unsafe extern "C" fn(*mut c_void, u32, f32, f32) -> i32;
type StartMotionFn = unsafe extern "C" fn(*mut c_void, u32, Span, i32, i32, f32, f32, bool) -> i32;
type StopAllMotionsFn = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type SetExpressionFn = unsafe extern "C" fn(*mut c_void, u32, Span) -> i32;
type StopExpressionFn = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type SetParametersFn = unsafe extern "C" fn(*mut c_void, u32, *const ParameterPair, usize) -> i32;
type ResetParametersFn = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type SetPartOpacityFn = unsafe extern "C" fn(*mut c_void, u32, Span, f32) -> i32;
type HitTestFn = unsafe extern "C" fn(*mut c_void, u32, Span, f32, f32, *mut i32) -> i32;
type QueryCanvasFn = unsafe extern "C" fn(*mut c_void, u32, *mut CanvasInfo) -> i32;
type QueryMotionsFn =
    unsafe extern "C" fn(*mut c_void, u32, i32, *mut MotionInfo, *mut i32) -> i32;
type QueryExpressionsFn =
    unsafe extern "C" fn(*mut c_void, u32, i32, *mut ExpressionInfo, *mut i32) -> i32;
type QueryHitAreasFn =
    unsafe extern "C" fn(*mut c_void, u32, i32, *mut HitAreaInfo, *mut i32) -> i32;
type QueryParametersFn =
    unsafe extern "C" fn(*mut c_void, u32, i32, *mut ParameterInfo, *mut i32) -> i32;
type QueryTexturesFn = unsafe extern "C" fn(*mut c_void, u32, i32, *mut TextureInfo, *mut i32) -> i32;
type PollEventFn = unsafe extern "C" fn(*mut c_void, u32, *mut EventDesc) -> i32;

/// The loaded plugin: library, function table and session handle.
pub struct PluginContext {
    _library: libloading::Library,
    handle: *mut c_void,
    /// The thread that created the context; every call must come from it.
    thread: ThreadId,

    abi_version: AbiVersionFn,
    create: CreateFn,
    /// The session lives for the whole process, so this is never called; it is
    /// loaded to keep the entry point set complete against the contract.
    #[allow(dead_code)]
    destroy: DestroyFn,
    plugin_last_error: PluginLastErrorFn,
    load_model: LoadModelFn,
    unload_model: UnloadModelFn,
    update: UpdateFn,
    get_frame: GetFrameFn,
    set_options: SetOptionsFn,
    set_look_target: SetLookTargetFn,
    start_motion: StartMotionFn,
    stop_all_motions: StopAllMotionsFn,
    set_expression: SetExpressionFn,
    stop_expression: StopExpressionFn,
    set_parameters: SetParametersFn,
    reset_parameters: ResetParametersFn,
    set_part_opacity: SetPartOpacityFn,
    hit_test: HitTestFn,
    query_canvas: QueryCanvasFn,
    query_motions_fn: QueryMotionsFn,
    query_expressions_fn: QueryExpressionsFn,
    query_hit_areas_fn: QueryHitAreasFn,
    query_parameters_fn: QueryParametersFn,
    query_textures_fn: QueryTexturesFn,
    poll_event: PollEventFn,
}

// SAFETY: the context is only ever used from the thread that created it (see
// `PluginContext::thread`); the raw handle never crosses threads.
unsafe impl Send for PluginContext {}
// SAFETY: as above — sharing the type across threads is fine because the only
// thread that ever touches it is the one that created it.
unsafe impl Sync for PluginContext {}

/// Process-wide plugin; `None` when the binary is missing or unusable.
static CONTEXT: OnceLock<Option<PluginContext>> = OnceLock::new();

/// Guards the wrong-thread warning so it is logged only once.
static THREAD_WARNING: std::sync::Once = std::sync::Once::new();

/// Returns the plugin, loading it on first use.
///
/// The first caller becomes the plugin thread: on the engine this is the script
/// thread, because plugins are polled there before any node runs. Calls from any
/// other thread are refused, because the contract does not make the session
/// thread safe.
pub fn context() -> Option<&'static PluginContext> {
    let context = CONTEXT.get_or_init(load).as_ref()?;
    if context.thread != std::thread::current().id() {
        THREAD_WARNING.call_once(|| {
            log::error!(
                "the Live2D plugin was called from another thread than the one that loaded it; \
                 the call is refused"
            );
        });
        return None;
    }
    Some(context)
}

/// The plugin error buffer; used to decorate failed calls.
pub fn last_error() -> String {
    let Some(context) = context() else {
        return "the Live2D plugin is not available".to_owned();
    };
    let span = (context.plugin_last_error)();
    unsafe { span.as_str() }.unwrap_or("").to_owned()
}

/// Generates one two-phase list query: a negative index asks for the count only.
macro_rules! query_list {
    ($name:ident, $entry:ident, $ty:ty) => {
        /// Reads one list entry, or the list count when `index` is negative.
        pub fn $name(&self, model: u32, index: i32) -> PluginResult<($ty, i32)> {
            let mut info = unsafe { std::mem::zeroed::<$ty>() };
            let mut count = 0;
            let out = if index < 0 {
                std::ptr::null_mut()
            } else {
                &mut info
            };
            let code =
                unsafe { (self.$entry)(self.handle, model, index.max(0), out, &mut count) };
            self.check(code)?;
            Ok((info, count))
        }
    };
}

impl PluginContext {
    query_list!(query_motions, query_motions_fn, MotionInfo);
    query_list!(query_expressions, query_expressions_fn, ExpressionInfo);
    query_list!(query_hit_areas, query_hit_areas_fn, HitAreaInfo);
    query_list!(query_parameters, query_parameters_fn, ParameterInfo);
    query_list!(query_textures, query_textures_fn, TextureInfo);
}

impl PluginContext {
    /// Checks a call result, keeping the plugin's error code and message.
    fn check(&self, code: i32) -> PluginResult<()> {
        if code == error::OK {
            return Ok(());
        }
        let span = (self.plugin_last_error)();
        let message = unsafe { span.as_str() }.unwrap_or("").to_owned();
        Err(PluginError {
            code,
            message: if message.is_empty() {
                "the plugin failed without leaving a message".to_owned()
            } else {
                message
            },
        })
    }

    pub fn load_model(&self, path: &str) -> PluginResult<u32> {
        let desc = ModelDesc {
            model_path: Span::from_bytes(path.as_bytes()),
        };
        let mut model = 0u32;
        let code = unsafe { (self.load_model)(self.handle, &desc, &mut model) };
        self.check(code)?;
        Ok(model)
    }

    /// Unloads a model; the caller decides what an error means for it.
    pub fn unload_model(&self, model: u32) -> PluginResult<()> {
        let code = unsafe { (self.unload_model)(self.handle, model) };
        self.check(code)
    }

    pub fn update(&self, model: u32, delta_seconds: f32) -> PluginResult<()> {
        let code = unsafe { (self.update)(self.handle, model, delta_seconds) };
        self.check(code)
    }

    pub fn frame(&self, model: u32) -> PluginResult<FrameDesc> {
        let mut frame = unsafe { std::mem::zeroed::<FrameDesc>() };
        let code = unsafe { (self.get_frame)(self.handle, model, &mut frame) };
        self.check(code)?;
        Ok(frame)
    }

    pub fn set_options(&self, model: u32, options: u32) -> PluginResult<()> {
        let code = unsafe { (self.set_options)(self.handle, model, options) };
        self.check(code)
    }

    pub fn set_look_target(&self, model: u32, x: f32, y: f32) -> PluginResult<()> {
        let code = unsafe { (self.set_look_target)(self.handle, model, x, y) };
        self.check(code)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_motion(
        &self,
        model: u32,
        group: &str,
        index: i32,
        priority: i32,
        fade_in: f32,
        fade_out: f32,
        loop_motion: bool,
    ) -> PluginResult<()> {
        let code = unsafe {
            (self.start_motion)(
                self.handle,
                model,
                Span::from_bytes(group.as_bytes()),
                index,
                priority,
                fade_in,
                fade_out,
                loop_motion,
            )
        };
        self.check(code)
    }

    pub fn stop_all_motions(&self, model: u32) -> PluginResult<()> {
        let code = unsafe { (self.stop_all_motions)(self.handle, model) };
        self.check(code)
    }

    pub fn set_expression(&self, model: u32, name: &str) -> PluginResult<()> {
        let code =
            unsafe { (self.set_expression)(self.handle, model, Span::from_bytes(name.as_bytes())) };
        self.check(code)
    }

    pub fn stop_expression(&self, model: u32) -> PluginResult<()> {
        let code = unsafe { (self.stop_expression)(self.handle, model) };
        self.check(code)
    }

    pub fn set_parameters(&self, model: u32, pairs: &[ParameterPair]) -> PluginResult<()> {
        let code = unsafe { (self.set_parameters)(self.handle, model, pairs.as_ptr(), pairs.len()) };
        self.check(code)
    }

    pub fn reset_parameters(&self, model: u32) -> PluginResult<()> {
        let code = unsafe { (self.reset_parameters)(self.handle, model) };
        self.check(code)
    }

    pub fn set_part_opacity(&self, model: u32, id: &str, value: f32) -> PluginResult<()> {
        let code = unsafe {
            (self.set_part_opacity)(self.handle, model, Span::from_bytes(id.as_bytes()), value)
        };
        self.check(code)
    }

    pub fn hit_test(&self, model: u32, area: &str, x: f32, y: f32) -> PluginResult<bool> {
        let mut hit = 0;
        let code = unsafe {
            (self.hit_test)(
                self.handle,
                model,
                Span::from_bytes(area.as_bytes()),
                x,
                y,
                &mut hit,
            )
        };
        self.check(code)?;
        Ok(hit != 0)
    }

    /// Reads one page of a two-phase list query.
    pub fn canvas(&self, model: u32) -> PluginResult<CanvasInfo> {
        let mut info = unsafe { std::mem::zeroed::<CanvasInfo>() };
        let code = unsafe { (self.query_canvas)(self.handle, model, &mut info) };
        self.check(code)?;
        Ok(info)
    }

    pub fn poll_event(&self, model: u32) -> PluginResult<EventDesc> {
        let mut event = unsafe { std::mem::zeroed::<EventDesc>() };
        let code = unsafe { (self.poll_event)(self.handle, model, &mut event) };
        self.check(code)?;
        Ok(event)
    }
}

/// Loads the plugin binary and creates the session.
fn load() -> Option<PluginContext> {
    let library = load_library()?;

    unsafe {
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                match library.get::<$ty>(concat!($name, "\0").as_bytes()) {
                    Ok(symbol) => *symbol,
                    Err(err) => {
                        log::warn!("The Live2D plugin does not provide {}: {}", $name, err);
                        return None;
                    }
                }
            };
        }

        let context = PluginContext {
            abi_version: symbol!("moyu_live2d_plugin_abi_version", AbiVersionFn),
            create: symbol!("moyu_live2d_plugin_create", CreateFn),
            destroy: symbol!("moyu_live2d_plugin_destroy", DestroyFn),
            plugin_last_error: symbol!("moyu_live2d_plugin_last_error", PluginLastErrorFn),
            load_model: symbol!("moyu_live2d_plugin_load_model", LoadModelFn),
            unload_model: symbol!("moyu_live2d_plugin_unload_model", UnloadModelFn),
            update: symbol!("moyu_live2d_plugin_update", UpdateFn),
            get_frame: symbol!("moyu_live2d_plugin_get_frame", GetFrameFn),
            set_options: symbol!("moyu_live2d_plugin_set_options", SetOptionsFn),
            set_look_target: symbol!("moyu_live2d_plugin_set_look_target", SetLookTargetFn),
            start_motion: symbol!("moyu_live2d_plugin_start_motion", StartMotionFn),
            stop_all_motions: symbol!("moyu_live2d_plugin_stop_all_motions", StopAllMotionsFn),
            set_expression: symbol!("moyu_live2d_plugin_set_expression", SetExpressionFn),
            stop_expression: symbol!("moyu_live2d_plugin_stop_expression", StopExpressionFn),
            set_parameters: symbol!("moyu_live2d_plugin_set_parameters", SetParametersFn),
            reset_parameters: symbol!("moyu_live2d_plugin_reset_parameters", ResetParametersFn),
            set_part_opacity: symbol!("moyu_live2d_plugin_set_part_opacity", SetPartOpacityFn),
            hit_test: symbol!("moyu_live2d_plugin_hit_test", HitTestFn),
            query_canvas: symbol!("moyu_live2d_plugin_query_canvas", QueryCanvasFn),
            query_motions_fn: symbol!("moyu_live2d_plugin_query_motions", QueryMotionsFn),
            query_expressions_fn: symbol!(
                "moyu_live2d_plugin_query_expressions",
                QueryExpressionsFn
            ),
            query_hit_areas_fn: symbol!("moyu_live2d_plugin_query_hit_areas", QueryHitAreasFn),
            query_parameters_fn: symbol!(
                "moyu_live2d_plugin_query_parameters",
                QueryParametersFn
            ),
            query_textures_fn: symbol!("moyu_live2d_plugin_query_textures", QueryTexturesFn),
            poll_event: symbol!("moyu_live2d_plugin_poll_event", PollEventFn),
            handle: std::ptr::null_mut(),
            thread: std::thread::current().id(),
            _library: library,
        };

        let version = (context.abi_version)();
        if !version::is_compatible(version) {
            log::warn!(
                "Live2D plugin ABI {}.{} does not match the engine's {}.{}; live2d nodes stay unavailable",
                version >> 16,
                version & 0xFFFF,
                version::ABI_VERSION >> 16,
                version::ABI_VERSION & 0xFFFF,
            );
            return None;
        }

        let init = PluginInit {
            read_file: Some(read_asset),
            user_data: std::ptr::null_mut(),
            core_path: Span::EMPTY,
        };
        let mut handle: *mut c_void = std::ptr::null_mut();
        let code = (context.create)(&init, &mut handle);
        let mut context = context;
        if let Err(err) = context.check(code) {
            log::warn!("Failed to create the Live2D plugin session: {err}");
            return None;
        }
        context.handle = handle;

        // The session lives for the whole process; the plugin is never destroyed
        // here, so models and everything the plugin loaded at start-up stay valid.
        log::info!("Live2D plugin loaded (ABI {}.{})", version >> 16, version & 0xFFFF);
        Some(context)
    }
}

/// Name the plugin binary carries on this platform.
#[cfg(target_os = "windows")]
const LIBRARY_NAME: &str = "moyu_live2d_plugin.dll";
/// Name the plugin binary carries on this platform.
#[cfg(target_os = "macos")]
const LIBRARY_NAME: &str = "libmoyu_live2d_plugin.dylib";
/// Name the plugin binary carries on this platform.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const LIBRARY_NAME: &str = "libmoyu_live2d_plugin.so";

/// Loads the plugin binary: `MOYU_LIVE2D_PLUGIN` first, then next to the
/// executable and in the working directory, then the system search path.
fn load_library() -> Option<libloading::Library> {
    if let Ok(path) = std::env::var("MOYU_LIVE2D_PLUGIN")
        && !path.is_empty()
    {
        match unsafe { libloading::Library::new(&path) } {
            Ok(library) => {
                log::info!("Loading Live2D plugin from: {path}");
                return Some(library);
            }
            Err(err) => {
                log::warn!("failed to load MOYU_LIVE2D_PLUGIN at {path}: {err}");
                return None;
            }
        }
    }

    let mut candidates: Vec<String> = Vec::new();
    if let Ok(executable) = std::env::current_exe()
        && let Some(dir) = executable.parent()
    {
        candidates.push(dir.join(LIBRARY_NAME).to_string_lossy().into_owned());
    }
    if let Ok(dir) = std::env::current_dir() {
        candidates.push(dir.join(LIBRARY_NAME).to_string_lossy().into_owned());
    }
    // The bare name comes last, so the loader's own search path can find it too.
    candidates.push(LIBRARY_NAME.to_owned());

    let mut last_error = String::new();
    for candidate in candidates {
        match unsafe { libloading::Library::new(&candidate) } {
            Ok(library) => {
                log::info!("Loading Live2D plugin from: {candidate}");
                return Some(library);
            }
            Err(err) => last_error = err.to_string(),
        }
    }

    log::info!("the Live2D plugin was not found ({last_error}); live2d nodes report an error");
    None
}

/// Host file access for the plugin: asset paths are read through the engine's
/// asset directory, blocking the script thread for the duration.
///
/// # Safety
///
/// Called by the plugin with the contract's two-phase semantics.
unsafe extern "C" fn read_asset(
    _user_data: *mut c_void,
    path: Span,
    offset: u64,
    buffer: *mut u8,
    inout_len: *mut usize,
) -> i32 {
    let Some(path) = (unsafe { path.as_str() }) else {
        return error::INVALID_ARGUMENT;
    };
    let Ok(url) = moyu_pal::dir::assets_dir().join(path) else {
        return error::INVALID_ARGUMENT;
    };
    let Ok(data) = moyu_pal::task::block_on_without_runtime(moyu_pal::fs::read(&url)) else {
        return error::FILE;
    };

    let offset = offset as usize;
    if offset > data.len() {
        return error::FILE;
    }
    let remaining = &data[offset..];

    let len = unsafe { &mut *inout_len };
    if buffer.is_null() {
        *len = remaining.len();
    } else {
        let copied = (*len).min(remaining.len());
        unsafe { std::ptr::copy_nonoverlapping(remaining.as_ptr(), buffer, copied) };
        *len = copied;
    }
    error::OK
}

/// The engine plugin: polls the loaded models once per frame on the script
/// thread. The engine calls it before it ticks the script runtime, so every
/// Live2D call in the process happens here or in a node command called from the
/// same thread.
#[derive(PluginDerive)]
pub struct Live2dPlugin;

impl Plugin for Live2dPlugin {
    fn plugin_name(&self) -> &'static str {
        "live2d"
    }

    fn update(&mut self, vsync: bool) {
        // The idle poll only keeps commands serviced; frames come with vsync.
        if !vsync {
            return;
        }
        job::drive_frame();
    }
}
