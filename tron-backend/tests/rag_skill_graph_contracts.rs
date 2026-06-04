use std::{
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
use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::{
        GrafeoAdapterConfig, GrafeoGraphStore, GrafeoStoreCore, GraphBackedRagRepository,
        sqlite::SqliteRagRepository,
    },
    application::{
        graph::{GraphService, ScopeResolver, WorkspaceDetailProvider},
        rag::{
            DefaultRagStrategyRegistry, RagConfigRepository, RagDocumentSourceCall,
            RagDocumentSourceRuntime, RagEmbeddingCall, RagEmbeddingResult, RagEmbeddingRuntime,
            RagIngestAccessContext, RagIngestRequest, RagIngestScope, RagIngestService,
            RagIngestSource, RagRerankCall, RagResolvedDocument, RagRetryIndexRequest,
            RagRetryIndexService, RagSearchContextRequest, RagSearchGraphExpansionRequest,
            RagSearchRequest, RagSearchRuntime, RagSearchService, RagSearchTraceRequest,
        },
    },
    domain::{
        graph::{
            GraphFilterOperator, GraphPropertyFilter, GraphPropertyValue, GraphQueryLimit,
            GraphScopeCommand, GraphSearchQuery,
        },
        rag::{RagError, RagStatus, RagStorageLayout},
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceLineageItem, WorkspaceStatus},
    },
};

#[test]
fn skill_graph_seed_is_additive_and_keeps_existing_skill_sqlite_binding() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();

    let skill = repository.get_corpus("skill").unwrap().unwrap();
    assert_eq!(skill.status, RagStatus::Enabled);
    assert_eq!(
        skill.storage_binding.layout,
        RagStorageLayout::EmbeddedSource
    );
    assert_eq!(
        skill.storage_binding.table.as_deref(),
        Some("rag_skill_text_units")
    );

    let skill_graph = repository.get_corpus("skill-graph").unwrap().unwrap();
    assert_eq!(skill_graph.status, RagStatus::Enabled);
    assert_eq!(
        skill_graph.storage_binding.layout,
        RagStorageLayout::GraphSingleNode
    );
    assert_eq!(
        skill_graph.storage_binding.namespace.as_deref(),
        Some("rag_skill")
    );
    assert_eq!(
        skill_graph.storage_binding.node_label.as_deref(),
        Some("RagSkill")
    );

    assert!(repository.get_profile("skill-ingest-v1").unwrap().is_some());
    assert!(
        repository
            .get_profile("skill-related-v1")
            .unwrap()
            .is_some()
    );
    assert!(
        repository
            .get_profile("skill-graph-ingest-v1")
            .unwrap()
            .is_some()
    );
    assert!(
        repository
            .get_profile("skill-graph-related-v1")
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn skill_graph_ingest_search_expand_debug_and_retry_use_graph_storage_with_fake_models() {
    let database_url = temp_database_path("rag-skill-graph")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    let ingest_service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let ingest = ingest_service
        .ingest(skill_graph_request(
            "rag-reviewer",
            "Reviews RAG retrieval changes before release.",
        ))
        .await
        .unwrap();

    assert_eq!(ingest.status, "ACTIVE");
    assert_eq!(ingest.text_unit_count, 1);
    assert_eq!(ingest.embedding_status, "INDEXED");
    assert_eq!(runtime.embedding_calls(), 1);

    let nodes = graph_service
        .search(
            graph_scope(),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "skillName".into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text("rag-reviewer".into()),
                }],
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec!["RagSkill".into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        nodes[0].node.properties.get("corpusId"),
        Some(&GraphPropertyValue::Text("skill-graph".into()))
    );
    assert!(nodes[0].node.properties.contains_key("embeddingJson"));
    let related_nodes = graph_service
        .search(
            graph_scope(),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "provider".into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text("git".into()),
                }],
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec!["RagTool".into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(related_nodes.len(), 1);
    assert_eq!(
        related_nodes[0].node.properties.get("priority"),
        Some(&GraphPropertyValue::Integer(10))
    );
    assert_eq!(
        related_nodes[0].node.properties.get("builtin"),
        Some(&GraphPropertyValue::Boolean(true))
    );

    ingest_service
        .ingest(skill_graph_request(
            "rag-reviewer",
            "Reviews RAG retrieval changes before release.",
        ))
        .await
        .unwrap();
    let idempotent_nodes = graph_service
        .search(
            graph_scope(),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "skillName".into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text("rag-reviewer".into()),
                }],
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec!["RagSkill".into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(idempotent_nodes.len(), 1);

    let search_service = RagSearchService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    let result = search_service
        .search(search_request("skill-graph-related-v1", "rag-reviewer"))
        .await
        .unwrap();
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].corpus_id, "skill-graph");
    assert_eq!(result.hits[0].fields["skillName"], json!("rag-reviewer"));
    let expanded_nodes = result.hits[0].metadata["relatedGraph"]["nodes"]
        .as_array()
        .expect("related graph nodes should be returned");
    let mut expanded_kinds = expanded_nodes
        .iter()
        .filter_map(|node| node["relatedKind"].as_str())
        .collect::<Vec<_>>();
    expanded_kinds.sort_unstable();
    assert_eq!(expanded_kinds, vec!["TAG", "TOOL"]);
    assert_eq!(
        result.hits[0].metadata["relatedGraph"]["nodes"][0]["labels"][0],
        json!("RagTool")
    );
    assert_eq!(
        result.hits[0].metadata["relatedGraph"]["nodes"][0]["properties"]["provider"],
        json!("git")
    );
    assert_eq!(
        result.hits[0].metadata["relatedGraph"]["nodes"][0]["properties"]["priority"],
        json!(10)
    );
    let mut tool_only_request = search_request("skill-graph-related-v1", "rag-reviewer");
    tool_only_request.context.graph_expansion.relationships = vec!["REQUIRES_TOOL".into()];
    let tool_only_result = search_service.search(tool_only_request).await.unwrap();
    let tool_only_nodes = tool_only_result.hits[0].metadata["relatedGraph"]["nodes"]
        .as_array()
        .expect("filtered related graph nodes should be returned");
    assert_eq!(tool_only_nodes.len(), 1);
    assert_eq!(tool_only_nodes[0]["relatedKind"], json!("TOOL"));
    assert_eq!(runtime.rerank_calls(), 0);

    let debug = tron_backend::application::rag::RagDiagnosticsService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
    )
    .debug_source(debug_request())
    .unwrap();
    assert_eq!(debug.source.corpus_id, "skill-graph");
    assert_eq!(debug.text_units.len(), 1);
    assert_eq!(debug.text_units[0].fulltext_status, "INDEXED");
    assert_eq!(debug.text_units[0].embedding_status, "INDEXED");

    let retry = RagRetryIndexService::new(repository.clone(), runtime.clone())
        .retry_index(retry_request(vec!["fulltextIndex"]))
        .await
        .unwrap();
    assert_eq!(retry.status, "success");
    assert_eq!(retry.stage_results[0].updated_count, 1);

    let vector_retry_error = RagRetryIndexService::new(repository, runtime)
        .retry_index(retry_request(vec!["vectorIndex"]))
        .await
        .unwrap_err();
    assert_eq!(vector_retry_error, RagError::RetryStageUnsupported);
}

