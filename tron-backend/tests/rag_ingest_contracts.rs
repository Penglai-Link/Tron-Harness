use std::{
    collections::{BTreeMap, VecDeque},
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository,
    application::llm::{
        LlmRuntimeClient, LlmRuntimeModelResolver, LlmRuntimeRerankHit, LlmRuntimeService,
    },
    application::model::RuntimeModelConfig,
    application::rag::{
        DefaultRagRuntime, DefaultRagStrategyRegistry, RagConfigRepository, RagDocumentParseCall,
        RagDocumentSourceCall, RagDocumentSourceRuntime, RagEmbeddingCall, RagEmbeddingResult,
        RagEmbeddingRuntime, RagIngestAccessContext, RagIngestRepository, RagIngestRequest,
        RagIngestScope, RagIngestService, RagIngestSource, RagPersistIngestCommand,
        RagResolvedDocument, RagService, RagStrategyRegistry, RagTextUnitWrite, RagTreeNodeWrite,
        RagTreeSummaryCall, RagTreeSummaryResult, RagTreeSummaryRuntime, StrategyRegistration,
        StrategyRegistrationStatus, compile_ingest_profile,
    },
    domain::model::{ModelError, ModelProtocol, ModelType},
    domain::rag::{RagError, RagProfileRecord, RagProfileType, RagStatus},
    shared::sqlite_extensions::default_sqlite_extension_context,
};

#[tokio::test]
async fn memory_text_contract_ingest_writes_business_text_unit_and_sidecar_indexes() {
    let database_path = temp_database_path("rag-ingest-memory-success");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    let embeddings = FakeEmbeddingRuntime::fixed(embedding_4096(0.25, 0.75));
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        embeddings.clone(),
    );

    let result = service.ingest(memory_request("v1")).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 1);
    assert_eq!(result.fulltext_status, "INDEXED");
    assert_eq!(result.embedding_status, "INDEXED");
    assert_eq!(embeddings.call_count(), 1);
    assert!(
        result
            .stage_results
            .iter()
            .any(|stage| stage.stage_type == "SPLIT_TEXT"
                && stage.output_artifact_keys == vec!["textUnits.generated", "tree.hints"])
    );
    assert!(
        result
            .stage_results
            .iter()
            .any(|stage| stage.node_id == "persistSource"
                && stage.output_artifact_keys == vec!["source.persisted"])
    );

    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_string(
            &connection,
            "SELECT source_status FROM rag_memory_text_units WHERE source_code = 'memory-rag-rule' LIMIT 1"
        ),
        "ACTIVE"
    );
    let unit = query_unit(&connection, "rag_memory_text_units");
    assert_eq!(
        unit["text"],
        json!("Always cite source titles in RAG answers.")
    );
    assert_index_payload_columns_absent(&connection, "rag_memory_text_units");
    assert!(table_exists(&connection, "rag_memory_text_units_fts"));
    assert!(table_exists(&connection, "rag_memory_text_units_vec_d4096"));
    assert_eq!(embeddings.last_model_code(), None);
    let source_metadata = serde_json::from_str::<Value>(&query_string(
        &connection,
        "SELECT source_metadata FROM rag_memory_text_units WHERE source_code = 'memory-rag-rule'",
    ))
    .unwrap();
    assert_eq!(
        source_metadata,
        json!({"seed": "contract-test", "source": "rag-ingest"})
    );
}

#[tokio::test]
async fn memory_ingest_maps_business_version_from_payload() {
    let database_path = temp_database_path("rag-ingest-memory-payload-version");
    let database_url = database_path.to_string_lossy().to_string();
    let service = RagIngestService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2)),
    );
    let mut request = memory_request("storage-v1");
    request.payload["domainFields"]["version"] = json!("business-v9");

    service.ingest(request).await.unwrap();

    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_string(
            &connection,
            "SELECT source_version FROM rag_memory_text_units WHERE source_code = 'memory-rag-rule'"
        ),
        "storage-v1"
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT version FROM rag_memory_text_units WHERE source_code = 'memory-rag-rule'"
        ),
        "business-v9"
    );
}

#[tokio::test]
async fn pipeline_only_memory_ingest_profile_preserves_existing_request_contract() {
    let database_path = temp_database_path("rag-ingest-pipeline-only-memory");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    insert_pipeline_only_ingest_profile(&database_url, "memory-card-pipeline-only-v1");
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.25, 0.75)),
    );
    let mut request = memory_request("v1");
    request.ingest_profile_id = "memory-card-pipeline-only-v1".into();

    let result = service.ingest(request).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 1);
    assert_eq!(
        result
            .stage_results
            .iter()
            .map(|stage| stage.node_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "validate-source",
            "prepare-payload",
            "split-text",
            "fulltext-index",
            "vector-index",
            "persist-source"
        ]
    );
}

#[tokio::test]
async fn pipeline_extracts_and_persists_tree_graph_without_request_related_graph() {
    let database_path = temp_database_path("rag-ingest-phase3-tree-graph");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = CapturingIngestRepository::connect(&database_url);
    insert_tree_pipeline_ingest_profile(&database_url, "memory-card-tree-pipeline-v1");
    let persisted_commands = repository.persisted_commands.clone();
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.25, 0.75)),
    );
    let mut request = memory_request("v1");
    request.ingest_profile_id = "memory-card-tree-pipeline-v1".into();
    assert!(
        request.payload.get("relatedGraph").is_none(),
        "test request must keep the existing ingest shape and omit relatedGraph"
    );

    let result = service.ingest(request).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(
        result
            .stage_results
            .iter()
            .map(|stage| stage.node_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "validate-source",
            "prepare-payload",
            "split-text",
            "extract-tree",
            "fulltext-index",
            "vector-index",
            "summarize-tree",
            "tree-fulltext-index",
            "tree-vector-index",
            "persist-tree",
            "persist-source"
        ]
    );
    assert!(
        result.stage_results.iter().any(|stage| {
            stage.node_id == "extract-tree"
                && stage.input_artifact_keys == vec!["tree.hints", "textUnits.generated"]
                && stage.output_artifact_keys == vec!["tree.extracted", "graph.related"]
        }),
        "EXTRACT_TREE should report the actual tree graph artifact handoff"
    );
    assert!(
        result.stage_results.iter().any(|stage| {
            stage.node_id == "persist-tree"
                && stage.input_artifact_keys == vec!["tree.summarized"]
                && stage.output_artifact_keys == vec!["tree.writePayload", "tree.persisted"]
        }),
        "PERSIST_TREE should report consuming the summarized tree baseline"
    );
    let commands = persisted_commands.lock().unwrap();
    let related_graph = commands
        .first()
        .and_then(|command| command.related_graph.as_ref())
        .expect("tree pipeline should hand a generated related graph to persistence");
    assert_eq!(related_graph["sourceCode"], json!("memory-rag-rule"));
    assert_eq!(related_graph["sourceVersion"], json!("v1"));
    assert!(
        related_graph["nodes"]
            .as_array()
            .is_some_and(
                |nodes| nodes.iter().any(|node| node["kind"] == json!("TEXT_UNIT")
                    && node["ref"] == json!("text-unit:memory-rag-rule:v1:0")
                    && node["code"] == json!("memory-rag-rule:v1:0")
                    && node["label"] == json!("RAG_TEXT_UNIT"))
            ),
        "generated related graph should include graph-writer-compatible text unit nodes"
    );
    assert!(
        related_graph["edges"]
            .as_array()
            .is_some_and(|edges| edges
                .iter()
                .any(|edge| edge["from"] == json!("source:memory-rag-rule")
                    && edge["to"] == json!("text-unit:memory-rag-rule:v1:0")
                    && edge["relationship"] == json!("CONTAINS"))),
        "generated related graph edges should reference node refs"
    );
    let tree_nodes = &commands.first().unwrap().tree_nodes;
    assert!(
        tree_nodes.iter().any(|node| node.parent_node_code == "0"
            && node.covered_text_unit_codes == vec!["memory-rag-rule:v1:0"]),
        "splitText should provide tree hints that summarize into a root node"
    );
    let root = tree_nodes
        .iter()
        .find(|node| node.parent_node_code == "0")
        .expect("tree pipeline should persist a root summary node");
    assert_eq!(root.summary_fulltext_status, "INDEXED");
    assert_eq!(root.summary_embedding_status, "INDEXED");
    assert_eq!(
        root.summary_embedding_model_code,
        Some("workspace-default-embedding".into())
    );
}

#[tokio::test]
async fn pipeline_persists_summary_tree_without_optional_tree_indexes() {
    let database_path = temp_database_path("rag-ingest-summary-tree-no-tree-indexes");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = CapturingIngestRepository::connect(&database_url);
    insert_summary_tree_only_ingest_profile(&database_url, "memory-card-summary-tree-only-v1");
    let persisted_commands = repository.persisted_commands.clone();
    let embeddings = FakeEmbeddingRuntime::fixed(embedding_4096(0.25, 0.75));
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        embeddings.clone(),
    );
    let mut request = memory_request("v1");
    request.ingest_profile_id = "memory-card-summary-tree-only-v1".into();

    let result = service.ingest(request).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(
        result
            .stage_results
            .iter()
            .map(|stage| stage.node_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "validate-source",
            "prepare-payload",
            "split-text",
            "extract-tree",
            "fulltext-index",
            "vector-index",
            "summarize-tree",
            "persist-tree",
            "persist-source"
        ]
    );
    assert_eq!(
        embeddings.call_count(),
        1,
        "tree vector indexing is absent, so only text units should be embedded"
    );
    assert!(result.stage_results.iter().any(|stage| {
        stage.node_id == "persist-tree" && stage.input_artifact_keys == vec!["tree.summarized"]
    }));
    let commands = persisted_commands.lock().unwrap();
    let root = commands
        .first()
        .expect("source should persist")
        .tree_nodes
        .iter()
        .find(|node| node.parent_node_code == "0")
        .expect("summary baseline should persist a root tree node");
    assert_eq!(root.summary_fulltext_status, "PENDING");
    assert_eq!(root.summary_embedding_status, "PENDING");
    assert_eq!(root.summary_embedding_model_code, None);
}

