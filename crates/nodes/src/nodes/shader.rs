use std::sync::atomic::AtomicBool;

use anyhow::Result;
use csscolorparser::Color;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Command, Node, NodeBaseTrait, NodeEventSource};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::ShaderSlotSpace;
use crate::events::ShaderEvent;
use crate::renderer::pass::SHADER_PARAM_SLOT_COUNT;

const DEFAULT_OUT: f64 = 0.5;
const DEFAULT_HOLD: f64 = 0.0;
const DEFAULT_IN: f64 = 0.5;
const SUM_TOLERANCE: f64 = 0.000_001;

fn default_wipe_softness() -> f64 {
    0.05
}

fn default_zoom_start_scale() -> f64 {
    0.0
}

fn default_zoom_end_scale() -> f64 {
    1.0
}

fn default_zoom_origin() -> [f64; 2] {
    [0.5, 0.5]
}

fn default_pixellate_steps() -> u32 {
    4
}

fn default_mask_softness() -> f64 {
    0.0625
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum RetainMode {
    #[default]
    Static,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum ShaderBuiltinName {
    #[default]
    Crossfade,
    Wipe,
    Fade,
    Push,
    Slideaway,
    Zoom,
    Pixellate,
    Mask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ShaderDirection {
    #[default]
    Left,
    Right,
    Up,
    Down,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "name"
)]
#[ts(export, optional_fields)]
pub enum ShaderBuiltin {
    Crossfade,
    Wipe {
        #[serde(default = "default_wipe_softness")]
        softness: f64,
        #[serde(default)]
        direction: ShaderDirection,
    },
    Fade {
        out: f64,
        hold: f64,
        #[serde(rename = "in")]
        in_ratio: f64,
        #[ts(type = "string")]
        color: Color,
    },
    Push {
        #[serde(default)]
        direction: ShaderDirection,
    },
    Slideaway {
        #[serde(default)]
        direction: ShaderDirection,
    },
    Zoom {
        #[serde(default = "default_zoom_start_scale")]
        start_scale: f64,
        #[serde(default = "default_zoom_end_scale")]
        end_scale: f64,
        #[serde(default = "default_zoom_origin")]
        origin: [f64; 2],
    },
    Pixellate {
        #[serde(default = "default_pixellate_steps")]
        steps: u32,
    },
    Mask {
        rule: String,
        #[serde(default = "default_mask_softness")]
        softness: f64,
        #[serde(default)]
        reverse: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
#[ts(export, optional_fields)]
pub enum ShaderSource {
    Builtin {
        #[serde(flatten)]
        builtin: ShaderBuiltin,
    },
    Raw {
        content: String,
        #[serde(default)]
        params: Option<Vec<ShaderParam>>,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShaderSlotLayout {
    pub empty: bool,
    pub is_static: bool,
    pub space: ShaderSlotSpace,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Node)]
pub struct Shader {
    pub(crate) shader: ShaderSource,
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
    pub(crate) slot_layouts: [Option<ShaderSlotLayout>; 4],
    pub(crate) shader_rect: moyu_core::base::Rect,
    pub(crate) render_width: u32,
    pub(crate) render_height: u32,
    pub(crate) render_sample_max_uv: [f32; 2],
    pub(crate) last_active_at: Option<f64>,
    pub(crate) prepare_capture_scheduled: bool,
    pub(crate) prepare_ready_latched: AtomicBool,
    pub(crate) from_texture_dirty: bool,
    pub(crate) channel_views: [Option<wgpu::TextureView>; 4],
    pub(crate) channel_msaa_views: [Option<wgpu::TextureView>; 4],
    pub(crate) channel_texture_widths: [u32; 4],
    pub(crate) channel_texture_heights: [u32; 4],
    pub(crate) display_view: Option<wgpu::TextureView>,
    pub(crate) display_texture_width: u32,
    pub(crate) display_texture_height: u32,
    pub(crate) display_rect: moyu_core::base::Rect,
    pub(crate) snapshot_display_view: Option<wgpu::TextureView>,
    pub(crate) snapshot_display_rect: moyu_core::base::Rect,
    pub(crate) snapshot_sample_max_uv: [f32; 2],
    pub(crate) channel_declared: [bool; 4],
    pub(crate) channel_empty: [bool; 4],
    pub(crate) channel_static: [bool; 4],
    pub(crate) channel_content_revisions: [u64; 4],
    pub(crate) channel_needs_redraw: [bool; 4],
    pub(crate) pipeline: Option<wgpu::RenderPipeline>,
    pub(crate) bind_group: Option<wgpu::BindGroup>,
    pub(crate) present_bind_group: Option<wgpu::BindGroup>,
    pub(crate) snapshot_bind_group: Option<wgpu::BindGroup>,
    pub(crate) render_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) hidden_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) builtins_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) params_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) snapshot_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) snapshot_hidden_uniform_buffer: Option<wgpu::Buffer>,

    #[base]
    node_base: NodeBase,
}

