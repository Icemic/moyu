use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export, optional_fields)]
pub enum TransitionContainerEvent {
    Finished,
}

impl Event for TransitionContainerEvent {
    fn name(&self) -> &'static str {
        match self {
            TransitionContainerEvent::Finished => "finished",
        }
    }
}
