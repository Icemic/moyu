//! The engine's own input vocabulary.
//!
//! Window events arrive as winit types; these types name the same concepts in engine
//! terms, so that input can be described and processed without depending on a windowing
//! backend. The window event paths in `pointer_events` and `keyboard_events` map onto
//! them at the edge.

use crate::events::{KeyboardEventKind, KeyboardLocation};

/// Mouse button an action uses. Mirrors the buttons a window can report; `Other` covers
/// the ones the engine tracks but attaches no gesture to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other,
}

/// Phase of a touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// Modifier keys held while a keyboard event is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyboardModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

/// One keyboard event to send.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeyInput {
    pub kind: KeyboardEventKind,
    pub key: String,
    pub code: String,
    pub location: KeyboardLocation,
    pub repeat: bool,
    pub modifiers: KeyboardModifiers,
}
