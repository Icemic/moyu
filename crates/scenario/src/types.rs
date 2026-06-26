use std::collections::HashMap;

use moyu_core::traits::Event;
use serde::{Deserialize, Serialize};
use sixu::Fingerprint;
use sixu::format::Block;
use sixu::format::{ResolvedCommandLine, ResolvedSystemCallLine};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WarpBoundary {
    Before,
    After,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", untagged)]
#[ts(export, optional_fields)]
pub enum ScenarioEvent {
    MarkerEnter(MarkerEnter),
    CommandLine(ResolvedCommandLine),
    ExtraSystemCall(ResolvedSystemCallLine),
    Text(TextLine),
    Finished,
    Waiting,
    WaitingCancelled,
    WarpFinished,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MarkerEnter {
    pub marker_id: String,
    pub story: String,
    pub paragraph: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct ExecutionCursor {
    pub story: String,
    pub paragraph: String,
    pub marker_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum StoryReplaceMode {
    Noop,
    Reposition,
    InvalidateOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[ts(export, optional_fields)]
pub enum ExecutableRestartBoundary {
    Checkpoint { checkpoint_key: String },
    RestartStory { story: String, entry: String },
    RestartParagraph { story: String, paragraph: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct StoryReplaceExecutionPlan {
    pub mode: StoryReplaceMode,
    pub boundary: Option<ExecutableRestartBoundary>,
    pub target_marker_id: Option<String>,
    pub changed_control_flow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct StoryReplaceOutcome {
    pub story: String,
    pub current_story_affected: bool,
    pub plan: StoryReplaceExecutionPlan,
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
            ScenarioEvent::MarkerEnter(_) => "scenarioMarkerEnter",
            ScenarioEvent::CommandLine(_) => "scenarioCommandLine",
            ScenarioEvent::ExtraSystemCall(_) => "scenarioExtraSystemCall",
            ScenarioEvent::Text(_) => "scenarioText",
            ScenarioEvent::Finished => "scenarioFinished",
            ScenarioEvent::Waiting => "scenarioWaiting",
            ScenarioEvent::WaitingCancelled => "scenarioWaitingCancelled",
            ScenarioEvent::WarpFinished => "scenarioWarpFinished",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BacklogState {
    pub records: Vec<ScenarioRecord>,
    pub blocks: HashMap<Fingerprint, Block>,
    pub next_record_serial: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedExecutionState {
    pub story: String,
    pub paragraph: String,
    pub block_fingerprint: Fingerprint,
    pub index: usize,
    pub is_loop_body: bool,
    pub locals: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    pub stack: Vec<SavedExecutionState>,
    pub variables: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RuntimeCheckpoint {
    pub cursor: Option<ExecutionCursor>,
    pub snapshot: RuntimeSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRecord {
    pub id: String,
    pub created_at: u64,
    pub meta: HashMap<String, serde_json::Value>,
    pub snapshot: RuntimeSnapshot,
}

impl ScenarioRecord {
    pub fn get_info(&self) -> ScenarioRecordInfo {
        ScenarioRecordInfo {
            id: self.id.clone(),
            created_at: self.created_at,
            meta: self.meta.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioRecordInfo {
    pub id: String,
    pub created_at: u64,
    pub meta: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    /// The current exact runtime state.
    pub current_state: RuntimeSnapshot,
    /// Backlog records in chronological order.
    pub records: Vec<ScenarioRecord>,
    /// Shared block pool referenced by all snapshots.
    pub blocks: HashMap<Fingerprint, Block>,
}

pub struct WaitingState {
    pub until: moyu_pal::time::Instant,
    pub skippable: bool,
}
