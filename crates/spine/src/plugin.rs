//! Shared library client and the engine plugin that drives it.
//!
//! The plugin binary is loaded once per process and pinned to the thread that
//! creates it — the script thread, where the engine polls plugins. Every call
//! below happens on that thread.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::thread::ThreadId;

use moyu_core::traits::{Plugin, PluginBaseTrait};
use moyu_macros::Plugin as PluginDerive;
use moyu_spine_abi::control::{HitDesc, PHYSICS_UPDATE, TrackDesc, UpdateDesc};
use moyu_spine_abi::error;
use moyu_spine_abi::event::EventDesc;
use moyu_spine_abi::frame::FrameDesc;
use moyu_spine_abi::init::PluginInit;
use moyu_spine_abi::page::PageDesc;
use moyu_spine_abi::query::{AnimationInfo, BoundingBoxInfo, ModelInfo, PluginInfo, SkinInfo};
use moyu_spine_abi::types::Span;
use moyu_spine_abi::version;

/// A call that failed, keeping the contract code the plugin reported.
#[derive(Debug, Clone)]
pub struct PluginError {
    /// `moyu_spine_abi::error` code.
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
type CapsFn = extern "C" fn() -> u32;
type InfoFn = unsafe extern "C" fn(*mut PluginInfo) -> i32;
type CreateFn = unsafe extern "C" fn(*const PluginInit, *mut *mut c_void) -> i32;
type DestroyFn = unsafe extern "C" fn(*mut c_void) -> i32;
type LastErrorFn = extern "C" fn() -> Span;
type LoadModelFn = unsafe extern "C" fn(Span, *const u8, usize, Span, *const u8, usize, *mut u32) -> i32;
type UnloadModelFn = unsafe extern "C" fn(u32) -> i32;
type ModelInfoFn = unsafe extern "C" fn(u32, *mut ModelInfo) -> i32;
type QueryCountFn = unsafe extern "C" fn(u32, i32, *mut i32) -> i32;
type QueryAnimationFn = unsafe extern "C" fn(u32, i32, *mut AnimationInfo) -> i32;
type QuerySkinFn = unsafe extern "C" fn(u32, i32, *mut SkinInfo) -> i32;
type QueryBoundingBoxFn = unsafe extern "C" fn(u32, i32, *mut BoundingBoxInfo) -> i32;
type QueryPageFn = unsafe extern "C" fn(u32, i32, *mut PageDesc) -> i32;
type UpdateFn = unsafe extern "C" fn(u32, *const UpdateDesc) -> i32;
type FrameFn = unsafe extern "C" fn(u32, *mut FrameDesc) -> i32;
type ClearTrackFn = unsafe extern "C" fn(u32, i32, f32) -> i32;
type ClearTracksFn = unsafe extern "C" fn(u32) -> i32;
type SetDefaultMixFn = unsafe extern "C" fn(u32, f32) -> i32;
type ResetToSetupPoseFn = unsafe extern "C" fn(u32) -> i32;
type ResetPhysicsFn = unsafe extern "C" fn(u32) -> i32;
type SetMixFn = unsafe extern "C" fn(u32, Span, Span, f32) -> i32;
type SetSkinFn = unsafe extern "C" fn(u32, Span) -> i32;
type SetAttachmentFn = unsafe extern "C" fn(u32, Span, Span) -> i32;
type TrackFn = unsafe extern "C" fn(u32, i32, *const TrackDesc, *mut u64) -> i32;
type PollEventFn = unsafe extern "C" fn(u32, *mut EventDesc) -> i32;
type HitTestFn = unsafe extern "C" fn(u32, f32, f32, *mut HitDesc) -> i32;

/// The loaded plugin: library, function table and session handle.
pub struct PluginContext {
    _library: libloading::Library,
    handle: *mut c_void,
    /// The thread that created the context; every call must come from it.
    thread: ThreadId,

