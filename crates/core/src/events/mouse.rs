use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::state::PointerLocation;
use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
pub enum MouseEventKind {
    MouseEnter,
    MouseLeave,
    MouseDown,
    MouseUp,
    MouseMove,
    Click,
    AuxClick,
    DoubleClick,
    ContextMenu,
}

/// A mouse event, carrying the button it is about and the buttons held at the time,
/// both in the DOM `MouseEvent` encoding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "RawMouseEvent")]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub target_id: u32,
    pub bubble_target_ids: Vec<u32>,
    /// The button this event is about: 0 left, 1 middle, 2 right, 3 back, 4 forward.
    /// Events that are not about a particular button carry 0.
    pub button: i32,
    /// Buttons held when the event fired: 1 left, 2 right, 4 middle, 8 back, 16 forward.
    pub buttons: u32,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
    #[serde(flatten)]
    pub location: PointerLocation,
}

impl Event for MouseEvent {
    fn name(&self) -> &'static str {
        "mouseevent"
    }
}
