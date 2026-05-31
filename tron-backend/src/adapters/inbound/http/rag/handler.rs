use std::sync::{Arc, Mutex};

use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};

use crate::{
    adapters::{
        inbound::http::auth::RequestAuthContext, outbound::persistence::sqlite::SqliteRagRepository,
    },
    application::{
        llm::LlmRuntimeService,
        rag::{
            DefaultRagStrategyRegistry, RagDiagnosticsService, RagIngestService,
            RagRetryIndexService, RagSearchService,
        },
        workspace::WorkspaceService,
    },
    config::AppConfig,
    domain::workspace::WorkspaceAccessListQuery,
    response::ApiResponse,
    shared::event_bus::EventBusHandle,
};

use super::{
    error_mapper::rag_error_response,
    request::{
        RagDebugProfileHttpQuery, RagDebugSourceHttpQuery, RagIngestHttpRequest,
        RagRetryIndexHttpRequest, RagSearchHttpRequest,
    },
    response::{
        RagDebugProfileResponse, RagDebugSourceResponse, RagIngestResponse, RagRetryIndexResponse,
        RagSearchResponse,
    },
};

pub async fn ingest(
    State(config): State<AppConfig>,
    State(workspaces): State<WorkspaceService>,
    State(llm_runtime): State<LlmRuntimeService>,
    State(event_bus): State<EventBusHandle>,
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
    )
    .with_event_bus(event_bus);
    match service.ingest(request).await {
        Ok(result) => Json(ApiResponse::success(RagIngestResponse::from(result))).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn search(
    State(config): State<AppConfig>,
    State(workspaces): State<WorkspaceService>,
    State(llm_runtime): State<LlmRuntimeService>,
    State(event_bus): State<EventBusHandle>,
    auth: RequestAuthContext,
    Json(request): Json<RagSearchHttpRequest>,
) -> Response {
    let request = request.into_application();
    if !has_required_search_request_fields(&request) {
        return rag_error_response(crate::domain::rag::RagError::SearchRequestInvalid);
    }
    if request.access_context.user_code != auth.user().user_code
        || request.workspace_code != request.access_context.workspace_code
        || !user_has_workspace_access(&workspaces, &request.workspace_code, &auth.user().user_code)
    {
        return rag_error_response(crate::domain::rag::RagError::SearchScopeInvalid);
    }

    let repository = match SqliteRagRepository::connect(&config.database_url) {
        Ok(repository) => repository,
        Err(error) => return rag_error_response(error),
    };
    let service = RagSearchService::new(
        Arc::new(Mutex::new(repository)),
        DefaultRagStrategyRegistry::default(),
        llm_runtime,
    )
    .with_event_bus(event_bus);
    match service.search(request).await {
        Ok(result) => Json(ApiResponse::success(RagSearchResponse::from(result))).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn debug_profile(
    State(config): State<AppConfig>,
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Query(query): Query<RagDebugProfileHttpQuery>,
) -> Response {
    let workspace_code = query.workspace_code.unwrap_or_default();
    let profile_id = query.profile_id.unwrap_or_default();
    if workspace_code.trim().is_empty() || profile_id.trim().is_empty() {
        return rag_error_response(crate::domain::rag::RagError::DebugRequestInvalid);
    }
    if !user_has_workspace_access(&workspaces, &workspace_code, &auth.user().user_code) {
        return rag_error_response(crate::domain::rag::RagError::DebugScopeInvalid);
    }
    let repository = match SqliteRagRepository::connect(&config.database_url) {
        Ok(repository) => repository,
        Err(error) => return rag_error_response(error),
    };
    let service = RagDiagnosticsService::new(repository, DefaultRagStrategyRegistry::default());
    match service.debug_profile(&workspace_code, &profile_id) {
        Ok(result) => {
            Json(ApiResponse::success(RagDebugProfileResponse::from(result))).into_response()
        }
        Err(error) => rag_error_response(error),
    }
}

pub async fn debug_source(
    State(config): State<AppConfig>,
    State(workspaces): State<WorkspaceService>,
    auth: RequestAuthContext,
    Query(query): Query<RagDebugSourceHttpQuery>,
) -> Response {
    let request = query.into_application();
    if !debug_source_required(&request) {
        return rag_error_response(crate::domain::rag::RagError::DebugRequestInvalid);
    }
    if !user_has_workspace_access(&workspaces, &request.workspace_code, &auth.user().user_code)
        || !scope_allowed_for_http(
            &request.scope_type,
            &request.scope_code,
            &request.workspace_code,
            &auth.user().user_code,
        )
    {
        return rag_error_response(crate::domain::rag::RagError::DebugScopeInvalid);
    }
    let repository = match SqliteRagRepository::connect(&config.database_url) {
        Ok(repository) => repository,
        Err(error) => return rag_error_response(error),
    };
    let service = RagDiagnosticsService::new(repository, DefaultRagStrategyRegistry::default());
    match service.debug_source(request) {
        Ok(result) => {
            Json(ApiResponse::success(RagDebugSourceResponse::from(result))).into_response()
        }
        Err(error) => rag_error_response(error),
    }
}

pub async fn retry_index(
    State(config): State<AppConfig>,
    State(workspaces): State<WorkspaceService>,
    State(llm_runtime): State<LlmRuntimeService>,
    State(event_bus): State<EventBusHandle>,
    auth: RequestAuthContext,
    Json(request): Json<RagRetryIndexHttpRequest>,
) -> Response {
    let request = request.into_application(auth.user().user_code.clone(), "AGENT_CODEX".into());
    if !retry_required(&request) {
        return rag_error_response(crate::domain::rag::RagError::RetryRequestInvalid);
    }
    if !user_has_workspace_access(&workspaces, &request.workspace_code, &auth.user().user_code)
        || !scope_allowed_for_http(
            &request.scope_type,
            &request.scope_code,
            &request.workspace_code,
            &auth.user().user_code,
        )
    {
        return rag_error_response(crate::domain::rag::RagError::RetryScopeInvalid);
    }
    let repository = match SqliteRagRepository::connect(&config.database_url) {
        Ok(repository) => repository,
        Err(error) => return rag_error_response(error),
    };
    let service = RagRetryIndexService::new(Arc::new(Mutex::new(repository)), llm_runtime)
        .with_event_bus(event_bus);
    match service.retry_index(request).await {
        Ok(result) => {
            Json(ApiResponse::success(RagRetryIndexResponse::from(result))).into_response()
        }
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
                size: Some(1),
                search: Some(user_code.to_string()),
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

fn has_required_search_request_fields(request: &crate::application::rag::RagSearchRequest) -> bool {
    [
        request.profile_id.as_str(),
        request.workspace_code.as_str(),
        request.query.as_str(),
        request.access_context.user_code.as_str(),
        request.access_context.agent_code.as_str(),
        request.access_context.workspace_code.as_str(),
        request.context.strategy.as_str(),
    ]
    .into_iter()
    .all(|value| !value.trim().is_empty())
        && request.filters.as_object().is_some()
        && request.limit > 0
}

fn debug_source_required(request: &crate::application::rag::RagDebugSourceRequest) -> bool {
    [
        request.workspace_code.as_str(),
        request.corpus_id.as_str(),
        request.source_code.as_str(),
        request.source_version.as_str(),
        request.scope_type.as_str(),
        request.scope_code.as_str(),
    ]
    .into_iter()
    .all(|value| !value.trim().is_empty())
}

fn retry_required(request: &crate::application::rag::RagRetryIndexRequest) -> bool {
    [
        request.workspace_code.as_str(),
        request.corpus_id.as_str(),
        request.source_code.as_str(),
        request.source_version.as_str(),
        request.scope_type.as_str(),
        request.scope_code.as_str(),
    ]
    .into_iter()
    .all(|value| !value.trim().is_empty())
        && !request.stages.is_empty()
}

fn scope_allowed_for_http(
    scope_type: &str,
    scope_code: &str,
    workspace_code: &str,
    user_code: &str,
) -> bool {
    match scope_type {
        "USER" => scope_code == user_code,
        "SPACE" => scope_code == workspace_code,
        "AGENT" => false,
        _ => false,
    }
}
