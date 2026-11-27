//! Unified background task, progress, and diagnostic log model.

use std::collections::{BTreeMap, VecDeque};

use super::DomainModel;

const MAX_TASK_LOG_LINES: usize = 2000;

pub type TaskId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warn,
    Error,
}

impl LogLevel {
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    DownloadEngine,
    DownloadVideoDecoder,
    DownloadAndroidTemplate,
    BuildProject,
    PackGame,
    PackAndroid,
    GenerateSchema,
    RunProject,
    InstallNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Completed,
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct TaskItem {
    pub id: TaskId,
    pub kind: TaskKind,
    pub title: String,
    pub stage: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone)]
pub struct TaskLogEntry {
    pub task_id: TaskId,
    pub level: LogLevel,
    pub line: String,
}

#[derive(Debug, Clone, Default)]
pub struct TasksModel {
    pub tasks: BTreeMap<TaskId, TaskItem>,
    pub logs: VecDeque<TaskLogEntry>,
}

#[derive(Debug, Clone)]
pub enum TaskEvent {
    Started {
        id: TaskId,
        kind: TaskKind,
        title: String,
    },
    Stage {
        id: TaskId,
        stage: String,
    },
    Progress {
        id: TaskId,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    },
    LogLine {
        id: TaskId,
        level: LogLevel,
        line: String,
    },
    Finished {
        id: TaskId,
        status: TaskStatus,
    },
}

impl DomainModel for TasksModel {
    type Event = TaskEvent;

    fn apply(&mut self, event: Self::Event) -> bool {
        match event {
            TaskEvent::Started { id, kind, title } => {
                self.tasks.insert(
                    id,
                    TaskItem {
                        id,
                        kind,
                        title,
                        stage: String::new(),
                        downloaded_bytes: 0,
                        total_bytes: None,
                        status: TaskStatus::Running,
                    },
                );
                true
            }
            TaskEvent::Stage { id, stage } => {
                if let Some(task) = self.tasks.get_mut(&id) {
                    task.stage = stage;
                    true
                } else {
                    false
                }
            }
            TaskEvent::Progress {
                id,
                downloaded_bytes,
                total_bytes,
            } => {
                if let Some(task) = self.tasks.get_mut(&id) {
                    task.downloaded_bytes = downloaded_bytes;
                    task.total_bytes = total_bytes;
                    true
                } else {
                    false
                }
            }
            TaskEvent::LogLine { id, level, line } => {
                if self.logs.len() >= MAX_TASK_LOG_LINES {
                    self.logs.pop_front();
                }
                self.logs.push_back(TaskLogEntry {
                    task_id: id,
                    level,
                    line,
                });
                true
            }
            TaskEvent::Finished { id, status } => {
                if let Some(task) = self.tasks.get_mut(&id) {
                    task.status = status;
                    true
                } else {
                    false
                }
            }
        }
    }
}
