//! Settings page session management: paged cross-workspace listing, batch
//! archive/delete/move and guaranteed run stopping.
use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::chat::{
        ChatSessionBatchResult, ChatSessionManagementQuery, ChatSessionMoveCommand,
    },
    response::{ApiResponse, EmptyData},
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tron_i18n::t;

const MAX_MANAGEMENT_BATCH: usize = 100;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListManagedSessionsQuery {
    #[serde(default)]
    pub workspace_code: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub current: Option<u32>,
    #[serde(default)]
    pub size: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BatchSessionsRequest {
    #[serde(default)]
    pub session_codes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MoveSessionsRequest {
    #[serde(default)]
    pub session_codes: Vec<String>,
    #[serde(default)]
    pub target_workspace_code: Option<String>,
    #[serde(default)]
    pub target_project_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StopSessionsRequest {
    #[serde(default)]
    pub session_codes: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ManagedSessionVo {
    pub session_code: String,
    pub workspace_code: String,
    pub project_code: Option<String>,
    pub directory_code: Option<String>,
    pub working_directory_code: Option<String>,
    pub task_code: Option<String>,
    pub source_type: String,
    pub title: String,
    pub status: String,
    pub active_run_code: Option<String>,
    pub running: bool,
    pub updated_at: i64,
    pub created_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ManagedSessionPageVo {
    pub records: Vec<ManagedSessionVo>,
    pub total: u64,
    pub current: u32,
    pub size: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BatchSessionResultVo {
    pub succeeded: u32,
    pub failed: u32,
    pub items: Vec<BatchSessionItemVo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BatchSessionItemVo {
    pub session_code: String,
    pub success: bool,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StopSessionsVo {
    pub stopped_session_codes: Vec<String>,
}

fn invalid_request(locale: SupportedLocale) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::<EmptyData>::error(
            "CHAT_SESSION_MANAGEMENT_INVALID",
            t!("error.chat.session_management_invalid", locale = locale),
        )),
    )
        .into_response()
}

/// Validate the session code list shape before reaching the service. Empty or
/// oversized batches are client errors, not business results.
fn validate_session_codes(codes: &[String], max: usize) -> Result<Vec<String>, tron_i18n::Message> {
    let mut normalized = Vec::with_capacity(codes.len());
    let mut seen = std::collections::BTreeSet::new();
    for code in codes {
        let code = code.trim().to_owned();
        if code.is_empty() {
            return Err(tron_i18n::Message::new(
                "error.chat.session_management_codes_empty_value",
            ));
        }
        if seen.insert(code.clone()) {
            normalized.push(code);
        }
    }
    if normalized.is_empty() {
        return Err(tron_i18n::Message::new(
            "error.chat.session_management_codes_required",
        ));
    }
    if normalized.len() > max {
        return Err(
            tron_i18n::Message::new("error.chat.session_management_codes_limit")
                .arg("max", max.to_string()),
        );
    }
    Ok(normalized)
}

pub(super) async fn list(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Query(query): Query<ListManagedSessionsQuery>,
) -> Response {
    let status = match query.status.as_deref() {
        None | Some("") => None,
        Some("ACTIVE") => Some(crate::domain::chat::ChatSessionStatus::Active),
        Some("ARCHIVED") => Some(crate::domain::chat::ChatSessionStatus::Archived),
        Some(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<EmptyData>::error(
                    "CHAT_SESSION_MANAGEMENT_INVALID",
                    t!(
                        "error.chat.session_management_status_invalid",
                        locale = SupportedLocale::from_raw(&_auth.locale)
                    ),
                )),
            )
                .into_response();
        }
    };
    let source_type = match query.source_type.as_deref() {
        None | Some("") => None,
        Some("MANUAL") => Some(crate::domain::chat::ChatSessionSourceType::Manual),
        Some("TASK") => Some(crate::domain::chat::ChatSessionSourceType::Task),
        Some("AUTOMATION") => Some(crate::domain::chat::ChatSessionSourceType::Automation),
        Some(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<EmptyData>::error(
                    "CHAT_SESSION_MANAGEMENT_INVALID",
                    t!(
                        "error.chat.session_management_source_type_invalid",
                        locale = SupportedLocale::from_raw(&_auth.locale)
                    ),
                )),
            )
                .into_response();
        }
    };
    let result = tokio::task::spawn_blocking(move || {
        state
            .chats
            .list_managed_sessions(ChatSessionManagementQuery {
                workspace_code: query.workspace_code,
                status,
                source_type,
                search: query.search,
                current: query.current.unwrap_or(1),
                size: query.size.unwrap_or(25),
            })
    })
    .await;
    let (rows, total) = match result {
        Ok(Ok(rows)) => rows,
        Ok(Err(error)) => {
            return crate::adapters::inbound::http::chats::error_mapper::chat_error_response(error);
        }
        Err(_) => {
            return crate::adapters::inbound::http::chats::error_mapper::chat_error_response(
                crate::domain::chat::ChatError::StorageFailure,
            );
        }
    };
    let current = query.current.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(25).clamp(1, 100);
    let records = rows
        .into_iter()
        .map(|row| ManagedSessionVo {
            session_code: row.session.session_code.clone(),
            workspace_code: row.session.workspace_code.clone(),
            project_code: row.session.project_code.clone(),
            directory_code: row.session.directory_code.clone(),
            working_directory_code: row.session.working_directory_code.clone(),
            task_code: row.session.task_code.clone(),
            source_type: format!("{:?}", row.session.source_type).to_uppercase(),
            title: row.session.title.clone(),
            status: match row.session.status {
                crate::domain::chat::ChatSessionStatus::Active => "ACTIVE".into(),
                crate::domain::chat::ChatSessionStatus::Archived => "ARCHIVED".into(),
            },
            active_run_code: row.active_run_code.clone(),
            running: row.running(),
            updated_at: crate::shared::time::timestamp_to_millis(&row.session.updated_at),
            created_at: crate::shared::time::timestamp_to_millis(&row.session.created_at),
        })
        .collect();
    Json(ApiResponse::success(ManagedSessionPageVo {
        records,
        total,
        current,
        size,
    }))
    .into_response()
}

fn batch_response(result: ChatSessionBatchResult) -> Response {
    Json(ApiResponse::success(BatchSessionResultVo {
        succeeded: result.succeeded,
        failed: result.failed,
        items: result
            .items
            .into_iter()
            .map(|item| BatchSessionItemVo {
                session_code: item.session_code,
                success: item.success,
                error_code: item.error_code,
                message: item.message,
            })
            .collect(),
    }))
    .into_response()
}

pub(super) async fn archive(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<BatchSessionsRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let locale = SupportedLocale::from_raw(&auth.locale);
    let Json(request) = match request {
        Ok(request) => request,
        Err(_) => return invalid_request(locale),
    };
    let codes = match validate_session_codes(&request.session_codes, MAX_MANAGEMENT_BATCH) {
        Ok(codes) => codes,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<EmptyData>::error(
                    "CHAT_SESSION_MANAGEMENT_INVALID",
                    message.render(locale),
                )),
            )
                .into_response();
        }
    };
    // Stop the sessions first so archive cannot race an active run.
    if let Err(error) = state.chats.stop_sessions(&codes).await {
        return stop_error_response(error, locale);
    }
    let result = tokio::task::spawn_blocking(move || state.chats.archive_sessions(&codes)).await;
    match result {
        Ok(Ok(result)) => {
            notify_sessions_changed(&result);
            batch_response(result)
        }
        Ok(Err(error)) => {
            crate::adapters::inbound::http::chats::error_mapper::chat_error_response(error)
        }
        Err(_) => crate::adapters::inbound::http::chats::error_mapper::chat_error_response(
            crate::domain::chat::ChatError::StorageFailure,
        ),
    }
}

