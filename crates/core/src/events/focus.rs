use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum FocusEventKind {
    Focus,
    Blur,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FocusEvent {
    pub kind: FocusEventKind,
    pub target_id: u32,
}

impl Event for FocusEvent {
    fn name(&self) -> &'static str {
        match self.kind {
            FocusEventKind::Focus => "focus",
            FocusEventKind::Blur => "blur",
        }
    }
}
