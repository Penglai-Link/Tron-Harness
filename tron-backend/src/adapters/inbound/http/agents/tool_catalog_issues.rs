use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, response::ApiResponse,
};
use axum::{Json, extract::State};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCatalogIssueView {
    tool_code: Option<String>,
    tool_name: Option<String>,
    provider_id: Option<String>,
    pack_id: Option<String>,
    message: String,
}

pub async fn list(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
) -> Json<ApiResponse<Vec<ToolCatalogIssueView>>> {
    let discovery_error = state
        .agent_framework
        .refresh_capability_catalog()
        .await
        .err();
    let mut issues = state
        .agent_framework
        .tool_catalog_issues()
        .into_iter()
        .map(|issue| ToolCatalogIssueView {
            tool_code: issue.tool_code,
            tool_name: issue.tool_name,
            provider_id: issue.provider_id,
            pack_id: issue.pack_id,
            message: issue.message,
        })
        .collect::<Vec<_>>();
    if let Some(message) = discovery_error {
        issues.push(ToolCatalogIssueView {
            tool_code: None,
            tool_name: None,
            provider_id: None,
            pack_id: None,
            message,
        });
    }
    Json(ApiResponse::success(issues))
}
