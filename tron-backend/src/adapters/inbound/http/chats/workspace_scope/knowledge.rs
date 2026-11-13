//! A knowledge conversation is platform-scoped, with its owner and library
//! derived from persisted state. A header identifies a namespace, never a grant.
use super::*;

pub(super) async fn require(
    state: AppState,
    mut request: Request,
    next: Next,
    kb: String,
) -> Response {
    let fail = || chat_error_response(ChatError::SessionNotFound);
    let Some(user) = request
        .extensions()
        .get::<RequestAuthContext>()
        .map(|a| a.user().user_code.clone())
    else {
        return fail();
    };
    if state.knowledge_bases.owned(&user, &kb).is_err()
        || request.headers().contains_key("x-dude-code")
    {
        return fail();
    }
    let path = request.uri().path().to_owned();
    let runs = path.starts_with("/api/v1/agents/runs");
    let replay = path.starts_with("/api/v1/agents/agui/streams/");
    // New turns enter through the bound chat service, which installs the trusted
    // knowledge context. A raw native-run POST must not bypass that admission.
    if runs && path == "/api/v1/agents/runs" && request.method() != axum::http::Method::GET {
        return fail();
    }
    let query: std::collections::BTreeMap<String, String> =
        url::form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes())
            .into_owned()
            .collect();
    if query.get("workspaceCode").is_some_and(|w| w != "PLATFORM")
        || request
            .headers()
            .get("x-workspace-code")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|w| w != "PLATFORM")
    {
        return fail();
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
            Ok(b) => b,
            Err(_) => return fail(),
        };
        payload = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => return fail(),
        };
        *request.body_mut() = Body::from(bytes);
    }
    if payload["workspaceCode"]
        .as_str()
        .is_some_and(|w| w != "PLATFORM")
    {
        return fail();
    }
    let mut candidates = Vec::new();
    for key in ["sessionCode", "conversationCode"] {
        if let Some(code) = query.get(key) {
            candidates.push(code.clone());
        }
        if let Some(code) = payload[key].as_str() {
            candidates.push(code.into());
        }
    }
    if let Some(code) = path
        .strip_prefix("/api/v1/chats/sessions/")
        .and_then(|tail| tail.split('/').next())
        .filter(|code| !is_session_operation(code))
    {
        candidates.push(code.into());
    }
    if let Some(code) = path
        .strip_prefix("/api/v1/agents/agui/streams/")
        .or_else(|| path.strip_prefix("/api/v1/agents/runs/"))
        .and_then(|tail| tail.split('/').next())
    {
        let session = if replay && state.chats.get_session(code).is_ok() {
            Some(code.into())
        } else if let Ok(record) = state.agent_runs.get_run(code) {
            record.conversation_code
        } else if let Ok(id) = eliteforge_agent_core::RunId::parse_str(code) {
            match state.agent_framework.storage_framework().await {
                Ok(f) => match f.runs() {
                    Ok(r) => r.get(id).await.ok().map(|run| run.session_id),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        } else {
            None
        };
        let Some(session) = session else {
            return fail();
        };
        candidates.push(session);
    }
    if candidates.is_empty() {
        return fail();
    }
    let expected = format!("__knowledge:{kb}");
    for code in &candidates {
        let session = match state.chats.get_session(code) {
            Ok(s) => s,
            Err(_) => return fail(),
        };
        if session.user_code.as_deref() != Some(user.as_str())
            || session.source_code.as_deref() != Some(expected.as_str())
            || session.workspace_code != "PLATFORM"
        {
            return fail();
        }
        if session.status != ChatSessionStatus::Active {
            return chat_error_response(ChatError::SessionArchived);
        }
        if request.method() != axum::http::Method::GET
            && !path.ends_with("/history/query")
            && !replay
        {
            if let Err(error) = state.chats.ensure_session_writable(code) {
                return chat_error_response(error);
            }
        }
    }
    if runs && path == "/api/v1/agents/runs" && !query.contains_key("conversationCode") {
        let mut pairs = url::form_urlencoded::Serializer::new(String::new());
        pairs.extend_pairs(query.iter());
        pairs.append_pair("conversationCode", &candidates[0]);
        let mut parts = request.uri().clone().into_parts();
        parts.path_and_query = format!("{path}?{}", pairs.finish()).parse().ok();
        match axum::http::Uri::from_parts(parts) {
            Ok(uri) => *request.uri_mut() = uri,
            Err(_) => return fail(),
        }
    }
    next.run(request).await
}
