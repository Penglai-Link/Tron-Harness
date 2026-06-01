use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository,
    application::rag::{
        DefaultRagStrategyRegistry, RagDebugSourceRequest, RagDiagnosticsService, RagEmbeddingCall,
        RagEmbeddingResult, RagEmbeddingRuntime, RagIngestAccessContext, RagIngestRequest,
        RagIngestScope, RagIngestService, RagRetryIndexRequest, RagRetryIndexService,
        RagSearchContextRequest, RagSearchRequest, RagSearchService, RagSearchTraceRequest,
    },
    shared::{
        event_bus::{BusRouteClass, EventBusConfig, EventBusRuntimeBuilder, RecordingEventSink},
        id::FixedSnowflakeIdProvider,
    },
};

#[tokio::test]
async fn debug_source_returns_warnings_without_embedding_payload() {
    let database_url = temp_database_path("rag-debug-source")
        .to_string_lossy()
        .to_string();
    seed_project_doc_debug_fixture(&database_url);

    let service = RagDiagnosticsService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
    );

    let result = service
        .debug_source(RagDebugSourceRequest {
            workspace_code: "TRON_CORE".into(),
            corpus_id: "project-doc".into(),
            source_code: "doc-debug".into(),
            source_version: "v1".into(),
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
            include_text_units: true,
            include_warnings: true,
        })
        .unwrap();

    assert_eq!(result.source.source_code, "doc-debug");
    assert_eq!(result.text_units.len(), 1);
    assert_eq!(result.text_units[0].embedding_status, "INDEXED");
    assert_eq!(result.warnings[0].code, "RAG_VECTOR_INDEX_WARNING");
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(!serialized.contains("embedding\":"));
    assert!(!serialized.contains("provider raw"));
    assert!(!serialized.contains("sk-test"));
}

#[tokio::test]
async fn retry_rebuilds_fulltext_and_vector_from_persisted_text_units() {
    let database_url = temp_database_path("rag-retry-success")
        .to_string_lossy()
        .to_string();
    seed_retry_fixture(&database_url);

    let service = RagRetryIndexService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        FakeEmbeddingRuntime::fixed(vec![0.5, 0.25]),
    );

    let result = service
        .retry_index(RagRetryIndexRequest {
            workspace_code: "TRON_CORE".into(),
            corpus_id: "project-doc".into(),
            source_code: "doc-retry".into(),
            source_version: "v1".into(),
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
            stages: vec!["fulltextIndex".into(), "vectorIndex".into()],
            embedding_model_code: Some("fake-embedding".into()),
            user_code: "USR-AUTH-0001".into(),
            agent_code: "AGENT_CODEX".into(),
        })
        .await
        .unwrap();

    assert_eq!(result.status, "success");
    assert_eq!(result.stage_results.len(), 2);
    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_string(
            &connection,
            "SELECT fulltext_status FROM rag_project_doc_text_units WHERE source_code = 'doc-retry'"
        ),
        "INDEXED"
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT embedding_status FROM rag_project_doc_text_units WHERE source_code = 'doc-retry'"
        ),
        "INDEXED"
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT embedding_dimension FROM rag_project_doc_text_units WHERE source_code = 'doc-retry'"
        ),
        2
    );
}

#[tokio::test]
async fn vector_retry_failure_persists_safe_warning_without_provider_payload() {
    let database_url = temp_database_path("rag-retry-vector-warning")
        .to_string_lossy()
        .to_string();
    seed_retry_fixture(&database_url);

    let service = RagRetryIndexService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        FakeEmbeddingRuntime::failing("provider raw sk-test-secret"),
    );

    let result = service
        .retry_index(RagRetryIndexRequest {
            workspace_code: "TRON_CORE".into(),
            corpus_id: "project-doc".into(),
            source_code: "doc-retry".into(),
            source_version: "v1".into(),
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
            stages: vec!["vectorIndex".into()],
            embedding_model_code: Some("fake-embedding".into()),
            user_code: "USR-AUTH-0001".into(),
            agent_code: "AGENT_CODEX".into(),
        })
        .await
        .unwrap();

    assert_eq!(result.status, "partial_warning");
    assert_eq!(
        result.warnings[0]["code"],
        json!("RAG_VECTOR_INDEX_WARNING")
    );
    let connection = Connection::open(&database_url).unwrap();
    let payload = query_string(
        &connection,
        "SELECT payload_json FROM rag_diagnostics WHERE record_type = 'INDEXING_WARNING'",
    );
    assert!(payload.contains("RAG_VECTOR_INDEX_WARNING"));
    assert!(!payload.contains("provider raw"));
    assert!(!payload.contains("sk-test"));
}

