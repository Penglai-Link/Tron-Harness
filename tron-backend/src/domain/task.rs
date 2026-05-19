use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Open,
    Designing,
    Red,
    Green,
    Regressioning,
    Review,
    InProgress,
    Reopen,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Feature,
    Bug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskSourceType {
    Internal,
    GitHub,
    GitLab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRole {
    FeatureRoot,
    FeatureChild,
    Bug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: SnowflakeId,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListQuery {
    pub workspace_code: Option<String>,
    pub current: Option<usize>,
    pub size: Option<usize>,
    pub search: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
    pub source_type: Option<TaskSourceType>,
    pub assignee_user_code: Option<String>,
    pub parent_task_code: Option<String>,
    pub include_archived: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    NotFound,
    WorkspaceRequired,
    InvalidWorkspace,
    InvalidParent,
    AssigneeNotInWorkspace,
    ActiveSubtaskExists,
    ArchivedMutationDenied,
    StatusRoleInvalid,
    InvalidProgress,
    InvalidRequest,
    StorageFailure,
}

impl fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("task was not found"),
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::InvalidWorkspace => formatter.write_str("task workspace is invalid"),
            Self::InvalidParent => formatter.write_str("task parent is invalid"),
            Self::AssigneeNotInWorkspace => formatter.write_str("task assignee is invalid"),
            Self::ActiveSubtaskExists => formatter.write_str("task has active subtasks"),
            Self::ArchivedMutationDenied => formatter.write_str("archived task cannot be modified"),
            Self::StatusRoleInvalid => {
                formatter.write_str("task status does not match the inferred task role")
            }
            Self::InvalidProgress => formatter.write_str("task progress is invalid"),
            Self::InvalidRequest => formatter.write_str("task request is invalid"),
            Self::StorageFailure => formatter.write_str("task storage failure"),
        }
    }
}

impl Error for TaskError {}

pub const TASK_STATUSES: [TaskStatus; 10] = [
    TaskStatus::Open,
    TaskStatus::Designing,
    TaskStatus::Red,
    TaskStatus::Green,
    TaskStatus::Regressioning,
    TaskStatus::Review,
    TaskStatus::InProgress,
    TaskStatus::Reopen,
    TaskStatus::Completed,
    TaskStatus::Archived,
];
pub const FEATURE_ROOT_STATUSES: [TaskStatus; 8] = [
    TaskStatus::Open,
    TaskStatus::Designing,
    TaskStatus::Red,
    TaskStatus::Green,
    TaskStatus::Regressioning,
    TaskStatus::Review,
    TaskStatus::Completed,
    TaskStatus::Archived,
];
pub const FEATURE_CHILD_STATUSES: [TaskStatus; 4] = [
    TaskStatus::Open,
    TaskStatus::InProgress,
    TaskStatus::Completed,
    TaskStatus::Archived,
];
pub const BUG_STATUSES: [TaskStatus; 5] = [
    TaskStatus::Open,
    TaskStatus::InProgress,
    TaskStatus::Completed,
    TaskStatus::Reopen,
    TaskStatus::Archived,
];
pub const TASK_PRIORITIES: [TaskPriority; 3] =
    [TaskPriority::Low, TaskPriority::Medium, TaskPriority::High];
pub const TASK_TYPES: [TaskType; 2] = [TaskType::Feature, TaskType::Bug];
pub const TASK_SOURCE_TYPES: [TaskSourceType; 3] = [
    TaskSourceType::Internal,
    TaskSourceType::GitHub,
    TaskSourceType::GitLab,
];

pub fn infer_task_role(task_type: TaskType, parent_task_code: Option<&str>) -> TaskRole {
    match task_type {
        TaskType::Bug => TaskRole::Bug,
        TaskType::Feature => {
            let has_parent = parent_task_code.is_some_and(|value| !value.trim().is_empty());
            if has_parent {
                TaskRole::FeatureChild
            } else {
                TaskRole::FeatureRoot
            }
        }
    }
}

pub fn task_statuses_for_role(role: TaskRole) -> &'static [TaskStatus] {
    match role {
        TaskRole::FeatureRoot => &FEATURE_ROOT_STATUSES,
        TaskRole::FeatureChild => &FEATURE_CHILD_STATUSES,
        TaskRole::Bug => &BUG_STATUSES,
    }
}

