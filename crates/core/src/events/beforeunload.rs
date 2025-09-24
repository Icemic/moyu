use serde::{Deserialize, Serialize};

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeUnloadEvent {}

impl Event for BeforeUnloadEvent {
    fn name(&self) -> &'static str {
        "beforeunloadevent"
    }
}
