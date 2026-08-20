use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppRagService,
    application::rag::{
        RagCorpusManagementInput, RagFilterValidateInput, RagProfileManagementInput,
    },
    application::workspace::WorkspaceService,
    response::ApiResponse,
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
    State(workspaces): State<WorkspaceService>,
    State(rag): State<AppRagService>,
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
    if !user_has_workspace_access(&workspaces, &request.workspace_code, &auth.user().user_code) {
        return rag_error_response(crate::domain::rag::RagError::IngestScopeInvalid);
    }

    match rag.ingest(request).await {
        Ok(result) => Json(ApiResponse::success(RagIngestResponse::from(result))).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn list_corpora_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
) -> Response {
    match rag.list_corpora_management() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn get_corpus_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Path(corpus_id): Path<String>,
) -> Response {
    match rag.get_corpus_management(&corpus_id) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn corpus_catalog(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
) -> Response {
    Json(ApiResponse::success(rag.corpus_catalog())).into_response()
}

pub async fn verify_corpus_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Json(request): Json<RagCorpusManagementInput>,
) -> Response {
    match rag.verify_corpus_management(request) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn create_corpus_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Json(request): Json<RagCorpusManagementInput>,
) -> Response {
    match rag.create_corpus_management(request) {
        Ok(result) => (StatusCode::CREATED, Json(ApiResponse::success(result))).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn update_corpus_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Path(corpus_id): Path<String>,
    Json(request): Json<RagCorpusManagementInput>,
) -> Response {
    match rag.update_corpus_management(&corpus_id, request) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn delete_corpus_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Path(corpus_id): Path<String>,
) -> Response {
    match rag.delete_corpus_management(&corpus_id) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn search(
    State(workspaces): State<WorkspaceService>,
    State(rag): State<AppRagService>,
    auth: RequestAuthContext,
    Json(request): Json<RagSearchHttpRequest>,
) -> Response {
    let request = request.into_application();
    if !has_required_search_request_fields(&request) {
        return rag_error_response(crate::domain::rag::RagError::SearchRequestInvalid);
    }
    if request.access_context.user_code != auth.user().user_code
        || !user_has_workspace_access(&workspaces, &request.workspace_code, &auth.user().user_code)
    {
        return rag_error_response(crate::domain::rag::RagError::SearchScopeInvalid);
    }

    match rag.search(request).await {
        Ok(result) => Json(ApiResponse::success(RagSearchResponse::from(result))).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn list_profiles_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Query(query): Query<RagProfileManagementQuery>,
) -> Response {
    match rag.list_profiles_management(query.profile_type.as_deref()) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn get_profile_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Path(profile_id): Path<String>,
) -> Response {
    match rag.get_profile_management(&profile_id) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn profile_catalog(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
) -> Response {
    match rag.profile_catalog() {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn pipeline_catalog(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Query(query): Query<RagPipelineCatalogQuery>,
) -> Response {
    let Some(profile_type) = query.profile_type.as_deref() else {
        return rag_error_response(crate::domain::rag::RagError::ManagementRequestInvalid);
    };
    match rag.pipeline_catalog(profile_type) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn verify_profile_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Json(request): Json<RagProfileManagementInput>,
) -> Response {
    match rag.verify_profile_management(request) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn create_profile_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Json(request): Json<RagProfileManagementInput>,
) -> Response {
    match rag.create_profile_management(request) {
        Ok(result) => (StatusCode::CREATED, Json(ApiResponse::success(result))).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn update_profile_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Path(profile_id): Path<String>,
    Json(request): Json<RagProfileManagementInput>,
) -> Response {
    match rag.update_profile_management(&profile_id, request) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn delete_profile_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Path(profile_id): Path<String>,
) -> Response {
    match rag.delete_profile_management(&profile_id) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn update_profile_management_post(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Json(request): Json<RagProfileManagementInput>,
) -> Response {
    let profile_id = request.profile_id.clone();
    if profile_id.trim().is_empty() {
        return rag_error_response(crate::domain::rag::RagError::ManagementRequestInvalid);
    }
    match rag.update_profile_management(&profile_id, request) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn validate_filters_management(
    State(rag): State<AppRagService>,
    _auth: RequestAuthContext,
    Json(request): Json<RagFilterValidateInput>,
) -> Response {
    match rag.validate_filters_management(request) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => rag_error_response(error),
    }
}

pub async fn debug_profile(
    State(workspaces): State<WorkspaceService>,
    State(rag): State<AppRagService>,
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
    match rag.debug_profile(&workspace_code, &profile_id) {
        Ok(result) => {
            Json(ApiResponse::success(RagDebugProfileResponse::from(result))).into_response()
        }
        Err(error) => rag_error_response(error),
    }
}

pub async fn debug_source(
    State(workspaces): State<WorkspaceService>,
    State(rag): State<AppRagService>,
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
    match rag.debug_source(request) {
        Ok(result) => {
            Json(ApiResponse::success(RagDebugSourceResponse::from(result))).into_response()
        }
        Err(error) => rag_error_response(error),
    }
}

pub async fn retry_index(
    State(workspaces): State<WorkspaceService>,
    State(rag): State<AppRagService>,
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
    match rag.retry_index(request).await {
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
        .user_has_effective_access(workspace_code, user_code)
        .unwrap_or(false)
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
        request.source.code.as_str(),
        request.source.version.as_str(),
    ]
    .into_iter()
    .all(|value| !value.trim().is_empty())
        && request.payload.as_object().is_some()
}

fn has_required_search_request_fields(request: &crate::application::rag::RagSearchRequest) -> bool {
    [
        request.profile_id.as_str(),
        request.workspace_code.as_str(),
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagProfileManagementQuery {
    #[serde(default)]
    pub profile_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPipelineCatalogQuery {
    #[serde(default)]
    pub profile_type: Option<String>,
}