#[tokio::test]
async fn llm_summary_tree_ingest_summarizes_bottom_up_with_configured_model() {
    let database_path = temp_database_path("rag-ingest-llm-summary-tree");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = CapturingIngestRepository::connect(&database_url);
    insert_llm_summary_tree_ingest_profile(
        &database_url,
        "memory-card-llm-summary-tree-v1",
        Some(json!({ "modelCode": "summary-model-alpha" })),
    );
    let persisted_commands = repository.persisted_commands.clone();
    let runtime = FakeEmbeddingRuntime::fixed(embedding_4096(0.25, 0.75)).with_summary_outputs([
        "leaf summary",
        "parent summary",
        "root summary",
    ]);
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    )
    .with_tree_summary_runtime(runtime.clone());
    let mut request = memory_markdown_tree_request("v1");
    request.ingest_profile_id = "memory-card-llm-summary-tree-v1".into();

    let result = service.ingest(request).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    let summarize_stage = result
        .stage_results
        .iter()
        .find(|stage| stage.node_id == "summarize-tree")
        .expect("summary stage should be reported");
    assert_eq!(summarize_stage.strategy_id, "RAG_TREE_LLM_SUMMARY_V1");
    let summary_calls = runtime.summary_calls();
    assert_eq!(summary_calls.len(), 3);
    assert!(
        summary_calls
            .iter()
            .all(|call| call.model_code == Some("summary-model-alpha".into()))
    );
    assert!(
        summary_calls[0]
            .prompt
            .contains("## Write Path\nPersist synchronously.")
    );
    assert!(summary_calls[1].prompt.contains("leaf summary"));
    assert!(summary_calls[1].prompt.contains("# RAG MVP\nIntro."));
    assert!(summary_calls[2].prompt.contains("parent summary"));

    let commands = persisted_commands.lock().unwrap();
    let tree_nodes = &commands
        .first()
        .expect("source should persist after all summaries complete")
        .tree_nodes;
    let root = tree_nodes
        .iter()
        .find(|node| node.parent_node_code == "0")
        .expect("root summary node should be persisted");
    assert_eq!(root.summary_text, "root summary");
    assert_eq!(
        root.metadata["summaryStrategy"],
        json!("RAG_TREE_LLM_SUMMARY_V1")
    );
    assert_eq!(
        root.metadata["summaryModelCode"],
        json!("summary-model-alpha")
    );
    assert_eq!(root.metadata["summaryStatus"], json!("SUMMARIZED"));
    assert!(
        tree_nodes
            .iter()
            .any(|node| node.title == "Write Path" && node.summary_text == "leaf summary")
    );
    assert!(
        tree_nodes
            .iter()
            .any(|node| node.title == "RAG MVP" && node.summary_text == "parent summary")
    );
}

#[tokio::test]
async fn llm_summary_tree_empty_response_falls_back_to_extractive_without_partial_failure() {
    let database_path = temp_database_path("rag-ingest-llm-summary-tree-empty");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = CapturingIngestRepository::connect(&database_url);
    insert_llm_summary_tree_ingest_profile(
        &database_url,
        "memory-card-llm-summary-tree-empty-v1",
        Some(json!({ "modelCode": "summary-model-alpha" })),
    );
    let persisted_commands = repository.persisted_commands.clone();
    let runtime =
        FakeEmbeddingRuntime::fixed(embedding_4096(0.25, 0.75)).with_summary_outputs([""]);
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    )
    .with_tree_summary_runtime(runtime.clone());
    let mut request = memory_markdown_tree_request("v1");
    request.ingest_profile_id = "memory-card-llm-summary-tree-empty-v1".into();

    let result = service.ingest(request).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(
        result.warnings[0]["code"],
        json!("RAG_TREE_LLM_SUMMARY_FALLBACK")
    );
    assert_eq!(runtime.summary_calls().len(), 1);
    let commands = persisted_commands.lock().unwrap();
    let tree_nodes = &commands
        .first()
        .expect("fallback summary should persist the source")
        .tree_nodes;
    assert_eq!(tree_nodes.len(), 3);
    assert!(tree_nodes.iter().all(|node| {
        node.metadata["summaryStrategy"] == json!("RAG_TREE_LLM_SUMMARY_V1")
            && node.metadata["summaryFallbackStrategy"] == json!("EXTRACTIVE_DETERMINISTIC_V1")
            && node.metadata["summaryStatus"] == json!("FALLBACK_EXTRACTIVE")
    }));
    assert_eq!(
        tree_nodes[0].metadata["summaryFallbackReason"],
        json!("LLM_SUMMARY_SKIPPED_AFTER_FAILURE")
    );
    assert_eq!(
        tree_nodes
            .iter()
            .find(|node| node.title == "Write Path")
            .expect("leaf node should be present")
            .metadata["summaryFallbackReason"],
        json!("LLM_SUMMARY_EMPTY_RESPONSE")
    );
    drop(commands);
    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_i64(&connection, "SELECT COUNT(*) FROM rag_memory_text_units"),
        2
    );
}

#[tokio::test]
async fn rag_service_ingest_auto_wires_default_runtime_llm_summary() {
    let database_path = temp_database_path("rag-ingest-default-runtime-llm-summary");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = CapturingIngestRepository::connect(&database_url);
    insert_llm_summary_tree_ingest_profile(
        &database_url,
        "memory-card-default-runtime-llm-summary-v1",
        Some(json!({ "modelCode": "summary-model-alpha" })),
    );
    let persisted_commands = repository.persisted_commands.clone();
    let client = FakeLlmClient::new()
        .with_generations(["leaf summary", "parent summary", "root summary"])
        .with_embeddings((0..8).map(|_| embedding_4096(0.25, 0.75)));
    let runtime = DefaultRagRuntime::new(
        LlmRuntimeService::new(Arc::new(FakeLlmModelResolver), Arc::new(client.clone())),
        std::env::temp_dir(),
    );
    let service = RagService::new(repository, DefaultRagStrategyRegistry::default(), runtime);
    let mut request = memory_markdown_tree_request("v1");
    request.ingest_profile_id = "memory-card-default-runtime-llm-summary-v1".into();

    let result = service.ingest(request).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(client.generate_inputs().len(), 3);
    assert!(
        client.generate_inputs()[0].contains("## Write Path\nPersist synchronously."),
        "default runtime should call LLM text generation for tree leaf summaries"
    );
    let commands = persisted_commands.lock().unwrap();
    let root = commands
        .first()
        .expect("source should persist")
        .tree_nodes
        .iter()
        .find(|node| node.parent_node_code == "0")
        .expect("root summary node should persist");
    assert_eq!(root.summary_text, "root summary");
    assert_eq!(
        root.metadata["summaryModelCode"],
        json!("summary-model-alpha")
    );
}

#[tokio::test]
async fn tree_vector_embedding_failure_fails_fast_source() {
    let database_path = temp_database_path("rag-ingest-tree-vector-fail-fast");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_memory_card_tree_table(&database_url, "rag_memory_tree_nodes");
    insert_tree_pipeline_ingest_profile(&database_url, "memory-card-tree-fail-fast-v1");
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.25, 0.75))
            .fail_after_successes(1, "fake tree vector unavailable"),
    );
    let mut request = memory_request("v1");
    request.ingest_profile_id = "memory-card-tree-fail-fast-v1".into();

    let error = service.ingest(request).await.unwrap_err();

    assert_eq!(error, RagError::EmbeddingFailed);
    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_i64(&connection, "SELECT COUNT(*) FROM rag_memory_text_units"),
        0
    );
    if table_exists(&connection, "rag_memory_tree_nodes") {
        assert_eq!(
            query_i64(&connection, "SELECT COUNT(*) FROM rag_memory_tree_nodes"),
            0
        );
    }
}

#[tokio::test]
async fn sqlite_tree_segment_creates_indexes_and_ingest_persists_summary_rows() {
    let database_path = temp_database_path("rag-ingest-summary-tree");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_memory_card_tree_table(&database_url, "rag_memory_tree_nodes");
    insert_tree_pipeline_ingest_profile(&database_url, "memory-card-tree-pipeline-v1");
    let embeddings = FakeEmbeddingRuntime::fixed(embedding_4096(0.25, 0.75));
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        embeddings.clone(),
    );
    let mut request = memory_request("v1");
    request.ingest_profile_id = "memory-card-tree-pipeline-v1".into();

    let result = service.ingest(request).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(embeddings.call_count(), 3);
    let connection = Connection::open(&database_url).unwrap();
    let columns = table_columns(&connection, "rag_memory_tree_nodes");
    for required in [
        "tree_node_code",
        "parent_node_code",
        "child_start_ordinal",
        "child_end_ordinal",
        "summary_text",
        "summary_embedding",
        "covered_text_unit_codes",
    ] {
        assert!(columns.iter().any(|column| column == required));
    }
    assert!(table_exists(&connection, "rag_memory_tree_nodes_fts"));
    assert!(table_exists(&connection, "rag_memory_tree_nodes_vec_d4096"));
    assert!(
        index_with_columns_exists(
            &connection,
            "rag_memory_tree_nodes",
            &[
                "source_code",
                "source_version",
                "scope_type",
                "scope_code",
                "parent_node_code",
                "ordinal",
            ],
            false,
        ),
        "tree storage should expose source/scope parent ordinal lookup index"
    );
    assert!(
        index_with_columns_exists(
            &connection,
            "rag_memory_tree_nodes",
            &[
                "source_code",
                "source_version",
                "scope_type",
                "scope_code",
                "path_key",
            ],
            false,
        ),
        "tree storage should expose source/scope path lookup index"
    );
    assert!(
        index_with_columns_exists(
            &connection,
            "rag_memory_tree_nodes",
            &[
                "source_code",
                "source_version",
                "scope_type",
                "scope_code",
                "tree_node_code",
            ],
            true,
        ),
        "tree storage unique key should be source/scope aware while preserving tree_node_code"
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT parent_node_code FROM rag_memory_tree_nodes ORDER BY ordinal LIMIT 1"
        ),
        "0"
    );
    assert!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_memory_tree_nodes WHERE summary_text LIKE '%Always cite source titles%'"
        ) >= 1
    );
    assert_eq!(
        serde_json::from_str::<Value>(&query_string(
            &connection,
            "SELECT covered_text_unit_codes FROM rag_memory_tree_nodes ORDER BY ordinal LIMIT 1"
        ))
        .unwrap(),
        json!(["memory-rag-rule:v1:0"])
    );
}

#[test]
fn sqlite_tree_rows_are_isolated_by_scope_for_same_tree_node_code() {
    let database_path = temp_database_path("rag-ingest-tree-scope-isolation");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    insert_tree_scope_corpus(
        &database_url,
        "tree-scope-corpus",
        "rag_tree_scope_tree_nodes",
    );

    repository
        .persist_ingest(tree_scope_command(
            "tree-scope-corpus",
            "SPACE",
            "TRON_CORE",
            "scope-space-unit",
            "Shared summary for space scope.",
        ))
        .unwrap();
    repository
        .persist_ingest(tree_scope_command(
            "tree-scope-corpus",
            "USER",
            "USR-AUTH-0001",
            "scope-user-unit",
            "Shared summary for user scope.",
        ))
        .unwrap();

    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_tree_scope_tree_nodes
             WHERE source_code = 'scope-collision-source'
               AND source_version = 'v1'
               AND path_key = '/'"
        ),
        2
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT summary_text FROM rag_tree_scope_tree_nodes
             WHERE scope_type = 'SPACE' AND scope_code = 'TRON_CORE'"
        ),
        "Shared summary for space scope."
    );
    assert_eq!(
        query_string(
            &connection,
            "SELECT summary_text FROM rag_tree_scope_tree_nodes
             WHERE scope_type = 'USER' AND scope_code = 'USR-AUTH-0001'"
        ),
        "Shared summary for user scope."
    );
}

#[tokio::test]
async fn sqlite_tree_rows_keep_stable_codes_but_do_not_collide_across_scopes() {
    let database_path = temp_database_path("rag-ingest-summary-tree-scope-isolation");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_memory_card_tree_table(&database_url, "rag_memory_tree_nodes");
    for scope_code in ["TRON_CORE", "TRON_ALT"] {
        repository
            .persist_ingest(tree_only_command(scope_code))
            .unwrap();
    }

    let connection = open_prepared_connection(&database_url);
    assert!(
        query_i64(
            &connection,
            "SELECT COUNT(DISTINCT tree_node_code) FROM rag_memory_tree_nodes WHERE source_code = 'memory-rag-rule'"
        ) < query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_memory_tree_nodes WHERE source_code = 'memory-rag-rule'"
        ),
        "tree_node_code stays stable across scopes"
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_memory_tree_nodes WHERE source_code = 'memory-rag-rule' AND source_version = 'v1'"
        ),
        2
    );
    assert!(table_exists(&connection, "rag_memory_tree_nodes_fts"));
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_memory_tree_nodes_vec_d4096"
        ),
        2,
        "vector rows should sync by the source/scope-aware rowid"
    );
}

