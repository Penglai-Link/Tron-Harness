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
    Storage,
    Runtime(String),
}
impl std::fmt::Display for DudeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotFound => "Dude not found",
            Self::Invalid => "Dude configuration is invalid",
            Self::Conflict => "Dude state changed; refresh and retry",
            Self::Storage => "Dude storage unavailable",
            Self::Runtime(message) => message,
        })
    }
}
impl std::error::Error for DudeError {}

impl DudeEvent {
    pub fn completion_notification(
        &self,
        status: DudeEventStatus,
        summary: &str,
        now: i64,
    ) -> Option<PlatformNotification> {
        if self.source == DudeEventSource::Heartbeat && status == DudeEventStatus::Completed {
            return None;
        }
        Some(PlatformNotification {
            id: format!("notification:{}:{status:?}", self.event_id),
            dedup_key: format!("event:{}:{status:?}", self.event_id),
            dude_code: Some(self.dude_code.clone()),
            title: match status {
                DudeEventStatus::Failed => "Dude 工作需要处理",
                DudeEventStatus::AwaitingInput => "Dude 需要你的回答",
                _ => "Dude 已完成工作",
            }
            .into(),
            body: summary.into(),
            category: format!("{status:?}"),
            target: serde_json::json!({"sessionCode":self.session_code,"taskCode":self.task_code,"eventId":self.event_id}),
            read_at: None,
            resolved_at: None,
            archived_at: None,
            created_at: now,
        })
    }
}
