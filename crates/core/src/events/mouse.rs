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
    DoubleClick,
    ContextMenu,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "RawMouseEvent")]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub target_id: u32,
    pub bubble_target_ids: Vec<u32>,
    #[serde(flatten)]
    pub location: PointerLocation,
}

impl Event for MouseEvent {
    fn name(&self) -> &'static str {
        "mouseevent"
    }
}
