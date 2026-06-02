use std::{
    collections::VecDeque,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository,
    application::rag::{
        DefaultRagStrategyRegistry, RagConfigRepository, RagDocumentSourceCall,
        RagDocumentSourceRuntime, RagEmbeddingCall, RagEmbeddingResult, RagEmbeddingRuntime,
        RagIngestAccessContext, RagIngestRequest, RagIngestScope, RagIngestService, RagRerankCall,
        RagResolvedDocument, RagSearchContextRequest, RagSearchRequest, RagSearchRuntime,
        RagSearchService, RagSearchTraceRequest, compile_retrieval_profile, merge_search_filters,
    },
    domain::rag::{RagError, RagProfileRecord, RagProfileType, RagStatus},
};

#[tokio::test]
async fn memory_search_returns_active_scoped_fulltext_vector_hits_and_context_pack() {
    let database_url = temp_database_path("rag-search-memory")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(vec![1.0, 0.0]),
    );

    let result = service
        .search(search_request(
            "memory-related-v1",
            "cite source titles",
            "MEMORY_CONTEXT_PACK",
            json!({
                "memoryType": "RULE",
                "injectable": true,
                "importanceScore": { "op": "gte", "value": 80 }
            }),
        ))
        .await
        .unwrap();

    assert_eq!(result.context_strategy, "MEMORY_CONTEXT_PACK");
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert_eq!(result.hits[0].source_version, "v1");
    assert_eq!(
        result.hits[0].route_ids,
        vec!["memory-fulltext", "memory-vector"]
    );
    assert_eq!(result.hits[0].scope_type, "USER");
    assert_eq!(
        result.hits[0].metadata["memory"]["memoryType"],
        json!("RULE")
    );
    assert_eq!(
        result.hits[0].metadata["source"]["title"],
        json!("RAG citation rule")
    );
    assert_eq!(
        result.hits[0].metadata["extra"]["seed"],
        json!("search-contract")
    );
    assert!(result.hits[0].metadata.get("payload").is_none());
    assert!(result.hits[0].metadata.get("sourceMetadata").is_none());
    assert!(result.context_pack.unwrap().token_count <= 32);
}

#[tokio::test]
async fn skill_search_returns_scoped_fulltext_and_vector_hits() {
    let database_url = temp_database_path("rag-search-skill")
        .to_string_lossy()
        .to_string();
    seed_skill_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(vec![0.5, 0.25]),
    );

    let result = service
        .search(search_request(
            "skill-related-v1",
            "rag reviewer",
            "RESULT_LIST",
            json!({ "skillName": "rag-reviewer" }),
        ))
        .await
        .unwrap();

    assert_eq!(result.context_strategy, "RESULT_LIST");
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].corpus_id, "skill");
    assert_eq!(result.hits[0].source_code, "skill:TRON_CORE:rag-reviewer");
    assert_eq!(result.hits[0].source_version, "v1");
    assert_eq!(
        result.hits[0].route_ids,
        vec!["skill-fulltext", "skill-vector"]
    );
    assert_eq!(result.hits[0].scope_type, "SPACE");
    assert_eq!(result.hits[0].scope_code, "TRON_CORE");
    assert_eq!(
        result.hits[0].metadata["source"]["title"],
        json!("rag-reviewer")
    );
    assert_eq!(
        result.hits[0].metadata["extra"]["skillName"],
        json!("rag-reviewer")
    );
    assert_eq!(
        result.hits[0].metadata["extra"]["skillDescription"],
        json!("Reviews RAG retrieval changes before release.")
    );
    assert!(result.context_pack.is_none());
}

#[tokio::test]
async fn project_doc_search_hydrates_heading_path_and_document_metadata() {
    let database_url = temp_database_path("rag-search-doc")
        .to_string_lossy()
        .to_string();
    seed_project_doc_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(vec![0.0, 1.0]),
    );

    let result = service
        .search(search_request(
            "project-doc-related-v1",
            "sync dag persist",
            "DOC_QA_CONTEXT_PACK",
            json!({ "documentType": ["DESIGN"], "repoCode": "REPO_TRON" }),
        ))
        .await
        .unwrap();

    let hit = result
        .hits
        .iter()
        .find(|hit| hit.metadata["textUnit"]["headingPath"] == json!(["RAG MVP", "Write Path"]))
        .expect("write path text unit should be hydrated");
    assert_eq!(hit.source_code, "doc-rag-mvp");
    assert_eq!(hit.metadata["source"]["title"], json!("RAG MVP"));
    assert_eq!(hit.metadata["document"]["path"], json!("docs/rag-mvp.md"));
    assert_eq!(
        hit.metadata["textUnit"]["headingPath"],
        json!(["RAG MVP", "Write Path"])
    );
    assert!(
        result.context_pack.unwrap().items[0]
            .text
            .contains("synchronous DAG")
    );
}

