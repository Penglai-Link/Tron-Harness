mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use eliteforge_agent_core::*;
use fixture::*;
use std::sync::atomic::Ordering;
use tron_backend::application::{
    document::{ChunkConfig, ChunkMode, chunk_document_input, parse_local_document},
    knowledge_quality::*,
    rag::RagSettings,
};

async fn ready() -> (
    Fixture,
    String,
    String,
    KnowledgeQualityService,
    Vec<RetrievalEvaluationCase>,
) {
    ready_with_vector(false).await
}
async fn ready_with_vector(
    vector: bool,
) -> (
    Fixture,
    String,
    String,
    KnowledgeQualityService,
    Vec<RetrievalEvaluationCase>,
) {
    let f = Fixture::new();
    let kb = f
        .service
        .create_with_index_settings(
            USER,
            "Quality",
            "",
            RagSettings {
                vector,
                graph: false,
                rerank_model_code: None,
                ..Default::default()
            },
        )
        .unwrap();
    let body=(0..8).map(|i|format!("# Topic {i}\n\nCondition {i} has its own specific requirements and exceptions. {}\n\n",format!("orchid{i} unique evidence with applicable conditions and exceptions. ").repeat(20))).collect::<String>();
    let doc = f
        .service
        .upload(USER, &kb.id, "guide.md", body.as_bytes())
        .unwrap();
    f.service
        .configure(
            USER,
            &kb.id,
            &doc.id,
            Some(ChunkConfig {
                mode: ChunkMode::Fixed,
                max_chars: 200,
                overlap_chars: 0,
                heading_depth: 3,
            }),
        )
        .unwrap();
    let doc = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(doc.status, "READY", "{:?}", doc.error);
    let input = chunk_document_input(
        &parse_local_document("guide.md", body.as_bytes()).unwrap(),
        &doc.id,
        "guide.md",
        6,
    );
    let cases = input
        .blocks
        .iter()
        .filter(|b| b.kind != ChunkBlockKind::Heading)
        .enumerate()
        .map(|(i, b)| RetrievalEvaluationCase {
            id: format!("case-{i}"),
            query: format!("orchid{i}"),
            expected: vec![RetrievalEvidenceTarget {
                document_id: doc.id.clone(),
                span: ChunkSourceSpan {
                    block_id: b.id.clone(),
                    start: 0,
                    end: b.text.chars().count(),
                },
            }],
            partition: if i % 2 == 0 {
                EvaluationPartition::Training
            } else {
                EvaluationPartition::Holdout
            },
            provenance: EvaluationProvenance::HumanLabeled,
        })
        .collect::<Vec<_>>();
    assert_eq!(cases.len(), 8);
    let quality = KnowledgeQualityService::new(f.service.clone());
    let (fp, _) = quality.source_excerpts(USER, &kb.id, 0, 12).await.unwrap();
    quality
        .save_dataset(USER, &kb.id, &fp, cases.clone())
        .await
        .unwrap();
    (f, kb.id, doc.id, quality, cases)
}
async fn finished(q: &KnowledgeQualityService, kb: &str, id: &str) -> QualityOptimizationRun {
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let run = q.run(USER, kb, id).unwrap();
            if !matches!(
                run.status.as_str(),
                "QUEUED" | "RUNNING" | "PUBLISHING" | "CANCEL_REQUESTED"
            ) {
                break run;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
fn options() -> QualityOptimizationOptions {
    QualityOptimizationOptions {
        top_k: 1,
        max_trials: 1,
        candidates: vec![ChunkConfig {
            mode: ChunkMode::Auto,
            max_chars: 1600,
            overlap_chars: 120,
            heading_depth: 3,
        }],
        ..Default::default()
    }
}

#[tokio::test]
async fn publishes_tested_improvement_and_removes_old_indexes_without_model_or_parser_calls() {
    let (f, kb, doc, q, _) = ready().await;
    let before = f.service.detail(USER, &kb).unwrap().documents.remove(0);
    let calls = f.client.calls.load(Ordering::SeqCst);
    let queued = q.start(USER, &kb, "auto-publish", options()).await.unwrap();
    let duplicate = q.start(USER, &kb, "auto-publish", options()).await.unwrap();
    assert_eq!(queued.id, duplicate.id);
    let run = finished(&q, &kb, &queued.id).await;
    assert_eq!(run.status, "PUBLISHED", "{:?}", run.error);
    assert!(run.published);
    assert!(run.target_reached);
    assert!(run.baseline_holdout.unwrap().recall < 0.5);
    assert_eq!(run.trials[0].holdout.as_ref().unwrap().recall, 1.0);
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
    let after = f.service.detail(USER, &kb).unwrap().documents.remove(0);
    assert_ne!(before.active_source_code, after.active_source_code);
    assert_eq!(after.chunk_config.as_ref().unwrap().mode, ChunkMode::Auto);
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    let old: usize = conn
        .query_row(
            &format!("SELECT count(*) FROM rag_{kb}_units WHERE source_code=?1"),
            [before.active_source_code.unwrap()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(old, 0);
    let contexts:usize=conn.query_row("SELECT count(*) FROM knowledge_chunk_contexts WHERE document_id=?1 AND source_code<>?2",rusqlite::params![doc,after.active_source_code],|r|r.get(0)).unwrap();
    assert_eq!(contexts, 0);
    let tasks = f.service.ingestion_tasks(USER, &kb, &doc).unwrap();
    assert!(
        tasks.len() > 1,
        "audit history survives retirement of index versions"
    );
    for task in tasks
        .iter()
        .filter(|task| Some(&task.id) != after.task_id.as_ref())
    {
        assert!(task.parsed_resource_code.is_none());
        assert!(task.chunks_resource_code.is_none());
        assert!(task.embedding_resource_code.is_none());
    }
    let persisted = KnowledgeQualityService::new(f.service.clone())
        .run(USER, &kb, &queued.id)
        .unwrap();
    assert!(persisted.published);
    assert!(q.run("OTHER-USER", &kb, &queued.id).is_err());
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let response = common::authenticated_request(
        &app,
        axum::http::Method::GET,
        &format!("/api/v1/knowledge-bases/{kb}/quality/runs/{}", queued.id),
        None,
    )
    .await;
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let json = common::response_json(response).await;
    assert_eq!(json["data"]["run"]["status"], "Published");
    assert_eq!(json["data"]["run"]["cases"].as_array().unwrap().len(), 8);
    assert!(json["data"]["run"]["createdAt"].is_number());
}
#[tokio::test]
async fn no_improvement_preserves_current_generation_and_unknown_strategy_rejected_before_work() {
    let (f, kb, _, q, _) = ready().await;
    let before = f.service.detail(USER, &kb).unwrap().documents[0]
        .active_source_code
        .clone();
    let mut o = options();
    o.candidates[0] = ChunkConfig {
        mode: ChunkMode::Fixed,
        max_chars: 200,
        overlap_chars: 0,
        heading_depth: 3,
    };
    let queued = q.start(USER, &kb, "no-improvement", o).await.unwrap();
    let run = finished(&q, &kb, &queued.id).await;
    assert_eq!(run.status, "COMPLETED", "{:?}", run.error);
    assert!(!run.published);
    assert_eq!(
        f.service.detail(USER, &kb).unwrap().documents[0].active_source_code,
        before
    );
    let doc = &f.service.detail(USER, &kb).unwrap().documents[0];
    assert!(
        f.service
            .configure(
                USER,
                &kb,
                &doc.id,
                Some(ChunkConfig {
                    mode: ChunkMode::Custom("unknown".into()),
                    ..Default::default()
                })
            )
            .is_err()
    );
}
#[tokio::test]
async fn dataset_rejects_leaked_holdout_stale_and_foreign_source() {
    let (_f, kb, _, q, mut cases) = ready().await;
    let (fp, _) = q.source_excerpts(USER, &kb, 0, 12).await.unwrap();
    assert!(
        q.save_dataset(USER, &kb, "stale", cases.clone())
            .await
            .is_err()
    );
    cases[1].expected = cases[0].expected.clone();
    assert!(q.save_dataset(USER, &kb, &fp, cases.clone()).await.is_err());
    cases[1].expected[0].document_id = "another-library".into();
    assert!(q.save_dataset(USER, &kb, &fp, cases).await.is_err());
    assert!(
        q.start("OTHER-USER", &kb, "foreign", options())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn quality_http_uses_owned_session_and_numeric_timestamps() {
    use axum::http::{Method, StatusCode};
    let f = Fixture::new();
    let kb = f.service.create(USER, "Quality chat", "").unwrap();
    let foreign = f.service.create("OTHER-USER", "Private", "").unwrap();
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let response = common::authenticated_request(
        &app,
        Method::GET,
        "/api/v1/knowledge-bases/chunking-strategies",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = common::response_json(response).await;
    assert_eq!(data["code"], "0");
    assert_eq!(data["data"]["strategies"].as_array().unwrap().len(), 4);
    let uri = format!("/api/v1/knowledge-bases/{}/quality/session", kb.id);
    let response =
        common::authenticated_request(&app, Method::POST, &uri, Some(serde_json::json!({}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let first = common::response_json(response).await;
    assert!(first["data"]["session"]["createdAt"].is_number());
    assert!(first["data"]["session"]["id"].is_string());
    let response =
        common::authenticated_request(&app, Method::POST, &uri, Some(serde_json::json!({}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let second = common::response_json(response).await;
    assert_eq!(
        first["data"]["session"]["sessionCode"],
        second["data"]["session"]["sessionCode"]
    );
    let response = common::authenticated_request(
        &app,
        Method::POST,
        &format!("/api/v1/knowledge-bases/{}/quality/session", foreign.id),
        Some(serde_json::json!({})),
    )
    .await;
    assert!(matches!(
        response.status(),
        StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
    ));
    let response = common::authenticated_request(
        &app,
        Method::GET,
        &format!("/api/v1/knowledge-bases/{}/quality/dataset", kb.id),
        None,
    )
    .await;
    assert_eq!(
        common::response_json(response).await["data"],
        serde_json::json!({"dataset":null})
    );
    assert_eq!(f.client.calls.load(Ordering::SeqCst), 0);
    let response = common::authenticated_request(
        &app,
        Method::POST,
        &format!("/api/v1/knowledge-bases/{}/quality/runs", kb.id),
        Some(serde_json::json!({"requestId":"invalid","options":"invalid"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(common::response_json(response).await["code"], "101027005");
}

#[tokio::test]
async fn provider_failure_never_becomes_zero_recall_or_publishes() {
    let (f, kb, _, q, _) = ready_with_vector(true).await;
    let before = f.service.detail(USER, &kb).unwrap().documents[0]
        .active_source_code
        .clone();
    f.client.failing.store(true, Ordering::SeqCst);
    let queued = q
        .start(USER, &kb, "provider-failure", options())
        .await
        .unwrap();
    let run = finished(&q, &kb, &queued.id).await;
    assert_eq!(run.status, "FAILED");
    assert!(run.baseline_holdout.is_none());
    assert!(!run.published);
    assert_eq!(
        f.service.detail(USER, &kb).unwrap().documents[0].active_source_code,
        before
    );
}

struct Questions {
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    blocking: bool,
}
#[async_trait::async_trait]
impl QualityQuestionGenerator for Questions {
    async fn generate(&self, prompt: String) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.blocking {
            std::future::pending::<()>().await;
        }
        let batch: Vec<serde_json::Value> =
            serde_json::from_str(prompt.rsplit_once("数据：").unwrap().1).unwrap();
        Ok(serde_json::to_string(&batch.iter().map(|b|{let text=b["text"].as_str().unwrap();let quote=text.chars().take(50).collect::<String>();serde_json::json!({"id":b["id"],"question":format!("What are the requirements: {quote}?"),"evidenceQuote":quote})}).collect::<Vec<_>>()).unwrap())
    }
}
fn remove_dataset(f: &Fixture, kb: &str) {
    rusqlite::Connection::open(&f.state.config.database_url)
        .unwrap()
        .execute(
            "DELETE FROM knowledge_evaluation_datasets WHERE knowledge_base_id=?1",
            [kb],
        )
        .unwrap();
}
#[tokio::test]
async fn cancellation_and_budget_stop_model_work_without_replacing_index() {
    let (f, kb, _, _, _) = ready().await;
    remove_dataset(&f, &kb);
    let before = f.service.detail(USER, &kb).unwrap().documents[0]
        .active_source_code
        .clone();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let q = KnowledgeQualityService::new(f.service.clone().with_quality_question_generator(
        std::sync::Arc::new(Questions {
            calls: calls.clone(),
            blocking: true,
        }),
    ));
    let queued = q.start(USER, &kb, "cancel", options()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(q.start(USER, &kb, "concurrent", options()).await.is_err());
    q.cancel(USER, &kb, &queued.id).unwrap();
    let run = finished(&q, &kb, &queued.id).await;
    assert_eq!(run.status, "CANCELLED");
    assert!(!run.published);
    let q = KnowledgeQualityService::new(f.service.clone().with_quality_question_generator(
        std::sync::Arc::new(Questions {
            calls: calls.clone(),
            blocking: false,
        }),
    ));
    let mut o = options();
    o.max_cases = 8;
    o.budget.max_model_operations = 1;
    let queued = q.start(USER, &kb, "budget", o).await.unwrap();
    let run = finished(&q, &kb, &queued.id).await;
    assert_eq!(run.status, "BUDGET_LIMIT", "{:?}", run.error);
    assert_eq!(run.model_operations, 1);
    assert!(!run.published);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        f.service.detail(USER, &kb).unwrap().documents[0].active_source_code,
        before
    );
}

#[tokio::test]
async fn changed_document_cannot_be_overwritten_by_evaluation() {
    let (f, kb, doc, q, _) = ready_with_vector(true).await;
    let before = f.service.detail(USER, &kb).unwrap().documents[0]
        .active_source_code
        .clone();
    f.client.blocking.store(true, Ordering::SeqCst);
    let queued = q.start(USER, &kb, "stale", options()).await.unwrap();
    f.service
        .configure(
            USER,
            &kb,
            &doc,
            Some(ChunkConfig {
                mode: ChunkMode::Heading,
                ..Default::default()
            }),
        )
        .unwrap();
    f.client.blocking.store(false, Ordering::SeqCst);
    let run = finished(&q, &kb, &queued.id).await;
    assert_eq!(run.status, "STALE", "{:?}", run.error);
    assert!(!run.published);
    assert_eq!(
        f.service.detail(USER, &kb).unwrap().documents[0].active_source_code,
        before
    );
}

#[tokio::test]
async fn equivalent_candidates_reuse_vectors_and_publication_does_not_embed_again() {
    let (f, kb, _, q, _) = ready_with_vector(true).await;
    let before = f.client.calls.load(Ordering::SeqCst);
    let mut o = options();
    o.max_trials = 2;
    o.candidates.push(ChunkConfig {
        mode: ChunkMode::Structure,
        ..o.candidates[0].clone()
    });
    let queued = q.start(USER, &kb, "reuse-vectors", o).await.unwrap();
    let run = finished(&q, &kb, &queued.id).await;
    assert_eq!(run.status, "PUBLISHED", "{:?}", run.error);
    assert_eq!(run.trials.len(), 2);
    let calls = f.client.calls.load(Ordering::SeqCst) - before;
    assert_eq!(
        calls, 9,
        "eight cached question vectors plus one unique candidate batch; no publication embeddings"
    );
    assert_eq!(run.model_operations, calls);
}

#[tokio::test]
async fn interrupted_runs_are_recovered_without_repeating_provider_work() {
    use tron_backend::adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository;
    let (f, kb, _, q, _) = ready().await;
    let queued = q.start(USER, &kb, "completed", options()).await.unwrap();
    let _ = finished(&q, &kb, &queued.id).await;
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    conn.execute("UPDATE knowledge_quality_runs SET state='RUNNING',payload_json=json_set(payload_json,'$.status','RUNNING','$.published',json('false')) WHERE id=?1",[&queued.id]).unwrap();
    let calls = f.client.calls.load(Ordering::SeqCst);
    SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url)
        .unwrap()
        .recover_quality_runs()
        .unwrap();
    assert_eq!(q.run(USER, &kb, &queued.id).unwrap().status, "INTERRUPTED");
    assert_eq!(f.client.calls.load(Ordering::SeqCst), calls);
}

struct Control;
impl tron_backend::domain::agent::ToolExecutionControl for Control {
    fn check_active(&self) -> Result<(), tron_backend::domain::agent::ToolError> {
        Ok(())
    }
    fn remaining(&self) -> std::time::Duration {
        std::time::Duration::from_secs(10)
    }
    fn cancelled(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(std::future::pending())
    }
}
#[tokio::test]
async fn knowledge_chat_tools_enforce_server_bound_library_and_identity() {
    use serde_json::json;
    let (f, kb, _, q, _) = ready().await;
    let other = f.service.create(USER, "Other owned library", "").unwrap();
    let session = f
        .state
        .chats
        .ensure_knowledge_quality_session(USER, &kb, "Quality")
        .unwrap();
    let context:tron_backend::domain::agent::AgentRunContext=serde_json::from_value(json!({
        "agentCode":f.agent,"conversationCode":session.session_code,"runCode":"quality-tool-test","workspaceCode":"PLATFORM","userCode":USER,"locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"fixture-chat","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture"},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":2,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[],"observations":[],"startedAt":"fixture"
    })).unwrap();
    let tool = tron_backend::application::agent::knowledge_quality_tools(q, f.state.chats.clone())
        .into_iter()
        .find(|t| t.name() == "rag_quality_cases")
        .unwrap();
    let request = tron_backend::domain::agent::ToolRequest {
        tool_name: tool.name().into(),
        invocation_id: None,
        input: json!({}),
    };
    let result = tool
        .exec_async(request.clone(), &context, &Control)
        .unwrap()
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result.observation.output).unwrap()["libraryId"],
        kb
    );
    let mut request = request;
    request.input = json!({"libraryId":other.id,"userCode":USER});
    assert!(
        tool.exec_async(request.clone(), &context, &Control)
            .unwrap()
            .await
            .is_err()
    );
    let mut forged = context;
    forged.user_code = Some("OTHER-USER".into());
    assert!(
        tool.exec_async(request, &forged, &Control)
            .unwrap()
            .await
            .is_err()
    );
}

#[tokio::test]
async fn knowledge_quality_tools_read_sources_and_manage_a_run() {
    use serde_json::{Value, json};
    use tron_backend::domain::agent::{AgentRunContext, ToolRequest};

    let (f, kb, _, quality, _) = ready().await;
    let session = f.state.chats.ensure_knowledge_quality_session(USER, &kb, "Quality").unwrap();
    let context: AgentRunContext = serde_json::from_value(json!({
        "agentCode":f.agent,"conversationCode":session.session_code,"runCode":"quality-lifecycle","workspaceCode":"PLATFORM","userCode":USER,"locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"fixture-chat","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture"},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":2,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[],"observations":[],"startedAt":"fixture"
    })).unwrap();
    let tools = tron_backend::application::agent::knowledge_quality_tools(quality.clone(), f.state.chats.clone());
    async fn call(tools: &[std::sync::Arc<dyn tron_backend::domain::agent::Tool>], context: &AgentRunContext, name: &str, input: Value) -> Value {
        let tool = tools.iter().find(|tool| tool.name() == name).unwrap();
        let result = tool.exec_async(ToolRequest { tool_name: name.into(), invocation_id: None, input }, context, &Control).unwrap().await.unwrap();
        serde_json::from_str(&result.observation.output).unwrap()
    }
    let sources = call(&tools, &context, "rag_quality_sources", json!({})).await;
    assert!(sources["sourceFingerprint"].as_str().is_some());
    assert!(!sources["records"].as_array().unwrap().is_empty());
    let cases = call(&tools, &context, "rag_quality_cases", json!({})).await;
    assert_eq!(cases["cases"].as_array().unwrap().len(), 8);
    let run = call(&tools, &context, "rag_quality_start", json!({"requestId":"quality-tool-lifecycle","options":options()})).await;
    let run_id = run["id"].as_str().unwrap();
    let status = call(&tools, &context, "rag_quality_status", json!({"runId":run_id})).await;
    assert_eq!(status["id"], run_id);
    let cancelled = call(&tools, &context, "rag_quality_cancel", json!({"runId":run_id})).await;
    assert_eq!(cancelled["id"], run_id);
    let _ = finished(&quality, &kb, run_id).await;
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn knowledge_namespace_allows_history_but_rejects_forged_headers_and_path_overrides() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let f = Fixture::new();
    let kb = f.service.create(USER, "Chat", "").unwrap();
    let other = f.service.create(USER, "Other", "").unwrap();
    let session = f
        .state
        .chats
        .ensure_knowledge_quality_session(USER, &kb.id, "Chat")
        .unwrap();
    let other_session = f
        .state
        .chats
        .ensure_knowledge_quality_session(USER, &other.id, "Other")
        .unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let paths = [
        (
            "POST",
            format!(
                "/api/v1/chats/sessions/{}/history/query",
                session.session_code
            ),
            serde_json::json!({"size":20,"cursor":null}),
        ),
        (
            "GET",
            format!("/api/v1/chats/sessions/{}/messages", session.session_code),
            serde_json::Value::Null,
        ),
        (
            "GET",
            format!(
                "/api/v1/chats/sessions/get-active-run?sessionCode={}",
                session.session_code
            ),
            serde_json::Value::Null,
        ),
        (
            "GET",
            format!(
                "/api/v1/agents/agui/streams/{}/protocol-events",
                session.session_code
            ),
            serde_json::Value::Null,
        ),
    ];
    for (method, path, body) in paths {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", "Bearer profile-test-token")
            .header("X-Knowledge-Base-Code", &kb.id)
            .header("X-Workspace-Code", "PLATFORM");
        let body = if body.is_null() {
            Body::empty()
        } else {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        };
        let response = app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{}",
            common::response_json(response).await
        );
    }
    for (header, path) in [
        (
            other.id.clone(),
            format!("/api/v1/chats/sessions/{}/messages", session.session_code),
        ),
        (
            kb.id.clone(),
            format!(
                "/api/v1/chats/sessions/{}/messages?sessionCode={}",
                other_session.session_code, session.session_code
            ),
        ),
    ] {
        let request = Request::builder()
            .uri(path)
            .header("authorization", "Bearer profile-test-token")
            .header("X-Knowledge-Base-Code", header)
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
    }
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agents/runs")
        .header("authorization", "Bearer profile-test-token")
        .header("X-Knowledge-Base-Code", &kb.id)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"conversationCode":session.session_code}).to_string(),
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
}

struct AuthQuestions(std::sync::Arc<std::sync::Mutex<Option<(String, String)>>>);
#[async_trait::async_trait]
impl QualityQuestionGenerator for AuthQuestions {
    async fn generate(&self, _: String) -> Result<String, String> {
        *self.0.lock().unwrap() = tron_backend::shared::auth_holder::AuthHolder::get()
            .map(|context| (context.user_code().to_owned(), context.locale.clone()));
        Err("fixture provider stopped after recording identity".into())
    }
}
#[tokio::test]
async fn background_evaluation_keeps_the_request_user_and_locale() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let (f, kb, _, _, _) = ready().await;
    remove_dataset(&f, &kb);
    let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
    let service = f
        .service
        .clone()
        .with_quality_question_generator(std::sync::Arc::new(AuthQuestions(observed.clone())));
    let q = KnowledgeQualityService::new(service.clone());
    let mut state = f.state.clone();
    state.knowledge_bases = service;
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let body = serde_json::json!({"requestId":"identity","options":options()});
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/knowledge-bases/{kb}/quality/runs"))
        .header("authorization", "Bearer profile-test-token")
        .header("content-type", "application/json")
        .header("accept-language", "en-US")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let json = common::response_json(response).await;
    let id = json["data"]["run"]["id"].as_str().unwrap();
    let run = finished(&q, &kb, id).await;
    assert_eq!(run.status, "FAILED");
    assert_eq!(
        *observed.lock().unwrap(),
        Some((USER.into(), "en-US".into()))
    );
    assert!(!run.published);
}
