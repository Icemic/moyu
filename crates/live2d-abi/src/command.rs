//! Command payloads and the flag constants they carry.

use crate::types::Span;

/// One parameter override: the value is applied after the effect chain on
/// every frame until overrides are reset.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ParameterPair {
    pub id: Span,
    pub value: f32,
}

/// Model option bits, sent through `set_options`.
pub mod options {
    /// Evaluate physics.
    pub const PHYSICS: u32 = 1 << 0;
    /// Evaluate pose opacity.
    pub const POSE: u32 = 1 << 1;
    /// Automatic eye blinking.
    pub const BLINK: u32 = 1 << 2;
    /// Automatic breathing.
    pub const BREATH: u32 = 1 << 3;
    /// Built-in look following, driven by `set_look_target`.
    pub const LOOK: u32 = 1 << 4;
    /// Automatic idle motions while the motion queue is empty.
    pub const IDLE: u32 = 1 << 5;
    /// Everything except look following.
    pub const DEFAULT: u32 = PHYSICS | POSE | BLINK | BREATH | IDLE;
}

/// Motion priorities, sent through `start_motion`.
pub mod priority {
    /// Never replaces a playing motion.
    pub const NONE: i32 = 0;
    /// Idle motions; replaced by everything else.
    pub const IDLE: i32 = 1;
    /// Default priority of project-started motions.
    pub const NORMAL: i32 = 2;
    /// Interrupts everything.
    pub const FORCE: i32 = 3;
}
