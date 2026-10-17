//! Model metadata available after load and valid until unload, plus the
//! pull-based event queue.
//!
//! List entries are read two-phase: a probe call reports the entry count, then
//! each entry is fetched by index. Entry strings are UTF-8 spans owned by the
//! plugin.

use crate::types::{Span, Vec2};

/// Canvas geometry.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CanvasInfo {
    /// Canvas dimensions in canvas pixels.
    pub size: Vec2,
    /// Model origin on the canvas, in canvas pixels.
    pub origin: Vec2,
    /// Scaling from canvas pixels to model units.
    pub pixels_per_unit: f32,
}

/// One motion entry; `index` is its position inside `group`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MotionInfo {
    /// Group name.
    pub group: Span,
    /// Index inside the group, as used by `start_motion`.
    pub index: i32,
    /// Duration in seconds.
    pub duration: f32,
    /// Fade-in duration in seconds.
    pub fade_in: f32,
    /// Fade-out duration in seconds.
    pub fade_out: f32,
    /// Sound file path relative to the model directory; empty when none.
    pub sound: Span,
}

/// One exclusive expression.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExpressionInfo {
    /// Expression name.
    pub name: Span,
    /// Fade-in duration in seconds.
    pub fade_in: f32,
    /// Fade-out duration in seconds.
    pub fade_out: f32,
}

/// One hit area.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HitAreaInfo {
    /// Hit area name.
    pub name: Span,
}

/// No parameter type is declared.
pub const PARAMETER_KIND_UNKNOWN: i32 = -1;

/// A regular parameter driven by a range of values.
pub const PARAMETER_KIND_NORMAL: i32 = 0;

/// A blend-shape parameter driven by weight.
pub const PARAMETER_KIND_BLEND_SHAPE: i32 = 1;

/// One parameter.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ParameterInfo {
    /// Parameter id.
    pub id: Span,
    /// Parameter type, one of `PARAMETER_KIND_*`.
    pub kind: i32,
    /// Minimum value.
    pub min: f32,
    /// Maximum value.
    pub max: f32,
    /// Default value.
    pub default_value: f32,
}

/// One part.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PartInfo {
    /// Part id.
    pub id: Span,
}

/// One texture; the path is relative to the asset root (`assets/`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TextureInfo {
    pub path: Span,
}

/// No pending event.
pub const EVENT_NONE: i32 = 0;

/// A motion finished playing.
pub const EVENT_MOTION_FINISHED: i32 = 1;

/// A motion fired an embedded event.
pub const EVENT_MOTION_EVENT: i32 = 2;

/// One event pulled from the queue. Payload strings stay valid until the next
/// call into the plugin.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EventDesc {
    /// `EVENT_*` kind; `EVENT_NONE` means the queue is empty.
    pub kind: i32,
    /// `EVENT_MOTION_FINISHED`: group name; empty otherwise.
    pub group: Span,
    /// `EVENT_MOTION_FINISHED`: index inside the group; unused otherwise.
    pub index: i32,
    /// `EVENT_MOTION_EVENT`: payload string; empty otherwise.
    pub value: Span,
}
