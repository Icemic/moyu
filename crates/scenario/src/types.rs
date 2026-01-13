use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use sixu::format::{ResolvedCommandLine, ResolvedSystemCallLine};
use sixu::runtime::ExecutionState;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", untagged)]
#[ts(export, optional_fields)]
pub enum ScenarioEvent {
    CommandLine(ResolvedCommandLine),
    ExtraSystemCall(ResolvedSystemCallLine),
    Text(TextLine),
    Finished,
    Waiting,
    WaitingCancelled,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TextLine {
    pub leading: Option<String>,
    pub text: Option<String>,
    pub tailing: Option<String>,
}

impl Event for ScenarioEvent {
    fn name(&self) -> &'static str {
        match self {
            ScenarioEvent::CommandLine(_) => "scenarioCommandLine",
            ScenarioEvent::ExtraSystemCall(_) => "scenarioExtraSystemCall",
            ScenarioEvent::Text(_) => "scenarioText",
            ScenarioEvent::Finished => "scenarioFinished",
            ScenarioEvent::Waiting => "scenarioWaiting",
            ScenarioEvent::WaitingCancelled => "scenarioWaitingCancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    /// The current execution state stack
    pub stack: Vec<ExecutionState>,
    /// Variables for the current game session
    pub variables: serde_json::Value,
}

pub struct WaitingState {
    pub until: moyu_pal::time::Instant,
    pub skippable: bool,
}
