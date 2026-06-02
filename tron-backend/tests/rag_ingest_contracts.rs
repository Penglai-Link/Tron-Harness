use std::{
    collections::VecDeque,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository,
    application::rag::{
        DefaultRagStrategyRegistry, RagConfigRepository, RagDocumentSourceCall,
        RagDocumentSourceRuntime, RagEmbeddingCall, RagEmbeddingResult, RagEmbeddingRuntime,
        RagIngestAccessContext, RagIngestRequest, RagIngestScope, RagIngestService,
        RagResolvedDocument, compile_ingest_profile,
    },
    domain::rag::{RagError, RagProfileRecord, RagProfileType, RagStatus},
};

#[tokio::test]
async fn memory_json_text_ingest_writes_business_text_unit_and_sidecar_indexes() {
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
    request.payload["version"] = json!("business-v9");

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
async fn skill_ingest_writes_name_description_scope_fulltext_and_vector_fields() {
    let database_path = temp_database_path("rag-ingest-skill-success");
    let database_url = database_path.to_string_lossy().to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    let embeddings = FakeEmbeddingRuntime::fixed(vec![0.5, 0.25]);
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
        unit["search_text"],
        json!("Skill: rag-reviewer\nDescription: Reviews RAG retrieval changes before release.")
    );
    assert_eq!(unit["embedding_status"], json!("INDEXED"));
    assert_eq!(unit["embedding_dimension"], json!(2));
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
    assert_eq!(
        metadata,
        json!({
            "resolver": {
                "strategyId": "PROJECT_DOC_MARKDOWN_RESOLVE_SOURCE_V1",
                "documentUri": "project-doc://TRON_CORE/docs/rag-mvp.md",
                "contentLength": project_doc_markdown().len()
            },
            "parser": {
                "strategyId": "PROJECT_DOC_MARKDOWN_PARSE_DOCUMENT_V1",
                "format": "MARKDOWN"
            }
        })
    );

    let headings = query_text_unit_headings(&connection);
    assert_eq!(headings, vec!["RAG MVP", "RAG MVP > Write Path"]);
    assert_index_payload_columns_absent(&connection, "rag_project_doc_text_units");
    assert!(table_exists(&connection, "rag_project_doc_text_units_fts"));
    assert!(table_exists(
        &connection,
        "rag_project_doc_text_units_vec_d4096"
    ));
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
        "title": "RAG MVP",
        "documentUri": "project-doc://TRON_CORE/docs/rag-mvp.md",
        "format": "MARKDOWN",
        "text": project_doc_markdown(),
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
async fn embedding_failure_degrades_project_doc_but_fails_memory() {
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

    let error = memory_service
        .ingest(memory_request("v1"))
        .await
        .expect_err("memory vector failure must fail the ingest");

    assert_eq!(error, RagError::EmbeddingFailed);
    let memory_connection = Connection::open(&memory_url).unwrap();
    assert_eq!(
        query_string(
            &memory_connection,
            "SELECT source_status FROM rag_memory_text_units WHERE source_code = 'memory-rag-rule' LIMIT 1"
        ),
        "FAILED"
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
            "artifact-incompatible",
            artifact_incompatible_dag(),
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
}

#[derive(Clone)]
struct FakeEmbeddingRuntime {
    result: Arc<Mutex<Result<RagEmbeddingResult, String>>>,
    documents: Arc<Mutex<VecDeque<Result<RagResolvedDocument, RagError>>>>,
    calls: Arc<AtomicUsize>,
    last_model_code: Arc<Mutex<Option<String>>>,
}

impl FakeEmbeddingRuntime {
    fn fixed(embedding: Vec<f32>) -> Self {
        Self {
            result: Arc::new(Mutex::new(Ok(RagEmbeddingResult {
                model_code: "workspace-default-embedding".into(),
                embedding,
            }))),
            documents: Arc::new(Mutex::new(VecDeque::new())),
            calls: Arc::new(AtomicUsize::new(0)),
            last_model_code: Arc::new(Mutex::new(None)),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            result: Arc::new(Mutex::new(Err(message.to_string()))),
            documents: Arc::new(Mutex::new(VecDeque::new())),
            calls: Arc::new(AtomicUsize::new(0)),
            last_model_code: Arc::new(Mutex::new(None)),
        }
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

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn last_model_code(&self) -> Option<String> {
        self.last_model_code.lock().unwrap().clone()
    }
}

impl RagEmbeddingRuntime for FakeEmbeddingRuntime {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.last_model_code.lock().unwrap() = call.model_code;
            self.result.lock().unwrap().clone()
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
        source_code: "memory-rag-rule".into(),
        source_version: source_version.into(),
        source_type: "PLAIN_TEXT".into(),
        payload: json!({
            "title": "RAG citation rule",
            "text": "Always cite source titles in RAG answers.",
            "version": source_version,
            "memoryType": "RULE",
            "memoryStatus": "ACTIVE",
            "reviewStatus": "APPROVED",
            "importanceScore": 90,
            "metadata": {
                "seed": "contract-test",
                "source": "rag-ingest"
            }
        }),
    }
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
        source_code: "skill:TRON_CORE:rag-reviewer".into(),
        source_version: source_version.into(),
        source_type: "SKILL".into(),
        payload: json!({
            "skillName": "rag-reviewer",
            "skillDescription": "Reviews RAG retrieval changes before release.",
            "metadata": {
                "source": "skill-registry"
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
        source_code: "doc-rag-mvp".into(),
        source_version: source_version.into(),
        source_type: "PROJECT_DOC_REF".into(),
        payload: json!({
            "title": "RAG MVP",
            "documentUri": "project-doc://TRON_CORE/docs/rag-mvp.md",
            "format": "MARKDOWN",
            "documentCode": "DOC_RAG_MVP",
            "documentType": "DESIGN",
            "path": "docs/rag-mvp.md",
            "repoCode": "REPO_TRON",
            "moduleCode": "MOD_RAG",
            "moduleName": "RAG"
        }),
    }
}

fn project_doc_markdown() -> &'static str {
    "# RAG MVP\nIntro.\n\n## Write Path\nPersist synchronously."
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
    config["splitter"]["headingDepth"] = json!(heading_depth);
    connection
        .execute(
            "UPDATE rag_profiles SET config_json = ?1 WHERE profile_id = 'project-doc-ingest-v1'",
            [config.to_string()],
        )
        .unwrap();
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
        "dag": {
            "nodes": ["validateSource", "splitText", "fulltextIndex", "vectorIndex", "persistSource"],
            "edges": [
                { "from": "validateSource", "to": "splitText" },
                { "from": "splitText", "to": "fulltextIndex" },
                { "from": "fulltextIndex", "to": "vectorIndex" },
                { "from": "vectorIndex", "to": "persistSource" }
            ]
        },
        "stages": {
            "validateSource": { "strategyId": "RAG_VALIDATE_SOURCE", "inputArtifact": "request", "outputArtifact": "source" },
            "splitText": { "strategyId": "RAG_SPLIT_TEXT", "inputArtifact": "source", "outputArtifact": "textUnits" },
            "fulltextIndex": { "strategyId": "RAG_FULLTEXT_INDEX", "inputArtifact": "textUnits", "outputArtifact": "fulltextUnits" },
            "vectorIndex": { "strategyId": "RAG_VECTOR_INDEX", "inputArtifact": "fulltextUnits", "outputArtifact": "vectorUnits" },
            "persistSource": { "strategyId": "RAG_PERSIST_SOURCE", "inputArtifact": "vectorUnits", "outputArtifact": "persisted" }
        },
        "payloadStrategyId": "RAG_JSON_TEXT_PAYLOAD_V1",
        "payloadMapping": {
            "supportedSourceTypes": ["PLAIN_TEXT"],
            "required": ["text", "version"],
            "title": "$.title",
            "text": "$.text",
            "metadataFrom": "$.metadata",
            "fields": {
                "version": {
                    "path": "$.version",
                    "target": "version",
                    "type": "string"
                }
            }
        }
    })
}

fn cycle_dag() -> Value {
    let mut value = valid_dag();
    value["dag"]["edges"] = json!([
        { "from": "validateSource", "to": "splitText" },
        { "from": "splitText", "to": "validateSource" }
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

fn artifact_incompatible_dag() -> Value {
    let mut value = valid_dag();
    value["stages"]["vectorIndex"]["inputArtifact"] = json!("unrelatedArtifact");
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
            "SELECT scope_type, scope_code, skill_name, skill_description, search_text,
                    embedding_status, embedding_dimension
             FROM rag_skill_text_units
             ORDER BY ordinal",
            [],
            |row| {
                Ok(json!({
                    "scope_type": row.get::<_, String>(0)?,
                    "scope_code": row.get::<_, String>(1)?,
                    "skill_name": row.get::<_, String>(2)?,
                    "skill_description": row.get::<_, String>(3)?,
                    "search_text": row.get::<_, String>(4)?,
                    "embedding_status": row.get::<_, String>(5)?,
                    "embedding_dimension": row.get::<_, i64>(6)?,
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
