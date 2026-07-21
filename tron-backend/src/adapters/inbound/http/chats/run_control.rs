use axum::{
    Json,
    extract::{
        Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState,
    application::chat::StopChatRunCommand, response::ApiResponse,
};

use super::{
    error_mapper::{
        active_chat_run_query_rejection_response, chat_run_control_error_response,
        stop_chat_run_request_rejection_response,
    },
    request::{GetActiveChatRunQuery, StopChatRunRequest},
    response::{GetActiveChatRunResponse, StopChatRunResponse},
};

pub async fn get_active_run(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    query: Result<Query<GetActiveChatRunQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(error) => return active_chat_run_query_rejection_response(error),
    };
    match state
        .chats
        .get_active_run(query.session_code.as_deref().unwrap_or_default())
    {
        Ok(result) => {
            Json(ApiResponse::success(GetActiveChatRunResponse::from(result))).into_response()
        }
        Err(error) => chat_run_control_error_response(error),
    }
}

pub async fn stop_run(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    request: Result<Json<StopChatRunRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match request {
        Ok(request) => request,
        Err(error) => return stop_chat_run_request_rejection_response(error),
    };
    match state.chats.stop_run(StopChatRunCommand {
        session_code: request.session_code,
        expected_run_code: request.expected_run_code,
    }) {
        Ok(result) => Json(ApiResponse::success(StopChatRunResponse::from(result))).into_response(),
        Err(error) => chat_run_control_error_response(error),
    }
}