    abi_version: AbiVersionFn,
    caps: CapsFn,
    info: InfoFn,
    create: CreateFn,
    /// The session lives for the whole process, so this is never called; it is
    /// loaded to keep the entry point set complete against the contract.
    #[allow(dead_code)]
    destroy: DestroyFn,
    last_error: LastErrorFn,
    load_model: LoadModelFn,
    unload_model: UnloadModelFn,
    model_info: ModelInfoFn,
    query_count: QueryCountFn,
    query_animation: QueryAnimationFn,
    query_skin: QuerySkinFn,
    query_bounding_box: QueryBoundingBoxFn,
    query_page: QueryPageFn,
    update: UpdateFn,
    frame: FrameFn,
    clear_track: ClearTrackFn,
    clear_tracks: ClearTracksFn,
    set_default_mix: SetDefaultMixFn,
    reset_to_setup_pose: ResetToSetupPoseFn,
    reset_physics: ResetPhysicsFn,
    set_mix: SetMixFn,
    set_skin: SetSkinFn,
    set_attachment: SetAttachmentFn,
    set_animation: TrackFn,
    add_animation: TrackFn,
    poll_event: PollEventFn,
    hit_test: HitTestFn,
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
                "the Spine plugin was called from another thread than the one that loaded it; \
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
        return "the Spine plugin is not available".to_owned();
    };
    let span = (context.last_error)();
    unsafe { span.as_str() }.unwrap_or("").to_owned()
}