#[tokio::test]
async fn skill_graph_related_payload_rejects_disallowed_relationships() {
    let database_url = temp_database_path("rag-skill-graph-reject")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-reject-store")
        .to_string_lossy()
        .to_string();
    let (repository, _) = build_repository(&database_url, &graph_path);
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeRuntime::new(embedding_4(1.0, 0.0)),
    );
    let mut request = skill_graph_request("rag-reviewer", "Reviews RAG retrieval changes.");
    request.payload["relatedGraph"]["edges"][0]["relationship"] = json!("RUNS_RAW_QUERY");

    assert_eq!(
        service.ingest(request).await.unwrap_err(),
        RagError::IngestRequestInvalid
    );
}

#[tokio::test]
async fn blank_query_structured_filter_does_not_execute_against_graph_single_node_corpus() {
    let database_url = temp_database_path("rag-skill-graph-structured-filter")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-structured-filter-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, _) = build_repository(&database_url, &graph_path);
    let ingest_service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    ingest_service
        .ingest(skill_graph_request(
            "rag-reviewer",
            "Reviews RAG retrieval changes before release.",
        ))
        .await
        .unwrap();
    insert_graph_structured_filter_profile(&database_url);
    let embedding_calls_after_ingest = runtime.embedding_calls();

    let search_service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    let mut request = search_request("skill-graph-structured-filter-v1", "");
    request.filters = json!({ "skillName": "rag-reviewer" });

    assert_eq!(
        search_service.search(request).await.unwrap_err(),
        RagError::SearchRequestInvalid
    );
    assert_eq!(runtime.embedding_calls(), embedding_calls_after_ingest);
    assert_eq!(runtime.rerank_calls(), 0);
}

