use super::response::*;
use crate::adapters::inbound::http::auth::RequestAuthContext;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    app::AppRagService,
    application::skill::{SkillRagIndexService, SkillRegistryService},
    domain::skill::{RegistrySkillListQuery, SkillRagIndexBatchSummary, SkillSearchMode},
    response::ApiResponse,
};

use super::{
    error_mapper::skill_error_response,
    request::{
        AgentSkillsRequest, InstallRegistrySkillRequest, RegistrySkillsQuery, SearchSkillsRequest,
        SkillAgentsRequest,
    },
};

type AppSkillRagIndexService = SkillRagIndexService<
    std::sync::Arc<
        std::sync::Mutex<crate::adapters::outbound::persistence::sqlite::SqliteRagRepository>,
    >,
    crate::application::rag::DefaultRagRuntime,
>;

pub async fn search_skills(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    Json(request): Json<SearchSkillsRequest>,
) -> Response {
    match skills.search_marketplace(
        &request.query,
        request.mode.unwrap_or(SkillSearchMode::Find),
    ) {
        Ok(records) => Json(ApiResponse::protocol_success(
            records
                .into_iter()
                .map(SkillSearchCandidateVo::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => skill_error_response(error, &auth.locale),
    }
}

pub async fn list_registry_skills(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    Query(query): Query<RegistrySkillsQuery>,
) -> Response {
    match skills.list_registry(RegistrySkillListQuery {
        search: query.search,
    }) {
        Ok(records) => Json(ApiResponse::protocol_success(skill_list(
            records,
            &auth.locale,
        )))
        .into_response(),
        Err(error) => skill_error_response(error, &auth.locale),
    }
}

pub async fn get_registry_skill(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    Path(skill_name): Path<String>,
) -> Response {
    match skills.get_registry_skill(&skill_name) {
        Ok(record) => {
            let mut vo = RegistrySkillDetailVo::from(record);
            if !vo.available && auth.locale.starts_with("en") {
                vo.disabled_reason = Some(format!(
                    "Capability pack {} is disabled",
                    vo.provider_pack_id.as_deref().unwrap_or("")
                ));
            }
            Json(ApiResponse::protocol_success(vo)).into_response()
        }
        Err(error) => skill_error_response(error, &auth.locale),
    }
}

pub async fn install_registry_skill(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
    Json(request): Json<InstallRegistrySkillRequest>,
) -> Response {
    match skills.install_skill(request.into()) {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index = indexer.index_registry_skill(&record.skill_name).await.ok();
            (
                StatusCode::CREATED,
                Json(ApiResponse::protocol_success(
                    SkillRegistryMutationResultVo::from(record),
                )),
            )
                .into_response()
        }
        Err(error) => skill_error_response(error, &auth.locale),
    }
}

pub async fn uninstall_registry_skill(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
    Path(skill_name): Path<String>,
) -> Response {
    match skills.uninstall_skill(&skill_name) {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index = Some(index_platform_summary(&indexer).await);
            Json(ApiResponse::protocol_success(
                SkillRegistryUninstallResultVo::from(record),
            ))
            .into_response()
        }
        Err(error) => skill_error_response(error, &auth.locale),
    }
}

pub async fn rescan_registry_skills(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
) -> Response {
    match skills.rescan_registry() {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index = Some(index_platform_summary(&indexer).await);
            Json(ApiResponse::protocol_success(
                SkillRegistrySyncResultVo::from(record),
            ))
            .into_response()
        }
        Err(error) => skill_error_response(error, &auth.locale),
    }
}

pub async fn check_registry_skill_updates(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
) -> Response {
    match skills.check_updates() {
        Ok(record) => Json(ApiResponse::protocol_success(
            SkillRegistrySyncResultVo::from(record),
        ))
        .into_response(),
        Err(error) => skill_error_response(error, &auth.locale),
    }
}

pub async fn update_registry_skills(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    State(rag): State<AppRagService>,
) -> Response {
    match skills.update_registry() {
        Ok(mut record) => {
            let indexer = SkillRagIndexService::new(skills, rag);
            record.rag_index = Some(index_platform_summary(&indexer).await);
            Json(ApiResponse::protocol_success(
                SkillRegistrySyncResultVo::from(record),
            ))
            .into_response()
        }
        Err(error) => skill_error_response(error, &auth.locale),
    }
}

async fn index_platform_summary(indexer: &AppSkillRagIndexService) -> SkillRagIndexBatchSummary {
    indexer.index_all().await.unwrap_or_else(index_error_batch)
}

fn index_error_batch(error: crate::domain::skill::SkillError) -> SkillRagIndexBatchSummary {
    SkillRagIndexBatchSummary {
        failed_count: 1,
        warnings: vec![error.to_string()],
        ..SkillRagIndexBatchSummary::default()
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillAgentsVo {
    agent_codes: Vec<String>,
}

pub async fn get_skill_agents(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    Path(name): Path<String>,
) -> Response {
    match skills.skill_agents(&name) {
        Ok(agent_codes) => {
            Json(ApiResponse::protocol_success(SkillAgentsVo { agent_codes })).into_response()
        }
        Err(e) => skill_error_response(e, &auth.locale),
    }
}
pub async fn put_skill_agents(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    Path(name): Path<String>,
    Json(body): Json<SkillAgentsRequest>,
) -> Response {
    match skills.replace_skill_agents(&name, &body.agent_codes) {
        Ok(()) => get_skill_agents(auth, State(skills), Path(name)).await,
        Err(e) => skill_error_response(e, &auth.locale),
    }
}
pub async fn get_agent_skills(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    Path(agent): Path<String>,
) -> Response {
    match skills.agent_skills(&agent) {
        Ok(items) => Json(ApiResponse::protocol_success(skill_list(
            items,
            &auth.locale,
        )))
        .into_response(),
        Err(e) => skill_error_response(e, &auth.locale),
    }
}
pub async fn put_agent_skills(
    auth: RequestAuthContext,
    State(skills): State<SkillRegistryService>,
    Path(agent): Path<String>,
    Json(body): Json<AgentSkillsRequest>,
) -> Response {
    match skills.replace_agent_skills(&agent, &body.skill_names) {
        Ok(()) => get_agent_skills(auth, State(skills), Path(agent)).await,
        Err(e) => skill_error_response(e, &auth.locale),
    }
}
