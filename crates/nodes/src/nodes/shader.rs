use anyhow::Result;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Command, Node, NodeBaseTrait, NodeEventSource};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::events::ShaderEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum RetainMode {
    #[default]
    Snapshot,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum ShaderBuiltinName {
    #[default]
    Crossfade,
    Wipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ShaderTimeControl {
    #[default]
    Auto,
    Manual,
    Transition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ShaderParamType {
    #[default]
    Float,
    Int,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct ShaderParam {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: ShaderParamType,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
#[ts(export, optional_fields)]
pub enum ShaderSource {
    Builtin { name: ShaderBuiltinName },
    Raw { content: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TransitionPhase {
    #[default]
    Stable,
    AwaitingPrepare,
    Prepared,
    Running,
    Finishing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingPrepare {
    pub from_channel: u32,
    pub to_channel: u32,
    pub mode: RetainMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PendingPerform {
    pub duration: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TransitionFromSource {
    #[default]
    Slot,
    Display,
}

#[derive(Debug, Node)]
pub struct Shader {
    pub(crate) shader: ShaderSource,
    pub(crate) params: Vec<ShaderParam>,
    pub(crate) time_control: ShaderTimeControl,
    pub(crate) display_channel: Option<u32>,
    pub(crate) retain: RetainMode,
    pub(crate) transition_phase: TransitionPhase,
    pub(crate) transition_from_channel: Option<u32>,
    pub(crate) transition_to_channel: Option<u32>,
    pub(crate) transition_duration: f64,
    pub(crate) transition_progress: f32,
    pub(crate) transition_from_source: TransitionFromSource,
    pub(crate) pending_prepare: Option<PendingPrepare>,
    pub(crate) pending_perform: Option<PendingPerform>,
    pub(crate) elapsed_time: f64,
    pub(crate) playing: bool,
    pub(crate) last_tick_at: Option<f64>,
    pub(crate) local_frame: u32,
    pub(crate) error_state: bool,
    pub(crate) needs_retry: bool,
    pub(crate) shader_dirty: bool,
    pub(crate) params_dirty: bool,
    pub(crate) slots_dirty: bool,
    pub(crate) slot_layout_key: Option<String>,
    pub(crate) shader_rect: moyu_core::base::Rect,
    pub(crate) render_width: u32,
    pub(crate) render_height: u32,
    pub(crate) last_active_at: Option<f64>,
    pub(crate) from_needs_redraw: bool,
    pub(crate) channel_views: [Option<wgpu::TextureView>; 4],
    pub(crate) channel_texture_widths: [u32; 4],
    pub(crate) channel_texture_heights: [u32; 4],
    pub(crate) display_view: Option<wgpu::TextureView>,
    pub(crate) display_texture_width: u32,
    pub(crate) display_texture_height: u32,
    pub(crate) display_rect: moyu_core::base::Rect,
    pub(crate) snapshot_display_view: Option<wgpu::TextureView>,
    pub(crate) snapshot_display_rect: moyu_core::base::Rect,
    pub(crate) channel_declared: [bool; 4],
    pub(crate) channel_empty: [bool; 4],
    pub(crate) channel_static: [bool; 4],
    pub(crate) channel_needs_redraw: [bool; 4],
    pub(crate) pipeline: Option<wgpu::RenderPipeline>,
    pub(crate) bind_group: Option<wgpu::BindGroup>,
    pub(crate) present_bind_group: Option<wgpu::BindGroup>,
    pub(crate) snapshot_bind_group: Option<wgpu::BindGroup>,
    pub(crate) render_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) builtins_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) params_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) snapshot_uniform_buffer: Option<wgpu::Buffer>,

    #[base]
    node_base: NodeBase,
}

impl Default for Shader {
    fn default() -> Self {
        Self {
            shader: ShaderSource::default(),
            params: Vec::new(),
            time_control: ShaderTimeControl::Auto,
            display_channel: None,
            retain: RetainMode::Snapshot,
            transition_phase: TransitionPhase::Stable,
            transition_from_channel: None,
            transition_to_channel: None,
            transition_duration: 0.0,
            transition_progress: 0.0,
            transition_from_source: TransitionFromSource::Slot,
            pending_prepare: None,
            pending_perform: None,
            elapsed_time: 0.0,
            playing: true,
            last_tick_at: None,
            local_frame: 0,
            error_state: false,
            needs_retry: false,
            shader_dirty: true,
            params_dirty: true,
            slots_dirty: true,
            slot_layout_key: None,
            shader_rect: moyu_core::base::Rect::default(),
            render_width: 0,
            render_height: 0,
            last_active_at: None,
            from_needs_redraw: false,
            channel_views: std::array::from_fn(|_| None),
            channel_texture_widths: [0; 4],
            channel_texture_heights: [0; 4],
            display_view: None,
            display_texture_width: 0,
            display_texture_height: 0,
            display_rect: moyu_core::base::Rect::default(),
            snapshot_display_view: None,
            snapshot_display_rect: moyu_core::base::Rect::default(),
            channel_declared: [false; 4],
            channel_empty: [false; 4],
            channel_static: [false; 4],
            channel_needs_redraw: [true; 4],
            pipeline: None,
            bind_group: None,
            present_bind_group: None,
            snapshot_bind_group: None,
            render_uniform_buffer: None,
            builtins_uniform_buffer: None,
            params_uniform_buffer: None,
            snapshot_uniform_buffer: None,
            node_base: NodeBase::default(),
        }
    }
}

impl Shader {
    pub(crate) const CHANNEL_COUNT: usize = 4;

    pub fn new(label: String) -> Self {
        Self {
            node_base: NodeBase::new(label),
            ..Default::default()
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        !matches!(self.time_control, ShaderTimeControl::Transition)
            || !matches!(self.transition_phase, TransitionPhase::Stable)
    }

    pub(crate) fn reset_timeline(&mut self) {
        self.elapsed_time = 0.0;
        self.last_tick_at = None;
        self.local_frame = 0;
    }

    pub(crate) fn advance_generic_timeline(&mut self, timestamp: f64) -> (f32, f32, u32) {
        let mut time_delta = 0.0;
        let frame = self.local_frame;

        if self.playing {
            if let Some(last_tick_at) = self.last_tick_at {
                time_delta = (timestamp - last_tick_at).max(0.0);
                self.elapsed_time += time_delta;
            }

            self.last_tick_at = Some(timestamp);
            self.local_frame = self.local_frame.saturating_add(1);
        } else {
            self.last_tick_at = None;
        }

        (self.elapsed_time as f32, time_delta as f32, frame)
    }

    pub(crate) fn advance_transition_timeline(
        &mut self,
        timestamp: f64,
    ) -> (f32, f32, u32, f32) {
        match self.transition_phase {
            TransitionPhase::Running => {
                let mut time_delta = 0.0;
                let frame = self.local_frame;

                if let Some(last_tick_at) = self.last_tick_at {
                    time_delta = (timestamp - last_tick_at).max(0.0);
                    self.elapsed_time += time_delta;
                }

                self.last_tick_at = Some(timestamp);
                self.local_frame = self.local_frame.saturating_add(1);

                if self.transition_duration <= 0.0 {
                    self.transition_progress = 1.0;
                    self.transition_phase = TransitionPhase::Finishing;
                } else {
                    self.transition_progress =
                        (self.elapsed_time / self.transition_duration).clamp(0.0, 1.0) as f32;

                    if self.transition_progress >= 1.0 {
                        self.transition_progress = 1.0;
                        self.transition_phase = TransitionPhase::Finishing;
                    }
                }

                (
                    self.elapsed_time as f32,
                    time_delta as f32,
                    frame,
                    self.transition_progress,
                )
            }
            TransitionPhase::Finishing => (
                self.transition_duration.max(0.0) as f32,
                0.0,
                self.local_frame,
                1.0,
            ),
            TransitionPhase::Stable | TransitionPhase::AwaitingPrepare | TransitionPhase::Prepared => {
                (0.0, 0.0, 0, self.transition_progress)
            }
        }
    }

    pub(crate) fn reset_transition_state(&mut self) {
        self.transition_phase = TransitionPhase::Stable;
        self.transition_from_channel = None;
        self.transition_to_channel = None;
        self.transition_duration = 0.0;
        self.transition_progress = 0.0;
        self.transition_from_source = TransitionFromSource::Slot;
        self.pending_prepare = None;
        self.pending_perform = None;
        self.from_needs_redraw = false;
        self.snapshot_display_view = None;
        self.snapshot_display_rect = moyu_core::base::Rect::default();
        self.snapshot_bind_group = None;
        self.reset_timeline();
    }

    pub(crate) fn apply_time_control(&mut self, time_control: ShaderTimeControl) {
        if self.time_control == time_control {
            return;
        }

        self.time_control = time_control;
        self.reset_transition_state();
        self.playing = matches!(time_control, ShaderTimeControl::Auto);
        self.transition_progress = 0.0;
        self.channel_needs_redraw = [true; Self::CHANNEL_COUNT];
    }

    pub(crate) fn clear_idle_runtime_state(&mut self) {
        self.channel_views = std::array::from_fn(|_| None);
        self.channel_texture_widths = [0; Self::CHANNEL_COUNT];
        self.channel_texture_heights = [0; Self::CHANNEL_COUNT];
        self.display_view = None;
        self.display_texture_width = 0;
        self.display_texture_height = 0;
        self.display_rect = moyu_core::base::Rect::default();
        self.snapshot_display_view = None;
        self.snapshot_display_rect = moyu_core::base::Rect::default();
        self.channel_declared = [false; Self::CHANNEL_COUNT];
        self.channel_empty = [false; Self::CHANNEL_COUNT];
        self.channel_static = [false; Self::CHANNEL_COUNT];
        self.channel_needs_redraw = [true; Self::CHANNEL_COUNT];
        self.bind_group = None;
        self.present_bind_group = None;
        self.snapshot_bind_group = None;
    }

    pub(crate) fn mark_error(&mut self, message: impl AsRef<str>) {
        log::error!("shader node {}: {}", self.base().id(), message.as_ref());
        self.error_state = true;
        self.bind_group = None;
        self.present_bind_group = None;
        self.needs_retry = false;
    }

    pub(crate) fn update_slot_layout_key(&mut self, slot_key: String) {
        if self.slot_layout_key.as_deref() == Some(slot_key.as_str()) {
            return;
        }

        self.slot_layout_key = Some(slot_key);
        self.slots_dirty = true;
        self.needs_retry = true;
        self.error_state = false;
        self.bind_group = None;
        self.present_bind_group = None;
        self.channel_needs_redraw = [true; Self::CHANNEL_COUNT];
    }

    pub(crate) fn finish_transition_if_ready(&mut self) {
        if matches!(self.transition_phase, TransitionPhase::Finishing)
            && self.pending_prepare.is_none()
            && self.pending_perform.is_none()
        {
            self.reset_transition_state();
            self.send_event(ShaderEvent::Finished);
        }
    }

    pub(crate) fn apply_prepare_request(
        &mut self,
        request: PendingPrepare,
        capture_display: bool,
    ) {
        self.retain = request.mode;
        self.transition_from_channel = Some(request.from_channel);
        self.transition_to_channel = Some(request.to_channel);
        self.transition_phase = TransitionPhase::AwaitingPrepare;
        self.transition_duration = 0.0;
        self.transition_progress = 0.0;
        self.transition_from_source = if capture_display {
            self.snapshot_display_view = self.display_view.clone();
            self.snapshot_display_rect = self.display_rect;
            TransitionFromSource::Display
        } else {
            self.snapshot_display_view = None;
            self.snapshot_display_rect = moyu_core::base::Rect::default();
            TransitionFromSource::Slot
        };
        self.from_needs_redraw = true;
        self.snapshot_bind_group = None;
        self.reset_timeline();
        self.channel_needs_redraw = [true; Self::CHANNEL_COUNT];
    }

    pub(crate) fn apply_perform_request(&mut self, duration: f64) {
        self.transition_duration = duration.max(0.0);
        self.transition_progress = if self.transition_duration <= 0.0 {
            1.0
        } else {
            0.0
        };
        self.from_needs_redraw = true;
        self.reset_timeline();

        if self.transition_duration <= 0.0 {
            self.transition_phase = TransitionPhase::Finishing;
        } else {
            self.transition_phase = TransitionPhase::Running;
        }
    }

    pub(crate) fn mark_prepare_captured(&mut self) {
        self.transition_phase = TransitionPhase::Prepared;
        self.from_needs_redraw = false;
    }

    pub(crate) fn finish_update(&mut self) {
        self.error_state = false;
        self.needs_retry = false;
        self.shader_dirty = false;
        self.params_dirty = false;
        self.slots_dirty = false;
    }
}

impl ShaderBuiltinName {
    pub(crate) fn effect_id(self) -> i32 {
        match self {
            Self::Crossfade => 0,
            Self::Wipe => 1,
        }
    }
}

impl ShaderSource {
    pub(crate) fn builtin_effect_id(&self) -> i32 {
        match self {
            Self::Builtin { name } => name.effect_id(),
            Self::Raw { .. } => -1,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct ShaderProps {
    pub shader: Patch<ShaderSource>,
    pub params: Patch<Vec<ShaderParam>>,
    pub time_control: Patch<ShaderTimeControl>,
    pub display_channel: Patch<Option<u32>>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "subCommand")]
#[ts(export, optional_fields, rename_all = "camelCase")]
#[allow(non_snake_case)]
pub enum ShaderCommand {
    Prepare {
        fromChannel: u32,
        toChannel: u32,
        mode: Option<RetainMode>,
    },
    Perform {
        duration: f64,
    },
    Start,
    Stop,
    Reset,
}

impl Node for Shader {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    fn node_type(&self) -> &'static str {
        "shader"
    }

    fn pre_update(&mut self, parent: &NodeBase) {
        let width = *parent.width();
        let height = *parent.height();

        if *self.base().width() != width || *self.base().height() != height {
            self.base_mut().set_size(width, height);
        }
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: ShaderProps = from_js(props).unwrap();

        match props.shader {
            Patch::Set(shader) => {
                self.shader = shader;
                self.shader_dirty = true;
                self.needs_retry = true;
                self.error_state = false;
                self.pipeline = None;
                self.bind_group = None;
            }
            Patch::Reset => {
                self.shader = ShaderSource::default();
                self.shader_dirty = true;
                self.needs_retry = true;
                self.error_state = false;
                self.pipeline = None;
                self.bind_group = None;
            }
            Patch::Missing => {}
        }

        match props.params {
            Patch::Set(params) => {
                self.params = params;
                self.params_dirty = true;
                self.needs_retry = true;
                self.error_state = false;
            }
            Patch::Reset => {
                self.params.clear();
                self.params_dirty = true;
                self.needs_retry = true;
                self.error_state = false;
            }
            Patch::Missing => {}
        }

        match props.time_control {
            Patch::Set(time_control) => self.apply_time_control(time_control),
            Patch::Reset => self.apply_time_control(ShaderTimeControl::Auto),
            Patch::Missing => {}
        }

        match props.display_channel {
            Patch::Set(display_channel) => {
                self.display_channel = display_channel;
            }
            Patch::Reset => {
                self.display_channel = None;
            }
            Patch::Missing => {}
        }

        self.base_mut().pend_update();
    }

    fn shadowed(&self, kind: moyu_core::traits::ShadowKind) -> bool {
        match kind {
            moyu_core::traits::ShadowKind::Rendering => self.error_state,
        }
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

impl NodeEventSource for Shader {
    type Event = ShaderEvent;
}

impl Command for Shader {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let command: ShaderCommand = from_js(payload)?;

        match command {
            ShaderCommand::Prepare {
                fromChannel,
                toChannel,
                mode,
            } => {
                if !matches!(self.time_control, ShaderTimeControl::Transition) {
                    log::warn!(
                        "shader node {}: prepare is only available in transition mode",
                        self.base().id()
                    );
                    return Ok(None);
                }

                if fromChannel > 3 || toChannel > 3 {
                    log::warn!(
                        "shader node {}: prepare channels must be in range 0..3, got {} and {}",
                        self.base().id(),
                        fromChannel,
                        toChannel
                    );
                    return Ok(None);
                }

                if fromChannel == toChannel {
                    log::warn!(
                        "shader node {}: prepare requires distinct fromChannel and toChannel",
                        self.base().id()
                    );
                    return Ok(None);
                }

                self.pending_prepare = Some(PendingPrepare {
                    from_channel: fromChannel,
                    to_channel: toChannel,
                    mode: mode.unwrap_or(self.retain),
                });
            }
            ShaderCommand::Perform { duration } => {
                if !matches!(self.time_control, ShaderTimeControl::Transition) {
                    log::warn!(
                        "shader node {}: perform is only available in transition mode",
                        self.base().id()
                    );
                    return Ok(None);
                }

                self.pending_perform = Some(PendingPerform {
                    duration: (duration / 1000.0).max(0.0),
                });
            }
            ShaderCommand::Start => {
                if matches!(self.time_control, ShaderTimeControl::Transition) {
                    log::warn!(
                        "shader node {}: start is not available in transition mode",
                        self.base().id()
                    );
                    return Ok(None);
                }

                self.playing = true;
                self.last_tick_at = None;
            }
            ShaderCommand::Stop => {
                if matches!(self.time_control, ShaderTimeControl::Transition) {
                    log::warn!(
                        "shader node {}: stop is not available in transition mode",
                        self.base().id()
                    );
                    return Ok(None);
                }

                self.playing = false;
                self.last_tick_at = None;
            }
            ShaderCommand::Reset => {
                if matches!(self.time_control, ShaderTimeControl::Transition) {
                    log::warn!(
                        "shader node {}: reset is not available in transition mode",
                        self.base().id()
                    );
                    return Ok(None);
                }

                self.reset_timeline();
                self.transition_progress = 0.0;
            }
        }

        self.base_mut().pend_update();

        Ok(None)
    }
}

impl Default for ShaderSource {
    fn default() -> Self {
        Self::Builtin {
            name: ShaderBuiltinName::Crossfade,
        }
    }
}
