use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    domain::directory::{DirectoryError, DirectoryView},
    response::{ApiResponse, EmptyData},
    shared::locale::SupportedLocale,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/directories", get(list).post(register))
        .route("/api/v1/directories/pick", post(pick))
        .route("/api/v1/directories/browse", get(browse))
        .route(
            "/api/v1/directories/{directory_code}",
            patch(rename).delete(archive),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListRequest {
    workspace_code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RegisterRequest {
    workspace_code: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameRequest {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListDirectoryResponse {
    directory_code: String,
    workspace_code: String,
    execution_host: String,
    path: String,
    name: String,
    parent_directory_code: Option<String>,
    available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterDirectoryResponse {
    directory_code: String,
    workspace_code: String,
    execution_host: String,
    path: String,
    name: String,
    parent_directory_code: Option<String>,
    available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PickDirectoryResponse {
    directory_code: String,
    workspace_code: String,
    execution_host: String,
    path: String,
    name: String,
    parent_directory_code: Option<String>,
    available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveDirectoryResponse {
    session_codes: Vec<String>,
}

impl From<DirectoryView> for ListDirectoryResponse {
    fn from(value: DirectoryView) -> Self {
        Self {
            directory_code: value.directory_code,
            workspace_code: value.workspace_code,
            execution_host: value.execution_host,
            path: value.path,
            name: value.name,
            parent_directory_code: value.parent_directory_code,
            available: value.available,
        }
    }
}

impl From<DirectoryView> for RegisterDirectoryResponse {
    fn from(value: DirectoryView) -> Self {
        Self {
            directory_code: value.directory_code,
            workspace_code: value.workspace_code,
            execution_host: value.execution_host,
            path: value.path,
            name: value.name,
            parent_directory_code: value.parent_directory_code,
            available: value.available,
        }
    }
}
impl From<DirectoryView> for PickDirectoryResponse {
    fn from(value: DirectoryView) -> Self {
        Self {
            directory_code: value.directory_code,
            workspace_code: value.workspace_code,
            execution_host: value.execution_host,
            path: value.path,
            name: value.name,
            parent_directory_code: value.parent_directory_code,
            available: value.available,
        }
    }
}

async fn list(
    State(state): State<AppState>,
    Query(request): Query<ListRequest>,
    headers: HeaderMap,
) -> Response {
    match state.directories.list(&request.workspace_code) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items
                .into_iter()
                .map(ListDirectoryResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterRequest>,
) -> Response {
    match state
        .directories
        .register(&request.workspace_code, &request.path)
    {
        Ok(item) => (
            StatusCode::CREATED,
            Json(ApiResponse::protocol_success(
                RegisterDirectoryResponse::from(item),
            )),
        )
            .into_response(),
        Err(error) => error_response(error, &headers),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BrowseRequest { workspace_code: String, path: Option<String> }
#[derive(Serialize)]
struct BrowseEntry { name: String, path: String }
#[derive(Serialize)]
struct BrowseResponse { path: String, parent: Option<String>, entries: Vec<BrowseEntry> }
async fn browse(State(state): State<AppState>, Query(request): Query<BrowseRequest>, headers: HeaderMap) -> Response {
    match state.directories.browse(&request.workspace_code, request.path.as_deref()) {
        Ok(page) => Json(ApiResponse::protocol_success(BrowseResponse { path: page.path, parent: page.parent, entries: page.entries.into_iter().map(|entry| BrowseEntry { name: entry.name, path: entry.path }).collect() })).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PickDirectoryRequest { workspace_code: String }

async fn pick(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<PickDirectoryRequest>,
) -> Response {
    // This opens a dialog on the execution machine, so only a local browser may
    // invoke it. The custom header prevents cross-origin simple form requests.
    let locale = SupportedLocale::from_request(headers.get("accept-language").and_then(|value| value.to_str().ok()));
    let local_origin = headers.get("origin").and_then(|value| value.to_str().ok())
        .and_then(|value| url::Url::parse(value).ok())
        .is_some_and(|url| matches!(url.scheme(), "http" | "https") &&
            url.host_str().is_some_and(|host| host == "localhost" || host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())));
    if !peer.ip().is_loopback() || !local_origin || headers.get("x-tron-native-picker").and_then(|v| v.to_str().ok()) != Some("1") {
        return (StatusCode::FORBIDDEN, Json(ApiResponse::protocol_error("DIRECTORY_PICKER_LOCAL_ONLY", if locale == SupportedLocale::EnUs { "Open Tron locally to choose a directory." } else { "请在本机打开 Tron 后选择目录。" }))).into_response();
    }
    match state.directories.pick_and_register(&request.workspace_code, if locale == SupportedLocale::EnUs { "en-US" } else { "zh-CN" }).await {
        Ok(item) => Json(ApiResponse::protocol_success(item.map(PickDirectoryResponse::from))).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn rename(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
    Json(request): Json<RenameRequest>,
) -> Response {
    match state.directories.rename(&code, &request.name) {
        Ok(()) => Json(ApiResponse::protocol_success(EmptyData {})).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn archive(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
) -> Response {
    match state.directories.archive(&code, &state.chats) {
        Ok(session_codes) => Json(ApiResponse::protocol_success(ArchiveDirectoryResponse { session_codes })).into_response(),
        Err(crate::application::directory::DirectoryArchiveError::Directory(error)) => {
            error_response(error, &headers)
        }
        Err(crate::application::directory::DirectoryArchiveError::Chat(error)) => {
            crate::adapters::inbound::http::chats::error_mapper::chat_error_response(error)
        }
    }
}

pub(crate) fn error_response(error: DirectoryError, headers: &HeaderMap) -> Response {
    let locale = SupportedLocale::from_request(
        headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
    );
    let (status, code, en, zh) = match error {
        DirectoryError::WorkspaceInvalid => (
            StatusCode::BAD_REQUEST,
            "DIRECTORY_WORKSPACE_INVALID",
            "The selected space is unavailable.",
            "所选空间不可用。",
        ),
        DirectoryError::PathInvalid => (
            StatusCode::BAD_REQUEST,
            "DIRECTORY_PATH_INVALID",
            "Choose an existing absolute directory path.",
            "请选择存在的绝对目录路径。",
        ),
        DirectoryError::NameInvalid => (
            StatusCode::BAD_REQUEST,
            "DIRECTORY_NAME_INVALID",
            "Enter a name of 1 to 200 characters.",
            "请输入 1 至 200 个字符的名称。",
        ),
        DirectoryError::NotFound => (
            StatusCode::NOT_FOUND,
            "DIRECTORY_NOT_FOUND",
            "The directory was not found.",
            "未找到该目录。",
        ),
        DirectoryError::PickerBusy => (
            StatusCode::CONFLICT, "DIRECTORY_PICKER_BUSY",
            "A folder picker is already open.", "文件夹选择器已打开。",
        ),
        DirectoryError::PickerUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE, "DIRECTORY_PICKER_UNAVAILABLE",
            "The system folder picker is unavailable. Open Tron on a computer with a desktop session.",
            "系统文件夹选择器不可用，请在有桌面环境的电脑上打开 Tron。",
        ),
        DirectoryError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "DIRECTORY_STORAGE_FAILED",
            "The directory could not be saved.",
            "无法保存目录。",
        ),
    };
    (
        status,
        Json(ApiResponse::protocol_error(
            code,
            if locale == SupportedLocale::EnUs {
                en
            } else {
                zh
            },
        )),
    )
        .into_response()
}
