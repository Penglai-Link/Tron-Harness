use axum::{
    Json, Router,
    routing::{get, post},
};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::mineru_documents::MineruKnowledgeDocumentParser,
    application::knowledge_base::parsing::KnowledgeDocumentParser, config::MineruConfig,
};

async fn remote_failure(reason: Option<String>) -> Value {
    let router = Router::new()
        .route("/v1/health", get(|| async { Json(json!({"status":"ok","version":"4.0.10"})) }))
        .route("/v1/tiers", get(|| async { Json(json!({"data":[{"id":"standard"}]})) }))
        .route("/v1/uploads", post(|| async { Json(json!({"status":"completed","file":{"id":"file-real"}})) }))
        .route("/v1/parse/jobs", post(|| async { Json(json!({"job_id":"job-real","status":"running"})) }))
        .route("/v1/parse/jobs/job-real", get(move || {
            let reason = reason.clone();
            async move { Json(json!({"job_id":"job-real","status":"failed", "files":[{"error":{"code":"PARSE_FAILED","message":reason}}]})) }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let parser = MineruKnowledgeDocumentParser::new(MineruConfig {
        base_url: format!("http://{address}"),
        ..Default::default()
    });
    let error = parser
        .parse("fixture.pdf", b"synthetic wire input", None)
        .await
        .unwrap_err()
        .to_string();
    server.abort();
    serde_json::from_str(error.split_once("MINERU_PARSE_FAILED:").unwrap().1).unwrap()
}

#[tokio::test]
async fn failed_parse_preserves_remote_model_diagnostic_and_job_id() {
    let reason = "[ONNXRuntimeError] INVALID_PROTOBUF: OCR model could not be loaded";
    let detail = remote_failure(Some(reason.into())).await;
    assert_eq!(detail["jobId"], "job-real");
    assert_eq!(detail["reason"], reason);
}

#[tokio::test]
async fn failed_parse_bounds_and_normalizes_remote_diagnostic() {
    let detail = remote_failure(Some(format!("  Model\nfailed\t{}", "字".repeat(2000)))).await;
    let reason = detail["reason"].as_str().unwrap();
    assert!(reason.starts_with("Model failed "));
    assert_eq!(reason.chars().count(), 1000);
}

#[tokio::test]
async fn failed_parse_allows_a_missing_remote_diagnostic() {
    let detail = remote_failure(None).await;
    assert_eq!(detail["jobId"], "job-real");
    assert!(detail["reason"].is_null());
}
