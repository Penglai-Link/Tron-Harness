//! Normal session access is always constrained by the caller's workspace.
mod knowledge;
use super::error_mapper::chat_error_response;
use crate::adapters::inbound::http::auth::RequestAuthContext;
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
    if let Some(kb) = request.headers().get("x-knowledge-base-code").and_then(|v|v.to_str().ok()).map(str::to_owned) {
        return knowledge::require(state,request,next,kb).await;
    }
    // Dude namespace is explicit and verified against durable ownership. The
    // submitted workspace is validated separately for execution, not ownership.
    if let Some(dude_code) = request
        .headers()
        .get("x-dude-code")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
    {
        if state.dudes.get(&dude_code).is_err() {
            return chat_error_response(ChatError::SessionNotFound);
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
                Ok(v) => v,
                Err(_) => return chat_error_response(ChatError::MessageRequired),
            };
            payload = match serde_json::from_slice(&bytes) {
                Ok(v) => v,
                Err(_) => return chat_error_response(ChatError::MessageRequired),
            };
            *request.body_mut() = Body::from(bytes);
        }
        let query: std::collections::BTreeMap<String, String> =
            url::form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes())
                .into_owned()
                .collect();
        let mut candidates = vec![];
        for key in ["sessionCode", "conversationCode"] {
            if let Some(v) = payload[key].as_str() {
                candidates.push(v.to_owned())
            }
            if let Some(v) = query.get(key) {
                candidates.push(v.clone())
            }
        }
        for prefix in ["/api/v1/chats/sessions/"] {
            if let Some(tail) = path.strip_prefix(prefix) {
                if let Some(code) = tail.split('/').next() {
                    // The path is authoritative even when it names an ordinary,
                    // missing, or foreign session. A query must never substitute it.
                    if !is_session_operation(code) {
                        candidates.push(code.to_owned());
                    }
                }
            }
        }
        for prefix in ["/api/v1/agents/agui/streams/", "/api/v1/agents/runs/"] {
            if let Some(code) = path
                .strip_prefix(prefix)
                .and_then(|tail| tail.split('/').next())
            {
                let session = if replay && state.chats.get_session(code).is_ok() {
                    Some(code.to_owned())
                } else if let Ok(record) = state.agent_runs.get_run(code) {
                    record.conversation_code
                } else if let Ok(id) = eliteforge_agent_core::RunId::parse_str(code) {
                    match state.agent_framework.storage_framework().await {
                        Ok(framework) => match framework.runs() {
                            Ok(runs) => runs.get(id).await.ok().map(|run| run.session_id),
                            Err(_) => None,
                        },
                        Err(_) => None,
                    }
                } else {
                    None
                };
                let Some(session) = session else {
                    return chat_error_response(ChatError::SessionNotFound);
                };
                candidates.push(session);
            }
        }
        if candidates.is_empty() {
            return chat_error_response(ChatError::SessionNotFound);
        }
        let Some(auth) = request.extensions().get::<RequestAuthContext>() else {
            return chat_error_response(ChatError::SessionNotFound);
        };
        for code in &candidates {
            let d = match state.dudes.session(code) {
                Ok(Some(d)) if d.dude_code == dude_code => d,
                _ => return chat_error_response(ChatError::SessionNotFound),
            };
            let chat = match state.chats.get_session(code) {
                Ok(chat) => chat,
                Err(_) => return chat_error_response(ChatError::SessionNotFound),
            };
            let expected_source = format!("__dude:{dude_code}");
            if chat.source_code.as_deref() != Some(expected_source.as_str())
                || chat.agent_code.as_deref() != Some(dude_code.as_str())
                || chat.workspace_code != d.workspace_code.as_deref().unwrap_or_default()
                || chat
                    .user_code
                    .as_deref()
                    .is_some_and(|user| user != auth.user().user_code)
            {
                return chat_error_response(ChatError::SessionNotFound);
            }
            if chat.status != ChatSessionStatus::Active {
                return chat_error_response(ChatError::SessionArchived);
            }
            if request.method() != axum::http::Method::GET && !path.ends_with("/history/query") && !replay {
                if let Err(error) = state.chats.ensure_session_writable(code) { return chat_error_response(error); }
            }
        }
        if runs && path == "/api/v1/agents/runs" {
            // Native chat admission derives ownership from the persisted session.
            if request.method() != axum::http::Method::GET {
                return chat_error_response(ChatError::SessionNotFound);
            }
            if !query.contains_key("conversationCode") {
                let mut pairs = url::form_urlencoded::Serializer::new(String::new());
                pairs.extend_pairs(query.iter());
                pairs.append_pair("conversationCode", &candidates[0]);
                let mut parts = request.uri().clone().into_parts();
                parts.path_and_query = format!("{path}?{}", pairs.finish()).parse().ok();
                match axum::http::Uri::from_parts(parts) {
                    Ok(uri) => *request.uri_mut() = uri,
                    Err(_) => return chat_error_response(ChatError::SessionNotFound),
                }
            }
        }
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
    // Run creation owns its workspace validation contract
    // (AGENT_WORKSPACE_INVALID); the middleware only needs the scope for
    // resolving existing run and session ownership.
    let run_creation = runs
        && path == "/api/v1/agents/runs"
        && request.method() != axum::http::Method::GET;
    if !run_creation && state.workspaces.get_workspace(&workspace).is_err() {
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
                    | "get-context-assembly"
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
            Err(error) => {
                if runs || replay {
                    // Run and stream scopes resolve their session best-effort:
                    // a code that maps to nothing persisted yet belongs to the
                    // handler, which creates the new conversation or answers
                    // with the domain error. Existing foreign sessions were
                    // already rejected by the resolution above.
                    return next.run(request).await;
                }
                return chat_error_response(error);
            }
        };
        if state.dudes.session(&session).ok().flatten().is_some() {
            return chat_error_response(ChatError::SessionNotFound);
        }
        if record.source_code.as_deref().is_some_and(|source|source.starts_with("__knowledge:")) {
            return chat_error_response(ChatError::SessionNotFound);
        }
        if record.workspace_code != workspace {
            return chat_error_response(ChatError::SessionNotFound);
        }
        // Deletion has its own inactive-run guard and accepts either lifecycle
        // state. Keep workspace and special-session ownership checks above it.
        let deleting_session = request.method() == axum::http::Method::DELETE
            && path == format!("/api/v1/chats/sessions/{session}");
        if !deleting_session && record.status != ChatSessionStatus::Active {
            return chat_error_response(ChatError::SessionArchived);
        }
        if !deleting_session && request.method() != axum::http::Method::GET && !path.ends_with("/history/query") && !replay {
            if let Err(error) = state.chats.ensure_session_writable(&session) { return chat_error_response(error); }
        }
    }
    next.run(request).await
}

// These routes address the session in their body/query rather than the path.
fn is_session_operation(code: &str) -> bool {
    matches!(
        code,
        "applications"
            | "get-active-run"
            | "steer-message"
            | "stop-run"
            | "get-todos"
            | "get-context-detail"
                    | "get-context-assembly"
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
    )
}