impl Default for Shader {
    fn default() -> Self {
        Self {
            shader: ShaderSource::default(),
            time_control: ShaderTimeControl::Auto,
            display_channel: None,
            retain: RetainMode::Static,
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
            slot_layouts: [None; 4],
            shader_rect: moyu_core::base::Rect::default(),
            render_width: 0,
            render_height: 0,
            render_sample_max_uv: [1.0, 1.0],
            last_active_at: None,
            prepare_capture_scheduled: false,
            prepare_ready_latched: AtomicBool::new(false),
            from_texture_dirty: false,
            channel_views: std::array::from_fn(|_| None),
            channel_msaa_views: std::array::from_fn(|_| None),
            channel_texture_widths: [0; 4],
            channel_texture_heights: [0; 4],
            display_view: None,
            display_texture_width: 0,
            display_texture_height: 0,
            display_rect: moyu_core::base::Rect::default(),
            snapshot_display_view: None,
            snapshot_display_rect: moyu_core::base::Rect::default(),
            snapshot_sample_max_uv: [1.0, 1.0],
            channel_declared: [false; 4],
            channel_empty: [false; 4],
            channel_static: [false; 4],
            channel_content_revisions: [0; 4],
            channel_needs_redraw: [true; 4],
            pipeline: None,
            bind_group: None,
            present_bind_group: None,
            snapshot_bind_group: None,
            render_uniform_buffer: None,
            hidden_uniform_buffer: None,
            builtins_uniform_buffer: None,
            params_uniform_buffer: None,
            snapshot_uniform_buffer: None,
            snapshot_hidden_uniform_buffer: None,
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

    pub(crate) fn advance_transition_timeline(&mut self, timestamp: f64) -> (f32, f32, u32, f32) {
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
            TransitionPhase::Stable
            | TransitionPhase::AwaitingPrepare
            | TransitionPhase::Prepared => (0.0, 0.0, 0, self.transition_progress),
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
        self.prepare_capture_scheduled = false;
        *self.prepare_ready_latched.get_mut() = false;
        self.from_texture_dirty = false;
        self.snapshot_display_view = None;
        self.snapshot_display_rect = moyu_core::base::Rect::default();
        self.snapshot_sample_max_uv = [1.0, 1.0];
        self.snapshot_bind_group = None;
        self.reset_timeline();
    }

    pub(crate) fn apply_time_control(&mut self, time_control: ShaderTimeControl) {
        if self.time_control == time_control {
            return;
        }

        self.time_control = time_control;
        self.reset_transition_state();
        self.pipeline = None;
        self.bind_group = None;
        self.playing = matches!(time_control, ShaderTimeControl::Auto);
        self.transition_progress = 0.0;
        self.channel_needs_redraw = [true; Self::CHANNEL_COUNT];
    }

    pub(crate) fn clear_idle_runtime_state(&mut self) {
        self.channel_views = std::array::from_fn(|_| None);
        self.channel_msaa_views = std::array::from_fn(|_| None);
        self.channel_texture_widths = [0; Self::CHANNEL_COUNT];
        self.channel_texture_heights = [0; Self::CHANNEL_COUNT];
        self.display_view = None;
        self.display_texture_width = 0;
        self.display_texture_height = 0;
        self.display_rect = moyu_core::base::Rect::default();
        self.snapshot_display_view = None;
        self.snapshot_display_rect = moyu_core::base::Rect::default();
        self.render_sample_max_uv = [1.0, 1.0];
        self.snapshot_sample_max_uv = [1.0, 1.0];
        self.channel_declared = [false; Self::CHANNEL_COUNT];
        self.channel_empty = [false; Self::CHANNEL_COUNT];
        self.channel_static = [false; Self::CHANNEL_COUNT];
        self.channel_content_revisions = [0; Self::CHANNEL_COUNT];
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

    pub(crate) fn update_slot_layouts(
        &mut self,
        slot_layouts: [Option<ShaderSlotLayout>; Self::CHANNEL_COUNT],
    ) {
        if self.slot_layouts == slot_layouts {
            return;
        }

        self.slot_layouts = slot_layouts;
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

    pub(crate) fn apply_prepare_request(&mut self, request: PendingPrepare, capture_display: bool) {
        self.retain = request.mode;
        self.transition_from_channel = Some(request.from_channel);
        self.transition_to_channel = Some(request.to_channel);
        self.transition_phase = TransitionPhase::AwaitingPrepare;
        self.transition_duration = 0.0;
        self.transition_progress = 0.0;
        self.transition_from_source = if capture_display {
            self.snapshot_display_view = self.display_view.clone();
            self.snapshot_display_rect = self.display_rect;
            self.snapshot_sample_max_uv = self.render_sample_max_uv;
            TransitionFromSource::Display
        } else {
            self.snapshot_display_view = None;
            self.snapshot_display_rect = moyu_core::base::Rect::default();
            self.snapshot_sample_max_uv = [1.0, 1.0];
            TransitionFromSource::Slot
        };
        self.prepare_capture_scheduled = false;
        *self.prepare_ready_latched.get_mut() = false;
        self.from_texture_dirty = true;
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
        self.prepare_capture_scheduled = false;
        *self.prepare_ready_latched.get_mut() = false;
        self.snapshot_bind_group = None;
        self.reset_timeline();

        if self.transition_duration <= 0.0 {
            self.transition_phase = TransitionPhase::Finishing;
        } else {
            self.transition_phase = TransitionPhase::Running;
        }
    }

    pub(crate) fn mark_prepare_captured(&mut self) {
        self.transition_phase = TransitionPhase::Prepared;
        self.prepare_capture_scheduled = false;
        *self.prepare_ready_latched.get_mut() = false;
        self.from_texture_dirty = false;
        self.snapshot_bind_group = None;
        self.send_event(ShaderEvent::Prepared);
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
            Self::Fade => 2,
            Self::Push => 3,
            Self::Slideaway => 4,
            Self::Zoom => 5,
            Self::Pixellate => 6,
            Self::Mask => 7,
        }
    }
}

impl ShaderDirection {
    fn slot_value(self) -> u32 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Up => 2,
            Self::Down => 3,
        }
    }
}

impl ShaderBuiltin {
    pub(crate) fn name(&self) -> ShaderBuiltinName {
        match self {
            Self::Crossfade => ShaderBuiltinName::Crossfade,
            Self::Wipe { .. } => ShaderBuiltinName::Wipe,
            Self::Fade { .. } => ShaderBuiltinName::Fade,
            Self::Push { .. } => ShaderBuiltinName::Push,
            Self::Slideaway { .. } => ShaderBuiltinName::Slideaway,
            Self::Zoom { .. } => ShaderBuiltinName::Zoom,
            Self::Pixellate { .. } => ShaderBuiltinName::Pixellate,
            Self::Mask { .. } => ShaderBuiltinName::Mask,
        }
    }

