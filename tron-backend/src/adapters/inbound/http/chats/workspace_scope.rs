//! Normal session access is always constrained by the caller's workspace.
use super::error_mapper::chat_error_response;
use crate::{
    app::AppState,
    domain::chat::{ChatError, ChatSessionStatus},
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
pub async fn require_workspace(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();
    let chat = path.starts_with("/api/v1/chats/");
    let replay = path.starts_with("/api/v1/agents/agui/streams/");
    let runs = path.starts_with("/api/v1/agents/runs");
    if !chat && !replay && !runs {
        return next.run(request).await;
    }
    let query: std::collections::BTreeMap<String, String> =
        url::form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes())
            .into_owned()
            .collect();
    if let (Some(query_workspace), Some(header_workspace)) = (
        query.get("workspaceCode"),
        request
            .headers()
            .get("x-workspace-code")
            .and_then(|v| v.to_str().ok()),
    ) {
        if query_workspace != header_workspace {
            return chat_error_response(ChatError::SessionNotFound);
        }
    }
    let workspace = query.get("workspaceCode").cloned().or_else(|| {
        request
            .headers()
            .get("x-workspace-code")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    });
    let Some(workspace) = workspace.filter(|w| !w.trim().is_empty()) else {
        return chat_error_response(ChatError::WorkspaceRequired);
    };
    if state.workspaces.get_workspace(&workspace).is_err() {
        return chat_error_response(ChatError::SessionNotFound);
    }
    // Query extractors downstream must receive the validated header scope too.
    if !query.contains_key("workspaceCode") {
        let mut pairs = url::form_urlencoded::Serializer::new(String::new());
        pairs.extend_pairs(query.iter());
        pairs.append_pair("workspaceCode", &workspace);
        let mut parts = request.uri().clone().into_parts();
        parts.path_and_query = format!("{path}?{}", pairs.finish()).parse().ok();
        match axum::http::Uri::from_parts(parts) {
            Ok(uri) => *request.uri_mut() = uri,
            Err(_) => return chat_error_response(ChatError::WorkspaceRequired),
        }
    }
    let mut payload = serde_json::Value::Null;
    if request
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"))
    {
        let body = std::mem::replace(request.body_mut(), Body::empty());
        let bytes = match to_bytes(body, 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<EmptyData>::error(
                        "CHAT_REQUEST_INVALID",
                        "request body is too large",
                    )),
                )
                    .into_response();
            }
        };
        payload = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<EmptyData>::error(
                        "CHAT_REQUEST_INVALID",
                        "invalid JSON",
                    )),
                )
                    .into_response();
            }
        };
        *request.body_mut() = Body::from(bytes);
    }
    if payload["workspaceCode"]
        .as_str()
        .is_some_and(|body| body != workspace)
    {
        return chat_error_response(ChatError::SessionNotFound);
    }
    let session = if runs {
        path.strip_prefix("/api/v1/agents/runs/")
            .and_then(|tail| tail.split('/').next())
            .map(str::to_owned)
            .or_else(|| payload["conversationCode"].as_str().map(str::to_owned))
    } else if replay {
        path.strip_prefix("/api/v1/agents/agui/streams/")
            .and_then(|tail| tail.split('/').next())
            .map(str::to_owned)
    } else {
        let tail = path.strip_prefix("/api/v1/chats/sessions/");
        tail.and_then(|tail| {
            let code = tail.split('/').next()?;
            (!matches!(
                code,
                "applications"
                    | "get-active-run"
                    | "steer-message"
                    | "stop-run"
                    | "get-todos"
                    | "get-context-detail"
                    | "get-latest-compaction"
                    | "create-compaction"
                    | "get-goal"
                    | "get-latest-goal"
                    | "set-goal"
                    | "pause-goal"
                    | "resume-goal"
                    | "cancel-goal"
                    | "clear-goal"
                    | "update-goal"
            ))
            .then(|| code.to_owned())
        })
        .or_else(|| query.get("sessionCode").cloned())
        .or_else(|| payload["sessionCode"].as_str().map(str::to_owned))
    };
    if let Some(mut session) = session {
        if (replay || runs) && state.chats.get_session(&session).is_err() {
            if let Ok(run) = eliteforge_agent_core::RunId::parse_str(&session) {
                let framework = match state.agent_framework.storage_framework().await {
                    Ok(value) => value,
                    Err(_) => return chat_error_response(ChatError::StorageFailure),
                };
                if let Ok(runs) = framework.runs() {
                    if let Ok(record) = runs.get(run).await {
                        session = record.session_id;
                    }
                }
            }
        }
        let record = match state.chats.get_session(&session) {
            Ok(record) => record,
            Err(error) => return chat_error_response(error),
        };
        if record.workspace_code != workspace {
            return chat_error_response(ChatError::SessionNotFound);
        }
        if record.status != ChatSessionStatus::Active {
            return chat_error_response(ChatError::SessionArchived);
        }
    }
    next.run(request).await
}
