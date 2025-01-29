use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum KeyboardEventKind {
    KeyDown,
    KeyUp,
    KeyPress,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum KeyboardLocation {
    Standard,
    Left,
    Right,
    Numpad,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardEvent {
    pub kind: KeyboardEventKind,
    pub target_id: u32,
    pub bubble_target_ids: Vec<u32>,
    pub key: String,
    pub code: String,
    pub location: KeyboardLocation,
    pub repeat: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
}

impl Event for KeyboardEvent {
    fn name(&self) -> &'static str {
        "keyboardevent"
    }
}