    pub(crate) fn sanitize(self, node_id: u32) -> Self {
        match self {
            Self::Wipe {
                mut softness,
                direction,
            } => {
                if !softness.is_finite() || !(0.0..=1.0).contains(&softness) {
                    log::warn!(
                        "shader node {}: wipe softness must be within 0..=1, got {}; using fallback {}",
                        node_id,
                        softness,
                        default_wipe_softness()
                    );
                    softness = default_wipe_softness();
                }

                Self::Wipe {
                    softness,
                    direction,
                }
            }
            Self::Fade {
                mut out,
                mut hold,
                mut in_ratio,
                color,
            } => {
                if !out.is_finite() || !(0.0..=1.0).contains(&out) {
                    log::warn!(
                        "shader node {}: fade out ratio must be within 0..=1, got {}; using fallback {}",
                        node_id,
                        out,
                        DEFAULT_OUT
                    );
                    out = DEFAULT_OUT;
                }

                if !hold.is_finite() || !(0.0..=1.0).contains(&hold) {
                    log::warn!(
                        "shader node {}: fade hold ratio must be within 0..=1, got {}; using fallback {}",
                        node_id,
                        hold,
                        DEFAULT_HOLD
                    );
                    hold = DEFAULT_HOLD;
                }

                if !in_ratio.is_finite() || !(0.0..=1.0).contains(&in_ratio) {
                    log::warn!(
                        "shader node {}: fade in ratio must be within 0..=1, got {}; using fallback {}",
                        node_id,
                        in_ratio,
                        DEFAULT_IN
                    );
                    in_ratio = DEFAULT_IN;
                }

                let total = out + hold + in_ratio;
                if !total.is_finite() || (total - 1.0).abs() > SUM_TOLERANCE {
                    log::warn!(
                        "shader node {}: fade ratios must sum to 1, got {}; using fallback ratios ({}, {}, {})",
                        node_id,
                        total,
                        DEFAULT_OUT,
                        DEFAULT_HOLD,
                        DEFAULT_IN
                    );
                    out = DEFAULT_OUT;
                    hold = DEFAULT_HOLD;
                    in_ratio = DEFAULT_IN;
                }

                Self::Fade {
                    out,
                    hold,
                    in_ratio,
                    color,
                }
            }
            Self::Zoom {
                mut start_scale,
                mut end_scale,
                mut origin,
            } => {
                if !start_scale.is_finite() || start_scale < 0.0 {
                    log::warn!(
                        "shader node {}: zoom startScale must be finite and >= 0, got {}; using fallback {}",
                        node_id,
                        start_scale,
                        default_zoom_start_scale()
                    );
                    start_scale = default_zoom_start_scale();
                }

                if !end_scale.is_finite() || end_scale < 0.0 {
                    log::warn!(
                        "shader node {}: zoom endScale must be finite and >= 0, got {}; using fallback {}",
                        node_id,
                        end_scale,
                        default_zoom_end_scale()
                    );
                    end_scale = default_zoom_end_scale();
                }

                for (index, coord) in origin.iter_mut().enumerate() {
                    if !coord.is_finite() || !(0.0..=1.0).contains(coord) {
                        log::warn!(
                            "shader node {}: zoom origin[{}] must be within 0..=1, got {}; using fallback {}",
                            node_id,
                            index,
                            coord,
                            0.5
                        );
                        *coord = 0.5;
                    }
                }

                Self::Zoom {
                    start_scale,
                    end_scale,
                    origin,
                }
            }
            Self::Mask {
                rule,
                mut softness,
                reverse,
            } => {
                if !softness.is_finite() || !(0.0..=1.0).contains(&softness) {
                    log::warn!(
                        "shader node {}: mask softness must be within 0..=1, got {}; using fallback {}",
                        node_id,
                        softness,
                        default_mask_softness()
                    );
                    softness = default_mask_softness();
                }

                Self::Mask {
                    rule,
                    softness,
                    reverse,
                }
            }
            _ => self,
        }
    }

