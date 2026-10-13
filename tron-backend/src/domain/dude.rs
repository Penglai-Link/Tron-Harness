//! Platform-owned always-on agents and durable wakeups.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DudeKind {
    Main,
    Memory,
    Custom,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dude {
    pub dude_code: String,
    pub kind: DudeKind,
    pub enabled: bool,
    pub heartbeat_minutes: u32,
    pub memory_auto_approve: bool,
    pub is_main: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DudeSessionPurpose {
    Interactive,
    Heartbeat,
    Task,
    Feedback,
    MemoryBatch,
    SkillAuthoring,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DudeSession {
    pub session_code: String,
    pub dude_code: String,
    pub purpose: DudeSessionPurpose,
    pub correlation_key: String,
    pub title: String,
    pub workspace_code: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DudeEventSource {
    UserMessage,
    Heartbeat,
    TaskReminder,
    ThreadFeedback,
    MemoryBatch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DudeEventStatus {
    Queued,
    Running,
    Completed,
    Failed,
    AwaitingInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DudeEvent {
    pub event_id: String,
    pub dude_code: String,
    pub source: DudeEventSource,
    pub dedup_key: String,
    pub session_code: Option<String>,
    pub task_code: Option<String>,
    pub workspace_code: Option<String>,
    pub payload: Value,
    pub status: DudeEventStatus,
    pub available_at: i64,
    pub lease_until: Option<i64>,
    pub lease_token: Option<String>,
    pub attempts: u32,
    pub result: Option<String>,
    pub created_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformNotification {
    pub id: String,
    pub dedup_key: String,
    pub dude_code: Option<String>,
    pub title: String,
    pub body: String,
    pub category: String,
    pub target: Value,
    pub read_at: Option<i64>,
    pub resolved_at: Option<i64>,
    pub archived_at: Option<i64>,
    pub created_at: i64,
}
#[derive(Debug, Clone)]
pub enum DudeError {
    NotFound,
    Invalid,
    Conflict,
    ExecutionBusy,
    Storage,
    Runtime(String),
}
impl std::fmt::Display for DudeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotFound => "Dude not found",
            Self::Invalid => "Dude configuration is invalid",
            Self::Conflict => "Dude state changed; refresh and retry",
            Self::ExecutionBusy => "Dude session is still executing; feedback remains queued",
            Self::Storage => "Dude storage unavailable",
            Self::Runtime(message) => message,
        })
    }
}
impl std::error::Error for DudeError {}

/// A Dude-assessed business change, independent of execution lifecycle and delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkChangeKind {
    Progress,
    Completed,
    NeedsDecision,
    Blocked,
}
impl WorkChangeKind {
    pub fn actionable(self) -> bool {
        matches!(self, Self::NeedsDecision | Self::Blocked)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DudeWorkChange {
    pub change_id: String,
    pub dude_code: String,
    pub task_code: String,
    pub change_key: String,
    pub kind: WorkChangeKind,
    pub summary: String,
    pub evidence: Vec<String>,
    pub requested_action: Option<String>,
    pub session_code: Option<String>,
    pub task_snapshot: Value,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
    pub resolution: Option<String>,
    pub resolution_evidence: Vec<String>,
    pub notification: Option<PlatformNotification>,
}