#[tokio::test]
async fn fusion_dedup_and_fake_rerank_produce_stable_ordering() {
    let database_url = temp_database_path("rag-search-rerank")
        .to_string_lossy()
        .to_string();
    seed_two_memory_fixture(&database_url).await;
    let runtime = FakeSearchRuntime::new()
        .with_embedding(vec![1.0, 0.0])
        .with_rerank(vec![(1, 0.99), (0, 0.10)]);
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let result = service
        .search(search_request(
            "memory-related-v1",
            "retrieval ordering",
            "RESULT_LIST",
            json!({}),
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 2);
    assert_eq!(result.hits[0].source_code, "memory-rerank-second");
    assert_eq!(result.hits[1].source_code, "memory-rag-rule");
    assert_eq!(
        result.hits[1].route_ids,
        vec!["memory-fulltext", "memory-vector"]
    );
    assert!(result.context_pack.is_none());
    assert_eq!(runtime.last_rerank_model_code(), None);
}

#[test]
fn retrieval_profile_compiler_rejects_invalid_routes_context_and_version() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();
    let corpus = repository.get_corpus("memory-card").unwrap().unwrap();
    let registry = DefaultRagStrategyRegistry::default();

    assert!(
        compile_retrieval_profile(
            &retrieval_profile(
                "ok",
                Some("v1"),
                json!({
                    "routes": [{"routeId": "memory-fulltext", "retriever": "FULLTEXT"}],
                    "contextPolicy": {"supportedStrategies": ["RESULT_LIST"]}
                }),
            ),
            &corpus,
            &registry
        )
        .is_ok()
    );

    for (name, version, config) in [
        (
            "missing-version",
            None,
            json!({"routes":[{"routeId":"r","retriever":"FULLTEXT"}]}),
        ),
        (
            "latest-version",
            Some("latest"),
            json!({"routes":[{"routeId":"r","retriever":"FULLTEXT"}]}),
        ),
        (
            "graph-route",
            Some("v1"),
            json!({"routes":[{"routeId":"r","retriever":"GRAPH"}]}),
        ),
        (
            "bad-context",
            Some("v1"),
            json!({
                "routes":[{"routeId":"r","retriever":"FULLTEXT"}],
                "contextPolicy":{"supportedStrategies":["GRAPH_CONTEXT_PACK"]}
            }),
        ),
    ] {
        assert_eq!(
            compile_retrieval_profile(
                &retrieval_profile(name, version, config),
                &corpus,
                &registry
            )
            .unwrap_err(),
            RagError::RetrievalProfileInvalid,
            "{name} must be rejected",
        );
    }
}

#[test]
fn search_filters_reject_unknown_types_and_prevent_profile_filter_broadening() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();
    let corpus = repository.get_corpus("memory-card").unwrap().unwrap();

    let merged = merge_search_filters(
        &corpus,
        &json!({ "memoryType": ["RULE", "DECISION"], "injectable": true }),
        &json!({ "memoryType": "RULE" }),
    )
    .unwrap();
    assert_eq!(merged["memoryType"], json!({"op": "in", "value": ["RULE"]}));
    assert_eq!(merged["injectable"], json!({"op": "eq", "value": true}));

    assert_eq!(
        merge_search_filters(&corpus, &json!({}), &json!({ "unknown": "x" })).unwrap_err(),
        RagError::SearchFilterInvalid
    );
    assert_eq!(
        merge_search_filters(
            &corpus,
            &json!({ "memoryType": "RULE" }),
            &json!({ "memoryType": ["RULE", "DECISION"] }),
        )
        .unwrap_err(),
        RagError::SearchFilterInvalid
    );
}

