use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WheelEventDeltaMode {
    Pixel = 0,
    Line = 1,
    Page = 2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WheelEvent {
    pub target_id: u32,
    pub bubble_target_ids: Vec<u32>,
    pub delta_x: f64,
    pub delta_y: f64,
    // not used, just for compatibility with web
    pub delta_z: f64,
    pub delta_mode: WheelEventDeltaMode,
}

impl Event for WheelEvent {
    fn name(&self) -> &'static str {
        "wheelevent"
    }
}