fn tree_only_command(scope_code: &str) -> RagPersistIngestCommand {
    RagPersistIngestCommand {
        corpus_id: "memory-card".into(),
        profile_id: "memory-card-tree-pipeline-v1".into(),
        source_code: "memory-rag-rule".into(),
        source_version: "v1".into(),
        scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: scope_code.into(),
        },
        source_status: "ACTIVE".into(),
        title: Some("RAG citation rule".into()),
        content_hash: "hash-tree-scope".into(),
        body_hash: None,
        metadata: json!({}),
        domain_fields: Default::default(),
        related_graph: None,
        graph_write_policy: json!({}),
        text_units: Vec::new(),
        tree_nodes: vec![RagTreeNodeWrite {
            tree_node_code: "memory-rag-rule:v1:tree:stable-root".into(),
            parent_node_code: "0".into(),
            source_code: "memory-rag-rule".into(),
            source_version: "v1".into(),
            scope_type: "SPACE".into(),
            scope_code: scope_code.into(),
            path_key: "/".into(),
            level: 0,
            ordinal: 0,
            title: "RAG citation rule".into(),
            summary_text: format!("{scope_code} scoped tree summary"),
            summary_token_count: 4,
            summary_content_hash: format!("summary-hash-{scope_code}"),
            summary_fulltext_status: "INDEXED".into(),
            summary_embedding_status: "INDEXED".into(),
            summary_embedding_model_code: Some("fake-embedding".into()),
            summary_embedding_version: Some("v1".into()),
            summary_embedding_text_hash: Some(format!("embedding-hash-{scope_code}")),
            summary_embedding_dimension: Some(4096),
            summary_embedding: Some(encoded_embedding_4096()),
            covered_text_unit_codes: vec!["memory-rag-rule:v1:0".into()],
            covered_text_unit_count: 1,
            child_start_ordinal: 0,
            child_end_ordinal: 0,
            metadata: json!({}),
            status: "ACTIVE".into(),
        }],
        warnings: Vec::new(),
        workspace_code: scope_code.into(),
        user_code: "USR-LARGE-ID".into(),
        agent_code: "AGENT_CODEX".into(),
    }
}

fn encoded_embedding_4096() -> Vec<u8> {
    embedding_4096(0.25, 0.75)
        .into_iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[tokio::test]
async fn skill_ingest_writes_name_description_scope_fulltext_and_vector_fields() {
    let database_path = temp_database_path("rag-ingest-skill-success");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    let embeddings = FakeEmbeddingRuntime::fixed(embedding_4096(0.5, 0.25));
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        embeddings.clone(),
    );

    let result = service.ingest(skill_request("v1")).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 1);
    assert_eq!(result.fulltext_status, "INDEXED");
    assert_eq!(result.embedding_status, "INDEXED");
    assert_eq!(embeddings.call_count(), 1);

    let connection = Connection::open(&database_url).unwrap();
    let unit = query_skill_unit(&connection);
    assert_eq!(unit["scope_type"], json!("SPACE"));
    assert_eq!(unit["scope_code"], json!("TRON_CORE"));
    assert_eq!(unit["skill_name"], json!("rag-reviewer"));
    assert_eq!(
        unit["skill_description"],
        json!("Reviews RAG retrieval changes before release.")
    );
    assert_eq!(
        unit["text"],
        json!("Skill: rag-reviewer\nDescription: Reviews RAG retrieval changes before release.")
    );
    assert_index_payload_columns_absent(&connection, "rag_skill_text_units");
    assert!(table_exists(&connection, "rag_skill_text_units_fts"));
    assert!(table_exists(&connection, "rag_skill_text_units_vec_d4096"));
    assert!(
        !table_columns(&connection, "rag_skill_text_units")
            .iter()
            .any(|column| column == "workspace_code")
    );
}

#[tokio::test]
async fn project_doc_markdown_ingest_persists_document_fields_and_heading_split_units() {
    let database_path = temp_database_path("rag-ingest-project-doc");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2)).with_document(project_doc_markdown()),
    );

    let result = service.ingest(project_doc_request("v1")).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 2);
    assert_eq!(result.embedding_status, "INDEXED");

    let connection = Connection::open(&database_url).unwrap();
    let source = query_project_doc_source(&connection);
    assert_eq!(source["document_code"], json!("DOC_RAG_MVP"));
    assert_eq!(source["document_type"], json!("DESIGN"));
    assert_eq!(source["path"], json!("docs/rag-mvp.md"));
    assert_eq!(source["repo_code"], json!("REPO_TRON"));
    assert_eq!(source["module_code"], json!("MOD_RAG"));
    let metadata = serde_json::from_str::<Value>(&query_string(
        &connection,
        "SELECT metadata FROM rag_project_doc_sources WHERE source_code = 'doc-rag-mvp'",
    ))
    .unwrap();
    assert_eq!(metadata, json!({}));

    let headings = query_text_unit_headings(&connection);
    assert_eq!(headings, vec!["RAG MVP", "RAG MVP > Write Path"]);
    assert_eq!(
        query_text_unit_document_codes(&connection),
        vec!["DOC_RAG_MVP", "DOC_RAG_MVP"],
    );
    assert_index_payload_columns_absent(&connection, "rag_project_doc_text_units");
    assert!(table_exists(&connection, "rag_project_doc_text_units_fts"));
    assert!(table_exists(
        &connection,
        "rag_project_doc_text_units_vec_d4096"
    ));
}

#[tokio::test]
async fn project_doc_markdown_split_enriches_index_and_embedding_text_without_rewriting_raw_text() {
    let database_path = temp_database_path("rag-ingest-project-doc-markdown-index-text");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = CapturingIngestRepository::connect(&database_url);
    let persisted_commands = repository.persisted_commands.clone();
    set_project_doc_split_strategy(
        &database_url,
        "RAG_SPLIT_MARKDOWN_HEADING_V1",
        json!({
            "splitter": {
                "headingDepth": 6,
                "includeHeadingInIndexText": true
            }
        }),
    );
    let embeddings =
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2)).with_document(project_doc_markdown());
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        embeddings.clone(),
    );

    let result = service.ingest(project_doc_request("v1")).await.unwrap();

    assert_eq!(result.text_unit_count, 2);
    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_string(
            &connection,
            "SELECT text FROM rag_project_doc_text_units WHERE ordinal = 1"
        ),
        "## Write Path\nPersist synchronously."
    );
    assert_eq!(
        persisted_commands.lock().unwrap()[0].text_units[1].search_text,
        "RAG MVP > Write Path\n## Write Path\nPersist synchronously."
    );
    assert_eq!(
        embeddings.embedding_inputs()[1],
        "RAG MVP > Write Path\n## Write Path\nPersist synchronously."
    );
}

#[tokio::test]
async fn project_doc_explicit_plain_split_keeps_parsed_markdown_as_one_text_unit() {
    let database_path = temp_database_path("rag-ingest-project-doc-plain-split");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_project_doc_split_strategy(
        &database_url,
        "RAG_SPLIT_PLAIN_TEXT_V1",
        json!({
            "splitter": {
                "maxTokens": 800,
                "overlapTokens": 80
            }
        }),
    );
    let embeddings =
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2)).with_document(project_doc_markdown());
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        embeddings.clone(),
    );

    let result = service.ingest(project_doc_request("v1")).await.unwrap();

    assert_eq!(result.text_unit_count, 1);
    assert_eq!(embeddings.call_count(), 1);
    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(query_text_unit_headings(&connection), vec!["RAG MVP"]);
    assert_eq!(
        query_string(
            &connection,
            "SELECT text FROM rag_project_doc_text_units WHERE ordinal = 0"
        ),
        project_doc_markdown()
    );
}

#[tokio::test]
async fn project_doc_pdf_mineru_ingest_uses_mineru_markdown_for_split() {
    let database_path = temp_database_path("rag-ingest-project-doc-mineru-pdf");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    let runtime = FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2))
        .with_document("binary source placeholder")
        .with_mineru_document(project_doc_mineru_markdown());
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let result = service
        .ingest(project_doc_mineru_request(
            "v1",
            "PDF",
            "project-doc://TRON_CORE/docs/rag-mvp.pdf",
        ))
        .await
        .unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 2);
    assert_eq!(runtime.mineru_call_count(), 1);

    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_text_unit_headings(&connection),
        vec!["MinerU Parsed", "MinerU Parsed > Parsed Details"]
    );
    assert!(
        query_string(
            &connection,
            "SELECT text FROM rag_project_doc_text_units WHERE ordinal = 1"
        )
        .contains("Text extracted from PDF by MinerU.")
    );
}

#[tokio::test]
async fn project_doc_docx_mineru_ingest_accepts_same_profile() {
    let database_path = temp_database_path("rag-ingest-project-doc-mineru-docx");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    let runtime = FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2))
        .with_document("docx source placeholder")
        .with_mineru_document("# DOCX Parsed\nText extracted from DOCX by MinerU.");
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let result = service
        .ingest(project_doc_mineru_request(
            "v1",
            "DOCX",
            "project-doc://TRON_CORE/docs/rag-mvp.docx",
        ))
        .await
        .unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 1);
    assert_eq!(runtime.mineru_call_count(), 1);
}

#[tokio::test]
async fn project_doc_ingest_populates_distinct_vector_storage_when_configured() {
    let database_path = temp_database_path("rag-ingest-project-doc-separate-vector");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_project_doc_vector_table(&database_url, "rag_project_doc_vectors");
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2)).with_document(project_doc_markdown()),
    );

    let result = service.ingest(project_doc_request("v1")).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 2);
    assert_eq!(result.embedding_status, "INDEXED");

    let connection = open_prepared_connection(&database_url);
    assert!(table_exists(&connection, "rag_project_doc_vectors"));
    assert!(table_exists(
        &connection,
        "rag_project_doc_vectors_vec_d4096"
    ));
    assert_eq!(
        query_i64(&connection, "SELECT COUNT(*) FROM rag_project_doc_vectors"),
        2
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_project_doc_vectors_vec_d4096"
        ),
        2
    );
    assert_eq!(
        query_text_unit_codes(&connection, "rag_project_doc_vectors"),
        query_text_unit_codes(&connection, "rag_project_doc_text_units"),
    );
    assert_eq!(
        query_ordinals(&connection, "rag_project_doc_vectors"),
        vec![0, 1]
    );
}

#[tokio::test]
async fn project_doc_ingest_populates_distinct_fulltext_storage_when_configured() {
    let database_path = temp_database_path("rag-ingest-project-doc-separate-fulltext");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_project_doc_fulltext_table(&database_url, "rag_project_doc_fulltext");
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2)).with_document(project_doc_markdown()),
    );

    let result = service.ingest(project_doc_request("v1")).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 2);
    assert_eq!(result.fulltext_status, "INDEXED");

    let connection = open_prepared_connection(&database_url);
    assert!(table_exists(&connection, "rag_project_doc_fulltext"));
    assert!(table_exists(&connection, "rag_project_doc_fulltext_fts"));
    assert_eq!(
        query_i64(&connection, "SELECT COUNT(*) FROM rag_project_doc_fulltext"),
        2
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_project_doc_fulltext_fts"
        ),
        2
    );
    assert_eq!(
        query_text_unit_codes(&connection, "rag_project_doc_fulltext"),
        query_text_unit_codes(&connection, "rag_project_doc_text_units"),
    );
    assert!(
        query_string(
            &connection,
            "SELECT search_text FROM rag_project_doc_fulltext WHERE ordinal = 1"
        )
        .contains("Persist synchronously.")
    );
}

