use serde::Serialize;

use crate::{
    application::chat::{
        ActiveChatRunSnapshot, ChatSteerResult, GetActiveChatRunResult, StopChatRunResult,
    },
    domain::chat::ChatUserInputRequest,
    domain::chat::{ChatMessageAttachment, ChatMessageChannel, ChatMessageRole},
    shared::time::{option_timestamp_to_millis, timestamp_to_millis},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetActiveChatRunResponse {
    pub session_code: String,
    pub active_run: Option<ActiveChatRunResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveChatRunResponse {
    pub run_code: String,
    pub runtime_adapter_key: String,
    pub supports_steer: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopChatRunResponse {
    pub session_code: String,
    pub run_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatUserInputRequestResponse {
    pub request_code: String,
    pub session_code: String,
    pub run_code: String,
    pub assistant_message_code: Option<String>,
    pub intro: String,
    pub questions: Vec<crate::domain::chat::ChatUserInputQuestion>,
    pub answers: Option<Vec<crate::domain::chat::ChatUserInputAnswer>>,
    pub status: crate::domain::chat::ChatUserInputRequestStatus,
    pub requested_at: i64,
    pub answered_at: Option<i64>,
}

impl From<ChatUserInputRequest> for ChatUserInputRequestResponse {
    fn from(value: ChatUserInputRequest) -> Self {
        Self {
            request_code: value.request_code,
            session_code: value.session_code,
            run_code: value.run_code,
            assistant_message_code: value.assistant_message_code,
            intro: value.intro,
            questions: value.questions,
            answers: value.answers,
            status: value.status,
            requested_at: timestamp_to_millis(&value.requested_at),
            answered_at: option_timestamp_to_millis(value.answered_at.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSteerResultResponse {
    pub session_code: String,
    pub run_code: String,
    pub message: SteeredChatMessageResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeredChatMessageResponse {
    pub id: String,
    pub message_code: String,
    pub session_code: String,
    pub run_code: String,
    pub role: ChatMessageRole,
    pub channel: ChatMessageChannel,
    pub actor: String,
    pub content: String,
    pub attachments: Vec<ChatMessageAttachment>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub created_at: String,
}

impl From<ChatSteerResult> for ChatSteerResultResponse {
    fn from(value: ChatSteerResult) -> Self {
        let message = value.message;
        Self {
            session_code: value.session_code,
            run_code: value.run_code,
            message: SteeredChatMessageResponse {
                id: message.id.to_string(),
                message_code: message.message_code,
                session_code: message.session_code,
                run_code: message.run_code.unwrap_or_default(),
                role: message.role,
                channel: message.channel,
                actor: message.actor,
                content: message.content,
                attachments: message.attachments,
                input_tokens: message.input_tokens,
                output_tokens: message.output_tokens,
                total_tokens: message.total_tokens,
                created_at: message.created_at,
            },
        }
    }
}

impl From<GetActiveChatRunResult> for GetActiveChatRunResponse {
    fn from(value: GetActiveChatRunResult) -> Self {
        Self {
            session_code: value.session_code,
            active_run: value.active_run.map(ActiveChatRunResponse::from),
        }
    }
}

impl From<ActiveChatRunSnapshot> for ActiveChatRunResponse {
    fn from(value: ActiveChatRunSnapshot) -> Self {
        Self {
            run_code: value.run_code,
            runtime_adapter_key: value.runtime_adapter_key,
            supports_steer: value.supports_steer,
            state: value.state.code().to_string(),
        }
    }
}

impl From<StopChatRunResult> for StopChatRunResponse {
    fn from(value: StopChatRunResult) -> Self {
        Self {
            session_code: value.session_code,
            run_code: value.run_code,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedSessionResponse {
    pub session_code: String,
    pub workspace_code: String,
    pub project_code: Option<String>,
    pub title: String,
    pub status: eliteforge_agent_core::SessionStatus,
    pub created_at: i64,
    pub updated_at: i64,
}
impl From<crate::domain::chat::ChatSession> for ArchivedSessionResponse {
    fn from(s: crate::domain::chat::ChatSession) -> Self {
        Self {
            session_code: s.session_code,
            workspace_code: s.workspace_code,
            project_code: s.project_code,
            title: s.title,
            status: match s.status {
                crate::domain::chat::ChatSessionStatus::Active => {
                    eliteforge_agent_core::SessionStatus::Active
                }
                crate::domain::chat::ChatSessionStatus::Archived => {
                    eliteforge_agent_core::SessionStatus::Archived
                }
            },
            created_at: timestamp_to_millis(&s.created_at),
            updated_at: timestamp_to_millis(&s.updated_at),
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedSessionPageResponse {
    pub records: Vec<ArchivedSessionResponse>,
    pub total: u64,
    pub current: u32,
    pub size: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedMessageResponse {
    pub message_code: String,
    pub run_code: Option<String>,
    pub role: String,
    pub actor: String,
    pub content: String,
    pub attachments: Vec<ChatMessageAttachment>,
    pub created_at: i64,
}
impl From<crate::domain::chat::ChatMessage> for ArchivedMessageResponse {
    fn from(m: crate::domain::chat::ChatMessage) -> Self {
        Self {
            message_code: m.message_code,
            run_code: m.run_code,
            role: format!("{:?}", m.role),
            actor: m.actor,
            content: m.content,
            attachments: m.attachments,
            created_at: timestamp_to_millis(&m.created_at),
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedGoalResponse {
    pub goal_code: String,
    pub objective: String,
    pub status: eliteforge_agent_core::GoalStatus,
    pub progress: serde_json::Value,
    pub criteria: Vec<eliteforge_agent_core::GoalCriterion>,
    pub last_decision: Option<eliteforge_agent_core::GoalDecision>,
    pub runs_used: u32,
    pub max_runs: u32,
    pub reason: Option<String>,
    pub updated_at: u64,
}
impl From<eliteforge_agent_core::GoalRecord> for ArchivedGoalResponse {
    fn from(g: eliteforge_agent_core::GoalRecord) -> Self {
        Self {
            goal_code: g.id,
            objective: g.objective,
            status: g.status,
            progress: g.progress,
            criteria: g.criteria,
            last_decision: g.last_decision,
            runs_used: g.runs_used,
            max_runs: g.max_runs,
            reason: g.reason,
            updated_at: g.updated_at_ms,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedGoalEventResponse {
    pub event_code: String,
    pub goal_code: String,
    pub run_code: Option<String>,
    pub kind: String,
    pub payload: serde_json::Value,
    pub created_at: u64,
}
impl From<eliteforge_agent_core::SessionEvent> for ArchivedGoalEventResponse {
    fn from(e: eliteforge_agent_core::SessionEvent) -> Self {
        Self {
            event_code: e.id,
            goal_code: e.goal_id.expect("goal event must have a goal identity"),
            run_code: e.run_id.map(|id| id.to_string()),
            kind: e.kind,
            payload: e.payload,
            created_at: e.created_at_ms,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedRunResponse {
    pub run_code: String,
    pub parent_run_code: Option<String>,
    pub branch: String,
    pub kind: String,
    pub status: eliteforge_agent_core::ManagedRunStatus,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub error: Option<String>,
}
impl From<eliteforge_agent_core::ManagedRun> for ArchivedRunResponse {
    fn from(r: eliteforge_agent_core::ManagedRun) -> Self {
        Self {
            run_code: r.id.to_string(),
            parent_run_code: r.parent_run_id.map(|id| id.to_string()),
            branch: r.conversation_branch,
            kind: r.kind,
            status: r.status,
            started_at: r.started_at_ms,
            finished_at: r.finished_at_ms,
            error: r.error,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedHistoryBranchResponse {
    pub branch: String,
    pub generation: String,
    pub messages: Vec<ArchivedHistoryMessageResponse>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedHistoryMessageResponse {
    pub message_code: String,
    pub role: String,
    pub content: String,
}
impl From<eliteforge_agent_core::ConversationSnapshot> for ArchivedHistoryBranchResponse {
    fn from(s: eliteforge_agent_core::ConversationSnapshot) -> Self {
        let prefix = format!("{}:{}:{}", s.key.session_id, s.key.branch, s.generation);
        Self {
            branch: s.key.branch,
            generation: s.generation,
            messages: s
                .messages
                .into_iter()
                .enumerate()
                .map(|(index, m)| ArchivedHistoryMessageResponse {
                    message_code: format!("{prefix}:{}", index + 1),
                    role: format!("{:?}", m.role),
                    content: m.content.text(),
                })
                .collect(),
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedSessionDetailResponse {
    pub session: ArchivedSessionResponse,
    pub messages: Vec<ArchivedMessageResponse>,
    pub goals: Vec<ArchivedGoalResponse>,
    pub goal_events: Vec<ArchivedGoalEventResponse>,
    pub runs: Vec<ArchivedRunResponse>,
    pub history: Vec<ArchivedHistoryBranchResponse>,
}
