use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum FullscreenEventKind {
    Change,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
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
