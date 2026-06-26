use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::{
        auth::RequestAuthContext,
        chats::{
            error_mapper::chat_send_error_response,
            handler::{prepare_send_model, resolve_chat_agent_runtime},
        },
    },
    app::AppState,
    application::{
        agent::{AgentInvocationService, generate_agent_run_code},
        chat::{CreateChatSessionCommand, SendChatMessageCommand},
        task::TaskService,
    },
    domain::{
        chat::ChatSessionSourceType,
        task::{TaskError, TaskExecutionResult, TaskListQuery},
    },
    response::ApiResponse,
};

use super::{
    error_mapper::task_error_response,
    request::{CreateTaskRequest, UpdateTaskRequest, UpdateTaskStatusRequest},
};

pub async fn options(State(tasks): State<TaskService>) -> Response {
    Json(ApiResponse::success(tasks.options())).into_response()
}

pub async fn kanban(
    State(tasks): State<TaskService>,
    Query(query): Query<TaskListQuery>,
) -> Response {
    match tasks.kanban(query) {
        Ok(columns) => Json(ApiResponse::success(columns)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn list_tasks(
    State(tasks): State<TaskService>,
    Query(query): Query<TaskListQuery>,
) -> Response {
    match tasks.list_tasks(query) {
        Ok(page) => Json(ApiResponse::success(page)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn create_task(
    State(tasks): State<TaskService>,
    Json(request): Json<CreateTaskRequest>,
) -> Response {
    match tasks.create_task(request.into()) {
        Ok(detail) => (StatusCode::CREATED, Json(ApiResponse::success(detail))).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn get_task(State(tasks): State<TaskService>, Path(task_code): Path<String>) -> Response {
    match tasks.get_task(&task_code) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn patch_task(
    State(tasks): State<TaskService>,
    Path(task_code): Path<String>,
    Json(request): Json<UpdateTaskRequest>,
) -> Response {
    match tasks.update_task(&task_code, request.into()) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn patch_status(
    State(tasks): State<TaskService>,
    Path(task_code): Path<String>,
    Json(request): Json<UpdateTaskStatusRequest>,
) -> Response {
    match tasks.update_status(&task_code, request.into()) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn execute_task(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(task_code): Path<String>,
) -> Response {
    let task = match state.tasks.get_task(&task_code) {
        Ok(task) => task,
        Err(error) => return task_error_response(error),
    };
    if task.archived_at.is_some() || task.status == crate::domain::task::TaskStatus::Archived {
        return task_error_response(TaskError::ArchivedMutationDenied);
    }

    let session = match state.chats.create_session(
        CreateChatSessionCommand {
            workspace_code: Some(task.workspace_code.clone()),
            title: Some(format!("Task: {}", task.title)),
            app_code: None,
            task_code: Some(task.task_code.clone()),
            source_type: Some(ChatSessionSourceType::Task),
            source_code: Some(task.task_code.clone()),
            agent_code: Some(task.agent_code.clone()),
        },
        Some(auth.user().user_code.clone()),
    ) {
        Ok(session) => session,
        Err(error) => {
            return crate::adapters::inbound::http::chats::error_mapper::chat_error_response(error);
        }
    };

    let run_code = generate_agent_run_code();
    let request = SendChatMessageCommand {
        run_code: Some(run_code.clone()),
        content: Some(task_execution_prompt(&task)),
        workspace_code: Some(task.workspace_code.clone()),
        app_code: None,
        task_code: Some(task.task_code.clone()),
        agent_code: Some(task.agent_code.clone()),
        model_code: task.model_code.clone(),
        system_model_id: task.model_code.clone(),
        limits: None,
        tools: Vec::new(),
        attachments: Vec::new(),
    };
    let runtime = match resolve_chat_agent_runtime(&state, &session, &request) {
        Ok(runtime) => runtime,
        Err(error) => return chat_send_error_response(error),
    };
    let model = match prepare_send_model(&state, &session, &request, &runtime) {
        Ok(model) => model,
        Err(error) => return chat_send_error_response(error),
    };
    let agent_service = AgentInvocationService::new(
        state.workspaces.clone(),
        state.apps.clone(),
        state.variables.clone(),
        state.skills.clone(),
        state.agent_management.clone(),
        state.agent_runs.clone(),
        state.mcp.clone(),
        state.event_bus.clone(),
        state.agent_model_client.clone(),
        state.agent_tool_registry.clone(),
        state.runtime_http_client.clone(),
    );
    let chats = state.chats.clone();
    let session_code = session.session_code.clone();
    let background_session_code = session_code.clone();
    let user_code = auth.user().user_code.clone();
    let locale = auth.locale.clone();
    let llm_runtime = state.llm_runtime.clone();
    let task_code_for_log = task.task_code.clone();
    let task_code_for_result = task.task_code.clone();
    let tasks = state.tasks.clone();
    tokio::spawn(async move {
        match chats
            .send_message(
                &background_session_code,
                request,
                model,
                Some(user_code),
                Some(locale),
                Some(llm_runtime),
                agent_service,
            )
            .await
        {
            Ok(result) => {
                if let Err(error) =
                    tasks.record_execution_result(&task_code_for_result, result.run.final_output)
                {
                    eprintln!(
                        "task execution {task_code_for_log} result persistence failed: {error:?}"
                    );
                }
            }
            Err(error) => {
                eprintln!("task execution {task_code_for_log} failed: {error:?}");
            }
        }
    });

    Json(ApiResponse::success(TaskExecutionResult {
        task_code: task.task_code,
        session_code,
        run_code,
        agent_code: session
            .agent_code
            .unwrap_or_else(|| "main-agent".to_string()),
        model_code: task.model_code,
        summary: "Task execution started.".to_string(),
    }))
    .into_response()
}

pub async fn archive_task(
    State(tasks): State<TaskService>,
    Path(task_code): Path<String>,
) -> Response {
    match tasks.archive_task(&task_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => task_error_response(error),
    }
}

fn task_execution_prompt(task: &crate::domain::task::TaskDetail) -> String {
    let description = task
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("No description provided.");
    let apps = if task.app_codes.is_empty() {
        "None".to_string()
    } else {
        task.app_codes.join(", ")
    };
    let source_url = task
        .source_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("None");
    format!(
        "Execute the following task and provide the final result. Use available task, app, memory, and platform tools when helpful. Do not store the final result in the task description; the platform records your final reply separately as the task execution result.\n\nTask code: {}\nTitle: {}\nType: {:?}\nStatus: {:?}\nPriority: {:?}\nProgress: {}%\nSource type: {:?}\nSource URL: {}\nApps: {}\n\nDescription:\n{}",
        task.task_code,
        task.title,
        task.task_type,
        task.status,
        task.priority,
        task.progress,
        task.source_type,
        source_url,
        apps,
        description,
    )
}

pub async fn restore_task(
    State(tasks): State<TaskService>,
    Path(task_code): Path<String>,
) -> Response {
    match tasks.restore_task(&task_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => task_error_response(error),
    }
}
