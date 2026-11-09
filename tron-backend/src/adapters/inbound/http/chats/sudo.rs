//! Private, one-shot sudo answers never enter chat/native interaction stores.
use super::super::auth::RequestAuthContext;
use crate::{
    app::AppState,
    application::agent::framework_runtime::{PendingSudoInput, SudoInputError},
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SudoInputItem {
    request_id: String,
    command: String,
    args: Vec<String>,
    cwd: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerSudoInputRequest {
    // null explicitly cancels. No Debug derive: this contains a credential.
    #[serde(deserialize_with = "deserialize_password")]
    password: Option<String>,
}
fn deserialize_password<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}
fn authorize(state: &AppState, auth: &RequestAuthContext, session: &str) -> Result<(), Response> {
    let chat = state
        .chats
        .get_session(session)
        .map_err(super::error_mapper::chat_error_response)?;
    if chat.user_code.as_deref() != Some(auth.user().user_code.as_str()) {
        return Err(error(SudoInputError::Unavailable, &auth.locale));
    }
    Ok(())
}
fn error(error: SudoInputError, locale: &str) -> Response {
    let descriptor = error.descriptor();
    let message = match (descriptor.message_key, locale.starts_with("en")) {
        ("tron.chat.sudo_input_invalid", true) => {
            "Enter a valid sudo password (one line, up to 4096 bytes)."
        }
        ("tron.chat.sudo_input_invalid", false) => {
            "请输入有效的 sudo 密码（单行，最多 4096 字节）。"
        }
        (_, true) => "This sudo input is unavailable, expired or already answered.",
        (_, false) => "此 sudo 输入请求不可用、已过期或已提交。",
    };
    (
        match error {
            SudoInputError::Invalid => StatusCode::BAD_REQUEST,
            SudoInputError::Unavailable => StatusCode::NOT_FOUND,
        },
        Json(ApiResponse::protocol_error(descriptor.code, message)),
    )
        .into_response()
}
pub async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(session): Path<String>,
) -> Result<Json<ApiResponse<Vec<SudoInputItem>>>, Response> {
    authorize(&state, &auth, &session)?;
    let inputs = state
        .agent_framework
        .pending_sudo_inputs(&session, &auth.user().user_code);
    Ok(Json(ApiResponse::protocol_success(
        inputs
            .into_iter()
            .map(
                |PendingSudoInput {
                     id,
                     command,
                     args,
                     cwd,
                 }| SudoInputItem {
                    request_id: id,
                    command,
                    args,
                    cwd,
                },
            )
            .collect(),
    )))
}
pub async fn answer(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((session, id)): Path<(String, String)>,
    body: Result<Json<AnswerSudoInputRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<EmptyData>>, Response> {
    let Json(body) = body.map_err(|_| error(SudoInputError::Invalid, &auth.locale))?;
    // Wrap immediately so denied, invalid and consumed values are wiped on drop.
    let password = body
        .password
        .map(eliteforge_agent_official_plugin_tools_local::SudoPassword::new);
    authorize(&state, &auth, &session)?;
    state
        .agent_framework
        .answer_sudo_input(&session, &auth.user().user_code, &id, password)
        .map_err(|e| error(e, &auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}

pub async fn private_response(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    // Require a non-simple header, so other sites cannot submit a form to this channel.
    let mut response = if request
        .headers()
        .get("x-tron-sudo-input")
        .and_then(|v| v.to_str().ok())
        == Some("1")
    {
        next.run(request).await
    } else {
        let locale = crate::shared::locale::resolve_request_locale(
            request
                .headers()
                .get("accept-language")
                .and_then(|value| value.to_str().ok()),
        );
        error(SudoInputError::Unavailable, &locale)
    };
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}
