use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeEvent {
    pub width: u32,
    pub height: u32,
}

impl Event for ResizeEvent {
    fn name(&self) -> &'static str {
        "resizeevent"
    }
}
