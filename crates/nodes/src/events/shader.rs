use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export, optional_fields)]
pub enum ShaderEvent {
    Prepared,
    Finished,
}

impl Event for ShaderEvent {
    fn name(&self) -> &'static str {
        match self {
            ShaderEvent::Prepared => "prepared",
            ShaderEvent::Finished => "finished",
        }
    }
}
