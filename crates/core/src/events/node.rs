use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeEventKind {
    Destory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEvent {
    pub kind: NodeEventKind,
    pub target_id: u32,
}

impl Event for NodeEvent {
    fn name(&self) -> &'static str {
        "nodeevent"
    }
}
