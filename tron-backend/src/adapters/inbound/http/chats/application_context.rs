use axum::{
    Json,
    extract::{Query, State},
    response::Response,
};
use serde::{Deserialize, Serialize};

use super::{super::auth::RequestAuthContext, error_mapper::chat_error_response};
use crate::{app::AppState, domain::agent::AgentExecutionScope, response::ApiResponse};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionApplicationsRequest {
    session_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionApplicationsResponse {
    project_code: Option<String>,
    primary_app_code: Option<String>,
    working_directory: Option<String>,
    applications: Vec<SessionApplicationResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionApplicationResponse {
    app_code: String,
    app_name: String,
    is_primary: bool,
    configured_directory: String,
    working_directory: Option<String>,
    repository_root: Option<String>,
    branch_name: Option<String>,
    head_commit: Option<String>,
    worktree_path: Option<String>,
    is_worktree: bool,
    status: crate::domain::agent::ApplicationDirectoryStatus,
}

impl From<AgentExecutionScope> for SessionApplicationsResponse {
    fn from(scope: AgentExecutionScope) -> Self {
        Self {
            project_code: scope.project_code,
            primary_app_code: scope.app_code,
            working_directory: scope.local_directory,
            applications: scope
                .applications
                .into_iter()
                .map(|app| SessionApplicationResponse {
                    app_code: app.app_code,
                    app_name: app.app_name,
                    is_primary: app.is_primary,
                    configured_directory: app.configured_directory,
                    working_directory: app.working_directory,
                    repository_root: app.repository_root,
                    branch_name: app.branch_name,
                    head_commit: app.head_commit,
                    worktree_path: app.worktree_path,
                    is_worktree: app.is_worktree,
                    status: app.status,
                })
                .collect(),
        }
    }
}

pub async fn get_session_applications(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Query(request): Query<SessionApplicationsRequest>,
) -> Result<Json<ApiResponse<SessionApplicationsResponse>>, Response> {
    let scope = crate::app::telemetry::spawn_blocking_traced(move || {
        state.chats.session_execution_scope(&request.session_code)
    })
    .await
    .map_err(|_| chat_error_response(crate::domain::chat::ChatError::StorageFailure))?
    .map_err(chat_error_response)?;
    Ok(Json(ApiResponse::protocol_success(scope.into())))
}
