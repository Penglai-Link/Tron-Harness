use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::{
        auth::RequestAuthContext,
        chats::{
            error_mapper::{chat_error_response, steer_chat_message_request_rejection_response},
            handler::stream_message_command,
        },
    },
    app::AppState,
    response::ApiResponse,
};

use super::{
    request::OpenApiAnswerChatUserInputRequest, response::OpenApiChatUserInputRequestResponse,
};

pub async fn list_user_input_requests(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
) -> Response {
    match state.chats.list_user_input_requests(&session_code) {
        Ok(requests) => Json(ApiResponse::success(
            requests
                .into_iter()
                .map(OpenApiChatUserInputRequestResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn answer_user_input(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((session_code, request_code)): Path<(String, String)>,
    request: Result<Json<OpenApiAnswerChatUserInputRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match request {
        Ok(request) => request,
        Err(error) => return steer_chat_message_request_rejection_response(error),
    };
    let answers = request
        .answers
        .or_else(|| {
            request
                .resume
                .first()
                .and_then(|entry| entry.payload.get("answers"))
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        });
    let Some(answers) = answers else {
        return chat_error_response(crate::domain::chat::ChatError::UserInputAnswerInvalid(
            "resume payload must contain answers".into(),
        ));
    };
    let command = match state
        .chats
        .answer_user_input(&session_code, &request_code, answers)
    {
        Ok(command) => command,
        Err(error) => return chat_error_response(error),
    };
    stream_message_command(state, auth, session_code, command).await
}
