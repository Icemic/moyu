use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeEvent {
    pub width: f64,
    pub height: f64,
}

impl Event for ResizeEvent {
    fn name(&self) -> &'static str {
        "resize"
    }
}
