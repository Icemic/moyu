use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
#[ts(export, optional_fields)]
pub enum NodeEvent {
    Destory { target_id: u32 },
}

impl Event for NodeEvent {
    fn name(&self) -> &'static str {
        "nodeevent"
    }
}
