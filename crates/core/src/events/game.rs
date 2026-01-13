use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub enum GameEvent {
    Ready,
}

impl Event for GameEvent {
    fn name(&self) -> &'static str {
        match self {
            GameEvent::Ready => "ready",
        }
    }
}