#[tokio::test]
async fn project_doc_markdown_heading_depth_controls_split_boundaries() {
    let depth_two_path = temp_database_path("rag-ingest-project-doc-heading-depth-two");
    let depth_two_url = depth_two_path.to_string_lossy().to_string();
    let depth_two_repository = SqliteRagRepository::connect(&depth_two_url).unwrap();
    set_project_doc_heading_depth(&depth_two_url, 2);
    let depth_two_service = RagIngestService::new(
        depth_two_repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.0))
            .with_document(project_doc_nested_markdown()),
    );

    let depth_two_result = depth_two_service
        .ingest(project_doc_request("v1"))
        .await
        .unwrap();

    assert_eq!(depth_two_result.text_unit_count, 1);
    let depth_two_connection = Connection::open(&depth_two_url).unwrap();
    assert_eq!(
        query_text_unit_headings(&depth_two_connection),
        vec!["RAG MVP > Scope"]
    );

    let depth_three_path = temp_database_path("rag-ingest-project-doc-heading-depth-three");
    let depth_three_url = depth_three_path.to_string_lossy().to_string();
    let depth_three_repository = SqliteRagRepository::connect(&depth_three_url).unwrap();
    set_project_doc_heading_depth(&depth_three_url, 3);
    let depth_three_service = RagIngestService::new(
        depth_three_repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.0))
            .with_document(project_doc_nested_markdown()),
    );

    let depth_three_result = depth_three_service
        .ingest(project_doc_request("v1"))
        .await
        .unwrap();

    assert_eq!(depth_three_result.text_unit_count, 2);
    let depth_three_connection = Connection::open(&depth_three_url).unwrap();
    assert_eq!(
        query_text_unit_headings(&depth_three_connection),
        vec!["RAG MVP > Scope > Ingest", "RAG MVP > Scope > Search"]
    );
}

#[tokio::test]
async fn project_doc_ingest_rejects_direct_markdown_payload() {
    let database_path = temp_database_path("rag-ingest-project-doc-direct-content");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(0.1, 0.2)),
    );
    let mut request = project_doc_request("v1");
    request.payload = json!({
        "source": {
            "code": "doc-rag-mvp",
            "version": "v1",
            "type": "PROJECT_DOC_REF"
        },
        "title": "RAG MVP",
        "document": {
            "uri": "project-doc://TRON_CORE/docs/rag-mvp.md",
            "format": "MARKDOWN"
        },
        "text": project_doc_markdown(),
        "domainFields": {
            "documentCode": "DOC_RAG_MVP",
            "documentType": "DESIGN",
            "path": "docs/rag-mvp.md",
            "repoCode": "REPO_TRON",
            "moduleCode": "MOD_RAG",
            "moduleName": "RAG"
        }
    });

    assert_eq!(
        service.ingest(request).await.unwrap_err(),
        RagError::IngestRequestInvalid
    );
}

#[tokio::test]
async fn same_source_version_is_idempotent_and_multiple_versions_coexist() {
    let database_path = temp_database_path("rag-ingest-idempotency");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::fixed(embedding_4096(1.0, 0.0)),
    );

    service.ingest(memory_request("v1")).await.unwrap();
    service.ingest(memory_request("v1")).await.unwrap();
    service.ingest(memory_request("v2")).await.unwrap();

    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(DISTINCT source_code || ':' || source_version || ':' || scope_type || ':' || scope_code) FROM rag_memory_text_units"
        ),
        2
    );
    assert_eq!(
        query_i64(&connection, "SELECT COUNT(*) FROM rag_memory_text_units"),
        2
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(DISTINCT source_code || ':' || source_version || ':' || scope_type || ':' || scope_code) FROM rag_memory_text_units WHERE source_version = 'v1'"
        ),
        1
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(DISTINCT source_code || ':' || source_version || ':' || scope_type || ':' || scope_code) FROM rag_memory_text_units WHERE source_version = 'v2'"
        ),
        1
    );
}

#[tokio::test]
async fn embedding_failure_degrades_project_doc_and_memory_with_warning() {
    let doc_path = temp_database_path("rag-ingest-project-doc-vector-failure");
    let doc_url = doc_path.to_string_lossy().to_string();
    let doc_repository = SqliteRagRepository::connect(&doc_url).unwrap();
    let doc_service = RagIngestService::new(
        doc_repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::failing("fake vector unavailable")
            .with_document(project_doc_markdown()),
    );

    let doc_result = doc_service.ingest(project_doc_request("v1")).await.unwrap();

    assert_eq!(doc_result.status, "ACTIVE");
    assert_eq!(doc_result.embedding_status, "FAILED");
    assert_eq!(doc_result.warnings.len(), 1);
    assert_eq!(
        doc_result.warnings[0]["code"],
        json!("RAG_VECTOR_INDEX_WARNING")
    );
    let doc_connection = Connection::open(&doc_url).unwrap();
    assert_eq!(
        query_i64(&doc_connection, "SELECT COUNT(*) FROM rag_diagnostics"),
        1
    );
    assert_eq!(
        query_string(
            &doc_connection,
            "SELECT status FROM rag_project_doc_sources WHERE source_code = 'doc-rag-mvp'"
        ),
        "ACTIVE"
    );

    let memory_path = temp_database_path("rag-ingest-memory-vector-failure");
    let memory_url = memory_path.to_string_lossy().to_string();
    let memory_repository = SqliteRagRepository::connect(&memory_url).unwrap();
    let memory_service = RagIngestService::new(
        memory_repository,
        DefaultRagStrategyRegistry::default(),
        FakeEmbeddingRuntime::failing("fake vector unavailable"),
    );

    let memory_result = memory_service.ingest(memory_request("v1")).await.unwrap();

    assert_eq!(memory_result.status, "ACTIVE");
    assert_eq!(memory_result.embedding_status, "FAILED");
    assert_eq!(memory_result.warnings.len(), 1);
    assert_eq!(
        memory_result.warnings[0]["code"],
        json!("RAG_VECTOR_INDEX_WARNING")
    );
    let memory_connection = Connection::open(&memory_url).unwrap();
    assert_eq!(
        query_i64(&memory_connection, "SELECT COUNT(*) FROM rag_diagnostics"),
        1
    );
    assert_eq!(
        query_string(
            &memory_connection,
            "SELECT source_status FROM rag_memory_text_units WHERE source_code = 'memory-rag-rule' LIMIT 1"
        ),
        "ACTIVE"
    );
}

#[test]
fn ingest_profile_compiler_rejects_invalid_dag_contracts() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();
    let corpus = repository.get_corpus("memory-card").unwrap().unwrap();
    let registry = DefaultRagStrategyRegistry::default();

    assert!(
        compile_ingest_profile(&ingest_profile("valid", valid_dag()), &corpus, &registry).is_ok()
    );

    for (name, config, expected) in [
        (
            "invalid-corpus",
            valid_dag(),
            RagError::IngestProfileInvalid,
        ),
        ("cycle", cycle_dag(), RagError::IngestProfileInvalid),
        (
            "missing-stage",
            missing_stage_dag(),
            RagError::IngestProfileInvalid,
        ),
        (
            "unregistered-strategy",
            unregistered_strategy_dag(),
            RagError::IngestProfileInvalid,
        ),
        (
            "missing-persist",
            missing_persist_dag(),
            RagError::IngestProfileInvalid,
        ),
    ] {
        let mut profile = ingest_profile(name, config);
        if name == "invalid-corpus" {
            profile.primary_corpus_id = Some("project-doc".into());
            profile.corpus_ids = vec!["project-doc".into()];
        }
        assert_eq!(
            compile_ingest_profile(&profile, &corpus, &registry).unwrap_err(),
            expected,
            "{name} should be rejected",
        );
    }
}

#[test]
fn ingest_profile_compiler_allows_default_llm_summary_model() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();
    let corpus = repository.get_corpus("memory-card").unwrap().unwrap();
    let registry = LlmSummaryTestRegistry;

    for (name, summary_config) in [
        ("missing-model", None),
        ("empty-model", Some(json!({ "modelCode": " " }))),
    ] {
        let compiled = compile_ingest_profile(
            &ingest_profile(name, llm_summary_tree_config(summary_config)),
            &corpus,
            &registry,
        )
        .expect("LLM summary profile should compile with the default text model");
        assert_eq!(
            compiled
                .stage_metadata
                .get("summarizeTree")
                .expect("summary stage metadata should compile")
                .strategy_id,
            "RAG_TREE_LLM_SUMMARY_V1",
            "{name} should use the LLM summary strategy",
        );
    }

    let compiled = compile_ingest_profile(
        &ingest_profile(
            "valid-llm-summary",
            llm_summary_tree_config(Some(json!({ "modelCode": "summary-model-alpha" }))),
        ),
        &corpus,
        &registry,
    )
    .expect("LLM summary profile should compile when modelCode is present");
    assert_eq!(
        compiled
            .stage_metadata
            .get("summarizeTree")
            .expect("summary stage metadata should compile")
            .strategy_id,
        "RAG_TREE_LLM_SUMMARY_V1"
    );
}

#[test]
fn ingest_profile_compiler_accepts_legacy_mineru_format_config_as_pdf_docx() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();
    let corpus = repository.get_corpus("project-doc").unwrap().unwrap();
    let registry = DefaultRagStrategyRegistry::default();
    let mut profile = repository
        .get_profile("project-doc-mineru-ingest-v1")
        .unwrap()
        .unwrap();

    profile.profile_id = "project-ingest-test-1".into();
    profile.config = legacy_mineru_pipeline_config();

    let compiled = compile_ingest_profile(&profile, &corpus, &registry).unwrap();

    assert_eq!(compiled.document_formats, vec!["PDF", "DOCX"]);
    assert!(compiled.supports_document_format("PDF"));
    assert!(compiled.supports_document_format("DOCX"));
    assert!(!compiled.supports_document_format("MARKDOWN"));
}

#[test]
fn ingest_boundary_does_not_add_search_debug_retry_or_graph_memory_flow_behavior() {
    let app_source = include_str!("../src/app.rs");
    let rag_source = include_str!("../src/application/rag/service.rs");

    for forbidden_route in [
        "/api/v1/rag/search",
        "/api/v1/rag/debug",
        "/api/v1/rag/retry",
    ] {
        assert!(
            !app_source.contains(forbidden_route),
            "AC-RID-07: ingest must not register {forbidden_route}",
        );
    }
    for forbidden_behavior in ["SummaryTree", "GraphService", "MemoryFlow"] {
        assert!(
            !rag_source.contains(forbidden_behavior),
            "AC-RID-07: ingest service must not add {forbidden_behavior} behavior",
        );
    }
}

