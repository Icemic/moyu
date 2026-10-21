//! Pull-based event queue.
//!
//! Events are enqueued from runtime listeners and copied into plugin-owned
//! storage immediately: the runtime reclaims its track entries right after the
//! listener returns, so nothing may be read later. The host drains the queue
//! after every command call and after every `update`, including while paused.

use crate::types::Span;

/// No pending event.
pub const EVENT_NONE: i32 = 0;

/// An animation entry started.
pub const EVENT_START: i32 = 1;

/// A queued or playing entry was replaced on its track.
pub const EVENT_INTERRUPT: i32 = 2;

/// An entry stopped being applied. Only produced when the entry has an end
/// time, such as when it is cleared or mixed out; a non-looping animation that
/// simply reaches its last frame does not produce one.
pub const EVENT_END: i32 = 3;

/// The current iteration of an entry reached the end. Looping animations
/// produce one per iteration, but an update that spans several iterations
/// produces a single event.
pub const EVENT_COMPLETE: i32 = 4;

/// A user event was fired.
pub const EVENT_USER: i32 = 5;

/// An entry is being reclaimed. This is the terminal signal for an entry's
/// lifetime: completion, interruption and cancellation all reach it.
pub const EVENT_DISPOSE: i32 = 6;

/// One event pulled from the queue. Payload strings stay valid until the next
/// call into the plugin.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EventDesc {
    /// `EVENT_*` kind; `EVENT_NONE` means the queue is empty.
    pub kind: i32,
    /// Track the entry belongs to.
    pub track_index: i32,
    /// Plugin-assigned entry identity, returned by `set_animation` and
    /// `add_animation`.
    pub entry_id: u64,
    /// Animation name.
    pub animation: Span,
    /// User event name (`EVENT_USER` only).
    pub name: Span,
    /// User event integer payload.
    pub int_value: i32,
    /// User event float payload.
    pub float_value: f32,
    /// User event string payload.
    pub string_value: Span,
    /// User event audio path; reported only, never played by the plugin.
    pub audio_path: Span,
    /// User event audio volume.
    pub volume: f32,
    /// User event audio balance.
    pub balance: f32,
}
