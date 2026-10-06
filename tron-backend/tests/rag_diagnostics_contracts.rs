use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository,
    application::rag::{
        DefaultRagStrategyRegistry, RagDebugSourceRequest, RagDiagnosticsService,
        RagDocumentSourceCall, RagDocumentSourceRuntime, RagEmbeddingCall, RagEmbeddingResult,
        RagEmbeddingRuntime, RagIngestAccessContext, RagIngestRequest, RagIngestScope,
        RagIngestService, RagIngestSource, RagResolvedDocument, RagRetryIndexRequest,
        RagRetryIndexService, RagSearchContextRequest, RagSearchRequest, RagSearchService,
        RagSearchTraceRequest,
    },
    shared::{
        event_bus::{
            BusEvent, BusRouteClass, EventBusConfig, EventBusRuntimeBuilder, RecordingEventSink,
        },
        id::FixedSnowflakeIdProvider,
        sqlite_extensions::default_sqlite_extension_context,
    },
};

#[tokio::test]
async fn debug_source_returns_warnings_without_embedding_payload() {
    let database_url = temp_database_path("rag-debug-source")
        .to_string_lossy()
        .to_string();
    seed_project_doc_debug_fixture(&database_url).await;

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
        FakeEmbeddingRuntime::fixed(embedding_1024(0.5, 0.25)),
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
    let diagnostics = RagDiagnosticsService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
    );
    let debug = diagnostics
        .debug_source(RagDebugSourceRequest {
            workspace_code: "TRON_CORE".into(),
            corpus_id: "project-doc".into(),
            source_code: "doc-retry".into(),
            source_version: "v1".into(),
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
            include_text_units: true,
            include_warnings: false,
        })
        .unwrap();
    assert_eq!(debug.text_units[0].fulltext_status, "INDEXED");
    assert_eq!(debug.text_units[0].embedding_status, "INDEXED");
    assert_eq!(debug.text_units[0].embedding_model_code, None);
}

#[tokio::test]
async fn retry_rebuilds_fulltext_and_vector_from_persisted_search_text_when_available() {
    let database_url = temp_database_path("rag-retry-search-text")
        .to_string_lossy()
        .to_string();
    seed_retry_fixture(&database_url);
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_retry_text_unit_fts_search_text(
        &database_url,
        "tu-retry-1",
        "Debug > Retry\nRetry can rebuild persisted text only.",
    );
    let runtime = FakeEmbeddingRuntime::fixed(embedding_1024(0.5, 0.25));

    let result = RagRetryIndexService::new(repository, runtime.clone())
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
    assert_eq!(
        runtime.embedding_inputs(),
        vec!["Debug > Retry\nRetry can rebuild persisted text only.".to_string()]
    );
    let connection = open_prepared_connection(&database_url);
    assert_eq!(
        query_string(
            &connection,
            "SELECT fts.search_text
             FROM rag_project_doc_text_units tu
             JOIN rag_project_doc_text_units_fts fts ON fts.rowid = tu.id
             WHERE tu.text_unit_code = 'tu-retry-1'"
        ),
        "Debug > Retry\nRetry can rebuild persisted text only."
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT text FROM rag_project_doc_text_units WHERE text_unit_code = 'tu-retry-1'"
        ),
        "Retry can rebuild persisted text only."
    );
}

