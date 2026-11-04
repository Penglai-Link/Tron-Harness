use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::task::TaskError, response::ApiResponse, shared::auth_holder::AuthHolder};

pub fn task_error_response(error: TaskError) -> Response {
    let locale = request_locale();
    let (status, code) = match &error {
        TaskError::NotFound => (StatusCode::NOT_FOUND, "TASK_NOT_FOUND"),
        TaskError::WorkspaceRequired => (StatusCode::BAD_REQUEST, "TASK_WORKSPACE_REQUIRED"),
        TaskError::InvalidWorkspace => (StatusCode::BAD_REQUEST, "TASK_WORKSPACE_INVALID"),
        TaskError::InvalidParent => (StatusCode::BAD_REQUEST, "TASK_PARENT_INVALID"),
        TaskError::AssigneeNotInWorkspace => (StatusCode::BAD_REQUEST, "TASK_ASSIGNEE_INVALID"),
        TaskError::InvalidApp => (StatusCode::BAD_REQUEST, "TASK_APP_INVALID"),
        TaskError::ActiveSubtaskExists => (StatusCode::BAD_REQUEST, "TASK_ACTIVE_SUBTASK_EXISTS"),
        TaskError::ArchivedMutationDenied => (StatusCode::BAD_REQUEST, "TASK_ARCHIVED_INVALID"),
        TaskError::StatusNotInFlow => (StatusCode::BAD_REQUEST, "TASK_STATUS_FLOW_INVALID"),
        TaskError::FlowNotFound => (StatusCode::NOT_FOUND, "TASK_FLOW_NOT_FOUND"),
        TaskError::FlowCodeConflict => (StatusCode::CONFLICT, "TASK_FLOW_CODE_CONFLICT"),
        TaskError::FlowInUse => (StatusCode::CONFLICT, "TASK_FLOW_IN_USE"),
        TaskError::WorkspaceFlowHasOpenTasks => (
            StatusCode::CONFLICT,
            "TASK_FLOW_SWITCH_REQUIRES_CLOSED_TASKS",
        ),
        TaskError::InvalidProgress | TaskError::InvalidRequest => {
            (StatusCode::BAD_REQUEST, "TASK_REQUEST_INVALID")
        }
        TaskError::StorageFailure => (StatusCode::INTERNAL_SERVER_ERROR, "TASK_STORAGE_FAILED"),
    };
    (
        status,
        Json(ApiResponse::error(
            code,
            localized_task_error(&error, &locale),
        )),
    )
        .into_response()
}

pub fn task_prompt_render_error_response() -> Response {
    let locale = request_locale();
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::error(
            "TASK_PROMPT_RENDER_FAILED",
            localized_message(
                &locale,
                "任务执行提示词无法渲染",
                "Task execution prompt could not be rendered",
            ),
        )),
    )
        .into_response()
}

fn request_locale() -> String {
    AuthHolder::get()
        .map(|context| context.locale.clone())
        .unwrap_or_else(|| "zh-CN".to_string())
}

fn localized_task_error(error: &TaskError, locale: &str) -> String {
    let (zh, en) = match error {
        TaskError::NotFound => ("任务不存在", "Task was not found"),
        TaskError::WorkspaceRequired => ("必须提供工作区编码", "workspaceCode is required"),
        TaskError::InvalidWorkspace => (
            "任务工作区必须是已启用的工作区",
            "Task workspace must reference an active workspace",
        ),
        TaskError::InvalidParent => (
            "父任务必须位于同一已启用工作区",
            "Parent task must exist in the same active workspace",
        ),
        TaskError::AssigneeNotInWorkspace => (
            "任务负责人必须是工作区中的有效用户",
            "Task assignees must be effective users in the task workspace",
        ),
        TaskError::InvalidApp => (
            "任务应用必须是工作区中的已启用应用",
            "Task apps must reference active apps in the task workspace",
        ),
        TaskError::ActiveSubtaskExists => (
            "归档父任务前请先归档进行中的子任务",
            "Archive active subtasks before archiving the parent task",
        ),
        TaskError::ArchivedMutationDenied => (
            "修改已归档任务前请先恢复任务",
            "Restore archived task before modifying it",
        ),
        TaskError::StatusNotInFlow => (
            "任务状态不属于当前工作流",
            "Task status is unavailable in the current task flow",
        ),
        TaskError::FlowNotFound => ("任务流不存在", "Task flow was not found"),
        TaskError::FlowCodeConflict => ("任务流编码已存在", "Task flow code already exists"),
        TaskError::FlowInUse => (
            "任务流已分配给工作区或任务",
            "Task flow is assigned to a workspace or task",
        ),
        TaskError::WorkspaceFlowHasOpenTasks => (
            "切换任务流前必须关闭工作区中的所有任务",
            "Close every task before switching the task flow",
        ),
        TaskError::InvalidProgress | TaskError::InvalidRequest => {
            ("任务请求无效", "Task request is invalid")
        }
        TaskError::StorageFailure => ("任务存储操作失败", "Task storage operation failed"),
    };
    localized_message(locale, zh, en)
}

fn localized_message(locale: &str, zh: &str, en: &str) -> String {
    if locale.to_ascii_lowercase().starts_with("zh") {
        zh.to_string()
    } else {
        en.to_string()
    }
}
