use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LayoutEvent {
    pub width: f32,
    pub height: f32,
}

impl Event for LayoutEvent {
    fn name(&self) -> &'static str {
        "layout"
    }
}
