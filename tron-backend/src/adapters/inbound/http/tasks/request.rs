use serde::Deserialize;

use crate::{
    application::task::{CreateTaskCommand, UpdateTaskCommand, UpdateTaskStatusCommand},
    domain::task::{TaskPriority, TaskSourceType, TaskStatus, TaskType},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateTaskRequest {
    pub workspace_code: String,
    pub parent_task_code: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub task_type: TaskType,
    pub source_type: TaskSourceType,
    pub source_url: Option<String>,
    pub priority: TaskPriority,
    #[serde(default)]
    pub assignee_user_codes: Vec<String>,
    #[serde(default)]
    pub agent_codes: Vec<String>,
    #[serde(default)]
    pub app_codes: Vec<String>,
}

impl From<CreateTaskRequest> for CreateTaskCommand {
    fn from(request: CreateTaskRequest) -> Self {
        Self {
            workspace_code: request.workspace_code,
            parent_task_code: request.parent_task_code,
            title: request.title,
            description: request.description,
            priority: request.priority,
            task_type: request.task_type,
            source_type: request.source_type,
            source_url: request.source_url,
            assignee_user_codes: request.assignee_user_codes,
            agent_codes: request.agent_codes,
            app_codes: request.app_codes,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
    pub source_type: Option<TaskSourceType>,
    pub source_url: Option<Option<String>>,
    pub assignee_user_codes: Option<Vec<String>>,
    pub agent_codes: Option<Vec<String>>,
    pub app_codes: Option<Vec<String>>,
}

impl From<UpdateTaskRequest> for UpdateTaskCommand {
    fn from(request: UpdateTaskRequest) -> Self {
        Self {
            title: request.title,
            description: request.description,
            priority: request.priority,
            task_type: request.task_type,
            source_type: request.source_type,
            source_url: request.source_url,
            assignee_user_codes: request.assignee_user_codes,
            agent_codes: request.agent_codes,
            app_codes: request.app_codes,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateTaskStatusRequest {
    pub status: TaskStatus,
    pub progress: Option<u8>,
}

impl From<UpdateTaskStatusRequest> for UpdateTaskStatusCommand {
    fn from(request: UpdateTaskStatusRequest) -> Self {
        Self {
            status: request.status,
            progress: request.progress,
        }
    }
}
