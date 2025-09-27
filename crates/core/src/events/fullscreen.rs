use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FullscreenEventKind {
    Change,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullScreenEvent {
    pub kind: FullscreenEventKind,
}

impl Event for FullScreenEvent {
    fn name(&self) -> &'static str {
        match self.kind {
            FullscreenEventKind::Change => "fullscreenchange",
            FullscreenEventKind::Error => "fullscreenerror",
        }
    }
}