#[tokio::test]
async fn search_trace_writes_lightweight_ttl_row_only_when_enabled() {
    let database_url = temp_database_path("rag-search-trace")
        .to_string_lossy()
        .to_string();
    seed_search_fixture(&database_url);

    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(vec![1.0, 0.0]),
    );
    service
        .search(search_request(false))
        .await
        .expect("trace disabled search should pass");
    assert_eq!(
        query_i64(
            &Connection::open(&database_url).unwrap(),
            "SELECT COUNT(*) FROM rag_diagnostics WHERE record_type = 'SEARCH_TRACE'",
        ),
        0
    );

    let result = service
        .search(search_request(true))
        .await
        .expect("trace enabled search should pass");
    assert!(result.trace_code.is_some());

    let connection = Connection::open(&database_url).unwrap();
    let query_text = query_string(
        &connection,
        "SELECT query_text FROM rag_diagnostics WHERE record_type = 'SEARCH_TRACE'",
    );
    assert_eq!(query_text, "cite sources");
    let ttl_ms = query_i64(
        &connection,
        "SELECT expires_at - created_at FROM rag_diagnostics WHERE record_type = 'SEARCH_TRACE'",
    );
    assert_eq!(ttl_ms, 7 * 24 * 60 * 60 * 1000);
    let payload = query_string(
        &connection,
        "SELECT payload_json FROM rag_diagnostics WHERE record_type = 'SEARCH_TRACE'",
    );
    assert!(payload.contains("candidateRefs"));
    assert!(!payload.contains("Always cite sources"));
    assert!(!payload.contains("embedding"));
}

#[tokio::test]
async fn rag_ingest_events_use_safe_payload_and_publish_failure_is_non_blocking() {
    let database_url = temp_database_path("rag-ingest-events")
        .to_string_lossy()
        .to_string();
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "rag-ingest-events",
        BusRouteClass::Audit,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(7_500)),
    )
    .register_sink(sink.clone())
    .build();
    let service = RagIngestService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(vec![0.5, 0.25]),
    )
    .with_event_bus(event_bus);

    service.ingest(ingest_request()).await.unwrap();

    assert!(sink.wait_for_len(5, Duration::from_secs(2)));
    let events = sink.events();
    let names = events
        .iter()
        .map(|event| event.event_name.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"rag.ingest.started"));
    assert!(names.contains(&"rag.stage.completed"));
    assert!(names.contains(&"rag.ingest.completed"));
    let serialized = format!("{events:?}");
    assert!(!serialized.contains("Always cite sources"));
    assert!(!serialized.contains("embedding"));
    assert!(!serialized.contains("sk-test"));
}

#[tokio::test]
async fn rag_search_events_use_safe_payload_and_publish_failure_is_non_blocking() {
    let database_url = temp_database_path("rag-search-events")
        .to_string_lossy()
        .to_string();
    seed_search_fixture(&database_url);
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "rag-events",
        BusRouteClass::Audit,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(7_000)),
    )
    .register_sink(sink.clone())
    .build();
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(vec![1.0, 0.0]),
    )
    .with_event_bus(event_bus);

    service.search(search_request(true)).await.unwrap();

    assert!(sink.wait_for_len(4, Duration::from_secs(2)));
    let events = sink.events();
    let names = events
        .iter()
        .map(|event| event.event_name.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"rag.search.started"));
    assert!(names.contains(&"rag.route.completed"));
    assert!(names.contains(&"rag.search.completed"));
    let serialized = format!("{events:?}");
    assert!(!serialized.contains("Always cite sources"));
    assert!(!serialized.contains("embedding"));
    assert!(!serialized.contains("sk-test"));
}

