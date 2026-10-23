//! Model control: advancing time, animation tracks, skins and hit testing.

use crate::types::Span;

/// Physics is neither updated nor applied.
pub const PHYSICS_NONE: i32 = 0;

/// Physics pose is applied without advancing the simulation.
pub const PHYSICS_POSE: i32 = 1;

/// Physics is advanced and applied.
pub const PHYSICS_UPDATE: i32 = 2;

/// How the host advances one model.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UpdateDesc {
    /// Elapsed seconds since the previous update, already clamped by the host.
    pub delta: f32,
    /// Time scaling applied to animation playback.
    pub time_scale: f32,
    /// Whether animation time advances; the pose is still evaluated when paused,
    /// so skin and attachment changes take effect immediately.
    pub paused: i32,
    /// Physics mode, one of `PHYSICS_*`.
    pub physics: i32,
}

/// Playback settings of one track animation.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrackDesc {
    /// Animation name.
    pub animation: Span,
    /// Whether to loop.
    pub loop_animation: i32,
    /// Transition duration in seconds; negative keeps the configured default.
    pub mix: f32,
    /// Delay before the animation starts, used by queueing. Negative means
    /// "derive from the mix duration", which is what the runtime does.
    pub delay: f32,
    /// Initial playback position in seconds.
    pub time: f32,
}

/// Outcome of a hit test.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HitDesc {
    /// Name of the bounding box that contained the point; empty when none did.
    pub name: Span,
}