#[tokio::test]
async fn context_assembly_supports_all_required_strategy_shapes() {
    let database_url = temp_database_path("rag-search-contexts")
        .to_string_lossy()
        .to_string();
    seed_project_doc_fixture(&database_url).await;

    for strategy in [
        "RESULT_LIST",
        "TOKEN_BUDGET_PACK",
        "MEMORY_CONTEXT_PACK",
        "DOC_QA_CONTEXT_PACK",
    ] {
        let service = RagSearchService::new(
            SqliteRagRepository::connect(&database_url).unwrap(),
            DefaultRagStrategyRegistry::default(),
            FakeSearchRuntime::new().with_embedding(vec![0.0, 1.0]),
        );
        let mut request = search_request(
            "project-doc-related-v1",
            "sync dag",
            strategy,
            json!({ "repoCode": "REPO_TRON" }),
        );
        request.token_budget = 8;

        let result = service.search(request).await.unwrap();

        assert_eq!(result.context_strategy, strategy);
        if strategy == "RESULT_LIST" {
            assert!(result.context_pack.is_none());
        } else {
            assert!(result.context_pack.unwrap().token_count <= 8);
        }
    }
}

#[tokio::test]
async fn vector_route_failure_degrades_to_fulltext_warning_without_trace_payload() {
    let database_url = temp_database_path("rag-search-vector-warning")
        .to_string_lossy()
        .to_string();
    seed_project_doc_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding_error("provider timeout"),
    );

    let result = service
        .search(search_request(
            "project-doc-related-v1",
            "sync dag",
            "RESULT_LIST",
            json!({ "repoCode": "REPO_TRON" }),
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(
        result.warnings[0]["code"],
        json!("RAG_VECTOR_ROUTE_UNAVAILABLE")
    );
    assert!(result.warnings[0].get("providerResponse").is_none());
    assert!(result.warnings[0].get("embedding").is_none());
}

#[test]
fn repository_search_boundary_keeps_graph_summary_and_memory_flow_behavior_out() {
    let rag_http_routes = include_str!("../src/adapters/inbound/http/rag/routes.rs");
    let rag_application = include_str!("../src/application/rag/search.rs");
    let repository_search =
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/search.rs");

    assert!(rag_http_routes.contains("/api/v1/rag/search"));
    for forbidden_route in ["/api/v1/rag/graph", "/api/v1/rag/memory-flow"] {
        assert!(!rag_http_routes.contains(forbidden_route));
    }
    for forbidden_behavior in ["GraphService", "SummaryTree", "MemoryFlow"] {
        assert!(!rag_application.contains(forbidden_behavior));
        assert!(!repository_search.contains(forbidden_behavior));
    }
}

#[test]
fn sqlite_rag_repository_uses_template_plans_instead_of_corpus_sql_branches() {
    let repository_ingest =
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/ingest.rs");
    let repository_search =
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/search.rs");
    let repository_retry =
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/retry.rs");
    let repository_diagnostics =
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/diagnostics.rs");

    for source in [
        repository_ingest,
        repository_search,
        repository_retry,
        repository_diagnostics,
    ] {
        for forbidden in [
            "rag_memory_sources",
            "rag_memory_text_units",
            "rag_project_doc_sources",
            "rag_project_doc_text_units",
        ] {
            assert!(
                !source.contains(forbidden),
                "SQLite RAG repository should read RAG table names from storageBinding, not {forbidden}",
            );
        }
        assert!(
            !source.contains("\"memory-card\" =>") && !source.contains("\"project-doc\" =>"),
            "SQLite RAG repository should not branch SQL templates by corpus id",
        );
    }
}

#[derive(Clone, Default)]
struct FakeSearchRuntime {
    embeddings: Arc<Mutex<VecDeque<Result<Vec<f32>, String>>>>,
    reranks: Arc<Mutex<VecDeque<Result<Vec<RagRerankItem>, String>>>>,
    documents: Arc<Mutex<VecDeque<Result<RagResolvedDocument, RagError>>>>,
    last_rerank_model_code: Arc<Mutex<Option<String>>>,
}

type RagRerankItem = (usize, f64);

impl FakeSearchRuntime {
    fn new() -> Self {
        Self::default()
    }

    fn with_embedding(self, embedding: Vec<f32>) -> Self {
        self.embeddings.lock().unwrap().push_back(Ok(embedding));
        self
    }

    fn with_embedding_error(self, message: &str) -> Self {
        self.embeddings
            .lock()
            .unwrap()
            .push_back(Err(message.to_string()));
        self
    }

    fn with_rerank(self, items: Vec<RagRerankItem>) -> Self {
        self.reranks.lock().unwrap().push_back(Ok(items));
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

    fn last_rerank_model_code(&self) -> Option<String> {
        self.last_rerank_model_code.lock().unwrap().clone()
    }
}

impl RagSearchRuntime for FakeSearchRuntime {
    fn embed<'a>(
        &'a self,
        _call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embeddings
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(vec![1.0, 0.0]))
        })
    }

    fn rerank<'a>(
        &'a self,
        call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RagRerankItem>, String>> + Send + 'a>> {
        Box::pin(async move {
            *self.last_rerank_model_code.lock().unwrap() = call.model_code;
            self.reranks
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(Vec::new()))
        })
    }
}