#[tokio::test]
async fn retry_vector_uses_distinct_fulltext_carrier_search_text_when_rowids_diverge() {
    let database_url = temp_database_path("rag-retry-distinct-fulltext-search-text")
        .to_string_lossy()
        .to_string();
    seed_retry_fixture(&database_url);
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_project_doc_fulltext_table(&database_url, "rag_project_doc_fulltext");
    seed_distinct_fulltext_carrier_with_diverged_rowid(
        &database_url,
        "Debug > Retry\nRetry can rebuild persisted text only.",
    );
    let runtime = FakeEmbeddingRuntime::fixed(embedding_1024(0.5, 0.25));

    let result = RagRetryIndexService::new(repository, runtime.clone())
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
    assert_eq!(
        runtime.embedding_inputs(),
        vec!["Debug > Retry\nRetry can rebuild persisted text only.".to_string()]
    );
    let connection = open_prepared_connection(&database_url);
    assert_eq!(
        query_i64(
            &connection,
            "SELECT CASE WHEN ft.id != tu.id THEN 1 ELSE 0 END
             FROM rag_project_doc_fulltext ft
             JOIN rag_project_doc_text_units tu ON tu.text_unit_code = ft.text_unit_code
             WHERE ft.text_unit_code = 'tu-retry-1'"
        ),
        1
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT fts.search_text
             FROM rag_project_doc_fulltext ft
             JOIN rag_project_doc_fulltext_fts fts ON fts.rowid = ft.id
             WHERE ft.text_unit_code = 'tu-retry-1'"
        ),
        "Debug > Retry\nRetry can rebuild persisted text only."
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT text FROM rag_project_doc_text_units WHERE text_unit_code = 'tu-retry-1'"
        ),
        "Retry can rebuild persisted text only."
    );
}

#[tokio::test]
async fn retry_vector_rebuild_supports_distinct_vector_storage_table() {
    let database_url = temp_database_path("rag-retry-separate-vector")
        .to_string_lossy()
        .to_string();
    seed_retry_fixture(&database_url);
    let retry_repository = SqliteRagRepository::connect(&database_url).unwrap();
    let diagnostics_repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_project_doc_vector_table(&database_url, "rag_project_doc_vectors");

    let connection = open_prepared_connection(&database_url);
    connection
        .execute(
            "DROP TABLE IF EXISTS rag_project_doc_text_units_vec_d1024",
            [],
        )
        .unwrap();
    drop(connection);

    let service = RagRetryIndexService::new(
        retry_repository,
        FakeEmbeddingRuntime::fixed(embedding_1024(0.5, 0.25)),
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

    assert_eq!(result.status, "success");
    let connection = open_prepared_connection(&database_url);
    assert_eq!(
        query_i64(&connection, "SELECT COUNT(*) FROM rag_project_doc_vectors"),
        1
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_project_doc_vectors_vec_d1024"
        ),
        1
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM sqlite_schema WHERE name = 'rag_project_doc_text_units_vec_d1024'"
        ),
        0
    );

    let diagnostics = RagDiagnosticsService::new(
        diagnostics_repository,
        DefaultRagStrategyRegistry::default(),
    );
    let debug = diagnostics
        .debug_source(RagDebugSourceRequest {
            workspace_code: "TRON_CORE".into(),
            corpus_id: "project-doc".into(),
            source_code: "doc-retry".into(),
            source_version: "v1".into(),
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
            include_text_units: true,
            include_warnings: false,
        })
        .unwrap();
    assert_eq!(debug.text_units[0].embedding_status, "INDEXED");
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
    seed_search_fixture(&database_url).await;

    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_1024(1.0, 0.0)),
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
    let payload: Value = serde_json::from_str(&payload).unwrap();
    assert!(payload.get("candidateRefs").is_some());
    let stage_results = payload["stageResults"]
        .as_array()
        .expect("search trace payload should include stageResults");
    let route_stage = stage_results
        .iter()
        .find(|stage| stage["stageType"] == json!("ROUTE_SEARCH"))
        .expect("search trace should summarize executed route search nodes");
    assert!(route_stage["nodeId"].as_str().is_some());
    assert!(route_stage["strategyId"].as_str().is_some());
    assert_eq!(route_stage["status"], json!("COMPLETED"));
    assert!(route_stage["durationMs"].as_u64().is_some());
    assert_eq!(route_stage["inputArtifactKeys"], json!(["query.effective"]));
    assert_eq!(route_stage["outputArtifactKeys"], json!(["hits.route"]));
    assert!(
        route_stage["warnings"].as_array().is_some(),
        "search trace stageResults should always include a warnings array"
    );
    let serialized_payload = payload.to_string();
    assert!(!serialized_payload.contains("Always cite sources"));
    assert!(!serialized_payload.contains("embedding"));
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
        FakeEmbeddingRuntime::fixed(embedding_1024(0.5, 0.25)),
    )
    .with_event_bus(event_bus);

    service.ingest(ingest_request()).await.unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    let events = loop {
        let events = sink.events();
        if events
            .iter()
            .any(|event| event.event_name == "rag.ingest.completed")
            || Instant::now() >= deadline
        {
            break events;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let names = event_names(&events);
    assert_all_audit(&events);
    assert!(names.contains(&"rag.ingest.started"));
    assert!(!names.contains(&"rag.stage.completed"));
    assert!(names.contains(&"rag.ingest.stage.completed"));
    assert!(names.contains(&"rag.ingest.completed"));
    let stage_ids = events
        .iter()
        .filter(|event| event.event_name == "rag.ingest.stage.completed")
        .filter_map(|event| attr(event, "stage_id"))
        .collect::<Vec<_>>();
    assert_eq!(
        stage_ids,
        vec!["fulltextIndex", "vectorIndex", "persistSource"]
    );
    let serialized = format!("{events:?}");
    assert!(!serialized.contains("Always cite sources"));
    assert!(!serialized.contains("embedding"));
    assert!(!serialized.contains("sk-test"));

    let failing_sink = Arc::new(RecordingEventSink::with_route_class(
        "rag-ingest-failed-events",
        BusRouteClass::Audit,
    ));
    let failing_event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(7_700)),
    )
    .register_sink(failing_sink.clone())
    .build();
    let failing_service = RagIngestService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_1024(0.5, 0.25)),
    )
    .with_event_bus(failing_event_bus);
    let mut failing_request = ingest_request();
    failing_request.corpus_id = "missing-corpus".into();

    assert!(failing_service.ingest(failing_request).await.is_err());
    assert!(failing_sink.wait_for_len(2, Duration::from_secs(2)));
    let failing_events = failing_sink.events();
    let failing_names = event_names(&failing_events);
    assert!(failing_names.contains(&"rag.ingest.started"));
    assert!(failing_names.contains(&"rag.ingest.failed"));
}

