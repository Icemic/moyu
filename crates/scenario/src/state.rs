use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScenarioState {
    pub current_line: usize,
}

impl Default for ScenarioState {
    fn default() -> Self {
        Self { current_line: 0 }
    }
}
