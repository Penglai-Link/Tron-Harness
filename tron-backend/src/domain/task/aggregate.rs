use crate::shared::id::SnowflakeId;

use super::{TaskPriority, TaskSourceType, TaskStatus, TaskType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: SnowflakeId,
    pub task_code: String,
    pub workspace_code: String,
    pub parent_task_code: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub execution_result: Option<String>,
    pub status: TaskStatus,
    pub previous_status: Option<TaskStatus>,
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
