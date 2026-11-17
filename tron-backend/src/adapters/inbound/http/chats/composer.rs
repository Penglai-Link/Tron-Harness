use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, response::ApiResponse,
};
use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputSuggestionsQuery {
    #[serde(rename = "workspaceCode")]
    _workspace_code: Option<String>,
    session_code: Option<String>,
    agent_code: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InputSuggestionsResponse {
    agent_code: String,
    commands: Vec<String>,
    skills: Vec<InputSkillResponse>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InputSkillResponse {
    name: String,
    description: String,
}

pub async fn suggestions(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Query(query): Query<InputSuggestionsQuery>,
) -> Response {
    // Match send admission: after the first run (and for PersonAIs), persisted Agent ownership wins.
    let session = match query
        .session_code
        .as_deref()
        .filter(|code| !code.is_empty())
    {
        Some(code) => match state.chats.get_session(code) {
            Ok(session) => Some(session),
            Err(error) => return super::error_mapper::chat_error_response(error),
        },
        None => None,
    };
    let pinned = session.as_ref().is_some_and(|session| {
        session.last_run_code.is_some()
            || session.source_code.as_deref().is_some_and(|source| {
                source.starts_with("__assistant:") || source.starts_with("__knowledge:")
            })
    });
    let agent = if pinned {
        session
            .as_ref()
            .and_then(|session| session.agent_code.clone())
    } else {
        query.agent_code.or_else(|| {
            session
                .as_ref()
                .and_then(|session| session.agent_code.clone())
        })
    }
    .filter(|code| !code.trim().is_empty())
    .unwrap_or_else(|| "main-agent".into());
    let skills = match state.skills.effective_agent_skills(&agent) {
        Ok(skills) => skills
            .into_iter()
            .map(|skill| InputSkillResponse {
                name: skill.skill_name,
                description: skill.description.unwrap_or_default(),
            })
            .collect(),
        Err(_) => {
            return super::error_mapper::chat_error_response(
                crate::domain::chat::ChatError::StorageFailure,
            );
        }
    };
    let commands = match state.skills.internal_agent_codes() {
        Ok(codes) => {
            if codes.contains(&agent) {
                vec!["goal".into(), "compact".into()]
            } else {
                vec![]
            }
        }
        Err(_) => {
            return super::error_mapper::chat_error_response(
                crate::domain::chat::ChatError::StorageFailure,
            );
        }
    };
    Json(ApiResponse::protocol_success(InputSuggestionsResponse {
        agent_code: agent,
        commands,
        skills,
    }))
    .into_response()
}