    fn write_param_slots(&self, slots: &mut [u32; SHADER_PARAM_SLOT_COUNT]) {
        match self {
            Self::Crossfade => {}
            Self::Wipe {
                softness,
                direction,
            } => {
                slots[0] = (*softness as f32).to_bits();
                slots[1] = direction.slot_value();
            }
            Self::Fade {
                out,
                hold,
                in_ratio,
                color,
            } => {
                slots[0] = (*out as f32).to_bits();
                slots[1] = (*hold as f32).to_bits();
                slots[2] = (*in_ratio as f32).to_bits();

                let color = color.to_array();
                slots[3] = color[0].to_bits();
                slots[4] = color[1].to_bits();
                slots[5] = color[2].to_bits();
                slots[6] = color[3].to_bits();
            }
            Self::Push { direction } | Self::Slideaway { direction } => {
                slots[0] = direction.slot_value();
            }
            Self::Zoom {
                start_scale,
                end_scale,
                origin,
            } => {
                slots[0] = (*start_scale as f32).to_bits();
                slots[1] = (*end_scale as f32).to_bits();
                slots[2] = (origin[0] as f32).to_bits();
                slots[3] = (origin[1] as f32).to_bits();
            }
            Self::Pixellate { steps } => {
                slots[0] = *steps;
            }
            Self::Mask {
                softness, reverse, ..
            } => {
                slots[0] = (*softness as f32).to_bits();
                slots[1] = u32::from(*reverse);
            }
        }
    }
}

impl ShaderSource {
    pub(crate) fn builtin_effect_id(&self) -> i32 {
        match self {
            Self::Builtin { builtin } => builtin.name().effect_id(),
            Self::Raw { .. } => -1,
        }
    }