#[test]
fn ingest_executor_uses_artifact_bundle_for_stage_handoffs() {
    let ingest_source = include_str!("../src/application/rag/ingest.rs");
    let artifact_source = include_str!("../src/application/rag/ingest_artifact.rs");

    assert!(ingest_source.contains("ArtifactBundle::from_request"));
    assert!(ingest_source.contains("execute_ingest_stages"));
    assert!(
        !ingest_source.contains("Option<RagResolvedDocument>"),
        "stage handoffs should stay inside ArtifactBundle instead of exposing stage-private types"
    );
    assert!(artifact_source.contains("pub(crate) struct RagArtifact"));
    assert!(artifact_source.contains("pub(crate) enum RagArtifactPayload"));
    assert!(
        ingest_source.contains("stage_results"),
        "ingest should return additive stageResults without changing the request payload"
    );
}

#[test]
fn ingest_executor_uses_generic_pipeline_dispatch_instead_of_fixed_stage_match() {
    let executor_source = [
        include_str!("../src/application/rag/ingest/executor.rs"),
        include_str!("../src/application/rag/ingest/executor/runtime.rs"),
        include_str!("../src/application/rag/ingest/executor/stage_helpers.rs"),
    ]
    .join("\n");

    assert!(
        !executor_source.contains("match stage_id.as_str()"),
        "ingest runtime should dispatch registered stage executors through the generic DAG executor"
    );
    assert!(
        executor_source.contains("RagPipelineExecutor"),
        "ingest runtime should use the shared pipeline executor"
    );
}

#[test]
fn summarize_tree_executor_consumes_tree_extracted_artifact_contract() {
    let tree_stage_source = include_str!("../src/application/rag/ingest/executor/tree_stages.rs");

    assert!(
        tree_stage_source.contains("store.get::<Value>(\"tree.extracted\")"),
        "summarizeTree should consume the catalog-declared tree.extracted artifact"
    );
    assert!(
        tree_stage_source.contains("tree_extracted_nodes"),
        "summarizeTree should build summary inputs from extracted tree nodes"
    );
    assert!(
        tree_stage_source.contains("tree_parent_child_edges"),
        "summarizeTree should preserve extracted tree parent-child edges"
    );
    assert!(
        !tree_stage_source.contains("prepared.tree_hints.iter().collect::<Vec<_>>()"),
        "summarizeTree must not rebuild LLM summary ordering directly from prepared tree hints"
    );
    assert!(
        !tree_stage_source.contains("tree_children_by_parent(prepared)"),
        "summarizeTree must not infer child edges from prepared tree hints"
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryCall {
    model_code: Option<String>,
    prompt: String,
}

#[derive(Clone)]
struct FakeEmbeddingRuntime {
    result: Arc<Mutex<Result<RagEmbeddingResult, String>>>,
    documents: Arc<Mutex<VecDeque<Result<RagResolvedDocument, RagError>>>>,
    mineru_documents: Arc<Mutex<VecDeque<Result<RagResolvedDocument, RagError>>>>,
    summary_outputs: Arc<Mutex<VecDeque<Result<String, String>>>>,
    summary_calls: Arc<Mutex<Vec<SummaryCall>>>,
    calls: Arc<AtomicUsize>,
    mineru_calls: Arc<AtomicUsize>,
    last_model_code: Arc<Mutex<Option<String>>>,
    embedding_inputs: Arc<Mutex<Vec<String>>>,
    fail_after_successes: Option<usize>,
    fail_after_message: String,
}

impl FakeEmbeddingRuntime {
    fn fixed(embedding: Vec<f32>) -> Self {
        Self {
            result: Arc::new(Mutex::new(Ok(RagEmbeddingResult {
                model_code: "workspace-default-embedding".into(),
                embedding,
            }))),
            documents: Arc::new(Mutex::new(VecDeque::new())),
            mineru_documents: Arc::new(Mutex::new(VecDeque::new())),
            summary_outputs: Arc::new(Mutex::new(VecDeque::new())),
            summary_calls: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(AtomicUsize::new(0)),
            mineru_calls: Arc::new(AtomicUsize::new(0)),
            last_model_code: Arc::new(Mutex::new(None)),
            embedding_inputs: Arc::new(Mutex::new(Vec::new())),
            fail_after_successes: None,
            fail_after_message: String::new(),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            result: Arc::new(Mutex::new(Err(message.to_string()))),
            documents: Arc::new(Mutex::new(VecDeque::new())),
            mineru_documents: Arc::new(Mutex::new(VecDeque::new())),
            summary_outputs: Arc::new(Mutex::new(VecDeque::new())),
            summary_calls: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(AtomicUsize::new(0)),
            mineru_calls: Arc::new(AtomicUsize::new(0)),
            last_model_code: Arc::new(Mutex::new(None)),
            embedding_inputs: Arc::new(Mutex::new(Vec::new())),
            fail_after_successes: None,
            fail_after_message: String::new(),
        }
    }

    fn fail_after_successes(mut self, success_count: usize, message: &str) -> Self {
        self.fail_after_successes = Some(success_count);
        self.fail_after_message = message.to_string();
        self
    }

    fn with_summary_outputs<I, S>(self, outputs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.summary_outputs
            .lock()
            .unwrap()
            .extend(outputs.into_iter().map(|output| Ok(output.into())));
        self
    }

    fn with_document(self, content_text: &str) -> Self {
        self.documents
            .lock()
            .unwrap()
            .push_back(Ok(RagResolvedDocument {
                document_uri: "project-doc://TRON_CORE/docs/rag-mvp.md".into(),
                content_length: content_text.len(),
                content_text: content_text.into(),
            }));
        self
    }

    fn with_mineru_document(self, content_text: &str) -> Self {
        self.mineru_documents
            .lock()
            .unwrap()
            .push_back(Ok(RagResolvedDocument {
                document_uri: "project-doc://TRON_CORE/docs/rag-mvp.pdf".into(),
                content_length: content_text.len(),
                content_text: content_text.into(),
            }));
        self
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn mineru_call_count(&self) -> usize {
        self.mineru_calls.load(Ordering::SeqCst)
    }

    fn last_model_code(&self) -> Option<String> {
        self.last_model_code.lock().unwrap().clone()
    }

    fn embedding_inputs(&self) -> Vec<String> {
        self.embedding_inputs.lock().unwrap().clone()
    }

    fn summary_calls(&self) -> Vec<SummaryCall> {
        self.summary_calls.lock().unwrap().clone()
    }
}

impl RagEmbeddingRuntime for FakeEmbeddingRuntime {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let call_index = self.calls.fetch_add(1, Ordering::SeqCst);
            self.embedding_inputs.lock().unwrap().push(call.input);
            *self.last_model_code.lock().unwrap() = call.model_code;
            if self
                .fail_after_successes
                .is_some_and(|success_count| call_index >= success_count)
            {
                return Err(self.fail_after_message.clone());
            }
            self.result.lock().unwrap().clone()
        })
    }
}

impl RagTreeSummaryRuntime for FakeEmbeddingRuntime {
    fn summarize_tree<'a>(
        &'a self,
        call: RagTreeSummaryCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagTreeSummaryResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.summary_calls.lock().unwrap().push(SummaryCall {
                model_code: call.model_code.clone(),
                prompt: call.prompt,
            });
            let summary_text = self
                .summary_outputs
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err("fake summary output missing".into()))?;
            Ok(RagTreeSummaryResult {
                model_code: call
                    .model_code
                    .unwrap_or_else(|| "workspace-default-summary".into()),
                summary_text,
            })
        })
    }
}

impl RagDocumentSourceRuntime for FakeEmbeddingRuntime {
    fn resolve_project_doc_markdown<'a>(
        &'a self,
        _call: RagDocumentSourceCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async move {
            self.documents
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Err(RagError::SourceResolveFailed))
        })
    }

    fn parse_project_doc_with_mineru<'a>(
        &'a self,
        call: RagDocumentParseCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async move {
            self.mineru_calls.fetch_add(1, Ordering::SeqCst);
            let mut document = self
                .mineru_documents
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Err(RagError::DocumentParseFailed))?;
            document.document_uri = call.document_uri;
            Ok(document)
        })
    }
}

#[derive(Clone)]
struct FakeLlmModelResolver;

impl LlmRuntimeModelResolver for FakeLlmModelResolver {
    fn text_runtime_model_config(
        &self,
        _workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        Ok(Some(llm_text_config(
            model_code.unwrap_or("workspace-default-summary"),
        )))
    }

    fn embedding_runtime_model_config(
        &self,
        _workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        Ok(Some(llm_embedding_config(
            model_code.unwrap_or("workspace-default-embedding"),
        )))
    }

    fn rerank_runtime_model_config(
        &self,
        _workspace_code: &str,
        _model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        Ok(None)
    }
}

#[derive(Clone)]
struct FakeLlmClient {
    generations: Arc<Mutex<VecDeque<Result<String, String>>>>,
    embeddings: Arc<Mutex<VecDeque<Result<Vec<f32>, String>>>>,
    generate_inputs: Arc<Mutex<Vec<String>>>,
}

impl FakeLlmClient {
    fn new() -> Self {
        Self {
            generations: Arc::new(Mutex::new(VecDeque::new())),
            embeddings: Arc::new(Mutex::new(VecDeque::new())),
            generate_inputs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_generations<I, S>(self, outputs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.generations
            .lock()
            .unwrap()
            .extend(outputs.into_iter().map(|output| Ok(output.into())));
        self
    }

    fn with_embeddings<I>(self, embeddings: I) -> Self
    where
        I: IntoIterator<Item = Vec<f32>>,
    {
        self.embeddings
            .lock()
            .unwrap()
            .extend(embeddings.into_iter().map(Ok));
        self
    }

    fn generate_inputs(&self) -> Vec<String> {
        self.generate_inputs.lock().unwrap().clone()
    }
}

impl LlmRuntimeClient for FakeLlmClient {
    fn generate<'a>(
        &'a self,
        _config: &'a RuntimeModelConfig,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            self.generate_inputs
                .lock()
                .unwrap()
                .push(prompt.to_string());
            self.generations
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err("missing fake text generation response".into()))
        })
    }

    fn embed<'a>(
        &'a self,
        _config: &'a RuntimeModelConfig,
        _input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embeddings
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err("missing fake embedding response".into()))
        })
    }

    fn rerank<'a>(
        &'a self,
        _config: &'a RuntimeModelConfig,
        _query: &'a str,
        _documents: &'a [String],
        _top_n: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        Box::pin(async move { Ok(Vec::new()) })
    }
}

fn llm_text_config(model_code: &str) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: model_code.into(),
        protocol: ModelProtocol::OpenAiCompatible,
        model_type: ModelType::Text,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "summary-test".into(),
        thinking_enabled: false,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: Some(0),
    }
}

fn llm_embedding_config(model_code: &str) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: model_code.into(),
        protocol: ModelProtocol::OpenAiCompatible,
        model_type: ModelType::Embedding,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "embed-test".into(),
        thinking_enabled: false,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: Some(0),
    }
}

#[derive(Clone)]
struct LlmSummaryTestRegistry;

impl RagStrategyRegistry for LlmSummaryTestRegistry {
    fn resolve_stage(&self, strategy_id: &str) -> StrategyRegistration {
        if strategy_id == "RAG_TREE_LLM_SUMMARY_V1" {
            return StrategyRegistration {
                strategy_id: strategy_id.into(),
                status: StrategyRegistrationStatus::Enabled,
            };
        }
        DefaultRagStrategyRegistry::default().resolve_stage(strategy_id)
    }

