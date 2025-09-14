use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
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
