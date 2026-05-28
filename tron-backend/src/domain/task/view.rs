use serde::Serialize;

use super::{TaskPriority, TaskSourceType, TaskStatus, TaskType};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusRules {
    pub feature_root: Vec<TaskStatus>,
    pub feature_child: Vec<TaskStatus>,
    pub bug: Vec<TaskStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDefaultStatuses {
    pub feature_root: TaskStatus,
    pub feature_child: TaskStatus,
    pub bug: TaskStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOptions {
    pub statuses: Vec<TaskStatus>,
    pub priorities: Vec<TaskPriority>,
    pub task_types: Vec<TaskType>,
    pub source_types: Vec<TaskSourceType>,
    pub status_rules: TaskStatusRules,
    pub default_statuses: TaskDefaultStatuses,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub task_code: String,
    pub title: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub progress: u8,
    pub task_type: TaskType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub id: String,
    pub task_code: String,
    pub workspace_code: String,
    pub parent_task_code: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub previous_status: Option<TaskStatus>,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    pub source_type: TaskSourceType,
    pub source_url: Option<String>,
    pub progress: u8,
    pub assignee_user_codes: Vec<String>,
    pub agent_codes: Vec<String>,
    pub app_codes: Vec<String>,
    pub parent: Option<TaskSummary>,
    pub subtasks: Vec<TaskSummary>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListItem {
    pub task_code: String,
    pub workspace_code: String,
    pub parent_task_code: Option<String>,
    pub title: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub progress: u8,
    pub task_type: TaskType,
    pub source_type: TaskSourceType,
    pub source_url: Option<String>,
    pub assignee_user_codes: Vec<String>,
    pub agent_codes: Vec<String>,
    pub app_codes: Vec<String>,
    pub subtask_count: usize,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskKanbanColumn {
    pub status: TaskStatus,
    pub title: String,
    pub tasks: Vec<TaskListItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskLifecycleResult {
    pub task_code: String,
    pub status: TaskStatus,
    pub previous_status: Option<TaskStatus>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedTaskRecords<T> {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<T>,
}