    fn resolve_payload_strategy(&self, strategy_id: &str) -> StrategyRegistration {
        DefaultRagStrategyRegistry::default().resolve_payload_strategy(strategy_id)
    }

    fn resolve_retriever(&self, strategy_id: &str) -> StrategyRegistration {
        DefaultRagStrategyRegistry::default().resolve_retriever(strategy_id)
    }

    fn resolve_policy(&self, strategy_id: &str) -> StrategyRegistration {
        DefaultRagStrategyRegistry::default().resolve_policy(strategy_id)
    }
}

fn memory_request(source_version: &str) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: "memory-card".into(),
        ingest_profile_id: "memory-card-ingest-v1".into(),
        workspace_code: "TRON_CORE".into(),
        access_context: RagIngestAccessContext {
            user_code: "USR-LARGE-ID".into(),
            agent_code: "AGENT_CODEX".into(),
            workspace_code: "TRON_CORE".into(),
        },
        target_scope: RagIngestScope {
            scope_type: "USER".into(),
            scope_code: "USR-LARGE-ID".into(),
        },
        source: RagIngestSource {
            code: "memory-rag-rule".into(),
            version: source_version.into(),
            source_type: String::new(),
        },
        payload: json!({
            "source": {
                "code": "memory-rag-rule",
                "version": source_version,
                "type": "MEMORY_CARD"
            },
            "title": "RAG citation rule",
            "text": "Always cite source titles in RAG answers.",
            "metadata": {
                "seed": "contract-test",
                "source": "rag-ingest"
            },
            "domainFields": {
                "version": source_version,
                "memoryType": "RULE",
                "memoryStatus": "ACTIVE",
                "reviewStatus": "APPROVED",
                "importanceScore": 90
            }
        }),
    }
}

fn memory_markdown_tree_request(source_version: &str) -> RagIngestRequest {
    let mut request = memory_request(source_version);
    request.payload["text"] = json!("# RAG MVP\nIntro.\n\n## Write Path\nPersist synchronously.");
    request
}

fn skill_request(source_version: &str) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: "skill".into(),
        ingest_profile_id: "skill-ingest-v1".into(),
        workspace_code: "TRON_CORE".into(),
        access_context: RagIngestAccessContext {
            user_code: "USR-LARGE-ID".into(),
            agent_code: "AGENT_CODEX".into(),
            workspace_code: "TRON_CORE".into(),
        },
        target_scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
        },
        source: RagIngestSource {
            code: "skill:TRON_CORE:rag-reviewer".into(),
            version: source_version.into(),
            source_type: String::new(),
        },
        payload: json!({
            "source": {
                "code": "skill:TRON_CORE:rag-reviewer",
                "version": source_version,
                "type": "SKILL"
            },
            "title": "rag-reviewer",
            "text": "Skill: rag-reviewer\nDescription: Reviews RAG retrieval changes before release.",
            "metadata": {
                "source": "skill-registry"
            },
            "domainFields": {
                "skillName": "rag-reviewer",
                "skillDescription": "Reviews RAG retrieval changes before release."
            }
        }),
    }
}

fn project_doc_request(source_version: &str) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: "project-doc".into(),
        ingest_profile_id: "project-doc-ingest-v1".into(),
        workspace_code: "TRON_CORE".into(),
        access_context: RagIngestAccessContext {
            user_code: "USR-LARGE-ID".into(),
            agent_code: "AGENT_CODEX".into(),
            workspace_code: "TRON_CORE".into(),
        },
        target_scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
        },
        source: RagIngestSource {
            code: "doc-rag-mvp".into(),
            version: source_version.into(),
            source_type: String::new(),
        },
        payload: json!({
            "source": {
                "code": "doc-rag-mvp",
                "version": source_version,
                "type": "PROJECT_DOC_REF"
            },
            "title": "RAG MVP",
            "document": {
                "uri": "project-doc://TRON_CORE/docs/rag-mvp.md",
                "format": "MARKDOWN"
            },
            "domainFields": {
                "documentCode": "DOC_RAG_MVP",
                "documentType": "DESIGN",
                "path": "docs/rag-mvp.md",
                "repoCode": "REPO_TRON",
                "moduleCode": "MOD_RAG",
                "moduleName": "RAG"
            }
        }),
    }
}

fn project_doc_mineru_request(
    source_version: &str,
    format: &str,
    document_uri: &str,
) -> RagIngestRequest {
    let mut request = project_doc_request(source_version);
    request.ingest_profile_id = "project-doc-mineru-ingest-v1".into();
    request.payload["document"]["uri"] = json!(document_uri);
    request.payload["document"]["format"] = json!(format);
    request.payload["domainFields"]["path"] =
        json!(document_uri.trim_start_matches("project-doc://TRON_CORE/"));
    request
}

fn project_doc_markdown() -> &'static str {
    "# RAG MVP\nIntro.\n\n## Write Path\nPersist synchronously."
}

fn project_doc_mineru_markdown() -> &'static str {
    "# MinerU Parsed\nIntro extracted from PDF.\n\n## Parsed Details\nText extracted from PDF by MinerU."
}

fn project_doc_nested_markdown() -> &'static str {
    "# RAG MVP\n## Scope\n### Ingest\nRead files.\n### Search\nFind chunks."
}

fn set_project_doc_heading_depth(database_url: &str, heading_depth: u64) {
    let connection = Connection::open(database_url).unwrap();
    let config_json = query_string(
        &connection,
        "SELECT config_json FROM rag_profiles WHERE profile_id = 'project-doc-ingest-v1'",
    );
    let mut config = serde_json::from_str::<Value>(&config_json).unwrap();
    config["stages"]["splitText"]["config"]["splitter"]["headingDepth"] = json!(heading_depth);
    connection
        .execute(
            "UPDATE rag_profiles SET config_json = ?1 WHERE profile_id = 'project-doc-ingest-v1'",
            [config.to_string()],
        )
        .unwrap();
}

fn set_project_doc_split_strategy(database_url: &str, strategy_id: &str, stage_config: Value) {
    let connection = Connection::open(database_url).unwrap();
    let config_json = query_string(
        &connection,
        "SELECT config_json FROM rag_profiles WHERE profile_id = 'project-doc-ingest-v1'",
    );
    let mut config = serde_json::from_str::<Value>(&config_json).unwrap();
    config["stages"]["splitText"]["strategyId"] = json!(strategy_id);
    config["stages"]["splitText"]["config"] = stage_config;
    connection
        .execute(
            "UPDATE rag_profiles SET config_json = ?1 WHERE profile_id = 'project-doc-ingest-v1'",
            [config.to_string()],
        )
        .unwrap();
}

fn set_project_doc_vector_table(database_url: &str, table_name: &str) {
    set_project_doc_segment_table(
        database_url,
        "vector",
        "project-doc-vector-store",
        table_name,
    );
}

