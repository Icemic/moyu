use serde::{Deserialize, Serialize};
use sixu::format::{CommandLine, SystemCallLine};
use sixu::runtime::ExecutionState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    rename_all = "lowercase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum ExecutionResult {
    CommandLine(CommandLine),
    ExtraSystemCall(SystemCallLine),
    Text {
        leading: Option<String>,
        text: Option<String>,
    },
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    /// The current execution state stack
    pub stack: Vec<ExecutionState>,
    /// Variables for the current game session
    pub variables: serde_json::Value,
}