    pub(crate) fn sanitize(self, node_id: u32) -> Self {
        match self {
            Self::Builtin { builtin } => Self::Builtin {
                builtin: builtin.sanitize(node_id),
            },
            Self::Raw { content, params } => Self::Raw { content, params },
        }
    }

    pub(crate) fn needs_pipeline_recompile(&self, next: &Self) -> bool {
        match (self, next) {
            (Self::Builtin { .. }, Self::Builtin { .. }) => false,
            (
                Self::Raw {
                    content: current_content,
                    ..
                },
                Self::Raw {
                    content: next_content,
                    ..
                },
            ) => current_content != next_content,
            _ => true,
        }
    }

    pub(crate) fn pack_params_uniform_bytes(
        &self,
    ) -> Result<[u8; SHADER_PARAM_SLOT_COUNT * std::mem::size_of::<u32>()], String> {
        let slots = match self {
            Self::Builtin { builtin } => {
                let mut slots = [0; SHADER_PARAM_SLOT_COUNT];
                builtin.write_param_slots(&mut slots);
                slots
            }
            Self::Raw { params, .. } => {
                let params = params.as_deref().unwrap_or(&[]);
                let mut slots = [0; SHADER_PARAM_SLOT_COUNT];

                for (index, param) in params.iter().enumerate() {
                    if index >= SHADER_PARAM_SLOT_COUNT {
                        return Err(format!(
                            "shader params exceed the current limit of {} 4-byte slots",
                            SHADER_PARAM_SLOT_COUNT
                        ));
                    }

                    slots[index] = match param.param_type {
                        ShaderParamType::Float => (param.value as f32).to_bits(),
                        ShaderParamType::Int => {
                            if param.value.fract() != 0.0 {
                                return Err(format!(
                                    "shader int param '{}' must be an integer, got {}",
                                    param.name, param.value
                                ));
                            }

                            if !(i32::MIN as f64..=i32::MAX as f64).contains(&param.value) {
                                return Err(format!(
                                    "shader int param '{}' is out of i32 range: {}",
                                    param.name, param.value
                                ));
                            }

                            param.value as i32 as u32
                        }
                    };
                }

                slots
            }
        };
        let mut bytes = [0u8; SHADER_PARAM_SLOT_COUNT * std::mem::size_of::<u32>()];

        for (index, slot) in slots.iter().enumerate() {
            let start = index * std::mem::size_of::<u32>();
            let end = start + std::mem::size_of::<u32>();
            bytes[start..end].copy_from_slice(&slot.to_ne_bytes());
        }

        Ok(bytes)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct ShaderProps {
    pub shader: Patch<ShaderSource>,
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
        let props: ShaderProps = match from_js(props) {
            Ok(props) => props,
            Err(err) => {
                log::error!(
                    "shader node {}: failed to parse props: {:?}",
                    self.base().id(),
                    err
                );
                return;
            }
        };

        match props.shader {
            Patch::Set(shader) => {
                let shader = shader.sanitize(*self.base().id());
                let shader_dirty = self.shader.needs_pipeline_recompile(&shader);
                let params_dirty = self.shader != shader;

                self.shader = shader;
                if shader_dirty {
                    self.shader_dirty = true;
                    self.pipeline = None;
                    self.bind_group = None;
                }
                if params_dirty {
                    self.params_dirty = true;
                }
                if shader_dirty || params_dirty {
                    self.needs_retry = true;
                    self.error_state = false;
                }
            }
            Patch::Reset => {
                let shader = ShaderSource::default();
                let shader_dirty = self.shader.needs_pipeline_recompile(&shader);
                let params_dirty = self.shader != shader;

                self.shader = shader;
                if shader_dirty {
                    self.shader_dirty = true;
                    self.pipeline = None;
                    self.bind_group = None;
                }
                if params_dirty {
                    self.params_dirty = true;
                }
                if shader_dirty || params_dirty {
                    self.needs_retry = true;
                    self.error_state = false;
                }
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

                let mode = mode.unwrap_or(self.retain);
                self.pending_prepare = Some(PendingPrepare {
                    from_channel: fromChannel,
                    to_channel: toChannel,
                    mode,
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
            builtin: ShaderBuiltin::Crossfade,
        }
    }
}
