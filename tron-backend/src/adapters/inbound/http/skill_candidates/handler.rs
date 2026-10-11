use super::{request::*, response::*};
use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, domain::dude_memory::*,
    response::ApiResponse,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub async fn list(auth: RequestAuthContext, State(state): State<AppState>) -> Response {
    match state.dude_memory.list_candidates(&auth.user().user_code) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items
                .into_iter()
                .map(ListCandidateVo::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(e) => error(e, &auth.locale),
    }
}
pub async fn detail(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Response {
    match state
        .dude_memory
        .get_candidate(&auth.user().user_code, &code)
    {
        Ok(item) => {
            Json(ApiResponse::protocol_success(DetailCandidateVo::from(item))).into_response()
        }
        Err(e) => error(e, &auth.locale),
    }
}
pub async fn batches(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(dude): Path<String>,
) -> Response {
    match state
        .dude_memory
        .list_batches(&auth.user().user_code, &dude)
    {
        Ok(items) => Json(ApiResponse::protocol_success(
            items
                .into_iter()
                .map(ListMemoryBatchVo::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(e) => error(e, &auth.locale),
    }
}
pub async fn approve(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(body): Json<ApproveCandidateRequest>,
) -> Response {
    match state.dude_memory.approve(
        &auth.user().user_code,
        &code,
        body.revision,
        &body.artifact_hash,
    ) {
        Ok(item) => {
            let item = index_if_installed(&state, &item).await;
            Json(ApiResponse::protocol_success(ApproveCandidateVo::from(
                item,
            )))
            .into_response()
        }
        Err(e) => error(e, &auth.locale),
    }
}
pub async fn reject(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(body): Json<RejectCandidateRequest>,
) -> Response {
    match state.dude_memory.reject(
        &auth.user().user_code,
        &code,
        body.revision,
        &body.artifact_hash,
        body.reason,
    ) {
        Ok(item) => {
            Json(ApiResponse::protocol_success(RejectCandidateVo::from(item))).into_response()
        }
        Err(e) => error(e, &auth.locale),
    }
}
pub async fn revise(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(body): Json<ReviseCandidateRequest>,
) -> Response {
    match state.dude_memory.request_revision(
        &auth.user().user_code,
        &code,
        body.revision,
        &body.artifact_hash,
        body.feedback,
    ) {
        Ok(item) => {
            Json(ApiResponse::protocol_success(ReviseCandidateVo::from(item))).into_response()
        }
        Err(e) => error(e, &auth.locale),
    }
}
pub async fn retry_install(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(body): Json<RetryCandidateInstallRequest>,
) -> Response {
    match state.dude_memory.retry_install(
        &auth.user().user_code,
        &code,
        body.revision,
        &body.artifact_hash,
    ) {
        Ok(item) => {
            let item = index_if_installed(&state, &item).await;
            Json(ApiResponse::protocol_success(
                RetryCandidateInstallVo::from(item),
            ))
            .into_response()
        }
        Err(e) => error(e, &auth.locale),
    }
}
async fn index_if_installed(state: &AppState, candidate: &SkillCandidate) -> SkillCandidate {
    match state
        .dude_memory
        .index_installed(candidate, state.rag.clone())
        .await
    {
        Ok(candidate) => candidate,
        Err(error) => {
            tracing::warn!(candidate_code=%candidate.candidate_code,%error,"approved Skill registry index checkpoint needs retry");
            candidate.clone()
        }
    }
}
fn error(error: DudeMemoryError, locale: &str) -> Response {
    let (status, code, key, en, zh) = match error {
        DudeMemoryError::NotFound => (
            StatusCode::NOT_FOUND,
            "101041001",
            "tron.dude_memory.not_found",
            "Record not found",
            "记录不存在",
        ),
        DudeMemoryError::Conflict => (
            StatusCode::CONFLICT,
            "101041002",
            "tron.dude_memory.conflict",
            "The artifact or installed skill changed; refresh and review again",
            "制品或已安装技能已变更，请刷新后重新审核",
        ),
        DudeMemoryError::Invalid(_) => (
            StatusCode::BAD_REQUEST,
            "101041003",
            "tron.dude_memory.invalid",
            "Invalid memory or Skill operation",
            "记忆或技能操作无效",
        ),
        DudeMemoryError::Dependency(_) => (
            StatusCode::CONFLICT,
            "101041004",
            "tron.dude_memory.creator_required",
            "Install and enable skill-creator first",
            "请先安装并启用 skill-creator",
        ),
        DudeMemoryError::Storage(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "101041005",
            "tron.dude_memory.storage",
            "Memory workflow storage failed",
            "记忆工作流存储失败",
        ),
        DudeMemoryError::Installation(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "101041006",
            "tron.dude_memory.installation",
            "Approved Skill installation failed",
            "已批准技能安装失败",
        ),
    };
    tracing::debug!(message_key = key, "Dude memory API error");
    (
        status,
        Json(ApiResponse::protocol_error(
            code,
            if locale.starts_with("en") { en } else { zh },
        )),
    )
        .into_response()
}