fn set_project_doc_fulltext_table(database_url: &str, table_name: &str) {
    set_project_doc_segment_table(
        database_url,
        "fulltext",
        "project-doc-fulltext-store",
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

fn set_memory_card_tree_table(database_url: &str, table_name: &str) {
    let connection = Connection::open(database_url).unwrap();
    let binding_json = query_string(
        &connection,
        "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'memory-card'",
    );
    let mut binding = serde_json::from_str::<Value>(&binding_json).unwrap();
    binding["bindings"]["tree"] = json!("memory-card-tree-store");
    binding["targets"]
        .as_array_mut()
        .expect("storage targets should be an array")
        .push(json!({
            "id": "memory-card-tree-store",
            "backend": "SQLITE",
            "layout": "TABLE",
            "table": table_name
        }));
    connection
        .execute(
            "UPDATE rag_corpora
             SET storage_binding_json = ?1
             WHERE corpus_id = 'memory-card'",
            params![binding.to_string()],
        )
        .unwrap();
}

fn insert_tree_scope_corpus(database_url: &str, corpus_id: &str, tree_table: &str) {
    let connection = Connection::open(database_url).unwrap();
    let text_table = "rag_tree_scope_text_units";
    let diagnostics_table = "rag_tree_scope_diagnostics";
    connection
        .execute(
            "INSERT INTO rag_corpora (
                corpus_id, display_name, status, description, capabilities_json,
                filter_schema_json, field_mapping_json, storage_binding_json, created_at, updated_at
            )
            VALUES (?1, 'Tree Scope Corpus', 'enabled', 'Tree scope test corpus.',
                ?2, '{\"fields\":{}}', ?3, ?4, 1, 1)",
            params![
                corpus_id,
                json!({
                    "fulltext": true,
                    "vector": true,
                    "tree": true,
                    "graph": false
                })
                .to_string(),
                json!({
                    "source": {
                        "sourceCode": "source_code",
                        "sourceVersion": "source_version",
                        "sourceType": "source_type",
                        "scopeType": "scope_type",
                        "scopeCode": "scope_code",
                        "status": "status",
                        "title": "title",
                        "metadata": "metadata"
                    },
                    "textUnit": {
                        "sourceCode": "source_code",
                        "sourceVersion": "source_version",
                        "scopeType": "scope_type",
                        "scopeCode": "scope_code",
                        "text": "text",
                        "textUnitStatus": "status"
                    }
                })
                .to_string(),
                json!({
                    "mode": "TARGETED",
                    "bindings": {
                        "source": "tree-scope-text-store",
                        "textUnit": "tree-scope-text-store",
                        "fulltext": "tree-scope-text-store",
                        "vector": "tree-scope-text-store",
                        "tree": "tree-scope-tree-store",
                        "diagnostics": "tree-scope-diagnostics-store"
                    },
                    "targets": [
                        {
                            "id": "tree-scope-text-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": text_table
                        },
                        {
                            "id": "tree-scope-tree-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": tree_table
                        },
                        {
                            "id": "tree-scope-diagnostics-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": diagnostics_table
                        }
                    ]
                })
                .to_string(),
            ],
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

fn ingest_profile(profile_id: &str, config: Value) -> RagProfileRecord {
    RagProfileRecord {
        profile_id: profile_id.into(),
        profile_type: RagProfileType::Ingest,
        status: RagStatus::Enabled,
        primary_corpus_id: Some("memory-card".into()),
        corpus_ids: vec!["memory-card".into()],
        required_source_version: None,
        config,
        filters: json!({}),
        degrade_policy: "FAIL_FAST".into(),
    }
}

fn valid_dag() -> Value {
    json!({
        "sourceInputPolicy": {
            "supportedSourceTypes": ["MEMORY_CARD"]
        },
        "dag": {
            "nodes": ["validateSource", "preparePayload", "splitText", "fulltextIndex", "vectorIndex", "persistSource"],
            "edges": [
                { "from": "validateSource", "to": "preparePayload" },
                { "from": "preparePayload", "to": "splitText" },
                { "from": "splitText", "to": "fulltextIndex" },
                { "from": "fulltextIndex", "to": "vectorIndex" },
                { "from": "vectorIndex", "to": "persistSource" }
            ]
        },
        "stages": {
            "validateSource": { "strategyId": "RAG_VALIDATE_SOURCE" },
            "preparePayload": { "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1" },
            "splitText": { "strategyId": "RAG_SPLIT_TEXT" },
            "fulltextIndex": { "strategyId": "RAG_FULLTEXT_INDEX" },
            "vectorIndex": { "strategyId": "RAG_VECTOR_INDEX" },
            "persistSource": { "strategyId": "RAG_PERSIST_SOURCE" }
        }
    })
}

fn legacy_mineru_pipeline_config() -> Value {
    json!({
        "sourceInputPolicy": {
            "supportedSourceTypes": ["PROJECT_DOC_REF"],
            "directContentEnabled": false
        },
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {
                    "nodeId": "validate-source",
                    "stageType": "VALIDATE_SOURCE",
                    "strategyId": "RAG_VALIDATE_SOURCE"
                },
                {
                    "nodeId": "prepare-payload",
                    "stageType": "PREPARE_PAYLOAD",
                    "strategyId": "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1"
                },
                {
                    "nodeId": "resolve-source",
                    "stageType": "RESOLVE_SOURCE",
                    "strategyId": "PROJECT_DOC_RESOLVE_SOURCE"
                },
                {
                    "nodeId": "parse-document",
                    "stageType": "PARSE_DOCUMENT",
                    "strategyId": "RAG_PARSE_MINERU_DOCUMENT",
                    "config": {
                        "format": "MARKDOWN",
                        "unsupportedFormatPolicy": "FAIL_FAST"
                    }
                },
                {
                    "nodeId": "split-text",
                    "stageType": "SPLIT_TEXT",
                    "strategyId": "RAG_SPLIT_TEXT"
                },
                {
                    "nodeId": "fulltext-index",
                    "stageType": "FULLTEXT_INDEX",
                    "strategyId": "RAG_FULLTEXT_INDEX"
                },
                {
                    "nodeId": "vector-index",
                    "stageType": "VECTOR_INDEX",
                    "strategyId": "RAG_VECTOR_INDEX"
                },
                {
                    "nodeId": "persist-source",
                    "stageType": "PERSIST_SOURCE",
                    "strategyId": "RAG_PERSIST_SOURCE"
                }
            ],
            "edges": [
                { "from": "validate-source", "to": "prepare-payload" },
                { "from": "prepare-payload", "to": "resolve-source" },
                { "from": "resolve-source", "to": "parse-document" },
                { "from": "parse-document", "to": "split-text" },
                { "from": "split-text", "to": "fulltext-index" },
                { "from": "split-text", "to": "vector-index" },
                { "from": "fulltext-index", "to": "persist-source" },
                { "from": "vector-index", "to": "persist-source" }
            ]
        },
        "embeddingVersion": "v1"
    })
}

fn cycle_dag() -> Value {
    let mut value = valid_dag();
    value["dag"]["edges"] = json!([
        { "from": "validateSource", "to": "preparePayload" },
        { "from": "preparePayload", "to": "validateSource" }
    ]);
    value
}

fn missing_stage_dag() -> Value {
    let mut value = valid_dag();
    value["dag"]["nodes"] = json!(["validateSource", "splitText", "persistSource"]);
    value["stages"].as_object_mut().unwrap().remove("splitText");
    value
}

fn unregistered_strategy_dag() -> Value {
    let mut value = valid_dag();
    value["stages"]["vectorIndex"]["strategyId"] = json!("RAG_UNKNOWN_VECTOR");
    value
}

fn missing_persist_dag() -> Value {
    let mut value = valid_dag();
    value["dag"]["nodes"] = json!([
        "validateSource",
        "splitText",
        "fulltextIndex",
        "vectorIndex"
    ]);
    value["dag"]["edges"] = json!([
        { "from": "validateSource", "to": "splitText" },
        { "from": "splitText", "to": "fulltextIndex" },
        { "from": "fulltextIndex", "to": "vectorIndex" }
    ]);
    value["stages"]
        .as_object_mut()
        .unwrap()
        .remove("persistSource");
    value
}

fn llm_summary_tree_config(summary_config: Option<Value>) -> Value {
    let mut summarize_tree_node = json!({
        "nodeId": "summarize-tree",
        "stageType": "SUMMARIZE_TREE",
        "strategyId": "RAG_TREE_LLM_SUMMARY_V1"
    });
    if let Some(config) = summary_config {
        summarize_tree_node["config"] = config;
    }
    json!({
        "sourceInputPolicy": {
            "supportedSourceTypes": ["MEMORY_CARD"]
        },
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {
                    "nodeId": "validate-source",
                    "stageType": "VALIDATE_SOURCE",
                    "strategyId": "RAG_VALIDATE_SOURCE"
                },
                {
                    "nodeId": "prepare-payload",
                    "stageType": "PREPARE_PAYLOAD",
                    "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1"
                },
                {
                    "nodeId": "split-text",
                    "stageType": "SPLIT_TEXT",
                    "strategyId": "RAG_SPLIT_MARKDOWN_HEADING_V1",
                    "config": {
                        "splitter": {
                            "headingDepth": 6,
                            "includeHeadingInIndexText": true
                        }
                    }
                },
                {
                    "nodeId": "extract-tree",
                    "stageType": "EXTRACT_TREE",
                    "strategyId": "RAG_MARKDOWN_TREE_EXTRACT_V1"
                },
                summarize_tree_node,
                {
                    "nodeId": "fulltext-index",
                    "stageType": "FULLTEXT_INDEX",
                    "strategyId": "RAG_FULLTEXT_INDEX"
                },
                {
                    "nodeId": "vector-index",
                    "stageType": "VECTOR_INDEX",
                    "strategyId": "RAG_VECTOR_INDEX"
                },
                {
                    "nodeId": "persist-tree",
                    "stageType": "PERSIST_TREE",
                    "strategyId": "RAG_PERSIST_TREE_GRAPH_V1"
                },
                {
                    "nodeId": "persist-source",
                    "stageType": "PERSIST_SOURCE",
                    "strategyId": "RAG_PERSIST_SOURCE"
                }
            ],
            "edges": [
                { "from": "validate-source", "to": "prepare-payload" },
                { "from": "prepare-payload", "to": "split-text" },
                { "from": "split-text", "to": "extract-tree" },
                { "from": "split-text", "to": "fulltext-index" },
                { "from": "split-text", "to": "vector-index" },
                { "from": "extract-tree", "to": "summarize-tree" },
                { "from": "summarize-tree", "to": "persist-tree" },
                { "from": "fulltext-index", "to": "persist-source" },
                { "from": "vector-index", "to": "persist-source" },
                { "from": "persist-tree", "to": "persist-source" }
            ]
        }
    })
}

fn insert_pipeline_only_ingest_profile(database_url: &str, profile_id: &str) {
    let connection = Connection::open(database_url).unwrap();
    let config = json!({
        "sourceInputPolicy": {
            "supportedSourceTypes": ["MEMORY_CARD"]
        },
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {
                    "nodeId": "validate-source",
                    "stageType": "VALIDATE_SOURCE",
                    "strategyId": "RAG_VALIDATE_SOURCE"
                },
                {
                    "nodeId": "prepare-payload",
                    "stageType": "PREPARE_PAYLOAD",
                    "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1"
                },
                {
                    "nodeId": "split-text",
                    "stageType": "SPLIT_TEXT",
                    "strategyId": "RAG_SPLIT_TEXT"
                },
                {
                    "nodeId": "fulltext-index",
                    "stageType": "FULLTEXT_INDEX",
                    "strategyId": "RAG_FULLTEXT_INDEX"
                },
                {
                    "nodeId": "vector-index",
                    "stageType": "VECTOR_INDEX",
                    "strategyId": "RAG_VECTOR_INDEX"
                },
                {
                    "nodeId": "persist-source",
                    "stageType": "PERSIST_SOURCE",
                    "strategyId": "RAG_PERSIST_SOURCE"
                }
            ],
            "edges": [
                { "from": "validate-source", "to": "prepare-payload" },
                { "from": "prepare-payload", "to": "split-text" },
                { "from": "split-text", "to": "fulltext-index" },
                { "from": "split-text", "to": "vector-index" },
                { "from": "fulltext-index", "to": "persist-source" },
                { "from": "vector-index", "to": "persist-source" }
            ]
        }
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'INGEST', 'enabled', 'memory-card', '[\"memory-card\"]',
                NULL, ?2, '{}', 'FAIL_FAST', 1, 1)",
            params![profile_id, config.to_string()],
        )
        .unwrap();
}

fn insert_tree_pipeline_ingest_profile(database_url: &str, profile_id: &str) {
    let connection = Connection::open(database_url).unwrap();
    let config = json!({
        "sourceInputPolicy": {
            "supportedSourceTypes": ["MEMORY_CARD"]
        },
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {
                    "nodeId": "validate-source",
                    "stageType": "VALIDATE_SOURCE",
                    "strategyId": "RAG_VALIDATE_SOURCE"
                },
                {
                    "nodeId": "prepare-payload",
                    "stageType": "PREPARE_PAYLOAD",
                    "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1"
                },
                {
                    "nodeId": "split-text",
                    "stageType": "SPLIT_TEXT",
                    "strategyId": "RAG_SPLIT_TEXT"
                },
                {
                    "nodeId": "extract-tree",
                    "stageType": "EXTRACT_TREE",
                    "strategyId": "RAG_MARKDOWN_TREE_EXTRACT_V1"
                },
                {
                    "nodeId": "summarize-tree",
                    "stageType": "SUMMARIZE_TREE",
                    "strategyId": "RAG_TREE_EXTRACTIVE_SUMMARY_V1"
                },
                {
                    "nodeId": "tree-fulltext-index",
                    "stageType": "TREE_FULLTEXT_INDEX",
                    "strategyId": "RAG_TREE_FULLTEXT_INDEX"
                },
                {
                    "nodeId": "fulltext-index",
                    "stageType": "FULLTEXT_INDEX",
                    "strategyId": "RAG_FULLTEXT_INDEX"
                },
                {
                    "nodeId": "vector-index",
                    "stageType": "VECTOR_INDEX",
                    "strategyId": "RAG_VECTOR_INDEX"
                },
                {
                    "nodeId": "tree-vector-index",
                    "stageType": "TREE_VECTOR_INDEX",
                    "strategyId": "RAG_TREE_VECTOR_INDEX"
                },
                {
                    "nodeId": "persist-tree",
                    "stageType": "PERSIST_TREE",
                    "strategyId": "RAG_PERSIST_TREE_GRAPH_V1"
                },
                {
                    "nodeId": "persist-source",
                    "stageType": "PERSIST_SOURCE",
                    "strategyId": "RAG_PERSIST_SOURCE"
                }
            ],
            "edges": [
                { "from": "validate-source", "to": "prepare-payload" },
                { "from": "prepare-payload", "to": "split-text" },
                { "from": "split-text", "to": "extract-tree" },
                { "from": "split-text", "to": "fulltext-index" },
                { "from": "split-text", "to": "vector-index" },
                { "from": "extract-tree", "to": "summarize-tree" },
                { "from": "summarize-tree", "to": "tree-fulltext-index" },
                { "from": "tree-fulltext-index", "to": "tree-vector-index" },
                { "from": "tree-vector-index", "to": "persist-tree" },
                { "from": "fulltext-index", "to": "persist-source" },
                { "from": "vector-index", "to": "persist-source" },
                { "from": "persist-tree", "to": "persist-source" }
            ]
        }
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'INGEST', 'enabled', 'memory-card', '[\"memory-card\"]',
                NULL, ?2, '{}', 'FAIL_FAST', 1, 1)",
            params![profile_id, config.to_string()],
        )
        .unwrap();
}

