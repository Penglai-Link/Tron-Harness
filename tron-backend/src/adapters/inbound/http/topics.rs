use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get},
};
use serde::Deserialize;

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::topic::AddTopicReference,
    domain::topic::{TopicError, TopicReferenceKind},
    response::ApiResponse,
    shared::locale::SupportedLocale,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/topics/{project_code}/memo",
            get(get_memo).put(save_memo),
        )
        .route(
            "/api/v1/topics/{project_code}/references",
            axum::routing::post(add_reference),
        )
        .route(
            "/api/v1/topics/{project_code}/references/{reference_code}",
            delete(remove_reference),
        )
        .route(
            "/api/v1/topics/{project_code}/references/{reference_code}/checkout",
            get(checkout_options).post(checkout),
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveMemoRequest {
    revision: u64,
    notes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AddReferenceRequest {
    revision: u64,
    kind: TopicReferenceKind,
    target_code: String,
    connection_code: Option<String>,
    label: String,
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionQuery {
    revision: u64,
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all_fields = "camelCase", deny_unknown_fields)]
enum CheckoutRequest {
    Existing { path: String },
    Clone { path: String, username: String },
}

async fn get_memo(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    match state.topics.get(&auth.user().user_code, &code) {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn save_memo(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<SaveMemoRequest>,
) -> Response {
    match state.topics.save_notes(
        &auth.user().user_code,
        &code,
        request.revision,
        &request.notes,
    ) {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn add_reference(
    State(state): State<AppState>,
    Path(code): Path<String>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<AddReferenceRequest>,
) -> Response {
    let input = AddTopicReference {
        kind: request.kind,
        target_code: request.target_code,
        connection_code: request.connection_code,
        label: request.label,
        url: request.url,
    };
    match state
        .topics
        .add_reference(&auth.user().user_code, &code, request.revision, input)
    {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn remove_reference(
    State(state): State<AppState>,
    Path((code, reference)): Path<(String, String)>,
    Query(query): Query<RevisionQuery>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    match state
        .topics
        .remove_reference(&auth.user().user_code, &code, query.revision, &reference)
    {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

async fn checkout(
    State(state): State<AppState>,
    Path((project, reference)): Path<(String, String)>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    Json(request): Json<CheckoutRequest>,
) -> Response {
    use crate::application::repository_checkout::{self, CheckoutChoice};
    let workspace = match state.projects.get(&project) {
        Ok(project) => project.workspace_code,
        Err(_) => return error_response(TopicError::NotFound, &headers),
    };
    let choice = match request {
        CheckoutRequest::Existing { path } => CheckoutChoice::Existing { path },
        CheckoutRequest::Clone { path, username } => CheckoutChoice::Clone { path, username },
    };
    match repository_checkout::prepare(
        &state.integrations,
        &state.directories,
        &state.topics,
        &workspace,
        &auth.user().user_code,
        &project,
        &reference,
        choice,
    )
    .await
    {
        Ok(memo) => Json(ApiResponse::protocol_success(memo)).into_response(),
        Err(error) => checkout_error_response(error, &headers),
    }
}

async fn checkout_options(
    State(state): State<AppState>,
    Path((project, reference)): Path<(String, String)>,
    auth: RequestAuthContext,
    headers: HeaderMap,
) -> Response {
    let workspace = match state.projects.get(&project) {
        Ok(project) => project.workspace_code,
        Err(_) => return error_response(TopicError::NotFound, &headers),
    };
    match crate::application::repository_checkout::options(
        &state.integrations,
        &state.directories,
        &state.topics,
        &workspace,
        &auth.user().user_code,
        &project,
        &reference,
    )
    .await
    {
        Ok(options) => Json(ApiResponse::protocol_success(options)).into_response(),
        Err(error) => checkout_error_response(error, &headers),
    }
}

fn checkout_error_response(
    error: crate::application::repository_checkout::CheckoutError,
    headers: &HeaderMap,
) -> Response {
    use crate::application::repository_checkout::CheckoutError;
    match error {
        CheckoutError::Connection(error) => super::connectors::error_response(error, headers),
        CheckoutError::Topic(error) => error_response(error, headers),
        CheckoutError::Directory(error) => super::directories::error_response(error, headers),
        error => {
            let (code, en, zh) = match error {
                CheckoutError::InvalidPath => (
                    "CHECKOUT_PATH_INVALID",
                    "Choose an absolute path that does not already exist and a valid Git username.",
                    "请选择尚不存在的绝对路径，并填写有效的 Git 用户名。",
                ),
                CheckoutError::DifferentRepository => (
                    "CHECKOUT_REPOSITORY_MISMATCH",
                    "The selected directory is not a checkout of this repository.",
                    "所选目录不是此仓库的检出目录。",
                ),
                CheckoutError::UnsupportedRemote => (
                    "CHECKOUT_REMOTE_UNSUPPORTED",
                    "The platform returned an unsupported HTTPS clone address.",
                    "平台返回了不受支持的 HTTPS 克隆地址。",
                ),
                CheckoutError::GitFailed => (
                    "CHECKOUT_GIT_FAILED",
                    "Clone failed. Check Git, the account name and repository access; the destination may contain partial files.",
                    "克隆失败，请检查 Git、账户名和仓库权限；目标目录可能包含未完成的文件。",
                ),
                CheckoutError::TimedOut => (
                    "CHECKOUT_TIMEOUT",
                    "Clone timed out; the destination may contain partial files.",
                    "克隆超时；目标目录可能包含未完成的文件。",
                ),
                _ => unreachable!(),
            };
            let locale = SupportedLocale::from_request(
                headers
                    .get("accept-language")
                    .and_then(|value| value.to_str().ok()),
            );
            (
                StatusCode::UNPROCESSABLE_ENTITY,
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
    }
}

fn error_response(error: TopicError, headers: &HeaderMap) -> Response {
    let locale = SupportedLocale::from_request(
        headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
    );
    let (status, code, en, zh) = match error {
        TopicError::NotFound => (
            StatusCode::NOT_FOUND,
            "TOPIC_NOT_FOUND",
            "The topic or reference was not found.",
            "未找到主题或引用。",
        ),
        TopicError::InvalidReference => (
            StatusCode::BAD_REQUEST,
            "TOPIC_REFERENCE_INVALID",
            "The resource reference is invalid or unavailable.",
            "资源引用无效或不可用。",
        ),
        TopicError::InvalidNotes => (
            StatusCode::BAD_REQUEST,
            "TOPIC_NOTES_INVALID",
            "Topic notes are too long.",
            "主题说明过长。",
        ),
        TopicError::Conflict => (
            StatusCode::CONFLICT,
            "TOPIC_REVISION_CONFLICT",
            "The topic changed. Refresh before saving.",
            "主题已更改，请刷新后再保存。",
        ),
        TopicError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "TOPIC_STORAGE_FAILED",
            "The topic could not be saved.",
            "无法保存主题。",
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
