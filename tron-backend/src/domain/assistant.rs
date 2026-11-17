//! Platform-owned always-on agents and durable wakeups.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const HEARTBEAT_HISTORY_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantKind {
    Main,
    Memory,
    Custom,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assistant {
    pub assistant_code: String,
    pub kind: AssistantKind,
    pub enabled: bool,
    pub heartbeat_minutes: u32,
    pub memory_auto_approve: bool,
    pub is_main: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantSessionPurpose {
    Interactive,
    Heartbeat,
    Task,
    Feedback,
    MemoryBatch,
    SkillAuthoring,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSession {
    pub session_code: String,
    pub assistant_code: String,
    pub purpose: AssistantSessionPurpose,
    pub correlation_key: String,
    pub title: String,
    pub workspace_code: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantEventSource {
    UserMessage,
    Heartbeat,
    TaskReminder,
    ThreadFeedback,
    MemoryBatch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantEventStatus {
    Queued,
    Running,
    Completed,
    Failed,
    AwaitingInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantEvent {
    pub event_id: String,
    pub assistant_code: String,
    pub source: AssistantEventSource,
    pub dedup_key: String,
    pub session_code: Option<String>,
    pub task_code: Option<String>,
    pub workspace_code: Option<String>,
    pub payload: Value,
    pub status: AssistantEventStatus,
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
    pub assistant_code: Option<String>,
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
pub enum AssistantError {
    NotFound,
    Invalid,
    Conflict,
    ExecutionBusy,
    Storage,
    Runtime(String),
}
impl std::fmt::Display for AssistantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotFound => "PersonAI not found",
            Self::Invalid => "PersonAI configuration is invalid",
            Self::Conflict => "PersonAI state changed; refresh and retry",
            Self::ExecutionBusy => "PersonAI session is still executing; feedback remains queued",
            Self::Storage => "PersonAI storage unavailable",
            Self::Runtime(message) => message,
        })
    }
}
impl std::error::Error for AssistantError {}

/// A PersonAI-assessed business change, independent of execution lifecycle and delivery.
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
pub struct AssistantWorkChange {
    pub change_id: String,
    pub assistant_code: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoulState {
    Uninitialized,
    AwaitingUser,
    Ready,
}

/// One platform-owned identity per PersonAI. No account or workspace copies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSoul {
    pub assistant_code: String,
    pub name: String,
    pub name_confirmed: bool,
    pub markdown: String,
    pub state: SoulState,
    pub session_code: Option<String>,
    pub user_code: Option<String>,
    pub pending_interaction_id: Option<String>,
    pub awakening_started: bool,
    pub awakening_deferred: bool,
}
impl AssistantSoul {
    pub fn new(code: &str, name: &str) -> Self {
        Self { assistant_code: code.into(), name: name.into(), name_confirmed: true,
            markdown: "# 性格与相处方式\n真诚、友善、好奇，清晰直接地沟通；尊重用户的选择，主动协助解决问题。".into(), state: SoulState::Ready, session_code: None,
            user_code: None, pending_interaction_id: None, awakening_started: false,
            awakening_deferred: false }
    }
}
