use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct DD;

/// Custom event for user-defined events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub struct CustomEvent<T>
where
    T: TS + Send + 'static,
{
    /// target node id, or 0 for global
    pub target_id: u32,
    /// event name for `addEventListener` or `onXXX`
    pub name: String,
    /// event body, pass as parameter to callback
    #[ts(optional)]
    pub body: Option<T>,
}

impl<T: Serialize + TS + Send + 'static> Event for CustomEvent<T> {
    fn name(&self) -> &'static str {
        "customevent"
    }
}
