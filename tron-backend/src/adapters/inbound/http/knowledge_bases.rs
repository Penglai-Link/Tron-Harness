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
mod library;
mod agent_grants;
mod platform_catalog;
mod library_agent_grants;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Create {
    name: String,
    #[serde(default)]
    description: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Configure {
    chunk_config: Option<ChunkConfig>,
    #[serde(default, deserialize_with = "present_nullable")]
    parsing_tier: Option<Option<crate::application::knowledge_base::parsing::ParsingTier>>,
    #[serde(default, deserialize_with = "present_nullable")]
    ocr_mode: Option<Option<crate::application::knowledge_base::parsing::OcrMode>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Grants {
    agent_codes: Vec<String>,
}
fn present_nullable<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::deserialize(deserializer).map(Some)
}
async fn parsing(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
) -> Response {
    response(
        kb.parsing_capabilities(&auth.user().user_code).await,
    )
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/knowledge-bases", get(list).post(create))
        .route("/api/v1/knowledge-bases/platform", get(platform_catalog::list))
        .route("/api/v1/knowledge-bases/{id}/agent-grants", get(library_agent_grants::get_grants).put(library_agent_grants::replace))
        .route("/api/v1/knowledge-bases/parsing-capabilities", get(parsing))
        .route("/api/v1/knowledge-bases/agent-grants/{agentCode}", get(agent_grants::list).put(agent_grants::replace))
        .route("/api/v1/knowledge-bases/{id}", get(detail).delete(library::remove))
        .route("/api/v1/knowledge-bases/{id}/resources/{resourceCode}/binary", get(binary))
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
                KnowledgeBaseError::NotEmpty => (StatusCode::CONFLICT, "101027001"),
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
) -> Response {
    match kb.platform_libraries(&auth.user().user_code) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items.into_iter().map(library::KnowledgeLibraryListVo::from).collect::<Vec<_>>(),
        )).into_response(),
        Err(error) => response::<()>(Err(error)),
    }
}
async fn create(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Json(q): Json<Create>,
) -> Response {
    response(kb.create(
        &auth.user().user_code,
        &q.name,
        &q.description,
    ))
}
async fn detail(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Response {
    response(kb.detail(&auth.user().user_code, &id))
}
async fn documents(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Response {
    response(
        kb.detail(&auth.user().user_code, &id)
            .map(|k| k.documents),
    )
}
async fn upload(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    let mut file = None;
    let mut request_key = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => match field.name().unwrap_or("") {
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
    match file {
        Some((name, bytes)) => {
            response(kb.upload_with_request(&auth.user().user_code, &id, &name, &bytes, request_key.as_deref()))
        }
        _ => response::<()>(Err(KnowledgeBaseError::Invalid(
            "需要file".into(),
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
            &id,
            &doc,
            q.chunk_config,
            q.parsing_tier,
            q.ocr_mode,
        )
        .await,
    )
}
async fn preview(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(
        kb.preview(&auth.user().user_code, &id, &doc)
            .await,
    )
}
async fn index(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(
        kb.start_index(&auth.user().user_code, &id, &doc),
    )
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IngestionTaskView {
    file_size_bytes: Option<u64>,
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
async fn tasks(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext, Path((id, doc)): Path<(String, String)>) -> Response {
    response(kb.ingestion_tasks(&auth.user().user_code, &id, &doc).map(|tasks| tasks.into_iter().map(|t| IngestionTaskView { file_size_bytes: t.file_size_bytes, id: t.id, document_id: t.document_id, state: t.state, stage: t.stage, attempt: t.attempt, failures: t.failures, cancellation_pending: t.cancellation_pending, retry_after_ms: t.retry_after_ms, endpoint_id: t.endpoint_id, error: t.error, wait_reason: t.wait_reason }).collect::<Vec<_>>()))
}
async fn cancel(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext, Path((id, doc)): Path<(String, String)>) -> Response {
    response(kb.cancel_index(&auth.user().user_code, &id, &doc).await)
}
async fn retry(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext, Path((id, doc)): Path<(String, String)>) -> Response {
    response(kb.retry_index(&auth.user().user_code, &id, &doc))
}
async fn remove(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(
        kb.remove_document(&auth.user().user_code, &id, &doc),
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
        &id,
        q.agent_codes,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChunkPageQuery {
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
        &id,
        &doc,
        q.page,
        q.page_size,
    ))
}

#[cfg(test)]
mod ocr_contract_tests {
    use super::*;
    use crate::application::knowledge_base::parsing::OcrMode;
    #[test]
    fn ocr_patch_distinguishes_omitted_inherited_and_selected_modes() {
        let omitted: Configure = serde_json::from_value(serde_json::json!({"chunkConfig":null})).unwrap();
        assert_eq!(omitted.ocr_mode, None);
        let inherited: Configure = serde_json::from_value(serde_json::json!({"ocrMode":null})).unwrap();
        assert_eq!(inherited.ocr_mode, Some(None));
        for mode in [OcrMode::Auto, OcrMode::Text, OcrMode::Ocr] {
            let selected: Configure = serde_json::from_value(serde_json::json!({"ocrMode":mode})).unwrap();
            assert_eq!(selected.ocr_mode, Some(Some(mode)));
        }
        assert!(serde_json::from_value::<Configure>(serde_json::json!({"ocrMode":"invalid"})).is_err());
    }
}

async fn binary(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext, Path((id, code)): Path<(String, String)>) -> Response {
    let user = auth.user().user_code.clone();
    let (file, length) = match tokio::task::spawn_blocking(move || kb.download_resource(&user, &id, &code)).await {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return response::<()>(Err(error)),
        Err(error) => return response::<()>(Err(KnowledgeBaseError::Storage(error.to_string()))),
    };
    let stream = async_stream::stream! {
        use tokio::io::AsyncReadExt;
        let mut file = tokio::fs::File::from_std(file);
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            match file.read(&mut buffer).await {
                Ok(0) => break,
                Ok(count) => yield Ok::<_, std::io::Error>(axum::body::Bytes::copy_from_slice(&buffer[..count])),
                Err(error) => { yield Err(error); break; }
            }
        }
    };
    let mut result = Response::new(axum::body::Body::from_stream(stream));
    result.headers_mut().insert(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("application/octet-stream"));
    if let Ok(value) = axum::http::HeaderValue::from_str(&length.to_string()) { result.headers_mut().insert(axum::http::header::CONTENT_LENGTH, value); }
    result
}