#[tokio::test]
async fn rag_search_events_use_safe_payload_and_publish_failure_is_non_blocking() {
    let database_url = temp_database_path("rag-search-events")
        .to_string_lossy()
        .to_string();
    seed_search_fixture(&database_url).await;
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
        FakeEmbeddingRuntime::fixed(embedding_1024(1.0, 0.0)),
    )
    .with_event_bus(event_bus);

    service.search(search_request(true)).await.unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    let events = loop {
        let events = sink.events();
        if events
            .iter()
            .any(|event| event.event_name == "rag.search.completed")
            || Instant::now() >= deadline
        {
            break events;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let names = event_names(&events);
    assert_all_audit(&events);
    assert!(names.contains(&"rag.search.started"));
    assert!(!names.contains(&"rag.route.completed"));
    assert!(names.contains(&"rag.search.route.completed"));
    assert!(names.contains(&"rag.search.completed"));
    let completed = events
        .iter()
        .find(|event| event.event_name == "rag.search.completed")
        .expect("search completed event should be published");
    assert_eq!(attr(completed, "result_count").as_deref(), Some("1"));
    assert_eq!(
        attr(completed, "context_budget_units").as_deref(),
        Some("100")
    );
    let serialized = format!("{events:?}");
    assert!(!serialized.contains("Always cite sources"));
    assert!(!serialized.contains("cite sources"));
    assert!(!serialized.contains("embedding"));
    assert!(!serialized.contains("sk-test"));

    let failing_sink = Arc::new(RecordingEventSink::with_route_class(
        "rag-search-failed-events",
        BusRouteClass::Audit,
    ));
    let failing_event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(7_100)),
    )
    .register_sink(failing_sink.clone())
    .build();
    let failing_service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_1024(1.0, 0.0)),
    )
    .with_event_bus(failing_event_bus);
    let mut failing_request = search_request(false);
    failing_request.profile_id = "missing-profile".into();

    assert!(failing_service.search(failing_request).await.is_err());
    assert!(failing_sink.wait_for_len(2, Duration::from_secs(2)));
    let failing_events = failing_sink.events();
    let failing_names = event_names(&failing_events);
    assert!(failing_names.contains(&"rag.search.started"));
    assert!(failing_names.contains(&"rag.search.failed"));
}

