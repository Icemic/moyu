use serde::{Deserialize, Serialize};
use sixu::format::{CommandLine, SystemCallLine};
use sixu::runtime::ExecutionState;
use strum::AsRefStr;

#[derive(Debug, Clone, Serialize, Deserialize, AsRefStr)]
pub enum ExecutionResult {
    CommandLine(CommandLine),
    ExtraSystemCall(SystemCallLine),
    Text(Option<String>, Option<String>),
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    /// The current execution state stack
    pub stack: Vec<ExecutionState>,
    /// Variables for the current game session
    pub variables: serde_json::Value,
}
