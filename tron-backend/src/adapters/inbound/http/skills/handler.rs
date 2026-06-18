use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    app::AppRagService,
    application::skill::{SkillRagIndexService, SkillRegistryService},
    domain::skill::{
        RegistrySkillListQuery, SkillRagIndexBatchSummary, SkillRagIndexStatus, SkillSearchMode,
    },
    response::ApiResponse,
};

use super::{
    error_mapper::skill_error_response,
    request::{
        InstallRegistrySkillRequest, RegistrySkillDetailQuery, RegistrySkillUninstallQuery,
        SearchSkillsRequest, WorkspaceSkillBindingRequest, WorkspaceSkillsQuery,
    },
};

type AppSkillRagIndexService = SkillRagIndexService<
    std::sync::Arc<
        std::sync::Mutex<crate::adapters::outbound::persistence::GraphBackedRagRepository>,
    >,
    crate::application::rag::DefaultRagStrategyRegistry,
    crate::application::rag::DefaultRagRuntime,
>;

pub async fn search_skills(
    State(skills): State<SkillRegistryService>,
    Json(request): Json<SearchSkillsRequest>,
) -> Response {
    match skills.search_marketplace(
        &request.query,
        request.mode.unwrap_or(SkillSearchMode::Find),
    ) {
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
    State(rag): State<AppRagService>,
    Json(request): Json<InstallRegistrySkillRequest>,
) -> Response {
    match skills.install_skill(request.into()) {
        Ok(mut record) => {
            if let Some(workspace_code) = record.workspace_code.clone() {
                let indexer = SkillRagIndexService::new(skills, rag);
                record.rag_index =
                    index_workspace_skill_summary(&indexer, &workspace_code, &record.skill_name)
                        .await;
            }
            (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response()
        }
        Err(error) => skill_error_response(error),
    }
}

pub async fn uninstall_registry_skill(
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
    Path(skill_name): Path<String>,
    Query(query): Query<RegistrySkillUninstallQuery>,
) -> Response {
    match skills.uninstall_skill(&skill_name, query.remove_bindings.unwrap_or(false)) {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index = Some(index_all_workspaces_summary(&indexer).await);
            Json(ApiResponse::success(record)).into_response()
        }
        Err(error) => skill_error_response(error),
    }
}

pub async fn rescan_registry_skills(
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
) -> Response {
    match skills.rescan_registry() {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index = Some(index_all_workspaces_summary(&indexer).await);
            Json(ApiResponse::success(record)).into_response()
        }
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
    State(rag): State<AppRagService>,
    Path(workspace_code): Path<String>,
) -> Response {
    match skills.refresh_workspace_skills(&workspace_code) {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index = Some(index_workspace_summary(&indexer, &workspace_code).await);
            Json(ApiResponse::success(record)).into_response()
        }
        Err(error) => skill_error_response(error),
    }
}

pub async fn retry_workspace_skill_rag_index(
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
    Path((workspace_code, skill_name)): Path<(String, String)>,
) -> Response {
    let indexer = SkillRagIndexService::new(skills, rag);
    match indexer
        .index_workspace_skill(&workspace_code, &skill_name)
        .await
    {
        Ok(summary) => Json(ApiResponse::success(summary)).into_response(),
        Err(error) => skill_error_response(error),
    }
}

pub async fn update_registry_skills(
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
) -> Response {
    match skills.update_registry() {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index = Some(index_all_workspaces_summary(&indexer).await);
            Json(ApiResponse::success(record)).into_response()
        }
        Err(error) => skill_error_response(error),
    }
}

pub async fn bind_workspace_skill(
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
    Path(workspace_code): Path<String>,
    Json(request): Json<WorkspaceSkillBindingRequest>,
) -> Response {
    match skills.bind_workspace_skill(
        &workspace_code,
        &request.skill_name,
        request.enabled.unwrap_or(true),
    ) {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index =
                index_workspace_skill_summary(&indexer, &workspace_code, &record.skill_name).await;
            Json(ApiResponse::success(record)).into_response()
        }
        Err(error) => skill_error_response(error),
    }
}

pub async fn unbind_workspace_skill(
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
    Path((workspace_code, skill_name)): Path<(String, String)>,
) -> Response {
    match skills.unbind_workspace_skill(&workspace_code, &skill_name) {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index =
                index_workspace_skill_summary(&indexer, &workspace_code, &record.skill_name).await;
            Json(ApiResponse::success(record)).into_response()
        }
        Err(error) => skill_error_response(error),
    }
}

async fn index_workspace_skill_summary(
    indexer: &AppSkillRagIndexService,
    workspace_code: &str,
    skill_name: &str,
) -> Option<crate::domain::skill::SkillRagIndexSummary> {
    match indexer
        .index_workspace_skill(workspace_code, skill_name)
        .await
    {
        Ok(summary) => Some(summary),
        Err(error) => Some(crate::domain::skill::SkillRagIndexSummary {
            desired_enabled: true,
            source_version: "v1".into(),
            vector_status: SkillRagIndexStatus::Failed,
            graph_status: SkillRagIndexStatus::Failed,
            content_hash: None,
            structure_hash: None,
            last_indexed_at: None,
            updated_at: crate::shared::time::current_timestamp(),
            last_error: Some(error.to_string()),
            warnings: vec![error.to_string()],
        }),
    }
}

async fn index_workspace_summary(
    indexer: &AppSkillRagIndexService,
    workspace_code: &str,
) -> SkillRagIndexBatchSummary {
    indexer
        .index_workspace(workspace_code)
        .await
        .unwrap_or_else(index_error_batch)
}

async fn index_all_workspaces_summary(
    indexer: &AppSkillRagIndexService,
) -> SkillRagIndexBatchSummary {
    indexer
        .index_all_active_workspaces()
        .await
        .unwrap_or_else(index_error_batch)
}

fn index_error_batch(error: crate::domain::skill::SkillError) -> SkillRagIndexBatchSummary {
    SkillRagIndexBatchSummary {
        failed_count: 1,
        warnings: vec![error.to_string()],
        ..SkillRagIndexBatchSummary::default()
    }
}
