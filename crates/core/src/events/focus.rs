use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FocusEventKind {
    Focus,
    Blur,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusEvent {
    pub kind: FocusEventKind,
    pub target_id: u32,
}

impl Event for FocusEvent {
    fn name(&self) -> &'static str {
        "focusevent"
    }
}
