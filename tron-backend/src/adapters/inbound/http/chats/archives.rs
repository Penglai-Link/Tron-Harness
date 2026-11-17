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
        let (status, zh, en) = match descriptor.message_key {
            "tron.archives.invalid_query" => (
                StatusCode::BAD_REQUEST,
                "归档类型或分页参数无效",
                "Invalid archive type or pagination",
            ),
            "tron.archives.not_found" => (
                StatusCode::NOT_FOUND,
                "归档记录不存在或已恢复",
                "Archive was not found or has already been restored",
            ),
            "tron.archives.workspace_inactive" => (
                StatusCode::CONFLICT,
                "请先恢复所属工作空间",
                "Restore the workspace first",
            ),
            "tron.archives.topic_name_conflict" => (
                StatusCode::CONFLICT,
                "当前工作空间已有同名的未归档工作主题，请先重命名该工作主题，再恢复归档工作主题。",
                "An active work topic with this name already exists. Rename it before restoring the archived work topic.",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "无法读取或更新归档记录",
                "Could not read or update archives",
            ),
        };
        (
            status,
            Json(ApiResponse::protocol_error(
                descriptor.code,
                if self.1 == SupportedLocale::EnUs {
                    en
                } else {
                    zh
                },
            )),
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