impl PluginContext {
    /// Checks a call result, keeping the plugin's error code and message.
    fn check(&self, code: i32) -> PluginResult<()> {
        if code == error::OK {
            return Ok(());
        }
        let span = (self.last_error)();
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

    pub fn abi_version(&self) -> u32 {
        (self.abi_version)()
    }

    pub fn caps(&self) -> u32 {
        (self.caps)()
    }

    /// Reads the plugin build information.
    pub fn info(&self) -> PluginResult<PluginInfo> {
        let mut info = unsafe { std::mem::zeroed::<PluginInfo>() };
        self.check(unsafe { (self.info)(&mut info) })?;
        Ok(info)
    }

    /// Loads a model from its skeleton and atlas bytes.
    ///
    /// Paths are only used to derive sibling paths and to report page locations;
    /// the bytes are what gets parsed.
    pub fn load_model(
        &self,
        skeleton_path: &str,
        skeleton: &[u8],
        atlas_path: &str,
        atlas: &[u8],
    ) -> PluginResult<u32> {
        let mut model = 0u32;
        let code = unsafe {
            (self.load_model)(
                Span::from_bytes(skeleton_path.as_bytes()),
                skeleton.as_ptr(),
                skeleton.len(),
                Span::from_bytes(atlas_path.as_bytes()),
                atlas.as_ptr(),
                atlas.len(),
                &mut model,
            )
        };
        self.check(code)?;
        Ok(model)
    }

    /// Unloads a model; the caller decides what an error means for it.
    pub fn unload_model(&self, model: u32) -> PluginResult<()> {
        self.check(unsafe { (self.unload_model)(model) })
    }

    pub fn model_info(&self, model: u32) -> PluginResult<ModelInfo> {
        let mut info = unsafe { std::mem::zeroed::<ModelInfo>() };
        self.check(unsafe { (self.model_info)(model, &mut info) })?;
        Ok(info)
    }

    /// Reads the entry count of one of the model's lists.
    fn count(&self, model: u32, list: i32) -> PluginResult<i32> {
        let mut count = 0i32;
        self.check(unsafe { (self.query_count)(model, list, &mut count) })?;
        Ok(count)
    }

    pub fn animations(&self, model: u32) -> PluginResult<Vec<AnimationInfo>> {
        let count = self.count(model, LIST_ANIMATIONS)?;
        let mut out = Vec::with_capacity(count.max(0) as usize);
        for index in 0..count {
            let mut info = unsafe { std::mem::zeroed::<AnimationInfo>() };
            self.check(unsafe { (self.query_animation)(model, index, &mut info) })?;
            out.push(info);
        }
        Ok(out)
    }

    pub fn skins(&self, model: u32) -> PluginResult<Vec<SkinInfo>> {
        let count = self.count(model, LIST_SKINS)?;
        let mut out = Vec::with_capacity(count.max(0) as usize);
        for index in 0..count {
            let mut info = unsafe { std::mem::zeroed::<SkinInfo>() };
            self.check(unsafe { (self.query_skin)(model, index, &mut info) })?;
            out.push(info);
        }
        Ok(out)
    }

    pub fn bounding_boxes(&self, model: u32) -> PluginResult<Vec<BoundingBoxInfo>> {
        let count = self.count(model, LIST_BOUNDING_BOXES)?;
        let mut out = Vec::with_capacity(count.max(0) as usize);
        for index in 0..count {
            let mut info = unsafe { std::mem::zeroed::<BoundingBoxInfo>() };
            self.check(unsafe { (self.query_bounding_box)(model, index, &mut info) })?;
            out.push(info);
        }
        Ok(out)
    }

    pub fn pages(&self, model: u32) -> PluginResult<Vec<PageDesc>> {
        let count = self.count(model, LIST_PAGES)?;
        let mut out = Vec::with_capacity(count.max(0) as usize);
        for index in 0..count {
            let mut info = unsafe { std::mem::zeroed::<PageDesc>() };
            self.check(unsafe { (self.query_page)(model, index, &mut info) })?;
            out.push(info);
        }
        Ok(out)
    }

    pub fn update(&self, model: u32, delta: f32, time_scale: f32, paused: bool) -> PluginResult<()> {
        let desc = UpdateDesc {
            delta,
            time_scale,
            paused: i32::from(paused),
            physics: PHYSICS_UPDATE,
        };
        self.check(unsafe { (self.update)(model, &desc) })
    }

    /// Fetches the newest frame; the description is valid until the next call.
    pub fn frame(&self, model: u32) -> PluginResult<FrameDesc> {
        let mut frame = unsafe { std::mem::zeroed::<FrameDesc>() };
        self.check(unsafe { (self.frame)(model, &mut frame) })?;
        Ok(frame)
    }

    pub fn clear_track(&self, model: u32, track: i32, mix: f32) -> PluginResult<()> {
        self.check(unsafe { (self.clear_track)(model, track, mix) })
    }

    pub fn clear_tracks(&self, model: u32) -> PluginResult<()> {
        self.check(unsafe { (self.clear_tracks)(model) })
    }

    pub fn set_default_mix(&self, model: u32, duration: f32) -> PluginResult<()> {
        self.check(unsafe { (self.set_default_mix)(model, duration) })
    }

    pub fn reset_to_setup_pose(&self, model: u32) -> PluginResult<()> {
        self.check(unsafe { (self.reset_to_setup_pose)(model) })
    }

    pub fn reset_physics(&self, model: u32) -> PluginResult<()> {
        self.check(unsafe { (self.reset_physics)(model) })
    }

    pub fn set_mix(&self, model: u32, from: &str, to: &str, duration: f32) -> PluginResult<()> {
        self.check(unsafe {
            (self.set_mix)(
                model,
                Span::from_bytes(from.as_bytes()),
                Span::from_bytes(to.as_bytes()),
                duration,
            )
        })
    }

    pub fn set_skin(&self, model: u32, name: &str) -> PluginResult<()> {
        self.check(unsafe { (self.set_skin)(model, Span::from_bytes(name.as_bytes())) })
    }

    pub fn set_attachment(&self, model: u32, slot: &str, name: &str) -> PluginResult<()> {
        self.check(unsafe {
            (self.set_attachment)(
                model,
                Span::from_bytes(slot.as_bytes()),
                Span::from_bytes(name.as_bytes()),
            )
        })
    }

    /// Replaces whatever runs on `track`; returns the identity of the new entry.
    pub fn set_animation(
        &self,
        model: u32,
        track: i32,
        animation: &str,
        loop_animation: bool,
        mix: f32,
        delay: f32,
        time: f32,
    ) -> PluginResult<u64> {
        self.track_call(
            self.set_animation,
            model,
            track,
            animation,
            loop_animation,
            mix,
            delay,
            time,
        )
    }

    /// Queues an animation behind whatever runs on `track`.
    pub fn add_animation(
        &self,
        model: u32,
        track: i32,
        animation: &str,
        loop_animation: bool,
        mix: f32,
        delay: f32,
        time: f32,
    ) -> PluginResult<u64> {
        self.track_call(
            self.add_animation,
            model,
            track,
            animation,
            loop_animation,
            mix,
            delay,
            time,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn track_call(
        &self,
        call: TrackFn,
        model: u32,
        track: i32,
        animation: &str,
        loop_animation: bool,
        mix: f32,
        delay: f32,
        time: f32,
    ) -> PluginResult<u64> {
        let desc = TrackDesc {
            animation: Span::from_bytes(animation.as_bytes()),
            loop_animation: i32::from(loop_animation),
            mix,
            delay,
            time,
        };
        let mut entry = 0u64;
        self.check(unsafe { call(model, track, &desc, &mut entry) })?;
        Ok(entry)
    }

    /// Reads the next queued event; `kind == 0` means the queue is empty.
    ///
    /// Strings in the returned description are only valid until the next call,
    /// so the caller has to copy them right away.
    pub fn poll_event(&self, model: u32) -> PluginResult<EventDesc> {
        let mut event = unsafe { std::mem::zeroed::<EventDesc>() };
        self.check(unsafe { (self.poll_event)(model, &mut event) })?;
        Ok(event)
    }

    /// Tests a point in world units against the model's bounding boxes.
    pub fn hit_test(&self, model: u32, x: f32, y: f32) -> PluginResult<Option<String>> {
        let mut hit = unsafe { std::mem::zeroed::<HitDesc>() };
        self.check(unsafe { (self.hit_test)(model, x, y, &mut hit) })?;
        Ok(unsafe { hit.name.as_str() }.map(str::to_owned))
    }
}

/// List identifiers the plugin's `query_count` accepts.
const LIST_ANIMATIONS: i32 = 0;
const LIST_SKINS: i32 = 1;
const LIST_PAGES: i32 = 2;
const LIST_BOUNDING_BOXES: i32 = 3;

/// Loads the plugin binary and creates the session.
fn load() -> Option<PluginContext> {
    let library = load_library()?;

    unsafe {
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                match library.get::<$ty>(concat!($name, "\0").as_bytes()) {
                    Ok(symbol) => *symbol,
                    Err(err) => {
                        log::warn!("The Spine plugin does not provide {}: {}", $name, err);
                        return None;
                    }
                }
            };
        }

        let context = PluginContext {
            abi_version: symbol!("moyu_spine_plugin_abi_version", AbiVersionFn),
            caps: symbol!("moyu_spine_plugin_caps", CapsFn),
            info: symbol!("moyu_spine_plugin_info", InfoFn),
            create: symbol!("moyu_spine_plugin_create", CreateFn),
            destroy: symbol!("moyu_spine_plugin_destroy", DestroyFn),
            last_error: symbol!("moyu_spine_plugin_last_error", LastErrorFn),
            load_model: symbol!("moyu_spine_plugin_load_model", LoadModelFn),
            unload_model: symbol!("moyu_spine_plugin_unload_model", UnloadModelFn),
            model_info: symbol!("moyu_spine_plugin_model_info", ModelInfoFn),
            query_count: symbol!("moyu_spine_plugin_query_count", QueryCountFn),
            query_animation: symbol!("moyu_spine_plugin_query_animation", QueryAnimationFn),
            query_skin: symbol!("moyu_spine_plugin_query_skin", QuerySkinFn),
            query_bounding_box: symbol!("moyu_spine_plugin_query_bounding_box", QueryBoundingBoxFn),
            query_page: symbol!("moyu_spine_plugin_query_page", QueryPageFn),
            update: symbol!("moyu_spine_plugin_update", UpdateFn),
            frame: symbol!("moyu_spine_plugin_frame", FrameFn),
            clear_track: symbol!("moyu_spine_plugin_clear_track", ClearTrackFn),
            clear_tracks: symbol!("moyu_spine_plugin_clear_tracks", ClearTracksFn),
            set_default_mix: symbol!("moyu_spine_plugin_set_default_mix", SetDefaultMixFn),
            reset_to_setup_pose: symbol!("moyu_spine_plugin_reset_to_setup_pose", ResetToSetupPoseFn),
            reset_physics: symbol!("moyu_spine_plugin_reset_physics", ResetPhysicsFn),
            set_mix: symbol!("moyu_spine_plugin_set_mix", SetMixFn),
            set_skin: symbol!("moyu_spine_plugin_set_skin", SetSkinFn),
            set_attachment: symbol!("moyu_spine_plugin_set_attachment", SetAttachmentFn),
            set_animation: symbol!("moyu_spine_plugin_set_animation", TrackFn),
            add_animation: symbol!("moyu_spine_plugin_add_animation", TrackFn),
            poll_event: symbol!("moyu_spine_plugin_poll_event", PollEventFn),
            hit_test: symbol!("moyu_spine_plugin_hit_test", HitTestFn),
            handle: std::ptr::null_mut(),
            thread: std::thread::current().id(),
            _library: library,
        };

        let version = (context.abi_version)();
        if !version::is_compatible(version) {
            log::warn!(
                "Spine plugin ABI {}.{} does not match the engine's {}.{}; spine nodes stay unavailable",
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
        };
        let mut handle: *mut c_void = std::ptr::null_mut();
        let code = (context.create)(&init, &mut handle);
        let mut context = context;
        if let Err(err) = context.check(code) {
            log::warn!("Failed to create the Spine plugin session: {err}");
            return None;
        }
        context.handle = handle;

        match context.info() {
            Ok(info) => log::info!(
                "Spine plugin loaded (ABI {}.{}, runtime {} rev {})",
                version >> 16,
                version & 0xFFFF,
                info.runtime_version.as_str().unwrap_or("?"),
                info.runtime_revision.as_str().unwrap_or("?"),
            ),
            Err(err) => log::warn!("Spine plugin has no build information: {err}"),
        }
        Some(context)
    }
}

/// Name the plugin binary carries on this platform.
#[cfg(target_os = "windows")]
const LIBRARY_NAME: &str = "moyu_spine_plugin.dll";
/// Name the plugin binary carries on this platform.
#[cfg(target_os = "macos")]
const LIBRARY_NAME: &str = "libmoyu_spine_plugin.dylib";
/// Name the plugin binary carries on this platform.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const LIBRARY_NAME: &str = "libmoyu_spine_plugin.so";

/// Loads the plugin binary: `MOYU_SPINE_PLUGIN` first, then next to the
/// executable and in the working directory, then the system search path.
fn load_library() -> Option<libloading::Library> {
    if let Ok(path) = std::env::var("MOYU_SPINE_PLUGIN")
        && !path.is_empty()
    {
        match unsafe { libloading::Library::new(&path) } {
            Ok(library) => {
                log::info!("Loading Spine plugin from: {path}");
                return Some(library);
            }
            Err(err) => {
                log::warn!("failed to load MOYU_SPINE_PLUGIN at {path}: {err}");
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
                log::info!("Loading Spine plugin from: {candidate}");
                return Some(library);
            }
            Err(err) => last_error = err.to_string(),
        }
    }

    log::info!("the Spine plugin was not found ({last_error}); spine nodes report an error");
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
/// thread.
///
/// The engine calls it before it ticks the script runtime, so every Spine call
/// in the process happens here or in a node command called from the same thread.
#[derive(PluginDerive)]
pub struct SpinePlugin;

impl Plugin for SpinePlugin {
    fn plugin_name(&self) -> &'static str {
        "spine"
    }

    fn update(&mut self, vsync: bool) {
        // The idle poll only keeps commands serviced; frames come with vsync.
        if !vsync {
            return;
        }
        crate::job::drive_frame();
    }
}