pub(super) async fn delete(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<BatchSessionsRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let locale = SupportedLocale::from_raw(&auth.locale);
    let Json(request) = match request {
        Ok(request) => request,
        Err(_) => return invalid_request(locale),
    };
    let codes = match validate_session_codes(&request.session_codes, MAX_MANAGEMENT_BATCH) {
        Ok(codes) => codes,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<EmptyData>::error(
                    "CHAT_SESSION_MANAGEMENT_INVALID",
                    message.render(locale),
                )),
            )
                .into_response();
        }
    };
    if let Err(error) = state.chats.stop_sessions(&codes).await {
        return stop_error_response(error, locale);
    }
    let result = tokio::task::spawn_blocking(move || state.chats.delete_sessions(&codes)).await;
    match result {
        Ok(Ok(result)) => {
            notify_sessions_changed(&result);
            batch_response(result)
        }
        Ok(Err(error)) => {
            crate::adapters::inbound::http::chats::error_mapper::chat_error_response(error)
        }
        Err(_) => crate::adapters::inbound::http::chats::error_mapper::chat_error_response(
            crate::domain::chat::ChatError::StorageFailure,
        ),
    }
}

pub(super) async fn move_sessions(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<MoveSessionsRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let locale = SupportedLocale::from_raw(&auth.locale);
    let Json(request) = match request {
        Ok(request) => request,
        Err(_) => return invalid_request(locale),
    };
    let target_workspace = request
        .target_workspace_code
        .map(|code| code.trim().to_owned())
        .filter(|code| !code.is_empty());
    let Some(target_workspace) = target_workspace else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<EmptyData>::error(
                "CHAT_SESSION_MANAGEMENT_INVALID",
                t!(
                    "error.chat.session_management_target_workspace",
                    locale = locale
                ),
            )),
        )
            .into_response();
    };
    let codes = match validate_session_codes(&request.session_codes, MAX_MANAGEMENT_BATCH) {
        Ok(codes) => codes,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<EmptyData>::error(
                    "CHAT_SESSION_MANAGEMENT_INVALID",
                    message.render(locale),
                )),
            )
                .into_response();
        }
    };
    if let Err(error) = state.chats.stop_sessions(&codes).await {
        return stop_error_response(error, locale);
    }
    let command = ChatSessionMoveCommand {
        session_codes: codes,
        target_workspace_code: target_workspace,
        target_project_code: request.target_project_code,
    };
    let result = tokio::task::spawn_blocking(move || state.chats.move_sessions(command)).await;
    match result {
        Ok(Ok(result)) => {
            notify_sessions_changed(&result);
            batch_response(result)
        }
        Ok(Err(error)) => {
            crate::adapters::inbound::http::chats::error_mapper::chat_error_response(error)
        }
        Err(_) => crate::adapters::inbound::http::chats::error_mapper::chat_error_response(
            crate::domain::chat::ChatError::StorageFailure,
        ),
    }
}

