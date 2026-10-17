//! Per-node driver state and the once-per-frame plugin pass.
//!
//! A job is the shared state of one `live2d` node: the node writes what it
//! wants (props, commands), [`drive_frame`] applies it to the plugin and leaves
//! a copy of the newest frame for the renderer. The driver runs on the script
//! thread, the renderer on the graphics thread, and they meet here.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Weak};

use moyu_core::events::CustomEvent;
use moyu_core::traits::Event;
use moyu_core::utils::dispatch_event::dispatch_event_async;
use moyu_pal::sync::Mutex;

use crate::event::Live2dEvent;
use crate::frame::FrameSnapshot;

use moyu_live2d_abi::command::options;
#[cfg(native)]
use moyu_live2d_abi::frame::FLAG_BLEND_UNSUPPORTED;
#[cfg(native)]
use moyu_live2d_abi::query::{EVENT_MOTION_EVENT, EVENT_MOTION_FINISHED};
#[cfg(native)]
use crate::event::{
    Live2dCanvas, Live2dExpression, Live2dMetadata, Live2dMotion, Live2dParameter,
};
#[cfg(native)]
use crate::plugin::{self, PluginContext, PluginError, PluginResult};

/// Epoch of a node without a loaded model.
pub const NO_MODEL_EPOCH: u64 = u64::MAX;

/// Longest time step handed to the plugin, so a stalled window does not make a
/// model jump.
#[cfg(native)]
const MAX_DELTA_SECONDS: f32 = 0.25;

/// Shared state of one `live2d` node.
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
    /// Texture paths of the model, relative to the asset root.
    pub textures: Vec<String>,
    /// Canvas size in pixels.
    pub canvas_size: [f32; 2],
}

/// What the node wants and what the plugin currently has.
///
/// The plugin driver only exists on native platforms, so its bookkeeping is dead
/// code in a web build.
#[cfg_attr(web, allow(dead_code))]
pub struct JobState {
    /// Model path wanted by the props.
    pub src: Option<String>,
    /// Option bits wanted by the props.
    pub options: u32,
    /// Plugin model id; 0 while unloaded.
    pub model: u32,
    /// Identity of the loaded model; `None` while unloaded.
    pub model_info: Option<Arc<ModelInfo>>,
    /// Latest frame, taken by the renderer.
    pub snapshot: Option<FrameSnapshot>,
    /// Look target wanted by pointer events, in view space (-1..1, y up).
    pub look_target: (f32, f32),

    /// Model path currently loaded.
    loaded_src: Option<String>,
    /// Model path whose load failed; not retried until the props change.
    failed_src: Option<String>,
    /// Option bits the plugin has.
    options_applied: u32,
    /// Look target the plugin has.
    look_applied: (f32, f32),
    /// Last model step, for the frame delta.
    last_time: Option<moyu_pal::time::Instant>,
    /// Counting epochs; only ever grows, so a reload is never mistaken for the
    /// model it replaces.
    epoch_counter: u64,
    /// Whether a failure was already reported for the current state.
    error_reported: bool,
    /// Whether an unsupported blend mode was already reported.
    blend_reported: bool,
}

