use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationFrameCallbackEvent {
    pub timestamp: u32,
}

impl Event for AnimationFrameCallbackEvent {
    fn name(&self) -> &'static str {
        "animationframecallbackevent"
    }
}
