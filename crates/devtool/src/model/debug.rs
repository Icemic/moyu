//! Runtime execution and debug bridge state model.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::DomainModel;

const MAX_LOG_BUFFER: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionMode {
    Native,
    Web,
    Attach,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugConnectionStatus {
    Stopped,
    Listening { port: u16 },
    Connected,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub seq: u64,
    pub level: String,
    pub target: String,
    pub message: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSummary {
    pub id: u32,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    pub visible: bool,
    pub children: Vec<NodeSummary>,
}

#[derive(Debug, Clone)]
pub struct DebugModel {
    pub mode: Option<SessionMode>,
    pub headless: bool,
    pub status: DebugConnectionStatus,
    pub web_url: Option<String>,
    pub attach_endpoint: Option<String>,
    pub engine_state: Option<serde_json::Value>,
    pub tree: Option<NodeSummary>,
    pub logs: VecDeque<LogEntry>,
    pub last_log_seq: u64,
}

impl Default for DebugModel {
    fn default() -> Self {
        Self {
            mode: None,
            headless: true,
            status: DebugConnectionStatus::Stopped,
            web_url: None,
            attach_endpoint: None,
            engine_state: None,
            tree: None,
            logs: VecDeque::new(),
            last_log_seq: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    SessionStarted {
        mode: SessionMode,
        headless: bool,
        port: u16,
        web_url: Option<String>,
        attach_endpoint: Option<String>,
    },
    EngineConnected,
    EngineReady,
    StateUpdated(serde_json::Value),
    TreeUpdated(NodeSummary),
    LogReceived(LogEntry),
    SessionStopped,
}

impl DomainModel for DebugModel {
    type Event = DebugEvent;

    fn apply(&mut self, event: Self::Event) -> bool {
        match event {
            DebugEvent::SessionStarted {
                mode,
                headless,
                port,
                web_url,
                attach_endpoint,
            } => {
                self.mode = Some(mode);
                self.headless = headless;
                self.status = DebugConnectionStatus::Listening { port };
                self.web_url = web_url;
                self.attach_endpoint = attach_endpoint;
                self.logs.clear();
                self.last_log_seq = 0;
                true
            }
            DebugEvent::EngineConnected => {
                if !matches!(self.status, DebugConnectionStatus::Ready) {
                    self.status = DebugConnectionStatus::Connected;
                }
                true
            }
            DebugEvent::EngineReady => {
                self.status = DebugConnectionStatus::Ready;
                true
            }
            DebugEvent::StateUpdated(val) => {
                self.engine_state = Some(val);
                true
            }
            DebugEvent::TreeUpdated(tree) => {
                self.tree = Some(tree);
                true
            }
            DebugEvent::LogReceived(entry) => {
                if entry.seq <= self.last_log_seq {
                    return false;
                }
                self.last_log_seq = entry.seq;
                if self.logs.len() >= MAX_LOG_BUFFER {
                    self.logs.pop_front();
                }
                self.logs.push_back(entry);
                true
            }
            DebugEvent::SessionStopped => {
                *self = Self::default();
                true
            }
        }
    }
}