impl JobState {
    fn new() -> Self {
        Self {
            src: None,
            options: options::DEFAULT,
            model: 0,
            model_info: None,
            snapshot: None,
            look_target: (0.0, 0.0),
            loaded_src: None,
            failed_src: None,
            options_applied: 0,
            look_applied: (f32::MAX, f32::MAX),
            last_time: None,
            epoch_counter: 0,
            error_reported: false,
            blend_reported: false,
        }
    }
}

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
pub fn send_event(node_id: u32, event: Live2dEvent) {
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
    let message = "the Live2D plugin is not available; put moyu_live2d_plugin next to the \
                   engine executable or set MOYU_LIVE2D_PLUGIN";
    #[cfg(web)]
    let message = "the web runtime has no Live2D plugin, so live2d nodes stay empty";

    send_event(
        node_id,
        Live2dEvent::Error {
            code: moyu_live2d_abi::error::CORE_UNAVAILABLE,
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
            log::warn!("live2d: failed to unload a destroyed node's model: {err}");
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

    if state.options != state.options_applied {
        if let Err(err) = context.set_options(state.model, state.options) {
            report(job.node_id, state, &err);
            return;
        }
        state.options_applied = state.options;
    }

    if state.options & options::LOOK != 0 {
        if state.look_target != state.look_applied {
            if let Err(err) =
                context.set_look_target(state.model, state.look_target.0, state.look_target.1)
            {
                report(job.node_id, state, &err);
                return;
            }
            state.look_applied = state.look_target;
        }
    } else {
        // Look following is off; forget the applied target so turning it back on
        // hands the current one to the plugin.
        state.look_applied = (f32::MAX, f32::MAX);
    }

    let delta = state
        .last_time
        .map(|last| last.elapsed().as_secs_f32().min(MAX_DELTA_SECONDS))
        .unwrap_or(0.0);
    state.last_time = Some(moyu_pal::time::Instant::now());

    if let Err(err) = context.update(state.model, delta) {
        report(job.node_id, state, &err);
        return;
    }
    drain_events(context, job.node_id, state);

    match context.frame(state.model) {
        Ok(desc) => {
            // SAFETY: the frame was just produced for this live model on this
            // thread and is valid until the next plugin call.
            let snapshot = unsafe { FrameSnapshot::copy(&desc) };
            if !state.blend_reported
                && let Some(drawable) = snapshot
                    .drawables
                    .iter()
                    .find(|drawable| drawable.flags & FLAG_BLEND_UNSUPPORTED != 0)
            {
                state.blend_reported = true;
                send_event(
                    job.node_id,
                    Live2dEvent::BlendModeUnsupported {
                        value: drawable.source_blend_mode,
                    },
                );
            }
            state.snapshot = Some(snapshot);
            // The model is healthy again, so a later failure is worth reporting.
            state.error_reported = false;
        }
        Err(err) => report(job.node_id, state, &err),
    }
}

/// Loads or unloads the model when the props ask for another one.
#[cfg(native)]
fn apply_src(context: &PluginContext, job: &NodeJob, state: &mut JobState) {
    if state.src == state.loaded_src || (state.src.is_some() && state.src == state.failed_src) {
        return;
    }

    if state.model != 0 {
        if let Err(err) = context.unload_model(state.model) {
            log::warn!("live2d node {}: {err}", job.node_id);
        }
        job.set_model(state, 0);
    }
    state.snapshot = None;
    state.model_info = None;
    state.loaded_src = None;
    state.failed_src = None;
    state.last_time = None;
    state.options_applied = 0;
    state.look_applied = (f32::MAX, f32::MAX);
    state.blend_reported = false;
    state.error_reported = false;
    state.epoch_counter += 1;
    let epoch = state.epoch_counter;

    let Some(path) = state.src.clone() else {
        return;
    };

    match context.load_model(&path) {
        Ok(model) => {
            job.set_model(state, model);
            state.loaded_src = Some(path);
            match collect_metadata(context, model) {
                Ok((metadata, textures)) => {
                    let info = ModelInfo {
                        epoch,
                        textures,
                        canvas_size: [metadata.canvas.width, metadata.canvas.height],
                    };
                    state.model_info = Some(Arc::new(info));
                    if let Err(err) = context.set_options(model, state.options) {
                        report(job.node_id, state, &err);
                        return;
                    }
                    state.options_applied = state.options;
                    send_event(job.node_id, Live2dEvent::Ready(metadata));
                }
                Err(err) => {
                    if let Err(unload) = context.unload_model(model) {
                        log::warn!("live2d node {}: {unload}", job.node_id);
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

/// Reads the model metadata and texture list, both valid until unload.
#[cfg(native)]
fn collect_metadata(
    context: &PluginContext,
    model: u32,
) -> PluginResult<(Live2dMetadata, Vec<String>)> {
    let canvas = context.canvas(model)?;

    let mut motions = Vec::new();
    let (_, motion_count) = context.query_motions(model, -1)?;
    for index in 0..motion_count {
        let (info, _) = context.query_motions(model, index)?;
        let sound = span_text(info.sound);
        motions.push(Live2dMotion {
            group: span_text(info.group),
            index: info.index,
            duration: info.duration,
            fade_in: info.fade_in,
            fade_out: info.fade_out,
            sound: (!sound.is_empty()).then_some(sound),
        });
    }

    let mut expressions = Vec::new();
    let (_, expression_count) = context.query_expressions(model, -1)?;
    for index in 0..expression_count {
        let (info, _) = context.query_expressions(model, index)?;
        expressions.push(Live2dExpression {
            name: span_text(info.name),
            fade_in: info.fade_in,
            fade_out: info.fade_out,
        });
    }

    let mut hit_areas = Vec::new();
    let (_, hit_area_count) = context.query_hit_areas(model, -1)?;
    for index in 0..hit_area_count {
        let (info, _) = context.query_hit_areas(model, index)?;
        hit_areas.push(span_text(info.name));
    }

    let mut parameters = Vec::new();
    let (_, parameter_count) = context.query_parameters(model, -1)?;
    for index in 0..parameter_count {
        let (info, _) = context.query_parameters(model, index)?;
        parameters.push(Live2dParameter {
            id: span_text(info.id),
            kind: info.kind,
            min: info.min,
            max: info.max,
            default_value: info.default_value,
        });
    }

    let mut textures = Vec::new();
    let (_, texture_count) = context.query_textures(model, -1)?;
    for index in 0..texture_count {
        let (info, _) = context.query_textures(model, index)?;
        textures.push(span_text(info.path));
    }

    let metadata = Live2dMetadata {
        canvas: Live2dCanvas {
            width: canvas.size[0],
            height: canvas.size[1],
            origin_x: canvas.origin[0],
            origin_y: canvas.origin[1],
            pixels_per_unit: canvas.pixels_per_unit,
        },
        motions,
        expressions,
        hit_areas,
        parameters,
    };
    Ok((metadata, textures))
}

/// Reports every event the plugin queued since the last frame.
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
        match event.kind {
            0 => return,
            EVENT_MOTION_FINISHED => send_event(
                node_id,
                Live2dEvent::MotionFinished {
                    group: span_text(event.group),
                    index: event.index,
                },
            ),
            EVENT_MOTION_EVENT => send_event(
                node_id,
                Live2dEvent::MotionEvent {
                    value: span_text(event.value),
                },
            ),
            _ => {}
        }
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
    log::warn!("live2d node {node_id}: {error}");
    send_event(
        node_id,
        Live2dEvent::Error {
            code: error.code,
            message: error.message.clone(),
        },
    );
}

/// Views a plugin string as owned text.
#[cfg(native)]
fn span_text(span: moyu_live2d_abi::types::Span) -> String {
    // SAFETY: the spans handed out by the plugin stay valid until the model is
    // unloaded or the next poll.
    unsafe { span.as_str() }.unwrap_or("").to_owned()
}
