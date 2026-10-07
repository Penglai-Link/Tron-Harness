use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::knowledge_base::{
        KnowledgeBaseError, KnowledgeBaseService, local_documents::ChunkConfig,
    },
    response::ApiResponse,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Scope {
    workspace_code: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Create {
    workspace_code: String,
    name: String,
    #[serde(default)]
    description: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Configure {
    workspace_code: String,
    chunk_config: Option<ChunkConfig>,
    #[serde(default, deserialize_with = "present_nullable")]
    parsing_tier: Option<Option<crate::application::knowledge_base::parsing::ParsingTier>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Grants {
    workspace_code: String,
    agent_codes: Vec<String>,
}
fn present_nullable<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<crate::application::knowledge_base::parsing::ParsingTier>>, D::Error> {
    Option::deserialize(deserializer).map(Some)
}
async fn parsing(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Query(q): Query<Scope>,
) -> Response {
    response(
        kb.parsing_capabilities(&auth.user().user_code, &q.workspace_code)
            .await,
    )
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/knowledge-bases", get(list).post(create))
        .route("/api/v1/knowledge-bases/parsing-capabilities", get(parsing))
        .route("/api/v1/knowledge-bases/{id}", get(detail))
        .route(
            "/api/v1/knowledge-bases/{id}/documents",
            get(documents).post(upload),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/documents/{doc}",
            axum::routing::patch(configure).delete(remove),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/documents/{doc}/preview",
            axum::routing::post(preview),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/documents/{doc}/index",
            axum::routing::post(index),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/documents/{doc}/retry",
            axum::routing::post(retry),
        )
        .route("/api/v1/knowledge-bases/{id}/documents/{doc}/cancel", axum::routing::post(cancel))
        .route("/api/v1/knowledge-bases/{id}/documents/{doc}/tasks", get(tasks))
        .route("/api/v1/knowledge-bases/{id}/documents/{doc}/chunks", get(chunks))
        .route(
            "/api/v1/knowledge-bases/{id}/grants",
            get(detail).put(grants),
        )
        .layer(DefaultBodyLimit::max(21 * 1024 * 1024))
}
fn response<T: serde::Serialize>(result: Result<T, KnowledgeBaseError>) -> Response {
    match result {
        Ok(v) => Json(ApiResponse::success(v)).into_response(),
        Err(e) => {
            let (status, code) = match e {
                KnowledgeBaseError::NotFound => (StatusCode::NOT_FOUND, "KB_NOT_FOUND"),
                KnowledgeBaseError::Forbidden => (StatusCode::FORBIDDEN, "KB_FORBIDDEN"),
                KnowledgeBaseError::Conflict => (StatusCode::CONFLICT, "KB_CONFLICT"),
                KnowledgeBaseError::Invalid(_) => (StatusCode::BAD_REQUEST, "KB_INVALID"),
                KnowledgeBaseError::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, "KB_STORAGE"),
            };
            (status, Json(ApiResponse::error(code, e.to_string()))).into_response()
        }
    }
}
async fn list(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Query(q): Query<Scope>,
) -> Response {
    response(kb.list(&auth.user().user_code, &q.workspace_code))
}
async fn create(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Json(q): Json<Create>,
) -> Response {
    response(kb.create(
        &auth.user().user_code,
        &q.workspace_code,
        &q.name,
        &q.description,
    ))
}
async fn detail(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    Query(q): Query<Scope>,
) -> Response {
    response(kb.detail(&auth.user().user_code, &q.workspace_code, &id))
}
async fn documents(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    Query(q): Query<Scope>,
) -> Response {
    response(
        kb.detail(&auth.user().user_code, &q.workspace_code, &id)
            .map(|k| k.documents),
    )
}
async fn upload(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    let mut workspace = None;
    let mut file = None;
    let mut request_key = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => match field.name().unwrap_or("") {
                "workspaceCode" => match field.text().await {
                    Ok(v) => workspace = Some(v),
                    Err(_) => {
                        return response::<()>(Err(KnowledgeBaseError::Invalid(
                            "workspaceCode无效".into(),
                        )));
                    }
                },
                "file" => {
                    let name = field.file_name().unwrap_or("").to_string();
                    match field.bytes().await {
                        Ok(bytes) => file = Some((name, bytes)),
                        Err(_) => {
                            return response::<()>(Err(KnowledgeBaseError::Invalid(
                                "文件超过上传限制或读取失败".into(),
                            )));
                        }
                    }
                }
                "idempotencyKey" => match field.text().await {
                    Ok(value) => request_key = Some(value),
                    Err(_) => return response::<()>(Err(KnowledgeBaseError::Invalid("入库请求标识无效".into()))),
                },
                _ => {}
            },
            Ok(None) => break,
            Err(_) => {
                return response::<()>(Err(KnowledgeBaseError::Invalid("上传表单无效".into())));
            }
        }
    }
    match (workspace, file) {
        (Some(w), Some((name, bytes))) => {
            response(kb.upload_with_request(&auth.user().user_code, &w, &id, &name, &bytes, request_key.as_deref()))
        }
        _ => response::<()>(Err(KnowledgeBaseError::Invalid(
            "需要workspaceCode和file".into(),
        ))),
    }
}
async fn configure(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
    Json(q): Json<Configure>,
) -> Response {
    response(
        kb.configure_and_start_index(
            &auth.user().user_code,
            &q.workspace_code,
            &id,
            &doc,
            q.chunk_config,
            q.parsing_tier,
        )
        .await,
    )
}
async fn preview(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
    Json(q): Json<Scope>,
) -> Response {
    response(
        kb.preview(&auth.user().user_code, &q.workspace_code, &id, &doc)
            .await,
    )
}
async fn index(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
    Json(q): Json<Scope>,
) -> Response {
    response(
        kb.start_index(&auth.user().user_code, &q.workspace_code, &id, &doc),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CancelRequest { workspace_code: String }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RetryRequest { workspace_code: String }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TasksQuery { workspace_code: String }
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IngestionTaskView {
    id: String,
    document_id: String,
    state: String,
    stage: crate::application::knowledge_base::ingestion::IngestionStage,
    attempt: u32,
    failures: u32,
    cancellation_pending: bool,
    retry_after_ms: i64,
    endpoint_id: Option<String>,
    error: Option<String>,
    wait_reason: Option<crate::application::knowledge_base::ingestion::IngestionWaitReason>,
}
async fn tasks(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext, Path((id, doc)): Path<(String, String)>, Query(q): Query<TasksQuery>) -> Response {
    response(kb.ingestion_tasks(&auth.user().user_code, &q.workspace_code, &id, &doc).map(|tasks| tasks.into_iter().map(|t| IngestionTaskView { id: t.id, document_id: t.document_id, state: t.state, stage: t.stage, attempt: t.attempt, failures: t.failures, cancellation_pending: t.cancellation_pending, retry_after_ms: t.retry_after_ms, endpoint_id: t.endpoint_id, error: t.error, wait_reason: t.wait_reason }).collect::<Vec<_>>()))
}
async fn cancel(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext, Path((id, doc)): Path<(String, String)>, Json(q): Json<CancelRequest>) -> Response {
    response(kb.cancel_index(&auth.user().user_code, &q.workspace_code, &id, &doc).await)
}
async fn retry(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext, Path((id, doc)): Path<(String, String)>, Json(q): Json<RetryRequest>) -> Response {
    response(kb.retry_index(&auth.user().user_code, &q.workspace_code, &id, &doc))
}
async fn remove(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
    Query(q): Query<Scope>,
) -> Response {
    response(
        kb.remove_document(&auth.user().user_code, &q.workspace_code, &id, &doc),
    )
}
async fn grants(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    Json(q): Json<Grants>,
) -> Response {
    response(kb.set_grants(
        &auth.user().user_code,
        &q.workspace_code,
        &id,
        q.agent_codes,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChunkPageQuery {
    workspace_code: String,
    page: usize,
    page_size: usize,
}
async fn chunks(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
    Query(q): Query<ChunkPageQuery>,
) -> Response {
    response(kb.published_chunks(
        &auth.user().user_code,
        &q.workspace_code,
        &id,
        &doc,
        q.page,
        q.page_size,
    ))
}