fn build_repository(
    database_url: &str,
    graph_path: &str,
) -> (
    Arc<Mutex<GraphBackedRagRepository>>,
    GraphService<ScopeResolver<FakeWorkspaceProvider>>,
) {
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: graph_path.to_string(),
    })
    .unwrap();
    let store = GrafeoGraphStore::new(core);
    let repository = GraphBackedRagRepository::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        Arc::new(store.clone()),
        Arc::new(ScopeResolver::with_workspace_provider(
            FakeWorkspaceProvider,
        )),
    );
    (
        Arc::new(Mutex::new(repository)),
        GraphService::new(
            store,
            ScopeResolver::with_workspace_provider(FakeWorkspaceProvider),
        ),
    )
}

fn insert_graph_structured_filter_profile(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    let config = json!({
        "routes": [{
            "routeId": "skill-graph-structured",
            "corpusId": "skill-graph",
            "retriever": "STRUCTURED_FILTER_SQLITE_V1",
            "filters": {"skillName": "rag-reviewer"}
        }],
        "fusionPolicy": {"type": "SUM_SCORE"},
        "dedupPolicy": {"type": "TEXT_UNIT"},
        "rerankPolicy": {"enabled": false},
        "contextPolicy": {"supportedStrategies": ["RESULT_LIST"]}
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'RETRIEVAL', 'enabled', 'skill-graph', '[\"skill-graph\"]',
                'v1', ?2, '{}', 'SKIP_VECTOR_ON_UNAVAILABLE', 1, 1)",
            params![
                "skill-graph-structured-filter-v1",
                serde_json::to_string(&config).unwrap()
            ],
        )
        .unwrap();
}

fn skill_graph_request(skill_name: &str, description: &str) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: "skill-graph".into(),
        ingest_profile_id: "skill-graph-ingest-v1".into(),
        workspace_code: "TRON_CORE".into(),
        access_context: access_context(),
        target_scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
        },
        source: RagIngestSource {
            code: format!("skill:TRON_CORE:{skill_name}"),
            version: "v1".into(),
            source_type: "SKILL".into(),
        },
        payload: json!({
            "source": {
                "code": format!("skill:TRON_CORE:{skill_name}"),
                "version": "v1",
                "type": "SKILL"
            },
            "title": skill_name,
            "text": format!("Skill: {skill_name}\nDescription: {description}"),
            "metadata": {},
            "domainFields": {
                "skillName": skill_name,
                "skillDescription": description
            },
            "relatedGraph": {
                "nodes": [
                    {
                        "ref": "tool.git",
                        "kind": "TOOL",
                        "label": "RagTool",
                        "code": "GIT",
                        "title": "Git",
                        "text": "Git operations",
                        "properties": {
                            "provider": "git",
                            "priority": 10,
                            "confidence": 0.95,
                            "builtin": true
                        }
                    },
                    {
                        "ref": "tag.review",
                        "kind": "TAG",
                        "label": "RagTag",
                        "code": "RAG_REVIEW",
                        "title": "RAG Review",
                        "text": "RAG review workflow",
                        "properties": {
                            "category": "review"
                        }
                    }
                ],
                "edges": [
                    {
                        "from": "$skill",
                        "to": "tool.git",
                        "relationship": "REQUIRES_TOOL"
                    },
                    {
                        "from": "$skill",
                        "to": "tag.review",
                        "relationship": "HAS_TAG"
                    }
                ]
            }
        }),
    }
}

fn search_request(profile_id: &str, query: &str) -> RagSearchRequest {
    RagSearchRequest {
        profile_id: profile_id.into(),
        workspace_code: "TRON_CORE".into(),
        query: query.into(),
        access_context: access_context(),
        filters: json!({ "skillName": "rag-reviewer" }),
        limit: 3,
        token_budget: 128,
        context: RagSearchContextRequest {
            strategy: "RESULT_LIST".into(),
            return_fields: vec!["skillName".into(), "skillDescription".into()],
            graph_expansion: RagSearchGraphExpansionRequest::default(),
        },
        trace: RagSearchTraceRequest { enabled: false },
    }
}

