use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
