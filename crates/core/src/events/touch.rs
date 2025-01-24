use serde::{Deserialize, Serialize};

use crate::state::PointerLocation;
use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TouchEventKind {
    TouchStart,
    TouchMove,
    TouchEnd,
    TouchCancel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchEvent {
    pub kind: TouchEventKind,
    pub target_id: u32,
    pub bubble_target_ids: Vec<u32>,
    #[serde(flatten)]
    pub location: PointerLocation,
    pub identifier: Option<u32>,
}

impl Event for TouchEvent {
    fn name(&self) -> &'static str {
        "touchevent"
    }
}
