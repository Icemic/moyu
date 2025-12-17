use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub enum TextEvent {
    Start,
    Progress(f64),
    Finish,
}

impl Event for TextEvent {
    fn name(&self) -> &'static str {
        match self {
            TextEvent::Start => "start",
            TextEvent::Progress(_) => "progress",
            TextEvent::Finish => "finish",
        }
    }
}