#[tokio::test]
async fn rag_retry_events_are_audit_only_and_include_terminal_failures() {
    let database_url = temp_database_path("rag-retry-events")
        .to_string_lossy()
        .to_string();
    seed_retry_fixture(&database_url);
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "rag-retry-events",
        BusRouteClass::Audit,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(7_900)),
    )
    .register_sink(sink.clone())
    .build();
    let service = RagRetryIndexService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        FakeEmbeddingRuntime::fixed(embedding_1024(0.5, 0.25)),
    )
    .with_event_bus(event_bus);

    service
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

    assert!(sink.wait_for_len(4, Duration::from_secs(2)));
    let events = sink.events();
    let names = event_names(&events);
    assert_all_audit(&events);
    assert!(names.contains(&"rag.retry.started"));
    assert!(names.contains(&"rag.retry.stage.completed"));
    assert!(names.contains(&"rag.retry.completed"));
    let serialized = format!("{events:?}");
    assert!(!serialized.contains("Retry can rebuild persisted text only."));

    let failing_sink = Arc::new(RecordingEventSink::with_route_class(
        "rag-retry-failed-events",
        BusRouteClass::Audit,
    ));
    let failing_event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(8_100)),
    )
    .register_sink(failing_sink.clone())
    .build();
    let failing_service = RagRetryIndexService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        FakeEmbeddingRuntime::fixed(embedding_1024(0.5, 0.25)),
    )
    .with_event_bus(failing_event_bus);
    assert!(
        failing_service
            .retry_index(RagRetryIndexRequest {
                workspace_code: "TRON_CORE".into(),
                corpus_id: "project-doc".into(),
                source_code: "missing-source".into(),
                source_version: "v1".into(),
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
                stages: vec!["fulltextIndex".into()],
                embedding_model_code: Some("fake-embedding".into()),
                user_code: "USR-AUTH-0001".into(),
                agent_code: "AGENT_CODEX".into(),
            })
            .await
            .is_err()
    );
    assert!(failing_sink.wait_for_len(2, Duration::from_secs(2)));
    let failing_events = failing_sink.events();
    let failing_names = event_names(&failing_events);
    assert!(failing_names.contains(&"rag.retry.started"));
    assert!(failing_names.contains(&"rag.retry.failed"));
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
            return_fields: Vec::new(),
            graph_expansion: Default::default(),
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
        source: RagIngestSource {
            code: "memory-ingest-events".into(),
            version: "v1".into(),
            source_type: String::new(),
        },
        payload: json!({
            "source": {
                "code": "memory-ingest-events",
                "version": "v1",
                "type": "MEMORY_CARD"
            },
            "title": "RAG citation rule",
            "text": "Always cite sources.",
            "domainFields": {
                "version": "v1",
                "memoryType": "RULE",
                "memoryStatus": "ACTIVE",
                "reviewStatus": "APPROVED",
                "importanceScore": 80
            }
        }),
    }
}

fn event_names(events: &[BusEvent]) -> Vec<&str> {
    events
        .iter()
        .map(|event| event.event_name.as_str())
        .collect()
}

fn assert_all_audit(events: &[BusEvent]) {
    assert!(
        events
            .iter()
            .all(|event| event.route_class == BusRouteClass::Audit),
        "all RAG events should use Audit route class: {events:?}"
    );
}

fn attr(event: &BusEvent, key: &str) -> Option<String> {
    event
        .attrs
        .iter()
        .find(|attr| attr.key == key)
        .map(|attr| attr.value.clone())
}

#[derive(Clone)]
struct FakeEmbeddingRuntime {
    result: Result<RagEmbeddingResult, String>,
    inputs: Arc<Mutex<Vec<String>>>,
}

impl FakeEmbeddingRuntime {
    fn fixed(embedding: Vec<f32>) -> Self {
        Self {
            result: Ok(RagEmbeddingResult {
                model_code: "workspace-default-embedding".into(),
                embedding,
            }),
            inputs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            result: Err(message.into()),
            inputs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn embedding_inputs(&self) -> Vec<String> {
        self.inputs
            .lock()
            .expect("fake embedding inputs mutex poisoned")
            .clone()
    }
}

impl RagEmbeddingRuntime for FakeEmbeddingRuntime {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.inputs
                .lock()
                .expect("fake embedding inputs mutex poisoned")
                .push(call.input);
            self.result.clone()
        })
    }
}

