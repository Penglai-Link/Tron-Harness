use crate::shared::id::SnowflakeId;

use super::{TaskPriority, TaskSourceType, TaskType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: SnowflakeId,
    pub task_code: String,
    pub workspace_code: Option<String>,
    pub owner_assistant_code: Option<String>,
    pub due_at: Option<i64>,
    pub planned_start_at: Option<i64>,
    pub next_followup_at: Option<i64>,
    pub work_session_code: Option<String>,
    pub schedule_version: i64,

    pub parent_task_code: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub execution_result: Option<String>,
    pub flow_code: String,
    pub status: String,
    pub previous_status: Option<String>,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    pub source_type: TaskSourceType,
    pub source_url: Option<String>,
    pub agent_code: String,
    pub model_code: Option<String>,
    pub progress: u8,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskUserAssignment {
    pub user_code: String,
    pub assigned_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCodeLink {
    pub code: String,
    pub linked_at: String,
}
