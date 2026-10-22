//! Per-node driver state and the once-per-frame plugin pass.
//!
//! A job is the shared state of one `spine` node: the node writes what it wants
//! (props, commands), [`drive_frame`] applies it to the plugin and leaves a copy
//! of the newest frame for the renderer. The driver runs on the script thread,
//! the renderer on the graphics thread, and they meet here.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Weak};

use moyu_core::events::CustomEvent;
use moyu_core::traits::Event;
use moyu_core::utils::dispatch_event::dispatch_event_async;
use moyu_pal::sync::Mutex;

use crate::event::SpineEvent;use crate::frame::FrameSnapshot;

#[cfg(native)]
use crate::event::{AlphaSource, EntryIdentity, SpineAnimation, SpineBounds, SpineMetadata, SpinePage};
#[cfg(native)]
use crate::plugin::{self, PluginContext, PluginError, PluginResult};
#[cfg(native)]
use moyu_spine_abi::event::{
    EVENT_COMPLETE, EVENT_DISPOSE, EVENT_END, EVENT_INTERRUPT, EVENT_START, EVENT_USER,
};
#[cfg(native)]
use moyu_spine_abi::page::ALPHA_PREMULTIPLIED;

/// Longest time step handed to the plugin, so a stalled window does not make a
/// model jump.
#[cfg(native)]
const MAX_DELTA_SECONDS: f32 = 0.25;

/// Epoch of a node without a loaded model.
pub const NO_MODEL_EPOCH: u64 = u64::MAX;

/// Shared state of one `spine` node.
pub struct NodeJob {
    /// The node this job belongs to.
    pub node_id: u32,
    /// Model id the plugin has loaded for this node, 0 while unloaded.
    ///
    /// Mirrored outside the state lock so a dying node can still tell which
    /// model it leaves behind; every write happens with the state lock held.
    model: Arc<AtomicU32>,
    pub state: Mutex<JobState>,
}

impl NodeJob {
    /// The model the plugin has loaded for this node, 0 while unloaded.
    pub fn model(&self) -> u32 {
        self.model.load(Ordering::SeqCst)
    }

    /// Publishes a model id to the job and to the registry entry.
    #[cfg(native)]
    fn set_model(&self, state: &mut JobState, model: u32) {
        state.model = model;
        self.model.store(model, Ordering::SeqCst);
    }
}

impl std::fmt::Debug for NodeJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeJob")
            .field("node_id", &self.node_id)
            .finish_non_exhaustive()
    }
}

/// Everything the renderer needs to know about the loaded model.
///
/// The whole record is replaced when the model changes, so the renderer can
/// never pair a frame with another model's resources.
pub struct ModelInfo {
    /// Bumped on every model change, including unloading.
    pub epoch: u64,
    /// Texture page paths, relative to the asset root.
    pub pages: Vec<PageInfo>,
    /// Reference bounds in world units, with y up.
    pub bounds: [f32; 4],
    /// Bounding box names, used by the built-in hit test.
    pub bounding_boxes: Vec<String>,
}

/// One texture page of a loaded model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageInfo {
    pub path: String,
    /// Whether the image already carries premultiplied alpha.
    pub premultiplied: bool,
    /// Sampling state the atlas declared, one of `moyu_spine_abi::page::*`.
    pub sampling: Sampling,
}

/// Sampling state an atlas page declares.
///
/// Stored as the contract constants so the renderer can map them straight onto
/// wgpu without a second enum in between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sampling {
    /// `PAGE_FILTER_*` for minification.
    pub min_filter: i32,
    /// `PAGE_FILTER_*` for magnification.
    pub mag_filter: i32,
    /// `PAGE_WRAP_*` horizontally.
    pub wrap_u: i32,
    /// `PAGE_WRAP_*` vertically.
    pub wrap_v: i32,
}

impl Default for Sampling {
    fn default() -> Self {
        // Linear filtering with clamped edges is what every sample atlas uses and
        // the safest fallback for one that declares nothing.
        Self {
            min_filter: moyu_spine_abi::page::PAGE_FILTER_LINEAR,
            mag_filter: moyu_spine_abi::page::PAGE_FILTER_LINEAR,
            wrap_u: moyu_spine_abi::page::PAGE_WRAP_CLAMP,
            wrap_v: moyu_spine_abi::page::PAGE_WRAP_CLAMP,
        }
    }
}