pub fn task_default_status_for_role(role: TaskRole) -> TaskStatus {
    match role {
        TaskRole::FeatureRoot | TaskRole::FeatureChild | TaskRole::Bug => TaskStatus::Open,
    }
}

pub fn task_status_allowed_for_role(role: TaskRole, status: TaskStatus) -> bool {
    task_statuses_for_role(role).contains(&status)
}

pub fn task_options() -> TaskOptions {
    TaskOptions {
        statuses: TASK_STATUSES.to_vec(),
        priorities: TASK_PRIORITIES.to_vec(),
        task_types: TASK_TYPES.to_vec(),
        source_types: TASK_SOURCE_TYPES.to_vec(),
        status_rules: TaskStatusRules {
            feature_root: FEATURE_ROOT_STATUSES.to_vec(),
            feature_child: FEATURE_CHILD_STATUSES.to_vec(),
            bug: BUG_STATUSES.to_vec(),
        },
        default_statuses: TaskDefaultStatuses {
            feature_root: task_default_status_for_role(TaskRole::FeatureRoot),
            feature_child: task_default_status_for_role(TaskRole::FeatureChild),
            bug: task_default_status_for_role(TaskRole::Bug),
        },
    }
}

pub fn task_status_to_storage(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Open => "Open",
        TaskStatus::Designing => "Designing",
        TaskStatus::Red => "Red",
        TaskStatus::Green => "Green",
        TaskStatus::Regressioning => "Regressioning",
        TaskStatus::Review => "Review",
        TaskStatus::InProgress => "InProgress",
        TaskStatus::Reopen => "Reopen",
        TaskStatus::Completed => "Completed",
        TaskStatus::Archived => "Archived",
    }
}

pub fn task_status_from_storage(value: &str) -> Result<TaskStatus, TaskError> {
    match value {
        "Open" => Ok(TaskStatus::Open),
        "Designing" => Ok(TaskStatus::Designing),
        "Red" => Ok(TaskStatus::Red),
        "Green" => Ok(TaskStatus::Green),
        "Regressioning" => Ok(TaskStatus::Regressioning),
        "Review" => Ok(TaskStatus::Review),
        "InProgress" => Ok(TaskStatus::InProgress),
        "Reopen" => Ok(TaskStatus::Reopen),
        "Completed" => Ok(TaskStatus::Completed),
        "Archived" => Ok(TaskStatus::Archived),
        _ => Err(TaskError::InvalidRequest),
    }
}

pub fn task_priority_to_storage(priority: TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Low => "Low",
        TaskPriority::Medium => "Medium",
        TaskPriority::High => "High",
    }
}

pub fn task_priority_from_storage(value: &str) -> Result<TaskPriority, TaskError> {
    match value {
        "Low" => Ok(TaskPriority::Low),
        "Medium" => Ok(TaskPriority::Medium),
        "High" => Ok(TaskPriority::High),
        _ => Err(TaskError::InvalidRequest),
    }
}

pub fn task_type_to_storage(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::Feature => "Feature",
        TaskType::Bug => "Bug",
    }
}

pub fn task_type_from_storage(value: &str) -> Result<TaskType, TaskError> {
    match value {
        "Feature" => Ok(TaskType::Feature),
        "Bug" => Ok(TaskType::Bug),
        _ => Err(TaskError::InvalidRequest),
    }
}

pub fn task_source_type_to_storage(source_type: TaskSourceType) -> &'static str {
    match source_type {
        TaskSourceType::Internal => "Internal",
        TaskSourceType::GitHub => "GitHub",
        TaskSourceType::GitLab => "GitLab",
    }
}

pub fn task_source_type_from_storage(value: &str) -> Result<TaskSourceType, TaskError> {
    match value {
        "Internal" => Ok(TaskSourceType::Internal),
        "GitHub" => Ok(TaskSourceType::GitHub),
        "GitLab" => Ok(TaskSourceType::GitLab),
        _ => Err(TaskError::InvalidRequest),
    }
}
