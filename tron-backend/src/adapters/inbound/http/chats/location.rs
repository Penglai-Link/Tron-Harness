use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use super::error_mapper::chat_error_response;
use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, domain::chat::ChatError,
    response::ApiResponse,
};

#[derive(Deserialize)]
enum Placement {
    Topic,
    Quick,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveRequest {
    placement: Placement,
    project_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkingDirectoryRequest {
    directory_code: Option<String>,
}

pub async fn move_session(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(code): Path<String>,
    Json(request): Json<MoveRequest>,
) -> Response {
    let project = match (request.placement, request.project_code) {
        (Placement::Topic, Some(code)) if !code.trim().is_empty() => Some(code),
        (Placement::Quick, None) => None,
        _ => return chat_error_response(ChatError::ProjectInvalid),
    };
    match state.chats.move_session(&code, project) {
        Ok(session) => Json(ApiResponse::protocol_success(session)).into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn change_directory(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(code): Path<String>,
    Json(request): Json<WorkingDirectoryRequest>,
) -> Response {
    match state
        .chats
        .change_session_directory(&code, request.directory_code)
    {
        Ok(session) => Json(ApiResponse::protocol_success(session)).into_response(),
        Err(error) => chat_error_response(error),
    }
}