/// What the node wants and what the plugin currently has.
///
/// The plugin driver only exists on native platforms, so its bookkeeping is dead
/// code in a web build.
#[cfg_attr(web, allow(dead_code))]
pub struct JobState {
    /// Skeleton path wanted by the props.
    pub src: Option<String>,
    /// Atlas path wanted by the props; empty means "next to the skeleton".
    pub atlas: Option<String>,
    /// Skin wanted by the props; empty means the model's default skin.
    pub skin: String,
    /// Speed multiplier wanted by the props.
    pub time_scale: f32,
    /// Whether time should stop.
    pub paused: bool,
    /// Physics mode wanted by the props: 0 none, 1 pose, 2 update.
    pub physics: i32,
    /// Plugin model id; 0 while unloaded.
    pub model: u32,
    /// Identity of the loaded model; `None` while unloaded.
    pub model_info: Option<Arc<ModelInfo>>,
    /// Latest frame, taken by the renderer.
    pub snapshot: Option<FrameSnapshot>,
    /// Bounding boxes of the model, mirrored for the hit test.
    pub bounding_boxes: Vec<String>,

    /// Skeleton path currently loaded.
    loaded_src: Option<String>,
    /// Atlas path currently loaded.
    loaded_atlas: Option<String>,
    /// Model path whose load failed; not retried until the props change.
    failed_src: Option<String>,
    /// Skin the plugin has applied.
    applied_skin: Option<String>,
    /// Physics mode the plugin has.
    applied_physics: i32,
    /// Last model step, for the frame delta.
    last_time: Option<moyu_pal::time::Instant>,
    /// Counting epochs; only ever grows, so a reload is never mistaken for the
    /// model it replaces.
    epoch_counter: u64,
    /// Entries the plugin reported as started but not yet disposed, keyed by
    /// entry id and holding the track and animation name.
    ///
    /// A model reload drops these without the plugin ever sending `dispose` for
    /// them, so the host replays the terminal event itself; otherwise a caller
    /// awaiting a specific playback would wait forever across a reload.
    live_entries: std::collections::HashMap<u64, (i32, String)>,
    /// Whether a failure was already reported for the current state.
    error_reported: bool,
}

impl JobState {
    /// The epoch of the model currently loaded, for stamping command results.
    pub fn epoch(&self) -> u64 {
        self.epoch_counter
    }

    fn new() -> Self {
        Self {
            src: None,
            atlas: None,
            skin: String::new(),
            time_scale: 1.0,
            paused: false,
            physics: PHYSICS_UPDATE_DEFAULT,
            model: 0,
            model_info: None,
            snapshot: None,
            bounding_boxes: Vec::new(),
            loaded_src: None,
            loaded_atlas: None,
            failed_src: None,
            applied_skin: None,
            applied_physics: -1,
            last_time: None,
            epoch_counter: 0,
            live_entries: std::collections::HashMap::new(),
            error_reported: false,
        }
    }
}

/// Physics mode the node starts with: the runtime updates its constraints.
const PHYSICS_UPDATE_DEFAULT: i32 = 2;