fn insert_llm_summary_tree_ingest_profile(
    database_url: &str,
    profile_id: &str,
    summary_config: Option<Value>,
) {
    let connection = Connection::open(database_url).unwrap();
    let config = llm_summary_tree_config(summary_config);
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'INGEST', 'enabled', 'memory-card', '[\"memory-card\"]',
                NULL, ?2, '{}', 'FAIL_FAST', 1, 1)",
            params![profile_id, config.to_string()],
        )
        .unwrap();
}

fn insert_summary_tree_only_ingest_profile(database_url: &str, profile_id: &str) {
    let connection = Connection::open(database_url).unwrap();
    let config = json!({
        "sourceInputPolicy": {
            "supportedSourceTypes": ["MEMORY_CARD"]
        },
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {
                    "nodeId": "validate-source",
                    "stageType": "VALIDATE_SOURCE",
                    "strategyId": "RAG_VALIDATE_SOURCE"
                },
                {
                    "nodeId": "prepare-payload",
                    "stageType": "PREPARE_PAYLOAD",
                    "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1"
                },
                {
                    "nodeId": "split-text",
                    "stageType": "SPLIT_TEXT",
                    "strategyId": "RAG_SPLIT_TEXT"
                },
                {
                    "nodeId": "extract-tree",
                    "stageType": "EXTRACT_TREE",
                    "strategyId": "RAG_MARKDOWN_TREE_EXTRACT_V1"
                },
                {
                    "nodeId": "summarize-tree",
                    "stageType": "SUMMARIZE_TREE",
                    "strategyId": "RAG_TREE_EXTRACTIVE_SUMMARY_V1"
                },
                {
                    "nodeId": "fulltext-index",
                    "stageType": "FULLTEXT_INDEX",
                    "strategyId": "RAG_FULLTEXT_INDEX"
                },
                {
                    "nodeId": "vector-index",
                    "stageType": "VECTOR_INDEX",
                    "strategyId": "RAG_VECTOR_INDEX"
                },
                {
                    "nodeId": "persist-tree",
                    "stageType": "PERSIST_TREE",
                    "strategyId": "RAG_PERSIST_TREE_GRAPH_V1"
                },
                {
                    "nodeId": "persist-source",
                    "stageType": "PERSIST_SOURCE",
                    "strategyId": "RAG_PERSIST_SOURCE"
                }
            ],
            "edges": [
                { "from": "validate-source", "to": "prepare-payload" },
                { "from": "prepare-payload", "to": "split-text" },
                { "from": "split-text", "to": "extract-tree" },
                { "from": "split-text", "to": "fulltext-index" },
                { "from": "split-text", "to": "vector-index" },
                { "from": "extract-tree", "to": "summarize-tree" },
                { "from": "summarize-tree", "to": "persist-tree" },
                { "from": "fulltext-index", "to": "persist-source" },
                { "from": "vector-index", "to": "persist-source" },
                { "from": "persist-tree", "to": "persist-source" }
            ]
        }
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'INGEST', 'enabled', 'memory-card', '[\"memory-card\"]',
                NULL, ?2, '{}', 'FAIL_FAST', 1, 1)",
            params![profile_id, config.to_string()],
        )
        .unwrap();
}

struct CapturingIngestRepository {
    inner: SqliteRagRepository,
    database_url: String,
    persisted_commands: Arc<Mutex<Vec<RagPersistIngestCommand>>>,
}

impl CapturingIngestRepository {
    fn connect(database_url: &str) -> Self {
        Self {
            inner: SqliteRagRepository::connect(database_url).unwrap(),
            database_url: database_url.into(),
            persisted_commands: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Clone for CapturingIngestRepository {
    fn clone(&self) -> Self {
        Self {
            inner: SqliteRagRepository::connect(&self.database_url).unwrap(),
            database_url: self.database_url.clone(),
            persisted_commands: self.persisted_commands.clone(),
        }
    }
}

impl RagConfigRepository for CapturingIngestRepository {
    fn list_corpora(&self) -> Result<Vec<tron_backend::domain::rag::RagCorpusRecord>, RagError> {
        self.inner.list_corpora()
    }

    fn get_corpus(
        &self,
        corpus_id: &str,
    ) -> Result<Option<tron_backend::domain::rag::RagCorpusRecord>, RagError> {
        self.inner.get_corpus(corpus_id)
    }

    fn list_profiles(
        &self,
        query: tron_backend::application::rag::RagProfileListQuery,
    ) -> Result<Vec<RagProfileRecord>, RagError> {
        self.inner.list_profiles(query)
    }

    fn get_profile(&self, profile_id: &str) -> Result<Option<RagProfileRecord>, RagError> {
        self.inner.get_profile(profile_id)
    }
}

impl RagIngestRepository for CapturingIngestRepository {
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        self.persisted_commands
            .lock()
            .unwrap()
            .push(command.clone());
        self.inner.persist_ingest(command)
    }
}

fn query_i64(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn query_string(connection: &Connection, sql: &str) -> String {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn query_unit(connection: &Connection, table: &str) -> Value {
    let sql = format!("SELECT text FROM {table} ORDER BY ordinal");
    connection
        .query_row(&sql, [], |row| {
            Ok(json!({
                "text": row.get::<_, String>(0)?,
            }))
        })
        .unwrap()
}

fn query_skill_unit(connection: &Connection) -> Value {
    connection
        .query_row(
            "SELECT scope_type, scope_code, skill_name, skill_description, text
             FROM rag_skill_text_units
             ORDER BY ordinal",
            [],
            |row| {
                Ok(json!({
                    "scope_type": row.get::<_, String>(0)?,
                    "scope_code": row.get::<_, String>(1)?,
                    "skill_name": row.get::<_, String>(2)?,
                    "skill_description": row.get::<_, String>(3)?,
                    "text": row.get::<_, String>(4)?,
                }))
            },
        )
        .unwrap()
}

fn assert_index_payload_columns_absent(connection: &Connection, table: &str) {
    let columns = table_columns(connection, table);
    for forbidden in [
        "search_text",
        "fulltext_status",
        "embedding_model_code",
        "embedding_version",
        "embedding_text_hash",
        "embedding_dimension",
        "embedding",
        "embedding_status",
    ] {
        assert!(
            !columns.iter().any(|column| column == forbidden),
            "{table} should not persist {forbidden} in the business table",
        );
    }
}

fn table_columns(connection: &Connection, table: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn table_exists(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE name = ?1 LIMIT 1",
            [table],
            |_| Ok(()),
        )
        .is_ok()
}

fn index_with_columns_exists(
    connection: &Connection,
    table: &str,
    expected_columns: &[&str],
    expected_unique: bool,
) -> bool {
    let mut statement = connection
        .prepare(&format!("PRAGMA index_list({table})"))
        .unwrap();
    let indexes = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2).map(|unique| unique == 1)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    indexes.into_iter().any(|(index_name, unique)| {
        if unique != expected_unique {
            return false;
        }
        let mut info = connection
            .prepare(&format!("PRAGMA index_info({index_name})"))
            .unwrap();
        let columns = info
            .query_map([], |row| row.get::<_, String>(2))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        columns == expected_columns
    })
}

fn query_project_doc_source(connection: &Connection) -> Value {
    connection
        .query_row(
            "SELECT document_code, document_type, path, repo_code, module_code
             FROM rag_project_doc_sources
             WHERE source_code = 'doc-rag-mvp'",
            [],
            |row| {
                Ok(json!({
                    "document_code": row.get::<_, String>(0)?,
                    "document_type": row.get::<_, String>(1)?,
                    "path": row.get::<_, String>(2)?,
                    "repo_code": row.get::<_, String>(3)?,
                    "module_code": row.get::<_, String>(4)?,
                }))
            },
        )
        .unwrap()
}

fn query_text_unit_headings(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT heading_path FROM rag_project_doc_text_units ORDER BY ordinal")
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn query_text_unit_document_codes(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT document_code FROM rag_project_doc_text_units ORDER BY ordinal")
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn query_text_unit_codes(connection: &Connection, table: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT text_unit_code FROM {table} ORDER BY ordinal"
        ))
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn query_ordinals(connection: &Connection, table: &str) -> Vec<i64> {
    let mut statement = connection
        .prepare(&format!("SELECT ordinal FROM {table} ORDER BY ordinal"))
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, i64>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn tree_scope_command(
    corpus_id: &str,
    scope_type: &str,
    scope_code: &str,
    text_unit_code: &str,
    summary_text: &str,
) -> RagPersistIngestCommand {
    RagPersistIngestCommand {
        corpus_id: corpus_id.into(),
        profile_id: "memory-card-tree-pipeline-v1".into(),
        source_code: "scope-collision-source".into(),
        source_version: "v1".into(),
        scope: RagIngestScope {
            scope_type: scope_type.into(),
            scope_code: scope_code.into(),
        },
        source_status: "ACTIVE".into(),
        title: Some("Scope collision source".into()),
        content_hash: format!("{scope_type}-{scope_code}-content"),
        body_hash: None,
        metadata: json!({}),
        domain_fields: BTreeMap::new(),
        related_graph: None,
        graph_write_policy: json!({}),
        text_units: vec![RagTextUnitWrite {
            text_unit_code: text_unit_code.into(),
            unit_type: "LEAF".into(),
            text: format!("Text unit for {scope_type}:{scope_code}."),
            search_text: format!("Text unit for {scope_type}:{scope_code}."),
            heading_path: "Scope".into(),
            ordinal: 0,
            token_count: 4,
            content_hash: format!("{text_unit_code}-hash"),
            status: "ACTIVE".into(),
            fulltext_status: "INDEXED".into(),
            embedding_model_code: Some("fake-embedding".into()),
            embedding_version: Some("v1".into()),
            embedding_text_hash: Some(format!("{text_unit_code}-embedding-hash")),
            embedding_dimension: Some(4096),
            embedding: Some(
                embedding_4096(0.25, 0.75)
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect(),
            ),
            embedding_status: "INDEXED".into(),
            metadata: json!({}),
        }],
        tree_nodes: vec![RagTreeNodeWrite {
            tree_node_code: "scope-collision-source:v1:tree:root".into(),
            parent_node_code: "0".into(),
            source_code: "scope-collision-source".into(),
            source_version: "v1".into(),
            scope_type: scope_type.into(),
            scope_code: scope_code.into(),
            path_key: "/".into(),
            level: 0,
            ordinal: 0,
            title: format!("{scope_type} summary"),
            summary_text: summary_text.into(),
            summary_token_count: 5,
            summary_content_hash: format!("{scope_type}-{scope_code}-summary"),
            summary_fulltext_status: "INDEXED".into(),
            summary_embedding_status: "INDEXED".into(),
            summary_embedding_model_code: Some("fake-embedding".into()),
            summary_embedding_version: Some("v1".into()),
            summary_embedding_text_hash: Some(format!("{scope_type}-{scope_code}-tree-embedding")),
            summary_embedding_dimension: Some(4096),
            summary_embedding: Some(
                embedding_4096(0.25, 0.75)
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect(),
            ),
            covered_text_unit_codes: vec![text_unit_code.into()],
            covered_text_unit_count: 1,
            child_start_ordinal: 0,
            child_end_ordinal: 0,
            metadata: json!({}),
            status: "ACTIVE".into(),
        }],
        warnings: Vec::new(),
        workspace_code: "TRON_CORE".into(),
        user_code: "USR-AUTH-0001".into(),
        agent_code: "AGENT_CODEX".into(),
    }
}

fn embedding_4096(first: f32, second: f32) -> Vec<f32> {
    let mut embedding = vec![0.0; 4096];
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
