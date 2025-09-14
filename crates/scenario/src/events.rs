use moyu_core::traits::Event;
use serde::Serialize;

use crate::types::ExecutionResult;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", untagged)]
pub enum ScenarioEvent {
    ExecutionResult(ExecutionResult),
}

impl Event for ScenarioEvent {
    fn name(&self) -> &'static str {
        match self {
            ScenarioEvent::ExecutionResult(_) => "scenarionextline",
        }
    }
}
