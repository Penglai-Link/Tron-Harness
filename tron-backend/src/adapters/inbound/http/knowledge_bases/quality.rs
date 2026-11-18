//! HTTP contract follows Rust FE consensus §§2.1, 2.4–2.6: endpoint DTOs/VOs,
//! camelCase fields, PascalCase new enums, millisecond timestamps and bounded bodies.
mod contracts;
mod request;
use super::*;
use crate::application::knowledge_quality::*;
use crate::response::EmptyData;
use contracts::*;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/knowledge-bases/chunking-strategies",
            get(strategies),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/quality/dataset",
            get(dataset).put(save_dataset),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/quality/runs",
            get(runs).post(start),
        )
        .route("/api/v1/knowledge-bases/{id}/quality/runs/{run}", get(run))
        .route(
            "/api/v1/knowledge-bases/{id}/quality/runs/{run}/cancel",
            axum::routing::post(cancel),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/quality/session",
            axum::routing::post(session),
        )
        .layer(DefaultBodyLimit::max(256 * 1024))
}
pub(super) struct QualityApiError(KnowledgeBaseError, String);
impl IntoResponse for QualityApiError {
    fn into_response(self) -> Response {
        use tron_i18n::t;
        let descriptor = self.0.library_deletion_descriptor();
        let status = match &self.0 {
            KnowledgeBaseError::NotFound => StatusCode::NOT_FOUND,
            KnowledgeBaseError::Forbidden => StatusCode::FORBIDDEN,
            KnowledgeBaseError::Conflict => StatusCode::CONFLICT,
            KnowledgeBaseError::Invalid(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let english = self.1.starts_with("en");
        let message: String = match &self.0 {
            KnowledgeBaseError::NotFound => {
                t!("knowledge.quality.not_found", locale = &self.1).into_owned()
            }
            KnowledgeBaseError::Forbidden => {
                t!("tron.knowledge_base.forbidden", locale = &self.1).into_owned()
            }
            KnowledgeBaseError::Conflict => {
                t!("knowledge.quality.conflict", locale = &self.1).into_owned()
            }
            KnowledgeBaseError::Invalid(_) if english => {
                t!("knowledge.quality.invalid", locale = &self.1).into_owned()
            }
            KnowledgeBaseError::Storage(_) if english => {
                t!("knowledge.quality.failed", locale = &self.1).into_owned()
            }
            // Remaining details are application-layer text; keep forwarding them.
            _ => self.0.to_string(),
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                message,
            )),
        )
            .into_response()
    }
}
async fn strategies(
    State(kb): State<KnowledgeBaseService>,
    _auth: RequestAuthContext,
) -> Json<ApiResponse<StrategyListVo>> {
    Json(ApiResponse::protocol_success(StrategyListVo {
        strategies: kb
            .chunking_strategies()
            .into_iter()
            .map(|d| StrategyVo {
                id: d.id,
                version: d.version,
            })
            .collect(),
    }))
}
async fn dataset(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DatasetGetVo>>, QualityApiError> {
    let data = KnowledgeQualityService::new(kb)
        .dataset(&auth.user().user_code, &id)
        .map_err(|e| QualityApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(DatasetGetVo {
        dataset: data.map(DatasetVo::from),
    })))
}
async fn save_dataset(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    request::Body(body): request::Body<DatasetPutDto>,
) -> Result<Json<ApiResponse<DatasetPutVo>>, QualityApiError> {
    let cases = body.cases.into_iter().map(Into::into).collect();
    let data = KnowledgeQualityService::new(kb)
        .save_dataset(&auth.user().user_code, &id, &body.source_fingerprint, cases)
        .await
        .map_err(|e| QualityApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(DatasetPutVo {
        dataset: DatasetVo::from(data),
    })))
}
async fn start(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    request::Body(body): request::Body<StartDto>,
) -> Result<Json<ApiResponse<StartVo>>, QualityApiError> {
    let data = KnowledgeQualityService::new(kb)
        .start(
            &auth.user().user_code,
            &id,
            &body.request_id,
            body.options.into(),
        )
        .await
        .map_err(|e| QualityApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(StartVo {
        run: RunVo::from(data),
    })))
}
async fn runs(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<RunsVo>>, QualityApiError> {
    let data = KnowledgeQualityService::new(kb)
        .runs(&auth.user().user_code, &id)
        .map_err(|e| QualityApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(RunsVo {
        runs: data.into_iter().map(RunVo::from).collect(),
    })))
}
async fn run(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, run)): Path<(String, String)>,
) -> Result<Json<ApiResponse<RunGetVo>>, QualityApiError> {
    let data = KnowledgeQualityService::new(kb)
        .run(&auth.user().user_code, &id, &run)
        .map_err(|e| QualityApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(RunGetVo {
        run: RunVo::from(data),
    })))
}
async fn cancel(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, run)): Path<(String, String)>,
    request::Body(_body): request::Body<CancelDto>,
) -> Result<Json<ApiResponse<CancelVo>>, QualityApiError> {
    let data = KnowledgeQualityService::new(kb)
        .cancel(&auth.user().user_code, &id, &run)
        .map_err(|e| QualityApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(CancelVo {
        run: RunVo::from(data),
    })))
}
async fn session(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    request::Body(_body): request::Body<SessionDto>,
) -> Result<Json<ApiResponse<SessionVo>>, QualityApiError> {
    let kb = state
        .knowledge_bases
        .detail(&auth.user().user_code, &id)
        .map_err(|e| QualityApiError(e, auth.locale.clone()))?;
    use tron_i18n::t;
    let title = t!(
        "knowledge.quality.session_title",
        locale = &auth.locale,
        name = kb.name
    );
    let s = state
        .chats
        .ensure_knowledge_quality_session(&auth.user().user_code, &id, &title)
        .map_err(|_| {
            QualityApiError(
                KnowledgeBaseError::Storage(
                    t!(
                        "knowledge.quality.session_create_failed",
                        locale = &auth.locale
                    )
                    .into_owned(),
                ),
                auth.locale,
            )
        })?;
    Ok(Json(ApiResponse::protocol_success(SessionVo {
        session: SessionRecordVo {
            id: s.id.to_string(),
            session_code: s.session_code,
            workspace_code: s.workspace_code,
            user_code: s.user_code,
            agent_code: s.agent_code,
            title: s.title,
            status: "ACTIVE".into(),
            created_at: chrono::DateTime::parse_from_rfc3339(&s.created_at)
                .map(|d| d.timestamp_millis())
                .unwrap_or_default(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&s.updated_at)
                .map(|d| d.timestamp_millis())
                .unwrap_or_default(),
        },
    })))
}