fn debug_request() -> tron_backend::application::rag::RagDebugSourceRequest {
    tron_backend::application::rag::RagDebugSourceRequest {
        workspace_code: "TRON_CORE".into(),
        corpus_id: "skill-graph".into(),
        source_code: "skill:TRON_CORE:rag-reviewer".into(),
        source_version: "v1".into(),
        scope_type: "SPACE".into(),
        scope_code: "TRON_CORE".into(),
        include_text_units: true,
        include_warnings: true,
    }
}

fn retry_request(stages: Vec<&str>) -> RagRetryIndexRequest {
    RagRetryIndexRequest {
        workspace_code: "TRON_CORE".into(),
        corpus_id: "skill-graph".into(),
        source_code: "skill:TRON_CORE:rag-reviewer".into(),
        source_version: "v1".into(),
        scope_type: "SPACE".into(),
        scope_code: "TRON_CORE".into(),
        stages: stages.into_iter().map(ToOwned::to_owned).collect(),
        embedding_model_code: Some("FAKE_EMBEDDING".into()),
        user_code: "USR-AUTH-0001".into(),
        agent_code: "AGENT_CODEX".into(),
    }
}

fn access_context() -> RagIngestAccessContext {
    RagIngestAccessContext {
        user_code: "USR-AUTH-0001".into(),
        agent_code: "AGENT_CODEX".into(),
        workspace_code: "TRON_CORE".into(),
    }
}

fn graph_scope() -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: "rag_skill".into(),
        workspace_code: "TRON_CORE".into(),
        app_code: None,
        user_code: None,
    }
}

fn embedding_4(left: f32, right: f32) -> Vec<f32> {
    vec![left, right, 0.0, 0.0]
}

#[derive(Clone)]
struct FakeRuntime {
    embedding: Vec<f32>,
    embedding_calls: Arc<AtomicUsize>,
    rerank_calls: Arc<AtomicUsize>,
}

impl FakeRuntime {
    fn new(embedding: Vec<f32>) -> Self {
        Self {
            embedding,
            embedding_calls: Arc::new(AtomicUsize::new(0)),
            rerank_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn embedding_calls(&self) -> usize {
        self.embedding_calls.load(Ordering::SeqCst)
    }

    fn rerank_calls(&self) -> usize {
        self.rerank_calls.load(Ordering::SeqCst)
    }
}

impl RagEmbeddingRuntime for FakeRuntime {
    fn embed<'a>(
        &'a self,
        _call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embedding_calls.fetch_add(1, Ordering::SeqCst);
            Ok(RagEmbeddingResult {
                model_code: "FAKE_EMBEDDING".into(),
                embedding: self.embedding.clone(),
            })
        })
    }
}

impl RagSearchRuntime for FakeRuntime {
    fn embed<'a>(
        &'a self,
        _call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embedding_calls.fetch_add(1, Ordering::SeqCst);
            Ok(RagEmbeddingResult {
                model_code: "FAKE_EMBEDDING".into(),
                embedding: self.embedding.clone(),
            })
        })
    }

    fn rerank<'a>(
        &'a self,
        _call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.rerank_calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![(0, 1.0)])
        })
    }
}

impl RagDocumentSourceRuntime for FakeRuntime {
    fn resolve_project_doc_markdown<'a>(
        &'a self,
        _call: RagDocumentSourceCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async { Err(RagError::SourceResolveFailed) })
    }
}

#[derive(Debug, Clone, Copy)]
struct FakeWorkspaceProvider;

impl WorkspaceDetailProvider for FakeWorkspaceProvider {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        if workspace_code != "TRON_CORE" {
            return Err(WorkspaceError::NotFound);
        }
        Ok(WorkspaceDetail {
            id: "1".into(),
            workspace_code: "TRON_CORE".into(),
            name: "Tron Core".into(),
            description: None,
            parent_workspace_code: None,
            depth: 1,
            status: WorkspaceStatus::Active,
            path: vec![WorkspaceLineageItem {
                workspace_code: "TRON_CORE".into(),
                name: "Tron Core".into(),
                depth: 1,
            }],
            children: Vec::new(),
            direct_user_count: 0,
            role_grant_count: 0,
            effective_user_count: 0,
            created_at: "2026-06-04T00:00:00Z".into(),
            updated_at: "2026-06-04T00:00:00Z".into(),
            archived_at: None,
        })
    }
}

fn temp_database_path(label: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{label}-{millis}.sqlite"))
}
