use serde::{Deserialize, Serialize};

use crate::traits::Event;

/// Custom event for user-defined events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub struct CustomEvent<T>
where
    T: Serialize + Send + 'static,
{
    /// target node id, or 0 for global
    pub target_id: u32,
    /// event name for `addEventListener` or `onXXX`
    pub name: String,
    /// event body, pass as parameter to callback
    pub body: Option<T>,
}

impl<T: Serialize + Send + 'static> Event for CustomEvent<T> {
    fn name(&self) -> &'static str {
        "customevent"
    }
}