impl RagDocumentSourceRuntime for FakeEmbeddingRuntime {
    fn resolve_project_doc_markdown<'a>(
        &'a self,
        _call: RagDocumentSourceCall,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<RagResolvedDocument, tron_backend::domain::rag::RagError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Err(tron_backend::domain::rag::RagError::SourceResolveFailed) })
    }
}

impl tron_backend::application::rag::RagSearchRuntime for FakeEmbeddingRuntime {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move { <Self as RagEmbeddingRuntime>::embed(self, call).await })
    }

    fn rerank<'a>(
        &'a self,
        _call: tron_backend::application::rag::RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

async fn seed_project_doc_debug_fixture(database_url: &str) {
    seed_retry_fixture(database_url);
    let retry_service = RagRetryIndexService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        FakeEmbeddingRuntime::fixed(embedding_1024(0.0, 0.0)),
    );
    retry_service
        .retry_index(RagRetryIndexRequest {
            workspace_code: "TRON_CORE".into(),
            corpus_id: "project-doc".into(),
            source_code: "doc-debug".into(),
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
                source_code, source_version, scope_type, scope_code, status,
                title, content_hash, document_code, document_type, path, body_hash, metadata,
                created_at, updated_at, repo_code
            )
            VALUES (?1, 'v1', 'SPACE', 'TRON_CORE', 'ACTIVE',
                'Debug Doc', 'hash', 'DOC_DEBUG', 'DESIGN', 'docs/debug.md', 'body-hash',
                '{}', 1, 1, 'REPO_TRON')",
            params!["doc-retry"],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_text_units (
                text_unit_code, source_code, source_version, document_code, scope_type, scope_code,
                unit_type, text, heading_path, ordinal, token_count,
                content_hash, status, metadata, created_at, updated_at
            )
            VALUES ('tu-retry-1', ?1, 'v1', 'DOC_DEBUG', 'SPACE', 'TRON_CORE', 'LEAF',
                'Retry can rebuild persisted text only.', 'Debug', 0, 7,
                'hash', 'ACTIVE', '{}', 1, 1)",
            params!["doc-retry"],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_sources (
                source_code, source_version, scope_type, scope_code, status,
                title, content_hash, document_code, document_type, path, body_hash, metadata,
                created_at, updated_at, repo_code
            )
            VALUES ('doc-debug', 'v1', 'SPACE', 'TRON_CORE', 'ACTIVE',
                'Debug Doc', 'hash', 'DOC_DEBUG', 'DESIGN', 'docs/debug.md', 'body-hash',
                '{}', 1, 1, 'REPO_TRON')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_text_units (
                text_unit_code, source_code, source_version, document_code, scope_type, scope_code,
                unit_type, text, heading_path, ordinal, token_count,
                content_hash, status, metadata, created_at, updated_at
            )
            VALUES ('tu-debug-1', 'doc-debug', 'v1', 'DOC_DEBUG', 'SPACE', 'TRON_CORE',
                'LEAF', 'Debug visible text.', 'Debug', 0, 3,
                'hash', 'ACTIVE', '{}', 1, 1)",
            [],
        )
        .unwrap();
}

fn set_retry_text_unit_fts_search_text(
    database_url: &str,
    text_unit_code: &str,
    search_text: &str,
) {
    let connection = open_prepared_connection(database_url);
    connection
        .execute(
            "INSERT OR REPLACE INTO rag_project_doc_text_units_fts(rowid, search_text)
             SELECT id, ?1
             FROM rag_project_doc_text_units
             WHERE text_unit_code = ?2",
            params![search_text, text_unit_code],
        )
        .unwrap();
}