pub(super) async fn stop(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<StopSessionsRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let locale = SupportedLocale::from_raw(&auth.locale);
    let Json(request) = match request {
        Ok(request) => request,
        Err(_) => return invalid_request(locale),
    };
    let codes = request.session_codes;
    if codes.iter().any(|code| code.trim().is_empty()) || codes.len() > MAX_MANAGEMENT_BATCH {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<EmptyData>::error(
                "CHAT_SESSION_MANAGEMENT_INVALID",
                t!(
                    "error.chat.session_management_codes_invalid",
                    locale = locale
                ),
            )),
        )
            .into_response();
    }
    match state.chats.stop_sessions(&codes).await {
        Ok(codes) => Json(ApiResponse::success(StopSessionsVo {
            stopped_session_codes: codes,
        }))
        .into_response(),
        Err(error) => stop_error_response(error, locale),
    }
}

fn stop_error_response(
    error: crate::application::chat::ChatSessionStopError,
    locale: SupportedLocale,
) -> Response {
    use crate::application::chat::ChatSessionStopError;
    let (status, code) = match error {
        ChatSessionStopError::SessionNotFound => (StatusCode::NOT_FOUND, "CHAT_SESSION_NOT_FOUND"),
        ChatSessionStopError::StopTimeout => (StatusCode::CONFLICT, "CHAT_SESSION_STOP_TIMEOUT"),
        ChatSessionStopError::Storage => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_SESSION_STOP_FAILED",
        ),
    };
    let message_key = match code {
        "CHAT_SESSION_NOT_FOUND" => "error.chat.session_management_not_found",
        "CHAT_SESSION_STOP_TIMEOUT" => "error.chat.session_management_stop_timeout",
        _ => "error.chat.session_management_stop_failed",
    };
    let message = t!(message_key, locale = locale);
    (status, Json(ApiResponse::<EmptyData>::error(code, message))).into_response()
}

/// Fan out shell events so open tabs and the chat sidebar drop changed rows.
fn notify_sessions_changed(result: &ChatSessionBatchResult) {
    for item in &result.items {
        if item.success {
            tracing::debug!(session_code = %item.session_code, "session management changed");
        }
    }
}
