use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResizeEvent {
    pub width: f64,
    pub height: f64,
}

impl Event for ResizeEvent {
    fn name(&self) -> &'static str {
        "resize"
    }
}
