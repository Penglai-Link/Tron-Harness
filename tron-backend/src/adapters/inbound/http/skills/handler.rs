use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::skill::SkillRegistryService,
    domain::skill::{RegistrySkillListQuery, SkillSearchMode},
    response::ApiResponse,
};

use super::{
    error_mapper::skill_error_response,
    request::{
        InstallRegistrySkillRequest, RegistrySkillDetailQuery, RegistrySkillUninstallQuery,
        SearchSkillsRequest, WorkspaceSkillBindingRequest, WorkspaceSkillsQuery,
    },
};

pub async fn search_skills(
    State(skills): State<SkillRegistryService>,
    Json(request): Json<SearchSkillsRequest>,
) -> Response {
    match skills.search_marketplace(&request.query, request.mode.unwrap_or(SkillSearchMode::Find)) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn list_registry_skills(
    State(skills): State<SkillRegistryService>,
    Query(query): Query<RegistrySkillListQuery>,
) -> Response {
    match skills.list_registry(query) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn list_workspace_skills(
    State(skills): State<SkillRegistryService>,
    Path(workspace_code): Path<String>,
    Query(query): Query<WorkspaceSkillsQuery>,
) -> Response {
    match skills.list_workspace_skills(&workspace_code, query) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn get_registry_skill(
    State(skills): State<SkillRegistryService>,
    Path(skill_name): Path<String>,
    Query(query): Query<RegistrySkillDetailQuery>,
) -> Response {
    match skills.get_registry_skill(&skill_name, query.workspace_code.as_deref()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn get_workspace_skill(
    State(skills): State<SkillRegistryService>,
    Path((workspace_code, skill_name)): Path<(String, String)>,
) -> Response {
    match skills.get_workspace_skill(&workspace_code, &skill_name) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn install_registry_skill(
    State(skills): State<SkillRegistryService>,
    Json(request): Json<InstallRegistrySkillRequest>,
) -> Response {
    match skills.install_skill(request.into()) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn uninstall_registry_skill(
    State(skills): State<SkillRegistryService>,
    Path(skill_name): Path<String>,
    Query(query): Query<RegistrySkillUninstallQuery>,
) -> Response {
    match skills.uninstall_skill(&skill_name, query.remove_bindings.unwrap_or(false)) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn rescan_registry_skills(State(skills): State<SkillRegistryService>) -> Response {
    match skills.rescan_registry() {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn check_registry_skill_updates(State(skills): State<SkillRegistryService>) -> Response {
    match skills.check_updates() {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn refresh_workspace_skills(
    State(skills): State<SkillRegistryService>,
    Path(workspace_code): Path<String>,
) -> Response {
    match skills.refresh_workspace_skills(&workspace_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn update_registry_skills(State(skills): State<SkillRegistryService>) -> Response {
    match skills.update_registry() {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn bind_workspace_skill(
    State(skills): State<SkillRegistryService>,
    Path(workspace_code): Path<String>,
    Json(request): Json<WorkspaceSkillBindingRequest>,
) -> Response {
    match skills.bind_workspace_skill(
        &workspace_code,
        &request.skill_name,
        request.enabled.unwrap_or(true),
    ) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn unbind_workspace_skill(
    State(skills): State<SkillRegistryService>,
    Path((workspace_code, skill_name)): Path<(String, String)>,
) -> Response {
    match skills.unbind_workspace_skill(&workspace_code, &skill_name) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => skill_error_response(error),
    }
}
