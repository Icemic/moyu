use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum NodeEvent {
    Destory { target_id: u32 },
}

impl Event for NodeEvent {
    fn name(&self) -> &'static str {
        "nodeevent"
    }
}