impl Default for JobState {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry entry of a node: its job plus the model to unload when it dies.
struct RegistryEntry {
    job: Weak<NodeJob>,
    model: Arc<AtomicU32>,
}

/// Registry of jobs, one per node; entries drop with their node.
static JOBS: LazyLock<Mutex<HashMap<u32, RegistryEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Models of destroyed nodes, unloaded by the next driver pass, which runs on
/// the plugin thread.
static PENDING_UNLOAD: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// Creates the job of a node and registers it.
pub fn register(node_id: u32) -> Arc<NodeJob> {
    let model = Arc::new(AtomicU32::new(0));
    let job = Arc::new(NodeJob {
        node_id,
        model: model.clone(),
        state: Mutex::new(JobState::new()),
    });
    JOBS.lock().insert(
        node_id,
        RegistryEntry {
            job: Arc::downgrade(&job),
            model,
        },
    );
    job
}

/// Upgrades the live jobs, queueing the models of destroyed nodes for unload.
fn live_jobs() -> Vec<Arc<NodeJob>> {
    let mut registry = JOBS.lock();
    let mut jobs = Vec::with_capacity(registry.len());
    registry.retain(|_, entry| match entry.job.upgrade() {
        Some(job) => {
            jobs.push(job);
            true
        }
        None => {
            let model = entry.model.load(Ordering::SeqCst);
            if model != 0 {
                PENDING_UNLOAD.lock().push(model);
            }
            false
        }
    });
    jobs
}

/// Reports an event of a node through the engine's custom event path.
pub fn send_event(node_id: u32, event: SpineEvent) {
    dispatch_event_async(CustomEvent {
        target_id: node_id,
        name: event.name().to_owned(),
        body: Some(event),
    });
}

/// Drives every registered model once; called from the engine's plugin poll.
pub fn drive_frame() {
    let jobs = live_jobs();
    if jobs.is_empty() {
        return;
    }

    #[cfg(native)]
    {
        let Some(context) = plugin::context() else {
            for job in &jobs {
                let mut state = job.state.lock();
                report_unavailable(job.node_id, &mut state);
            }
            return;
        };
        unload_orphans(context);

        for job in &jobs {
            let mut state = job.state.lock();
            drive_model(context, job, &mut state);
        }
    }

    #[cfg(web)]
    for job in &jobs {
        let mut state = job.state.lock();
        report_unavailable(job.node_id, &mut state);
    }
}

/// Reports that no plugin can drive this node.
///
/// A platform without a plugin and a missing binary share this path.
fn report_unavailable(node_id: u32, state: &mut JobState) {
    if state.src.is_none() || state.error_reported {
        return;
    }
    state.error_reported = true;

    #[cfg(native)]
    let message = "the Spine plugin is not available; put moyu_spine_plugin next to the \
                   engine executable or set MOYU_SPINE_PLUGIN";
    #[cfg(web)]
    let message = "the web runtime has no Spine plugin, so spine nodes stay empty";

    send_event(
        node_id,
        SpineEvent::Error {
            code: moyu_spine_abi::error::UNSUPPORTED,
            message: message.to_owned(),
        },
    );
}

/// Unloads the models of destroyed nodes; only called on the plugin thread.
#[cfg(native)]
fn unload_orphans(context: &PluginContext) {
    let pending = std::mem::take(&mut *PENDING_UNLOAD.lock());
    for model in pending {
        if let Err(err) = context.unload_model(model) {
            log::warn!("spine: failed to unload a destroyed node's model: {err}");
        }
    }
}

/// Applies pending state changes right away.
///
/// Called on the plugin thread, so that a command running right after a `src`
/// change already sees the new model instead of the previous one.
#[cfg(native)]
pub fn sync_now(job: &NodeJob) {
    let Some(context) = plugin::context() else {
        return;
    };
    let mut state = job.state.lock();
    apply_src(context, job, &mut state);
}

/// Applies pending state changes right away; the web runtime has no plugin.
#[cfg(web)]
pub fn sync_now(_job: &NodeJob) {}

/// Applies the wanted state to one model and leaves a fresh frame behind.
#[cfg(native)]
fn drive_model(context: &PluginContext, job: &NodeJob, state: &mut JobState) {
    apply_src(context, job, state);

    if state.model == 0 {
        return;
    }

    if state.applied_skin.as_deref() != Some(state.skin.as_str()) {
        if let Err(err) = context.set_skin(state.model, &state.skin) {
            report(job.node_id, state, &err);
        } else {
            state.applied_skin = Some(state.skin.clone());
            // A skin change invalidates the pose the previous one set up.
            state.last_time = None;
        }
    }

    if state.applied_physics != state.physics {
        state.applied_physics = state.physics;
        state.last_time = None;
    }

    let delta = if state.paused {
        0.0
    } else {
        state
            .last_time
            .map(|last| last.elapsed().as_secs_f32().min(MAX_DELTA_SECONDS))
            .unwrap_or(0.0)
    };
    state.last_time = Some(moyu_pal::time::Instant::now());

    if let Err(err) = context.update(state.model, delta, state.time_scale, state.paused) {
        report(job.node_id, state, &err);
        return;
    }
    // Commands and updates both queue events; drain right away so nothing waits
    // for the next frame.
    drain_events(context, job.node_id, state);

    match context.frame(state.model) {
        Ok(desc) => {
            // SAFETY: the frame was just produced for this live model on this
            // thread, and no other call has happened since.
            let snapshot = unsafe { FrameSnapshot::copy(&desc) };
            state.snapshot = Some(snapshot);
            // The model is healthy again, so a later failure is worth reporting.
            state.error_reported = false;
        }
        Err(err) => report(job.node_id, state, &err),
    }

    // A command that ran this frame may have queued more events.
    drain_events(context, job.node_id, state);
}

/// Loads or unloads the model when the props ask for another one.
#[cfg(native)]
fn apply_src(context: &PluginContext, job: &NodeJob, state: &mut JobState) {
    if state.src == state.loaded_src && state.atlas == state.loaded_atlas {
        return;
    }
    if state.src.is_some() && state.src == state.failed_src {
        return;
    }

    if state.model != 0 {
        // The plugin drops every entry with the model without reporting them, so
        // the terminal event is replayed here for whatever was still running.
        // Waiters compare the whole identity, so these carry the outgoing epoch.
        let outgoing = state.epoch_counter;
        for (entry, (track, animation)) in state.live_entries.drain() {
            send_event(
                job.node_id,
                SpineEvent::TrackDispose {
                    track,
                    id: EntryIdentity {
                        entry,
                        epoch: outgoing,
                    },
                    animation,
                },
            );
        }

        if let Err(err) = context.unload_model(state.model) {
            log::warn!("spine node {}: {err}", job.node_id);
        }
        job.set_model(state, 0);
    }
    state.live_entries.clear();
    state.snapshot = None;
    state.model_info = None;
    state.bounding_boxes.clear();
    state.loaded_src = None;
    state.loaded_atlas = None;
    state.failed_src = None;
    state.applied_skin = None;
    state.applied_physics = -1;
    state.last_time = None;
    state.error_reported = false;
    state.epoch_counter += 1;
    let epoch = state.epoch_counter;

    let Some(path) = state.src.clone() else {
        return;
    };

    match load_model(context, &path, state.atlas.as_deref()) {
        Ok(model) => {
            job.set_model(state, model);
            state.loaded_src = Some(path);
            state.loaded_atlas = state.atlas.clone();
            match collect_metadata(context, model, epoch) {
                Ok((metadata, info, bounding_boxes)) => {
                    state.model_info = Some(Arc::new(info));
                    state.bounding_boxes = bounding_boxes;
                    state.applied_skin = None;
                    send_event(job.node_id, SpineEvent::Ready(metadata));
                }
                Err(err) => {
                    if let Err(unload) = context.unload_model(model) {
                        log::warn!("spine node {}: {unload}", job.node_id);
                    }
                    job.set_model(state, 0);
                    state.model_info = None;
                    state.loaded_src = None;
                    state.failed_src = state.src.clone();
                    report(job.node_id, state, &err);
                }
            }
        }
        Err(err) => {
            state.failed_src = state.src.clone();
            report(job.node_id, state, &err);
        }
    }
}

/// Reads the skeleton and atlas bytes and hands them to the plugin.
///
/// Both files are read here rather than through the plugin's file callback:
/// the atlas path is what decides which file to read, and the host already
/// knows the asset root.
#[cfg(native)]
fn load_model(
    context: &PluginContext,
    skeleton_path: &str,
    atlas_path: Option<&str>,
) -> PluginResult<u32> {
    let atlas_path = atlas_path
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| derive_atlas_path(skeleton_path));

