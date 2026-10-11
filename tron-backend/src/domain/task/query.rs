use serde::Deserialize;

use super::{TaskPriority, TaskSourceType, TaskType};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListQuery {
    pub workspace_code: Option<String>,
    pub owner_dude_code: Option<String>,
    pub current: Option<usize>,
    pub size: Option<usize>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
    pub source_type: Option<TaskSourceType>,
    pub assignee_user_code: Option<String>,
    pub parent_task_code: Option<String>,
    pub include_archived: Option<bool>,
}
