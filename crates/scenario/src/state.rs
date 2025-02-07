use serde::{Deserialize, Serialize};

/// State struct of scenario, this represents the state of the running scenario.
/// Also as known as game save data.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScenarioState {
    /// Current line number of the scenario.
    pub current_line: usize,
    /// Extra data of the scenario.
    pub extra_data: Option<String>,
}

impl Default for ScenarioState {
    fn default() -> Self {
        Self {
            current_line: 0,
            extra_data: None,
        }
    }
}