    let skeleton = read_asset_bytes(skeleton_path).map_err(|message| PluginError {
        code: moyu_spine_abi::error::FILE,
        message,
    })?;
    let atlas = read_asset_bytes(&atlas_path).map_err(|message| PluginError {
        code: moyu_spine_abi::error::FILE,
        message,
    })?;

    context.load_model(skeleton_path, &skeleton, &atlas_path, &atlas)
}

/// Replaces the skeleton extension with `.atlas`, next to the skeleton.
fn derive_atlas_path(skeleton_path: &str) -> String {
    match skeleton_path.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.atlas"),
        None => format!("{skeleton_path}.atlas"),
    }
}

/// Reads one asset through the engine's asset directory.
#[cfg(native)]
fn read_asset_bytes(path: &str) -> Result<Vec<u8>, String> {
    let url = moyu_pal::dir::assets_dir()
        .join(path)
        .map_err(|err| format!("'{path}' is not a valid asset path: {err}"))?;
    moyu_pal::task::block_on_without_runtime(moyu_pal::fs::read(&url))
        .map_err(|err| format!("failed to read '{path}': {err}"))
}

/// Reads the model metadata, its texture pages and its bounding box names.
#[cfg(native)]
fn collect_metadata(
    context: &PluginContext,
    model: u32,
    epoch: u64,
) -> PluginResult<(SpineMetadata, ModelInfo, Vec<String>)> {
    let info = context.model_info(model)?;
    let data_version = span_text(info.data_version);
    let bounds = info.reference_bounds;

    let mut animations = Vec::new();
    for animation in context.animations(model)? {
        animations.push(SpineAnimation {
            name: span_text(animation.name),
            duration: animation.duration,
        });
    }

    let mut skins = Vec::new();
    for skin in context.skins(model)? {
        skins.push(span_text(skin.name));
    }

    let mut bounding_boxes = Vec::new();
    for bounding_box in context.bounding_boxes(model)? {
        bounding_boxes.push(span_text(bounding_box.name));
    }

    let mut pages = Vec::new();
    let mut page_info = Vec::new();
    for page in context.pages(model)? {
        let path = span_text(page.path);
        let premultiplied = page.alpha == ALPHA_PREMULTIPLIED;
        let alpha_source = if premultiplied {
            AlphaSource::Atlas
        } else {
            AlphaSource::Image
        };
        let sampling = Sampling {
            min_filter: page.min_filter,
            mag_filter: page.mag_filter,
            wrap_u: page.wrap_u,
            wrap_v: page.wrap_v,
        };
        page_info.push(PageInfo {
            path: path.clone(),
            premultiplied,
            sampling,
        });
        pages.push(SpinePage {
            path,
            width: page.width.max(0) as u32,
            height: page.height.max(0) as u32,
            premultiplied,
            alpha_source,
        });
    }

    let metadata = SpineMetadata {
        data_version,
        bounds: SpineBounds {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        },
        animations,
        skins,
        pages,
        bounding_boxes: bounding_boxes.clone(),
    };

    let model_info = ModelInfo {
        epoch,
        pages: page_info,
        bounds: [bounds.x, bounds.y, bounds.width, bounds.height],
        bounding_boxes: bounding_boxes.clone(),
    };
    Ok((metadata, model_info, bounding_boxes))
}

