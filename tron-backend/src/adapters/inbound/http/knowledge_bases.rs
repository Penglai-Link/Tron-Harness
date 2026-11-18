mod settings;
use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::knowledge_base::{
        KnowledgeBase, KnowledgeBaseDocument, KnowledgeBaseError, KnowledgeBasePreview,
        KnowledgeBaseService, local_documents::ChunkConfig,
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
mod agent_grants;
mod creation;
mod library;
mod library_agent_grants;
mod platform_catalog;
mod preflight;
mod quality;
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
async fn parsing(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext) -> Response {
    response(kb.parsing_capabilities(&auth.user().user_code).await)
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(quality::routes())
        .route("/api/v1/knowledge-bases", get(list).post(creation::create))
        .route(
            "/api/v1/knowledge-bases/platform",
            get(platform_catalog::list),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/agent-grants",
            get(library_agent_grants::get_grants).put(library_agent_grants::replace),
        )
        .route("/api/v1/knowledge-bases/parsing-capabilities", get(parsing))
        .route(
            "/api/v1/knowledge-bases/agent-grants/{agentCode}",
            get(agent_grants::list).put(agent_grants::replace),
        )
        .route(
            "/api/v1/knowledge-bases/{id}",
            get(detail).delete(library::remove),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/settings",
            get(settings::get).put(settings::update),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/resources/{resourceCode}/binary",
            get(binary),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/documents/preflight",
            axum::routing::post(preflight::inspect),
        )
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
        .route(
            "/api/v1/knowledge-bases/{id}/documents/{doc}/cancel",
            axum::routing::post(cancel),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/documents/{doc}/tasks",
            get(tasks),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/documents/{doc}/chunks",
            get(chunks),
        )
        .route(
            "/api/v1/knowledge-bases/{id}/grants",
            get(detail).put(grants),
        )
        .layer(DefaultBodyLimit::max(21 * 1024 * 1024))
}
fn request_locale() -> String {
    crate::shared::auth_holder::AuthHolder::get()
        .map(|context| context.locale.clone())
        .unwrap_or_else(|| tron_i18n::DEFAULT_LOCALE.as_str().to_owned())
}

/// Render persisted, locale-independent document diagnostics for the request
/// locale. Legacy plain-text rows (written before the migration) pass through
/// unchanged so no data migration is required.
fn localize_document(mut document: KnowledgeBaseDocument, locale: &str) -> KnowledgeBaseDocument {
    use crate::shared::i18n_message::{render_optional_stored, render_stored};
    document.error = render_optional_stored(
        document.error.as_deref(),
        locale,
        Some("knowledge.base.failure_detail"),
    );
    document.warnings = document
        .warnings
        .iter()
        .map(|warning| render_stored(warning, locale, None))
        .collect();
    document
}

fn localize_library(mut library: KnowledgeBase, locale: &str) -> KnowledgeBase {
    library.documents = library
        .documents
        .into_iter()
        .map(|document| localize_document(document, locale))
        .collect();
    library
}

fn localize_preview(mut preview: KnowledgeBasePreview, locale: &str) -> KnowledgeBasePreview {
    use crate::shared::i18n_message::render_stored;
    preview.warnings = preview
        .warnings
        .iter()
        .map(|warning| render_stored(warning, locale, None))
        .collect();
    preview
}

fn response<T: serde::Serialize>(result: Result<T, KnowledgeBaseError>) -> Response {
    match result {
        Ok(v) => Json(ApiResponse::success(v)).into_response(),
        Err(e) => {
            let locale = request_locale();
            let (status, code) = match &e {
                KnowledgeBaseError::NotFound => (StatusCode::NOT_FOUND, "KB_NOT_FOUND"),
                KnowledgeBaseError::Forbidden => (StatusCode::FORBIDDEN, "KB_FORBIDDEN"),
                KnowledgeBaseError::Conflict => (StatusCode::CONFLICT, "KB_CONFLICT"),
                KnowledgeBaseError::NotEmpty => (StatusCode::CONFLICT, "101027001"),
                KnowledgeBaseError::Invalid(_) => (StatusCode::BAD_REQUEST, "KB_INVALID"),
                KnowledgeBaseError::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, "KB_STORAGE"),
            };
            // Structured application messages are rendered with the request locale
            // at this display boundary; no translated text crosses the layer below.
            let message = e.localized(&locale);
            (status, Json(ApiResponse::error(code, message))).into_response()
        }
    }
}
async fn list(State(kb): State<KnowledgeBaseService>, auth: RequestAuthContext) -> Response {
    match kb.platform_libraries(&auth.user().user_code) {
        Ok(items) => Json(ApiResponse::protocol_success(
            items
                .into_iter()
                .map(library::KnowledgeLibraryListVo::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => response::<()>(Err(error)),
    }
}
async fn detail(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Response {
    response(
        kb.detail(&auth.user().user_code, &id)
            .map(|library| localize_library(library, &auth.locale)),
    )
}
async fn documents(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Response {
    response(
        kb.detail(&auth.user().user_code, &id)
            .map(|library| localize_library(library, &auth.locale).documents),
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
    let mut preference = None;
    let mut plan = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => match field.name().unwrap_or("") {
                "file" => {
                    let name = field.file_name().unwrap_or("").to_string();
                    match field.bytes().await {
                        Ok(bytes) => file = Some((name, bytes)),
                        Err(_) => {
                            return response::<()>(Err(KnowledgeBaseError::invalid(
                                "knowledge.upload.file_unreadable",
                            )));
                        }
                    }
                }
                "idempotencyKey" => match field.text().await {
                    Ok(value) => request_key = Some(value),
                    Err(_) => {
                        return response::<()>(Err(KnowledgeBaseError::invalid(
                            "knowledge.upload.request_key_invalid",
                        )));
                    }
                },
                "parsePreference" => {
                    preference = match field.text().await.ok().and_then(|v| {
                        serde_json::from_value::<
                            crate::application::knowledge_base::DocumentParsePreference,
                        >(serde_json::Value::String(v))
                        .ok()
                    }) {
                        Some(value) => Some(value),
                        None => {
                            return response::<()>(Err(KnowledgeBaseError::invalid(
                                "knowledge.upload.preference_invalid",
                            )));
                        }
                    };
                }
                "parsePlan" => {
                    plan = match field.text().await.ok().and_then(|v| {
                        serde_json::from_str::<
                                crate::application::knowledge_base::DocumentParsePlan,
                            >(&v)
                            .ok()
                    }) {
                        Some(value) => Some(value),
                        None => {
                            return response::<()>(Err(KnowledgeBaseError::invalid(
                                "knowledge.upload.parse_plan_invalid",
                            )));
                        }
                    };
                }
                _ => {}
            },
            Ok(None) => break,
            Err(_) => {
                return response::<()>(Err(KnowledgeBaseError::invalid(
                    "knowledge.upload.form_invalid",
                )));
            }
        }
    }
    match file {
        Some((name, bytes)) => response(
            kb.upload_with_parsing_options(
                &auth.user().user_code,
                &id,
                &name,
                &bytes,
                request_key.as_deref(),
                preference,
                plan.as_ref(),
            )
            .map(|document| localize_document(document, &auth.locale)),
        ),
        _ => response::<()>(Err(KnowledgeBaseError::invalid(
            "knowledge.upload.file_required",
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
        .await
        .map(|document| localize_document(document, &auth.locale)),
    )
}
async fn preview(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(
        kb.preview(&auth.user().user_code, &id, &doc)
            .await
            .map(|preview| localize_preview(preview, &auth.locale)),
    )
}
async fn index(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(
        kb.start_index(&auth.user().user_code, &id, &doc)
            .map(|document| localize_document(document, &auth.locale)),
    )
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IngestionTaskView {
    file_size_bytes: Option<u64>,
    id: String,
    document_id: String,
    kind: crate::application::knowledge_base::ingestion::IngestionKind,
    parsing_task_id: Option<String>,
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
async fn tasks(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(
        kb.ingestion_tasks(&auth.user().user_code, &id, &doc)
            .map(|tasks| {
                tasks
                    .into_iter()
                    .map(|t| IngestionTaskView {
                        file_size_bytes: t.file_size_bytes,
                        id: t.id,
                        document_id: t.document_id,
                        kind: t.kind,
                        parsing_task_id: t.parsing_task_id,
                        state: t.state,
                        stage: t.stage,
                        attempt: t.attempt,
                        failures: t.failures,
                        cancellation_pending: t.cancellation_pending,
                        retry_after_ms: t.retry_after_ms,
                        endpoint_id: t.endpoint_id,
                        error: crate::shared::i18n_message::render_optional_stored(
                            t.error.as_deref(),
                            &auth.locale,
                            Some("knowledge.base.failure_detail"),
                        ),
                        wait_reason: t.wait_reason,
                    })
                    .collect::<Vec<_>>()
            }),
    )
}
async fn cancel(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(kb.cancel_index(&auth.user().user_code, &id, &doc).await)
}
async fn retry(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(
        kb.retry_index(&auth.user().user_code, &id, &doc)
            .map(|document| localize_document(document, &auth.locale)),
    )
}
async fn remove(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, doc)): Path<(String, String)>,
) -> Response {
    response(kb.remove_document(&auth.user().user_code, &id, &doc))
}
async fn grants(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    Json(q): Json<Grants>,
) -> Response {
    response(kb.set_grants(&auth.user().user_code, &id, q.agent_codes))
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
    response(kb.published_chunks(&auth.user().user_code, &id, &doc, q.page, q.page_size))
}

async fn binary(
    State(kb): State<KnowledgeBaseService>,
    auth: RequestAuthContext,
    Path((id, code)): Path<(String, String)>,
) -> Response {
    let user = auth.user().user_code.clone();
    let (file, length) = match crate::app::telemetry::spawn_blocking_traced(move || {
        kb.download_resource(&user, &id, &code)
    })
    .await
    {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return response::<()>(Err(error)),
        Err(error) => {
            return response::<()>(Err(KnowledgeBaseError::storage_detail(error.to_string())));
        }
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
    result.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/octet-stream"),
    );
    if let Ok(value) = axum::http::HeaderValue::from_str(&length.to_string()) {
        result
            .headers_mut()
            .insert(axum::http::header::CONTENT_LENGTH, value);
    }
    result
}
