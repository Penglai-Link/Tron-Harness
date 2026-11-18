use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::chat::{ArchiveError, ArchivedContainerKind},
    response::ApiResponse,
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{
        Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tron_i18n::t;

#[derive(Deserialize)]
enum ArchiveKind {
    Topic,
    Project,
}
impl From<ArchiveKind> for ArchivedContainerKind {
    fn from(kind: ArchiveKind) -> Self {
        match kind {
            ArchiveKind::Topic => Self::Topic,
            ArchiveKind::Project => Self::Project,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ArchiveQueryRequest {
    kind: ArchiveKind,
    workspace_code: Option<String>,
    current: u32,
    size: u32,
}
#[derive(Deserialize)]
pub(super) struct RestoreArchiveRequest {
    kind: ArchiveKind,
    code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ArchivedContainerVo {
    code: String,
    workspace_code: String,
    name: String,
    path: Option<String>,
    chat_count: u64,
}
#[derive(Serialize)]
pub(super) struct PageData<T> {
    current: u32,
    size: u32,
    total: u64,
    records: Vec<T>,
}
#[derive(Serialize)]
pub(super) struct RestoreArchivedContainerVo {
    code: String,
}

pub(super) struct ApiError(ArchiveError, SupportedLocale);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.descriptor();
        let locale = self.1;
        let (status, message) = match descriptor.message_key {
            "tron.archives.invalid_query" => (
                StatusCode::BAD_REQUEST,
                t!("tron.archives.invalid_query", locale = locale),
            ),
            "tron.archives.not_found" => (
                StatusCode::NOT_FOUND,
                t!("tron.archives.not_found", locale = locale),
            ),
            "tron.archives.workspace_inactive" => (
                StatusCode::CONFLICT,
                t!("tron.archives.workspace_inactive", locale = locale),
            ),
            "tron.archives.topic_name_conflict" => (
                StatusCode::CONFLICT,
                t!("tron.archives.topic_name_conflict", locale = locale),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                t!("tron.archives.storage_failed", locale = locale),
            ),
        };
        (
            status,
            Json(ApiResponse::protocol_error(descriptor.code, message)),
        )
            .into_response()
    }
}

pub(super) async fn query(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<ArchiveQueryRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<PageData<ArchivedContainerVo>>>, ApiError> {
    let locale = SupportedLocale::from_raw(&auth.locale);
    let Json(request) = request.map_err(|_| ApiError(ArchiveError::InvalidQuery, locale))?;
    let (records, total) = state
        .chats
        .archived_containers(
            request.kind.into(),
            request.workspace_code.as_deref(),
            request.current,
            request.size,
        )
        .map_err(|error| ApiError(error, locale))?;
    Ok(Json(ApiResponse::protocol_success(PageData {
        current: request.current,
        size: request.size,
        total,
        records: records
            .into_iter()
            .map(|item| ArchivedContainerVo {
                code: item.code,
                workspace_code: item.workspace_code,
                name: item.name,
                path: item.path,
                chat_count: item.chat_count,
            })
            .collect(),
    })))
}

pub(super) async fn restore(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Path<RestoreArchiveRequest>, PathRejection>,
) -> Result<Json<ApiResponse<RestoreArchivedContainerVo>>, ApiError> {
    let locale = SupportedLocale::from_raw(&auth.locale);
    let Path(request) = request.map_err(|_| ApiError(ArchiveError::InvalidQuery, locale))?;
    state
        .chats
        .restore_archived_container(request.kind.into(), &request.code)
        .map_err(|error| ApiError(error, locale))?;
    Ok(Json(ApiResponse::protocol_success(
        RestoreArchivedContainerVo { code: request.code },
    )))
}
