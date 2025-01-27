use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeEventKind {
    Destory,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEvent<T>
where
    T: Serialize + Send + 'static,
{
    pub kind: NodeEventKind,
    pub target_id: u32,
    pub custom_kind: Option<String>,
    pub custom_body: Option<T>,
}

impl<T: Serialize + Send + 'static> Event for NodeEvent<T> {
    fn name(&self) -> &'static str {
        "nodeevent"
    }
}
