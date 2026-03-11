use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export, optional_fields)]
pub enum VideoEvent {
    Ended,
    StateChange(String),
}

impl Event for VideoEvent {
    fn name(&self) -> &'static str {
        match self {
            VideoEvent::Ended => "ended",
            VideoEvent::StateChange(_) => "stateChange",
        }
    }
}
