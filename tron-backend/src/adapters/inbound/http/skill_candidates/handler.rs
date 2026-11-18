use super::{request::*, response::*};
use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, domain::assistant_memory::*,
    response::ApiResponse,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub async fn list(auth: RequestAuthContext, State(state): State<AppState>) -> Response {
    match state
        .assistant_memory
        .list_candidates(&auth.user().user_code)
    {
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
        .assistant_memory
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
    Path(assistant): Path<String>,
) -> Response {
    match state
        .assistant_memory
        .list_batches(&auth.user().user_code, &assistant)
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
    match state.assistant_memory.approve(
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
    match state.assistant_memory.reject(
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
    match state.assistant_memory.request_revision(
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
    match state.assistant_memory.retry_install(
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
        .assistant_memory
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
fn error(error: AssistantMemoryError, locale: &str) -> Response {
    let (status, code, message_key) = match error {
        AssistantMemoryError::NotFound => (
            StatusCode::NOT_FOUND,
            "101041001",
            "tron.assistant_memory.not_found",
        ),
        AssistantMemoryError::Conflict => (
            StatusCode::CONFLICT,
            "101041002",
            "tron.assistant_memory.conflict",
        ),
        AssistantMemoryError::ExecutionBusy => (
            StatusCode::CONFLICT,
            "101041007",
            "tron.assistant_memory.busy",
        ),
        AssistantMemoryError::Invalid(_) => (
            StatusCode::BAD_REQUEST,
            "101041003",
            "tron.assistant_memory.invalid",
        ),
        AssistantMemoryError::Dependency(_) => (
            StatusCode::CONFLICT,
            "101041004",
            "tron.assistant_memory.creator_required",
        ),
        AssistantMemoryError::Storage(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "101041005",
            "tron.assistant_memory.storage",
        ),
        AssistantMemoryError::Installation(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "101041006",
            "tron.assistant_memory.installation",
        ),
    };
    tracing::debug!(message_key = message_key, "PersonAI memory API error");
    (
        status,
        Json(ApiResponse::protocol_error(
            code,
            tron_i18n::t!(message_key, locale = locale),
        )),
    )
        .into_response()
}