fn search_request(trace_enabled: bool) -> RagSearchRequest {
    RagSearchRequest {
        profile_id: "memory-related-v1".into(),
        workspace_code: "TRON_CORE".into(),
        query: "cite sources".into(),
        access_context: RagIngestAccessContext {
            user_code: "USR-AUTH-0001".into(),
            agent_code: "AGENT_CODEX".into(),
            workspace_code: "TRON_CORE".into(),
        },
        filters: json!({ "memoryType": "RULE" }),
        limit: 3,
        token_budget: 100,
        context: RagSearchContextRequest {
            strategy: "RESULT_LIST".into(),
        },
        trace: RagSearchTraceRequest {
            enabled: trace_enabled,
        },
    }
}

fn ingest_request() -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: "memory-card".into(),
        ingest_profile_id: "memory-card-ingest-v1".into(),
        workspace_code: "TRON_CORE".into(),
        access_context: RagIngestAccessContext {
            user_code: "USR-AUTH-0001".into(),
            agent_code: "AGENT_CODEX".into(),
            workspace_code: "TRON_CORE".into(),
        },
        target_scope: RagIngestScope {
            scope_type: "USER".into(),
            scope_code: "USR-AUTH-0001".into(),
        },
        source_code: "memory-ingest-events".into(),
        source_version: "v1".into(),
        source_type: "PLAIN_TEXT".into(),
        payload: json!({
            "title": "RAG citation rule",
            "text": "Always cite sources.",
            "memoryType": "RULE",
            "memoryStatus": "ACTIVE",
            "reviewStatus": "APPROVED",
            "importanceScore": 80
        }),
    }
}

#[derive(Clone)]
struct FakeEmbeddingRuntime {
    result: Result<RagEmbeddingResult, String>,
}

impl FakeEmbeddingRuntime {
    fn fixed(embedding: Vec<f32>) -> Self {
        Self {
            result: Ok(RagEmbeddingResult {
                model_code: "workspace-default-embedding".into(),
                embedding,
            }),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            result: Err(message.into()),
        }
    }
}

impl RagEmbeddingRuntime for FakeEmbeddingRuntime {
    fn embed<'a>(
        &'a self,
        _call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move { self.result.clone() })
    }
}

impl tron_backend::application::rag::RagSearchRuntime for FakeEmbeddingRuntime {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move {
            <Self as RagEmbeddingRuntime>::embed(self, call)
                .await
                .map(|result| result.embedding)
        })
    }

    fn rerank<'a>(
        &'a self,
        _call: tron_backend::application::rag::RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

fn seed_project_doc_debug_fixture(database_url: &str) {
    seed_retry_fixture(database_url);
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "INSERT INTO rag_diagnostics (
                record_type, record_code, corpus_id, profile_id, source_code, source_version,
                scope_type, scope_code, workspace_code, user_code, agent_code, stage_id,
                message, payload_json, created_at
            )
            VALUES ('INDEXING_WARNING', 'warn-debug', 'project-doc', 'project-doc-ingest-v1',
                'doc-debug', 'v1', 'SPACE', 'TRON_CORE', 'TRON_CORE', 'USR-AUTH-0001',
                'AGENT_CODEX', 'vectorIndex', 'vector warning',
                '{\"code\":\"RAG_VECTOR_INDEX_WARNING\"}', 1)",
            [],
        )
        .unwrap();
}

