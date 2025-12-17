use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnimationFrameCallbackEvent {
    pub timestamp: u32,
}

impl Event for AnimationFrameCallbackEvent {
    fn name(&self) -> &'static str {
        "animationframecallbackevent"
    }
}
