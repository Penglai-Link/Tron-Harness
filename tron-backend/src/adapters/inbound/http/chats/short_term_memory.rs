use axum::{
    Json,
    extract::rejection::{JsonRejection, QueryRejection},
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    app::AppState,
    application::short_term_memory::{
        CreateCompactionCommand, GoalSessionCommand, ModelCompactionFailure, SetGoalCommand,
        UpdateGoalCommand,
    },
    domain::short_term_memory::ShortTermMemoryError,
    response::{ApiResponse, EmptyData},
};

use super::super::auth::RequestAuthContext;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionCodeQuery {
    session_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateCompactionRequest {
    session_code: String,
    model_code: Option<String>,
    reason: Option<String>,
    note: Option<String>,
    todo_enabled: Option<bool>,
}

pub async fn get_todos(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    query: Result<Query<SessionCodeQuery>, QueryRejection>,
) -> Response {
    let session_code = match parse_session_code_query(query) {
        Ok(session_code) => session_code,
        Err(response) => return response,
    };
    match state.short_term_memory.list_todos(&session_code) {
        Ok(todos) => Json(ApiResponse::success(todos)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextDetailQuery {
    session_code: String,
    block_code: String,
}

pub async fn get_context_detail(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    query: Result<Query<ContextDetailQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(error) => return invalid_request_response(error.to_string()),
    };
    if query.session_code.trim().is_empty()
        || !matches!(
            query.block_code.as_str(),
            "systemPrompt" | "toolSchema" | "familyContext" | "applicationContext"
        )
    {
        return invalid_request_response("sessionCode and a supported blockCode are required");
    }
    match state
        .short_term_memory
        .context_detail(&query.session_code, &query.block_code)
    {
        Ok(Some(detail)) => Json(ApiResponse::success(detail)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "CHAT_CONTEXT_DETAIL_NOT_FOUND",
                "context detail is not available for this session",
            )),
        )
            .into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn get_latest_compaction(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    query: Result<Query<SessionCodeQuery>, QueryRejection>,
) -> Response {
    let session_code = match parse_session_code_query(query) {
        Ok(session_code) => session_code,
        Err(response) => return response,
    };
    match state.short_term_memory.latest_compaction(&session_code) {
        Ok(compaction) => Json(ApiResponse::success(compaction)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn create_compaction(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    command: Result<Json<CreateCompactionRequest>, JsonRejection>,
) -> Response {
    let command = match parse_json_command(command) {
        Ok(command) => command,
        Err(response) => return response,
    };
    let model_code = command.model_code.clone();
    let workspace_code = match state
        .short_term_memory
        .session_workspace_for_compaction(&command.session_code)
    {
        Ok(workspace_code) => workspace_code,
        Err(error) => return short_term_memory_error_response(error),
    };
    let base_model = match state
        .models
        .text_agent_model_config(&workspace_code, model_code.as_deref())
    {
        Ok(Some(model)) => model,
        Ok(None) => {
            return compaction_model_error_response(
                "COMPACTION_MODEL_NOT_CONFIGURED",
                "no default text model is configured for this workspace",
            );
        }
        Err(_) => return short_term_memory_error_response(ShortTermMemoryError::StorageFailure),
    };
    let model = state.short_term_memory.compaction_model_config(&base_model);
    let command = CreateCompactionCommand {
        session_code: command.session_code,
        reason: command.reason,
        note: command.note,
        todo_enabled: command.todo_enabled,
    };
    let input = match state
        .short_term_memory
        .prepare_manual_model_compaction_input(
            command,
            model.context_window_tokens,
            model.max_tokens,
        ) {
        Ok(input) => input,
        Err(error) => return short_term_memory_error_response(error),
    };
    let request = state
        .short_term_memory
        .compaction_model_request(&input, &auth.locale);
    let response = match state.agent_model_client.generate(&model, &request).await {
        Ok(response) => response,
        Err(message) => {
            let failure = ModelCompactionFailure {
                error_code: "COMPACTION_MODEL_REQUEST_FAILED".into(),
                message: message.clone(),
            };
            let _ = state
                .short_term_memory
                .record_compaction_failed(&input, &failure);
            return compaction_model_error_response("COMPACTION_MODEL_REQUEST_FAILED", message);
        }
    };
    let output = match state.short_term_memory.model_compaction_output(&response) {
        Ok(output) => output,
        Err(failure) => {
            let _ = state
                .short_term_memory
                .record_compaction_failed(&input, &failure);
            return compaction_model_error_response(
                "COMPACTION_MODEL_RESPONSE_INVALID",
                failure.message,
            );
        }
    };
    match state
        .short_term_memory
        .commit_model_compaction(input, output, &model)
    {
        Ok(compaction) => Json(ApiResponse::success(compaction)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

fn compaction_model_error_response(code: &'static str, message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(ApiResponse::<EmptyData>::error(code, message.into())),
    )
        .into_response()
}

pub async fn get_goal(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    query: Result<Query<SessionCodeQuery>, QueryRejection>,
) -> Response {
    let session_code = match parse_session_code_query(query) {
        Ok(session_code) => session_code,
        Err(response) => return response,
    };
    match state.short_term_memory.get_active_goal(&session_code) {
        Ok(goal) => Json(ApiResponse::success(goal)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn get_latest_goal(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    query: Result<Query<SessionCodeQuery>, QueryRejection>,
) -> Response {
    let session_code = match parse_session_code_query(query) {
        Ok(session_code) => session_code,
        Err(response) => return response,
    };
    match state.short_term_memory.get_latest_goal(&session_code) {
        Ok(goal) => Json(ApiResponse::success(goal)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn set_goal(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    command: Result<Json<SetGoalCommand>, JsonRejection>,
) -> Response {
    let command = match parse_json_command(command) {
        Ok(command) => command,
        Err(response) => return response,
    };
    match state
        .short_term_memory
        .set_goal_with_operator(command, Some(auth.user().user_code.clone()))
    {
        Ok(goal) => Json(ApiResponse::success(goal)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn pause_goal(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    command: Result<Json<GoalSessionCommand>, JsonRejection>,
) -> Response {
    let command = match parse_json_command(command) {
        Ok(command) => command,
        Err(response) => return response,
    };
    match state.short_term_memory.pause_goal(command) {
        Ok(goal) => Json(ApiResponse::success(goal)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn resume_goal(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    command: Result<Json<GoalSessionCommand>, JsonRejection>,
) -> Response {
    let command = match parse_json_command(command) {
        Ok(command) => command,
        Err(response) => return response,
    };
    match state.short_term_memory.resume_goal(command) {
        Ok(goal) => Json(ApiResponse::success(goal)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn cancel_goal(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    command: Result<Json<GoalSessionCommand>, JsonRejection>,
) -> Response {
    let command = match parse_json_command(command) {
        Ok(command) => command,
        Err(response) => return response,
    };
    match state.short_term_memory.cancel_goal(command) {
        Ok(goal) => Json(ApiResponse::success(goal)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn clear_goal(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    command: Result<Json<GoalSessionCommand>, JsonRejection>,
) -> Response {
    let command = match parse_json_command(command) {
        Ok(command) => command,
        Err(response) => return response,
    };
    match state.short_term_memory.clear_goal(command) {
        Ok(goal) => Json(ApiResponse::success(goal)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

pub async fn update_goal(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    command: Result<Json<UpdateGoalCommand>, JsonRejection>,
) -> Response {
    let command = match parse_json_command(command) {
        Ok(command) => command,
        Err(response) => return response,
    };
    match state.short_term_memory.update_goal(command) {
        Ok(goal) => Json(ApiResponse::success(goal)).into_response(),
        Err(error) => short_term_memory_error_response(error),
    }
}

fn parse_session_code_query(
    query: Result<Query<SessionCodeQuery>, QueryRejection>,
) -> Result<String, Response> {
    let Query(query) = query.map_err(|error| invalid_request_response(error.to_string()))?;
    let session_code = query.session_code.trim();
    if session_code.is_empty() {
        return Err(invalid_request_response("sessionCode is required"));
    }
    Ok(session_code.to_string())
}

fn parse_json_command<T>(command: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    command
        .map(|Json(command)| command)
        .map_err(|error| invalid_request_response(error.to_string()))
}

fn invalid_request_response(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(
            "SHORT_TERM_MEMORY_REQUEST_INVALID",
            message.into(),
        )),
    )
        .into_response()
}

fn short_term_memory_error_response(error: ShortTermMemoryError) -> Response {
    let (status, code, message) = match error {
        ShortTermMemoryError::SessionNotFound => (
            StatusCode::NOT_FOUND,
            "CHAT_SESSION_NOT_FOUND",
            "chat session was not found".to_string(),
        ),
        ShortTermMemoryError::CompactionConflict => (
            StatusCode::CONFLICT,
            "SHORT_TERM_MEMORY_COMPACTION_CONFLICT",
            "session has an active agent run".to_string(),
        ),
        ShortTermMemoryError::RunConflict => (
            StatusCode::CONFLICT,
            "SHORT_TERM_MEMORY_RUN_CONFLICT",
            "session already has an active agent run".to_string(),
        ),
        ShortTermMemoryError::CompactionInputTooLarge => (
            StatusCode::BAD_REQUEST,
            "SHORT_TERM_MEMORY_COMPACTION_INPUT_TOO_LARGE",
            "compaction input is too large".to_string(),
        ),
        ShortTermMemoryError::CompactionModelFailed => (
            StatusCode::BAD_GATEWAY,
            "SHORT_TERM_MEMORY_COMPACTION_MODEL_FAILED",
            "compaction model failed".to_string(),
        ),
        ShortTermMemoryError::ContextBudgetExceeded => (
            StatusCode::BAD_REQUEST,
            "SHORT_TERM_MEMORY_CONTEXT_BUDGET_EXCEEDED",
            "short-term memory context exceeded budget".to_string(),
        ),
        ShortTermMemoryError::TodoInvalid => (
            StatusCode::BAD_REQUEST,
            "SHORT_TERM_MEMORY_TODO_INVALID",
            "todo update is invalid".to_string(),
        ),
        ShortTermMemoryError::GoalNotFound => (
            StatusCode::NOT_FOUND,
            "SHORT_TERM_MEMORY_GOAL_NOT_FOUND",
            "session goal was not found".to_string(),
        ),
        ShortTermMemoryError::GoalConflict => (
            StatusCode::CONFLICT,
            "SHORT_TERM_MEMORY_GOAL_CONFLICT",
            "session already has an active goal".to_string(),
        ),
        ShortTermMemoryError::GoalStateConflict => (
            StatusCode::CONFLICT,
            "SHORT_TERM_MEMORY_GOAL_STATE_CONFLICT",
            "session goal was concurrently modified".to_string(),
        ),
        ShortTermMemoryError::GoalInvalid => (
            StatusCode::BAD_REQUEST,
            "SHORT_TERM_MEMORY_GOAL_INVALID",
            "goal command is invalid".to_string(),
        ),
        ShortTermMemoryError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SHORT_TERM_MEMORY_STORAGE_FAILED",
            "short-term memory storage failed".to_string(),
        ),
    };
    (status, Json(ApiResponse::error(code, message))).into_response()
}
