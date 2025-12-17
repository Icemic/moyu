use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::Event;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BeforeUnloadEvent {}

impl Event for BeforeUnloadEvent {
    fn name(&self) -> &'static str {
        "beforeunload"
    }
}