fn seed_retry_fixture(database_url: &str) {
    SqliteRagRepository::connect(database_url).expect("RAG schema should initialize");
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_sources (
                source_code, source_version, source_type, scope_type, scope_code, status,
                title, content_hash, document_code, document_type, path, body_hash, metadata,
                created_at, updated_at, repo_code
            )
            VALUES (?1, 'v1', 'MARKDOWN', 'SPACE', 'TRON_CORE', 'ACTIVE',
                'Debug Doc', 'hash', 'DOC_DEBUG', 'DESIGN', 'docs/debug.md', 'body-hash',
                '{}', 1, 1, 'REPO_TRON')",
            params!["doc-retry"],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_text_units (
                text_unit_code, source_code, source_version, document_code, scope_type, scope_code,
                unit_type, text, search_text, heading_path, ordinal, token_count,
                content_hash, status, fulltext_status, embedding_status, metadata, created_at, updated_at
            )
            VALUES ('tu-retry-1', ?1, 'v1', 'DOC_DEBUG', 'SPACE', 'TRON_CORE', 'LEAF',
                'Retry can rebuild persisted text only.', '', 'Debug', 0, 7,
                'hash', 'ACTIVE', 'FAILED', 'FAILED', '{}', 1, 1)",
            params!["doc-retry"],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_sources (
                source_code, source_version, source_type, scope_type, scope_code, status,
                title, content_hash, document_code, document_type, path, body_hash, metadata,
                created_at, updated_at, repo_code
            )
            VALUES ('doc-debug', 'v1', 'MARKDOWN', 'SPACE', 'TRON_CORE', 'ACTIVE',
                'Debug Doc', 'hash', 'DOC_DEBUG', 'DESIGN', 'docs/debug.md', 'body-hash',
                '{}', 1, 1, 'REPO_TRON')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_text_units (
                text_unit_code, source_code, source_version, document_code, scope_type, scope_code,
                unit_type, text, search_text, heading_path, ordinal, token_count,
                content_hash, status, fulltext_status, embedding_model_code, embedding_version,
                embedding_text_hash, embedding_dimension, embedding, embedding_status, metadata,
                created_at, updated_at
            )
            VALUES ('tu-debug-1', 'doc-debug', 'v1', 'DOC_DEBUG', 'SPACE', 'TRON_CORE',
                'LEAF', 'Debug visible text.', 'Debug visible text.', 'Debug', 0, 3,
                'hash', 'ACTIVE', 'INDEXED', 'fake-embedding', 'v1', 'hash', 2,
                x'0000000000000000', 'INDEXED', '{}', 1, 1)",
            [],
        )
        .unwrap();
}

fn seed_search_fixture(database_url: &str) {
    SqliteRagRepository::connect(database_url).expect("RAG schema should initialize");
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "INSERT INTO rag_memory_text_units (
                text_unit_code, source_code, source_version, source_type, scope_type, scope_code,
                source_status, title, source_content_hash, source_metadata, source_created_at,
                source_updated_at, memory_type, memory_status, review_status, version, injectable,
                unit_type, text, search_text, heading_path, ordinal, token_count,
                content_hash, status, fulltext_status, embedding_model_code, embedding_version,
                embedding_text_hash, embedding_dimension, embedding, embedding_status, metadata,
                created_at, updated_at
            )
            VALUES ('tu-trace-1', 'memory-trace', 'v1', 'PLAIN_TEXT', 'SPACE', 'TRON_CORE',
                'ACTIVE', 'Trace memory rule', 'hash', '{}', 1, 1, 'RULE', 'ACTIVE', 'APPROVED', 'v1', 1,
                'LEAF',
                'Always cite sources in answers.', 'Always cite sources in answers.',
                '', 0, 5, 'hash', 'ACTIVE', 'INDEXED', 'fake-embedding', 'v1',
                'hash', 2, x'0000803F00000000', 'INDEXED', '{}', 1, 1)",
            [],
        )
        .unwrap();
}

fn query_string(connection: &Connection, sql: &str) -> String {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn query_i64(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tron-{test_name}-{nanos}-{}.sqlite3",
        std::process::id()
    ))
}