impl RagEmbeddingRuntime for FakeSearchRuntime {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            <Self as RagSearchRuntime>::embed(self, call)
                .await
                .map(|embedding| RagEmbeddingResult {
                    model_code: "workspace-default-embedding".into(),
                    embedding,
                })
        })
    }
}

impl RagDocumentSourceRuntime for FakeSearchRuntime {
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

fn search_request(
    profile_id: &str,
    query: &str,
    strategy: &str,
    filters: Value,
) -> RagSearchRequest {
    RagSearchRequest {
        profile_id: profile_id.into(),
        workspace_code: "TRON_CORE".into(),
        query: query.into(),
        access_context: RagIngestAccessContext {
            user_code: "USR-LARGE-ID".into(),
            agent_code: "AGENT_CODEX".into(),
            workspace_code: "TRON_CORE".into(),
        },
        filters,
        limit: 5,
        token_budget: 32,
        context: RagSearchContextRequest {
            strategy: strategy.into(),
        },
        trace: RagSearchTraceRequest { enabled: false },
    }
}

async fn seed_memory_fixture(database_url: &str) {
    let service = RagIngestService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(vec![1.0, 0.0]),
    );
    service
        .ingest(memory_request(
            "memory-rag-rule",
            "v1",
            "USER",
            "USR-LARGE-ID",
        ))
        .await
        .unwrap();
    service
        .ingest(memory_request(
            "memory-rag-rule-old",
            "v2",
            "USER",
            "USR-LARGE-ID",
        ))
        .await
        .unwrap();
}

async fn seed_two_memory_fixture(database_url: &str) {
    seed_memory_fixture(database_url).await;
    let service = RagIngestService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(vec![0.9, 0.1]),
    );
    service
        .ingest(memory_request(
            "memory-rerank-second",
            "v1",
            "SPACE",
            "TRON_CORE",
        ))
        .await
        .unwrap();
}

async fn seed_project_doc_fixture(database_url: &str) {
    let service = RagIngestService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new()
            .with_embedding(vec![0.0, 1.0])
            .with_document(project_doc_markdown()),
    );
    service.ingest(project_doc_request()).await.unwrap();
}

async fn seed_skill_fixture(database_url: &str) {
    let service = RagIngestService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(vec![0.5, 0.25]),
    );
    service.ingest(skill_request()).await.unwrap();
}

fn memory_request(
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> RagIngestRequest {
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
            scope_type: scope_type.into(),
            scope_code: scope_code.into(),
        },
        source_code: source_code.into(),
        source_version: source_version.into(),
        source_type: "PLAIN_TEXT".into(),
        payload: json!({
            "title": "RAG citation rule",
            "text": "Always cite source titles in RAG answers for retrieval ordering.",
            "memoryType": "RULE",
            "memoryStatus": "ACTIVE",
            "reviewStatus": "APPROVED",
            "importanceScore": 90,
            "injectable": true,
            "metadata": {
                "seed": "search-contract"
            }
        }),
    }
}

fn project_doc_request() -> RagIngestRequest {
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
        source_version: "v1".into(),
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

fn skill_request() -> RagIngestRequest {
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
        source_version: "v1".into(),
        source_type: "SKILL".into(),
        payload: json!({
            "skillName": "rag-reviewer",
            "skillDescription": "Reviews RAG retrieval changes before release."
        }),
    }
}

fn project_doc_markdown() -> &'static str {
    "# RAG MVP\nIntro.\n\n## Write Path\nThe write path uses a synchronous DAG to persist source and text units."
}

fn retrieval_profile(profile_id: &str, version: Option<&str>, config: Value) -> RagProfileRecord {
    RagProfileRecord {
        profile_id: profile_id.into(),
        profile_type: RagProfileType::Retrieval,
        status: RagStatus::Enabled,
        primary_corpus_id: Some("memory-card".into()),
        corpus_ids: vec!["memory-card".into()],
        required_source_version: version.map(str::to_owned),
        config,
        filters: json!({}),
        degrade_policy: "SKIP_VECTOR_ON_UNAVAILABLE".into(),
    }
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
