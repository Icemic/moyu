use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::state::PointerLocation;
use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
pub enum TouchEventKind {
    TouchStart,
    TouchMove,
    TouchEnd,
    TouchCancel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
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
