//! Wire format of the debug bridge.
//!
//! A request is named `<namespace>:<action>` ("engine:state"); its answer reuses the
//! request name with a `:done` or `:error` suffix; push messages carry no `requestId`.
//! See `rfcs/2026-09-25-runtime-debug-bridge.md` for the full protocol.

use moyu_pal::logger::buffer::LogEntry;
use serde::{Deserialize, Serialize};

use super::nodes::{NodeDetails, NodeSummary};
use super::state::EngineStateSnapshot;

/// Sent once right after the connection is established.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub session_id: &'a str,
    pub platform: &'static str,
    pub engine_version: &'static str,
    pub entry: String,
    /// Requests this engine can answer.
    pub capabilities: Vec<&'static str>,
    /// Whether the engine has finished starting up. See [`ReadyPush`].
    pub ready: bool,
}

/// Pushed once the engine has finished starting up.
///
/// A client that connected earlier knows from `engine:hello` or `engine:state` that the
/// engine was still starting, and uses this message to stop waiting.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyPush<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub session_id: &'a str,
}

/// `engine:eval` request payload.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalRequest {
    pub code: String,
    pub timeout_ms: Option<u64>,
}

/// `engine:logs` request payload.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsRequest {
    pub since_seq: Option<u64>,
    pub level: Option<String>,
    pub limit: Option<usize>,
    pub subscribe: Option<bool>,
}

/// Outcome of one evaluation, as reported by the platform runtime.
pub(super) enum EvalOutcome {
    /// The snippet returned a value.
    Value {
        /// Display form, such as `42` or `[object Object]`.
        repr: String,
        /// JSON text of the value, absent when it has no JSON form or could not be
        /// serialized.
        json: Option<String>,
    },
    /// The snippet threw.
    Thrown { message: String, stack: Option<String> },
}

/// Answer to `engine:state`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateDone<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub session_id: &'a str,
    pub request_id: u64,
    #[serde(flatten)]
    pub state: EngineStateSnapshot,
}

/// Answer to `engine:eval`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalDone<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub session_id: &'a str,
    pub request_id: u64,
    /// Display form of the result, such as `42` or `[object Object]`.
    pub repr: String,
    /// The result itself, omitted when it has no JSON form or was too large.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// Whether the result exceeded the size limit and was left out.
    pub truncated: bool,
}

/// Answer to `engine:logs`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsDone<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub session_id: &'a str,
    pub request_id: u64,
    pub entries: Vec<LogEntry>,
    /// Cursor to pass as `sinceSeq` on the next call.
    pub next_seq: u64,
    pub dropped: u64,
}

/// Pushed for every recorded entry while the client is subscribed.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPush<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub session_id: &'a str,
    pub entry: LogEntry,
}

/// `engine:tree` request payload.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeRequest {
    /// Defaults to the root node.
    pub node_id: Option<u32>,
    /// Levels of children to include; defaults to one, so that a response stays small
    /// and the client reads deeper levels by asking for a child's id.
    pub depth: Option<u32>,
}

/// `engine:props` request payload.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropsRequest {
    pub node_id: u32,
}

/// Answer to `engine:tree`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeDone<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub session_id: &'a str,
    pub request_id: u64,
    pub node: NodeSummary,
}

/// Answer to `engine:props`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropsDone<'a> {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub session_id: &'a str,
    pub request_id: u64,
    pub node: NodeDetails,
}

/// Answer to a request that could not be fulfilled.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestError<'a> {
    #[serde(rename = "type")]
    pub kind: &'a str,
    pub session_id: &'a str,
    pub request_id: Option<u64>,
    /// Omitted for errors raised by the evaluated snippet itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<&'static str>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}
