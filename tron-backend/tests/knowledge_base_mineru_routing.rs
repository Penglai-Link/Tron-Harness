mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
};
use fixture::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tron_backend::{
    adapters::outbound::{
        mineru_documents::MineruKnowledgeDocumentParser,
        persistence::sqlite::SqliteKnowledgeBaseRepository,
    },
    application::{
        knowledge_base::{
            KnowledgeBaseService,
            parsing::{KnowledgeDocumentParser, OcrMode},
        },
        llm::LlmRuntimeService,
    },
    config::{KnowledgeIngestionConfig, MineruConfig, MineruNode},
};
// Transport/lifecycle fixture only; this does not validate OCR recognition quality.
fn synthetic_archive() -> Vec<u8> {
    use std::io::{Cursor, Write};
    let middle = json!({"schema":"docvortex.middle","schema_version":"2.0","metadata":{"file_suffix":"pdf"},"extensions":{"mineru":{"tier":"standard"}},"pages":[{"page_idx":0,"blocks":[{"index":0,"type":"text","content":"Synthetic routing evidence"}]}]});
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("middle_json.json", serde_json::to_vec(&middle).unwrap()),
        ("structured_content.json", b"{}".to_vec()),
        ("markdown.md", b"Synthetic routing evidence".to_vec()),
    ] {
        archive
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(&bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}
#[derive(Clone)]
struct NodeState {
    id: String,
    expected_ocr: Arc<Mutex<String>>,
    healthy: Arc<AtomicBool>,
    complete: Arc<AtomicBool>,
    lose_post_response: Arc<AtomicBool>,
    malformed_post_response: Arc<AtomicBool>,
    cancel_unavailable: Arc<AtomicBool>,
    uploads: Arc<AtomicUsize>,
    posts: Arc<AtomicUsize>,
    polls: Arc<AtomicUsize>,
    lists: Arc<AtomicUsize>,
    cancels: Arc<AtomicUsize>,
    names: Arc<Mutex<BTreeMap<String, String>>>,
    jobs: Arc<Mutex<BTreeMap<String, Value>>>,
}
impl NodeState {
    fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            expected_ocr: Arc::new(Mutex::new("txt".into())),
            healthy: Arc::new(AtomicBool::new(true)),
            complete: Default::default(),
            lose_post_response: Default::default(),
            malformed_post_response: Default::default(),
            cancel_unavailable: Default::default(),
            uploads: Default::default(),
            posts: Default::default(),
            polls: Default::default(),
            lists: Default::default(),
            cancels: Default::default(),
            names: Default::default(),
            jobs: Default::default(),
        }
    }
}
async fn health(State(s): State<NodeState>) -> (axum::http::StatusCode, Json<Value>) {
    (
        if s.healthy.load(Ordering::SeqCst) {
            axum::http::StatusCode::OK
        } else {
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        },
        Json(json!({"status":"ok","version":"4.0.10"})),
    )
}
async fn upload(State(s): State<NodeState>, Json(body): Json<Value>) -> Json<Value> {
    let n = s.uploads.fetch_add(1, Ordering::SeqCst) + 1;
    let file = format!("{}_file_{n}", s.id);
    s.names
        .lock()
        .unwrap()
        .insert(file.clone(), body["filename"].as_str().unwrap().into());
    Json(json!({"status":"completed","file":{"id":file}}))
}
async fn start(State(s): State<NodeState>, Json(body): Json<Value>) -> axum::response::Response {
    assert_eq!(
        body["ocr_mode"].as_str().unwrap(),
        s.expected_ocr.lock().unwrap().as_str()
    );
    let file = body["files"][0]["source"]["file_id"].as_str().unwrap();
    assert!(file.starts_with(&s.id), "file must stay on its node");
    let n = s.posts.fetch_add(1, Ordering::SeqCst) + 1;
    let id = format!("{}_job_{n}", s.id);
    let job = json!({"job_id":id,"status":"running","tier":"standard","ocr_mode":body["ocr_mode"],"files":[{"file_id":file,"name":s.names.lock().unwrap().get(file).unwrap()}]});
    s.jobs.lock().unwrap().insert(id, job.clone());
    if s.malformed_post_response.swap(false, Ordering::SeqCst) {
        return (axum::http::StatusCode::ACCEPTED, "{broken JSON").into_response();
    }
    (
        if s.lose_post_response.swap(false, Ordering::SeqCst) {
            axum::http::StatusCode::BAD_GATEWAY
        } else {
            axum::http::StatusCode::ACCEPTED
        },
        Json(job),
    )
        .into_response()
}
async fn job(State(s): State<NodeState>, Path(id): Path<String>) -> Json<Value> {
    assert!(id.starts_with(&s.id), "job must stay on its node");
    s.polls.fetch_add(1, Ordering::SeqCst);
    let mut job = s.jobs.lock().unwrap().get(&id).unwrap().clone();
    if s.complete.load(Ordering::SeqCst) && job["status"] != "canceled" {
        job["status"] = "completed".into();
        job["files"][0]["parse"] = json!({"parser_version":"4.0.10"});
        job["files"][0]["output_files"] = json!({"zip":{"file_id":format!("{}_zip",s.id)}});
    }
    Json(job)
}
async fn list(State(s): State<NodeState>) -> Json<Value> {
    s.lists.fetch_add(1, Ordering::SeqCst);
    Json(
        json!({"data":s.jobs.lock().unwrap().values().cloned().collect::<Vec<_>>(),"has_more":false}),
    )
}
async fn cancel(State(s): State<NodeState>, Path(id): Path<String>) -> axum::response::Response {
    assert!(id.starts_with(&s.id));
    s.cancels.fetch_add(1, Ordering::SeqCst);
    if s.cancel_unavailable.load(Ordering::SeqCst) {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    s.jobs.lock().unwrap().get_mut(&id).unwrap()["status"] = "canceled".into();
    Json(json!({"job_id":id,"status":"canceled"})).into_response()
}
async fn zip(State(s): State<NodeState>, Path(id): Path<String>) -> Vec<u8> {
    assert_eq!(id, format!("{}_zip", s.id));
    synthetic_archive()
}
async fn node(id: &str) -> (NodeState, MineruNode, tokio::task::JoinHandle<()>) {
    let state = NodeState::new(id);
    let router = Router::new()
        .route("/v1/health", get(health))
        .route(
            "/v1/tiers",
            get(|| async { Json(json!({"data":[{"id":"standard"},{"id":"flash"}]})) }),
        )
        .route("/v1/uploads", post(upload))
        .route("/v1/parse/jobs", get(list).post(start))
        .route("/v1/parse/jobs/{id}", get(job).delete(cancel))
        .route("/v1/files/{id}/content", get(zip))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (
        state,
        MineruNode {
            id: id.into(),
            base_url: format!("http://{address}"),
            enabled: true,
            allow_remote: false,
            capacity: 1,
            weight: 1,
            resource_group: id.into(),
            resource_group_capacity: 1,
        },
        server,
    )
}
fn config(nodes: Vec<MineruNode>) -> MineruConfig {
    let mut config = MineruConfig::default();
    config.nodes = nodes;
    config.ocr_mode = "ocr".into();
    config.ingestion = KnowledgeIngestionConfig {
        parse_capacity: 2,
        embedding_capacity: 1,
        lease_ms: 600,
        poll_ms: 10,
        max_attempts: 4,
        retry_backoff_ms: 10,
        ..Default::default()
    };
    config
}
fn service(f: &Fixture, config: &MineruConfig) -> KnowledgeBaseService {
    KnowledgeBaseService::new(
        SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap(),
        f.state.rag.with_runtime(tron_backend::application::rag::DefaultRagRuntime::new(
            LlmRuntimeService::new(Arc::new(EmbeddingResolver), Arc::new(f.client.clone())), FixtureGraphRouter,
        )),
        f.state.resources.clone(),
        f.state.agent_management.clone(),
        LlmRuntimeService::new(Arc::new(EmbeddingResolver), Arc::new(f.client.clone())),
    )
    .with_document_parser(Arc::new(MineruKnowledgeDocumentParser::new(config.clone())))
    .with_ingestion_config(config.ingestion.clone())
}
async fn until(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(8), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn two_nodes_keep_file_job_download_and_cancel_affinity() {
    let (a, na, sa) = node("node_a").await;
    let (b, nb, sb) = node("node_b").await;
    let f = Fixture::new();
    let cfg = config(vec![na, nb]);
    let service = service(&f, &cfg);
    let kb = service.create(USER, "Nodes", "").unwrap();
    let one = service
        .upload_and_start_index(USER, &kb.id, "one.pdf", &pdf())
        .unwrap();
    let two = service
        .upload_and_start_index(USER, &kb.id, "two.pdf", &pdf())
        .unwrap();
    until(|| a.posts.load(Ordering::SeqCst) == 1 && b.posts.load(Ordering::SeqCst) == 1).await;
    let tasks: Vec<_> = [&one, &two]
        .iter()
        .map(|d| {
            service
                .ingestion_tasks(USER, &kb.id, &d.id)
                .unwrap()
                .remove(0)
        })
        .collect();
    assert_ne!(tasks[0].endpoint_id, tasks[1].endpoint_id);
    let on_a = tasks
        .iter()
        .find(|t| t.endpoint_id.as_deref() == Some("node_a"))
        .unwrap();
    service
        .cancel_index(USER, &kb.id, &on_a.document_id)
        .await
        .unwrap();
    assert!(a.cancels.load(Ordering::SeqCst) >= 1);
    assert_eq!(b.cancels.load(Ordering::SeqCst), 0);
    b.complete.store(true, Ordering::SeqCst);
    until(|| {
        service
            .detail(USER, &kb.id)
            .unwrap()
            .documents
            .iter()
            .any(|d| d.status == "READY")
    })
    .await;
    assert_eq!(a.uploads.load(Ordering::SeqCst), 1);
    assert_eq!(b.uploads.load(Ordering::SeqCst), 1);
    service.stop_ingestion_workers();
    sa.abort();
    sb.abort();
}
#[tokio::test]
async fn restart_resumes_recorded_remote_job_on_same_node_without_upload_post_or_recent_list_scan()
{
    let (a, na, sa) = node("node_a").await;
    let (b, nb, sb) = node("node_b").await;
    let f = Fixture::new();
    let cfg = config(vec![na, nb]);
    *a.expected_ocr.lock().unwrap() = "ocr".into();
    let first = service(&f, &cfg);
    let kb = first.create(USER, "Restart affinity", "").unwrap();
    let doc = first.upload(USER, &kb.id, "one.pdf", &pdf()).unwrap();
    first
        .configure_parsing(USER, &kb.id, &doc.id, None, None, Some(Some(OcrMode::Ocr)))
        .await
        .unwrap();
    first.start_index(USER, &kb.id, &doc.id).unwrap();
    until(|| a.polls.load(Ordering::SeqCst) > 0).await;
    let before = first
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    assert!(before.remote_job_id.is_some());
    first.stop_ingestion_workers();
    a.complete.store(true, Ordering::SeqCst);
    let mut changed = cfg.clone();
    changed.ocr_mode = "txt".into();
    let resumed = service(&f, &changed);
    resumed.start_ingestion_workers().unwrap();
    until(|| resumed.detail(USER, &kb.id).unwrap().documents[0].status == "READY").await;
    let after = resumed
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap().into_iter().find(|task| task.id == before.id).unwrap();
    assert_eq!(after.id, before.id);
    assert_eq!(after.endpoint_id, before.endpoint_id);
    assert_eq!(after.remote_job_id, before.remote_job_id);
    assert_eq!(a.uploads.load(Ordering::SeqCst), 1);
    assert_eq!(a.posts.load(Ordering::SeqCst), 1);
    assert_eq!(a.lists.load(Ordering::SeqCst), 0);
    assert_eq!(b.uploads.load(Ordering::SeqCst), 0);
    assert_eq!(b.posts.load(Ordering::SeqCst), 0);
    resumed.stop_ingestion_workers();
    sa.abort();
    sb.abort();
}
#[tokio::test]
async fn lost_job_post_response_reconciles_original_file_and_does_not_submit_duplicate() {
    let (a, na, sa) = node("node_a").await;
    a.lose_post_response.store(true, Ordering::SeqCst);
    a.complete.store(true, Ordering::SeqCst);
    let f = Fixture::new();
    let cfg = config(vec![na]);
    let service = service(&f, &cfg);
    let kb = service.create(USER, "Unknown POST", "").unwrap();
    let doc = service
        .upload_and_start_index(USER, &kb.id, "one.pdf", &pdf())
        .unwrap();
    until(|| service.detail(USER, &kb.id).unwrap().documents[0].status == "READY").await;
    let task = service
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap().into_iter().find(|task| task.kind == tron_backend::application::knowledge_base::ingestion::IngestionKind::Parsing).unwrap();
    assert!(!task.reconciling);
    assert!(task.remote_job_id.is_some());
    assert_eq!(a.uploads.load(Ordering::SeqCst), 1);
    assert_eq!(a.posts.load(Ordering::SeqCst), 1);
    assert!(a.lists.load(Ordering::SeqCst) > 0);
    service.stop_ingestion_workers();
    sa.abort();
}
#[tokio::test]
async fn unhealthy_nodes_are_excluded_and_unauthorized_remote_nodes_are_rejected() {
    let (a, na, sa) = node("node_a").await;
    let (b, nb, sb) = node("node_b").await;
    a.healthy.store(false, Ordering::SeqCst);
    b.complete.store(true, Ordering::SeqCst);
    let f = Fixture::new();
    let cfg = config(vec![na, nb]);
    let service = service(&f, &cfg);
    let kb = service.create(USER, "Health", "").unwrap();
    service
        .upload_and_start_index(USER, &kb.id, "one.pdf", &pdf())
        .unwrap();
    until(|| service.detail(USER, &kb.id).unwrap().documents[0].status == "READY").await;
    assert_eq!(a.uploads.load(Ordering::SeqCst), 0);
    assert_eq!(b.posts.load(Ordering::SeqCst), 1);
    let mut cfg = config(vec![MineruNode {
        id: "remote".into(),
        base_url: "http://192.0.2.1:8000".into(),
        enabled: true,
        ..Default::default()
    }]);
    let parser = MineruKnowledgeDocumentParser::new(cfg.clone());
    assert!(parser.healthy_endpoints().await.is_err());
    cfg.nodes[0].enabled = false;
    assert!(
        MineruKnowledgeDocumentParser::new(cfg)
            .healthy_endpoints()
            .await
            .unwrap()
            .is_empty()
    );
    service.stop_ingestion_workers();
    sa.abort();
    sb.abort();
}

#[tokio::test]
async fn malformed_successful_job_post_is_reconciled_without_duplicate_submission() {
    let (a, na, server) = node("node_a").await;
    a.malformed_post_response.store(true, Ordering::SeqCst);
    a.complete.store(true, Ordering::SeqCst);
    let f = Fixture::new();
    let cfg = config(vec![na]);
    let service = service(&f, &cfg);
    let kb = service.create(USER, "Broken response", "").unwrap();
    service
        .upload_and_start_index(USER, &kb.id, "one.pdf", &pdf())
        .unwrap();
    until(|| service.detail(USER, &kb.id).unwrap().documents[0].status == "READY").await;
    assert_eq!(a.posts.load(Ordering::SeqCst), 1);
    assert_eq!(a.uploads.load(Ordering::SeqCst), 1);
    assert!(a.lists.load(Ordering::SeqCst) > 0);
    service.stop_ingestion_workers();
    server.abort();
}

#[tokio::test]
async fn ambiguous_post_cancellation_recovers_after_restart_even_when_node_is_disabled() {
    let (a, na, server) = node("node_a").await;
    a.lose_post_response.store(true, Ordering::SeqCst);
    a.cancel_unavailable.store(true, Ordering::SeqCst);
    let f = Fixture::new();
    let mut cfg = config(vec![na]);
    cfg.ingestion.retry_backoff_ms = 10_000;
    let first = service(&f, &cfg);
    let kb = first.create(USER, "Durable cancel", "").unwrap();
    let doc = first
        .upload_and_start_index(USER, &kb.id, "one.pdf", &pdf())
        .unwrap();
    until(|| first.ingestion_tasks(USER, &kb.id, &doc.id).unwrap()[0].reconciling).await;
    assert_eq!(
        first
            .cancel_index(USER, &kb.id, &doc.id)
            .await
            .unwrap()
            .status,
        "CANCELLED"
    );
    let cancelled = first
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    assert!(cancelled.cancellation_pending);
    assert!(cancelled.remote_job_id.is_none());
    first.stop_ingestion_workers();
    let mut disabled = cfg.clone();
    disabled.nodes[0].enabled = false;
    disabled.ocr_mode = "txt".into();
    let resumed = service(&f, &disabled);
    resumed.start_ingestion_workers().unwrap();
    // Retry remains queued until the original upstream has acknowledged cancel.
    assert_eq!(
        resumed.retry_index(USER, &kb.id, &doc.id).unwrap().status,
        "QUEUED"
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(a.posts.load(Ordering::SeqCst), 1);
    a.cancel_unavailable.store(false, Ordering::SeqCst);
    until(|| !resumed.ingestion_tasks(USER, &kb.id, &doc.id).unwrap()[0].cancellation_pending)
        .await;
    let confirmed = resumed
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    assert_eq!(confirmed.id, cancelled.id);
    assert!(confirmed.remote_job_id.is_some());
    assert_eq!(confirmed.endpoint_id, cancelled.endpoint_id);
    resumed.stop_ingestion_workers();
    a.complete.store(true, Ordering::SeqCst);
    let enabled = service(&f, &cfg);
    enabled.start_ingestion_workers().unwrap();
    until(|| enabled.detail(USER, &kb.id).unwrap().documents[0].status == "READY").await;
    assert_eq!(a.posts.load(Ordering::SeqCst), 2);
    assert_eq!(a.uploads.load(Ordering::SeqCst), 1);
    enabled.stop_ingestion_workers();
    server.abort();
}

#[tokio::test]
async fn document_ocr_override_changes_request_cache_and_task_snapshot() {
    let (node, endpoint, server) = node("ocr_modes").await;
    node.complete.store(true, Ordering::SeqCst);
    let fixture = Fixture::new();
    let config = config(vec![endpoint]);
    let service = service(&fixture, &config);
    let kb = service.create(USER, "OCR modes", "").unwrap();
    let doc = service.upload(USER, &kb.id, "modes.pdf", &pdf()).unwrap();
    assert_eq!(
        service
            .parsing_capabilities(USER)
            .await
            .unwrap()
            .default_ocr_mode,
        OcrMode::Ocr
    );
    *node.expected_ocr.lock().unwrap() = "txt".into();
    let selected = service
        .configure_parsing(USER, &kb.id, &doc.id, None, None, Some(Some(OcrMode::Text)))
        .await
        .unwrap();
    assert_eq!(selected.ocr_mode, Some(OcrMode::Text));
    service.preview(USER, &kb.id, &doc.id).await.unwrap();
    let ready = service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(ready.status, "READY");
    assert_eq!(ready.ocr_mode, Some(OcrMode::Text));
    assert_eq!(
        node.posts.load(Ordering::SeqCst),
        1,
        "same-mode preview and indexing reuse the cache"
    );
    let text_signature = ready.parsing_artifact.as_ref().unwrap().signature.clone();
    *node.expected_ocr.lock().unwrap() = "ocr".into();
    let forced = service
        .configure_and_index(USER, &kb.id, &doc.id, None, None, Some(Some(OcrMode::Ocr)))
        .await
        .unwrap();
    assert_eq!(forced.status, "READY");
    assert_eq!(
        node.posts.load(Ordering::SeqCst),
        2,
        "changing OCR must reparse"
    );
    assert_ne!(
        forced.parsing_artifact.as_ref().unwrap().signature,
        text_signature
    );
    let tasks = service.ingestion_tasks(USER, &kb.id, &doc.id).unwrap();
    assert!(
        tasks
            .iter()
            .any(|task| task.configuration["ocrMode"] == "Ocr"
                && task.configuration["parser"]["ocrMode"] == "ocr")
    );
    let inherited = service
        .configure_parsing(USER, &kb.id, &doc.id, None, None, Some(None))
        .await
        .unwrap();
    assert_eq!(inherited.ocr_mode, Some(OcrMode::Text));
    *node.expected_ocr.lock().unwrap() = "txt".into();
    let automatic = service
        .configure_and_index(USER, &kb.id, &doc.id, None, None, Some(Some(OcrMode::Auto)))
        .await
        .unwrap();
    assert_eq!(automatic.status, "READY");
    assert_eq!(automatic.ocr_mode, Some(OcrMode::Text));
    assert_eq!(node.posts.load(Ordering::SeqCst), 3);
    assert_eq!(
        node.lists.load(Ordering::SeqCst),
        0,
        "upstream history cannot prove the requested OCR mode"
    );
    let markdown = service
        .upload(USER, &kb.id, "readme.md", b"readme")
        .unwrap();
    assert!(
        service
            .configure_parsing(
                USER,
                &kb.id,
                &markdown.id,
                None,
                None,
                Some(Some(OcrMode::Ocr))
            )
            .await
            .is_err()
    );
    service.stop_ingestion_workers();
    server.abort();
}