fn seed_distinct_fulltext_carrier_with_diverged_rowid(database_url: &str, search_text: &str) {
    let connection = open_prepared_connection(database_url);
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS rag_project_doc_fulltext (
                id INTEGER PRIMARY KEY,
                text_unit_code TEXT NOT NULL,
                source_code TEXT NOT NULL,
                source_version TEXT NOT NULL,
                scope_type TEXT NOT NULL,
                scope_code TEXT NOT NULL,
                unit_type TEXT,
                text TEXT,
                search_text TEXT,
                heading_path TEXT,
                ordinal INTEGER,
                token_count INTEGER,
                status TEXT NOT NULL,
                metadata TEXT,
                content_hash TEXT,
                created_at INTEGER,
                updated_at INTEGER,
                UNIQUE (text_unit_code),
                UNIQUE (source_code, source_version, scope_type, scope_code, ordinal)
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS rag_project_doc_fulltext_fts
                USING fts5(search_text, tokenize='unicode61');",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_fulltext (
                text_unit_code, source_code, source_version, scope_type, scope_code,
                unit_type, text, search_text, heading_path, ordinal, token_count,
                status, metadata, content_hash, created_at, updated_at
             )
             VALUES (
                'tu-carrier-dummy', 'doc-dummy', 'v1', 'SPACE', 'TRON_CORE',
                'LEAF', 'dummy raw text', 'dummy index text', '', 0, 1,
                'ACTIVE', '{}', 'dummy-hash', 1, 1
             )",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_fulltext (
                text_unit_code, source_code, source_version, scope_type, scope_code,
                unit_type, text, search_text, heading_path, ordinal, token_count,
                status, metadata, content_hash, created_at, updated_at
             )
             SELECT text_unit_code, source_code, source_version, scope_type, scope_code,
                    unit_type, text, ?1, heading_path, ordinal, token_count,
                    status, metadata, content_hash, created_at, updated_at
             FROM rag_project_doc_text_units
             WHERE text_unit_code = 'tu-retry-1'",
            params![search_text],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_fulltext_fts(rowid, search_text)
             SELECT id, search_text
             FROM rag_project_doc_fulltext
             WHERE text_unit_code = 'tu-retry-1'",
            [],
        )
        .unwrap();
}

fn set_project_doc_fulltext_table(database_url: &str, table_name: &str) {
    set_project_doc_segment_table(
        database_url,
        "fulltext",
        "project-doc-fulltext-store",
        table_name,
    );
}

fn set_project_doc_vector_table(database_url: &str, table_name: &str) {
    set_project_doc_segment_table(
        database_url,
        "vector",
        "project-doc-vector-store",
        table_name,
    );
}

fn set_project_doc_segment_table(
    database_url: &str,
    segment: &str,
    target_id: &str,
    table_name: &str,
) {
    let connection = Connection::open(database_url).unwrap();
    let binding_json = query_string(
        &connection,
        "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'project-doc'",
    );
    let mut binding = serde_json::from_str::<Value>(&binding_json).unwrap();
    binding["bindings"][segment] = json!(target_id);
    let targets = binding["targets"]
        .as_array_mut()
        .expect("storage targets should be an array");
    if let Some(target) = targets
        .iter_mut()
        .find(|target| target["id"] == json!(target_id))
    {
        target["table"] = json!(table_name);
    } else {
        targets.push(json!({
            "id": target_id,
            "backend": "SQLITE",
            "layout": "TABLE",
            "table": table_name
        }));
    }
    connection
        .execute(
            "UPDATE rag_corpora
             SET storage_binding_json = ?1
             WHERE corpus_id = 'project-doc'",
            params![binding.to_string()],
        )
        .unwrap();
}

fn open_prepared_connection(database_url: &str) -> Connection {
    let connection = Connection::open(database_url).unwrap();
    default_sqlite_extension_context()
        .prepare_connection(&connection)
        .unwrap();
    connection
}

async fn seed_search_fixture(database_url: &str) {
    let service = RagIngestService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_1024(1.0, 0.0)),
    );
    service.ingest(ingest_request()).await.unwrap();
}

fn query_string(connection: &Connection, sql: &str) -> String {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn query_i64(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn embedding_1024(first: f32, second: f32) -> Vec<f32> {
    let mut embedding = vec![0.0; 1024];
    embedding[0] = first;
    embedding[1] = second;
    embedding
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
