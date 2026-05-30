use std::sync::{Arc, Mutex};

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::{
        inbound::http::auth::RequestAuthContext, outbound::persistence::sqlite::SqliteRagRepository,
    },
    application::{
        llm::LlmRuntimeService,
        rag::{DefaultRagStrategyRegistry, RagIngestService},
        workspace::WorkspaceService,
    },
    config::AppConfig,
    domain::workspace::WorkspaceAccessListQuery,
    response::ApiResponse,
};

use super::{
    error_mapper::rag_error_response, request::RagIngestHttpRequest, response::RagIngestResponse,
};

pub async fn ingest(
    State(config): State<AppConfig>,
    State(workspaces): State<WorkspaceService>,
    State(llm_runtime): State<LlmRuntimeService>,
    auth: RequestAuthContext,
    Json(request): Json<RagIngestHttpRequest>,
) -> Response {
    let request = request.into_application();
    if !has_required_request_fields(&request) {
        return rag_error_response(crate::domain::rag::RagError::IngestRequestInvalid);
    }
    if request.access_context.user_code != auth.user().user_code {
        return rag_error_response(crate::domain::rag::RagError::IngestScopeInvalid);
    }
    if request.workspace_code != request.access_context.workspace_code {
        return rag_error_response(crate::domain::rag::RagError::IngestScopeInvalid);
    }
    if !user_has_workspace_access(&workspaces, &request.workspace_code, &auth.user().user_code) {
        return rag_error_response(crate::domain::rag::RagError::IngestScopeInvalid);
    }

    let repository = match SqliteRagRepository::connect(&config.database_url) {
        Ok(repository) => repository,
        Err(error) => return rag_error_response(error),
    };
    let service = RagIngestService::new(
        Arc::new(Mutex::new(repository)),
        DefaultRagStrategyRegistry::default(),
        llm_runtime,
    );
    match service.ingest(request).await {
        Ok(result) => Json(ApiResponse::success(RagIngestResponse::from(result))).into_response(),
        Err(error) => rag_error_response(error),
    }
}

fn user_has_workspace_access(
    workspaces: &WorkspaceService,
    workspace_code: &str,
    user_code: &str,
) -> bool {
    workspaces
        .list_effective_users(
            workspace_code,
            WorkspaceAccessListQuery {
                current: Some(1),
                size: Some(500),
                search: None,
            },
        )
        .is_ok_and(|page| page.records.iter().any(|user| user.user_code == user_code))
}

fn has_required_request_fields(request: &crate::application::rag::RagIngestRequest) -> bool {
    [
        request.corpus_id.as_str(),
        request.ingest_profile_id.as_str(),
        request.workspace_code.as_str(),
        request.access_context.user_code.as_str(),
        request.access_context.agent_code.as_str(),
        request.access_context.workspace_code.as_str(),
        request.target_scope.scope_type.as_str(),
        request.target_scope.scope_code.as_str(),
        request.source_code.as_str(),
        request.source_version.as_str(),
        request.source_type.as_str(),
    ]
    .into_iter()
    .all(|value| !value.trim().is_empty())
        && request.payload.as_object().is_some()
}