/// Copies a span into an owned string.
#[cfg(native)]
fn span_text(span: Span) -> String {
    unsafe { span.as_str() }.unwrap_or("").to_owned()
}

#[cfg(native)]
use moyu_spine_abi::types::Span;

/// Reports every event the plugin queued since the last drain.
#[cfg(native)]
fn drain_events(context: &PluginContext, node_id: u32, state: &mut JobState) {
    loop {
        let event = match context.poll_event(state.model) {
            Ok(event) => event,
            Err(err) => {
                report(node_id, state, &err);
                return;
            }
        };
        if event.kind == 0 {
            return;
        }

        let track = event.track_index;
        let id = EntryIdentity {
            entry: event.entry_id,
            epoch: state.epoch_counter,
        };
        let animation = span_text(event.animation);

        let observed = match event.kind {
            EVENT_START => {
                state
                    .live_entries
                    .insert(event.entry_id, (track, animation.clone()));
                SpineEvent::TrackStart {
                    track,
                    id,
                    animation,
                }
            }
            EVENT_COMPLETE => SpineEvent::TrackComplete {
                track,
                id,
                animation,
            },
            EVENT_END => SpineEvent::TrackEnd {
                track,
                id,
                animation,
            },
            EVENT_INTERRUPT => SpineEvent::TrackEnd {
                track,
                id,
                animation,
            },
            EVENT_DISPOSE => {
                state.live_entries.remove(&event.entry_id);
                SpineEvent::TrackDispose {
                    track,
                    id,
                    animation,
                }
            }
            EVENT_USER => SpineEvent::TrackEvent {
                track,
                id,
                animation,
                name: span_text(event.name),
                int_value: event.int_value,
                float_value: event.float_value,
                string_value: span_text(event.string_value),
                audio_path: span_text(event.audio_path),
                volume: event.volume,
                balance: event.balance,
            },
            _ => continue,
        };
        send_event(node_id, observed);
    }
}

/// Reports a plugin failure through the node's error event.
///
/// The driver retries on every frame, so a failure that persists is reported
/// once; a model that recovers and fails again reports again, with the code the
/// plugin returned.
#[cfg(native)]
fn report(node_id: u32, state: &mut JobState, error: &PluginError) {
    if state.error_reported {
        return;
    }
    state.error_reported = true;
    log::warn!("spine node {node_id}: {error}");
    send_event(
        node_id,
        SpineEvent::Error {
            code: error.code,
            message: error.message.clone(),
        },
    );
}
