use std::{
    collections::BTreeMap,
    future::Future,
    hash::{Hash, Hasher},
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
    adapters::outbound::persistence::{
        GrafeoAdapterConfig, GrafeoGraphStore, GrafeoStoreCore, GraphBackedRagRepository,
        sqlite::SqliteRagRepository,
    },
    application::{
        graph::{GraphService, ScopeResolver, WorkspaceDetailProvider},
        rag::{
            DefaultRagStrategyRegistry, RagConfigRepository, RagDocumentSourceCall,
            RagDocumentSourceRuntime, RagEmbeddingCall, RagEmbeddingResult, RagEmbeddingRuntime,
            RagIngestAccessContext, RagIngestRepository, RagIngestRequest, RagIngestScope,
            RagIngestService, RagIngestSource, RagManagementService, RagPersistIngestCommand,
            RagRerankCall, RagResolvedDocument, RagRetryIndexRequest, RagRetryIndexService,
            RagRouteSearchQuery, RagSearchContextRequest, RagSearchGraphExpansionRequest,
            RagSearchRepository, RagSearchRequest, RagSearchRuntime, RagSearchScope,
            RagSearchService, RagSearchTraceRequest, RagTextUnitWrite, RagTreeNodeWrite,
        },
    },
    domain::{
        graph::{
            GraphFilterOperator, GraphNodeWrite, GraphPropertyFilter, GraphPropertyValue,
            GraphQueryLimit, GraphScopeCommand, GraphSearchQuery,
        },
        rag::{RagError, RagStatus, RagStorageBackend},
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceLineageItem, WorkspaceStatus},
    },
};

#[test]
fn skill_graph_seed_is_additive_and_keeps_existing_skill_sqlite_binding() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();

    let skill = repository.get_corpus("skill").unwrap().unwrap();
    assert_eq!(skill.status, RagStatus::Enabled);
    assert_eq!(
        skill.storage_binding.sqlite_table_for_segment("textUnit"),
        Some("rag_skill_text_units")
    );

    let skill_graph = repository.get_corpus("skill-graph").unwrap().unwrap();
    assert_eq!(skill_graph.status, RagStatus::Enabled);
    let skill_graph_target = skill_graph
        .storage_binding
        .target_for_segment("textUnit")
        .expect("skill graph textUnit target should be configured");
    assert_eq!(skill_graph_target.backend, RagStorageBackend::Graph);
    assert_eq!(skill_graph_target.namespace.as_deref(), Some("rag_skill"));
    assert_eq!(skill_graph_target.node_label.as_deref(), Some("RagSkill"));

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
    let skill_graph_ingest = repository
        .get_profile("skill-graph-ingest-v1")
        .unwrap()
        .unwrap();
    let graph_write_policy = &skill_graph_ingest.config["graphWritePolicy"];
    assert!(graph_write_policy.get("allowedNodeKinds").is_none());
    assert!(graph_write_policy.get("allowedRelationships").is_none());
}

#[test]
fn graph_backed_repository_dispatches_by_backend_before_layout() {
    let database_url = temp_database_path("rag-storage-backend-dispatch")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-storage-backend-dispatch-graph")
        .to_string_lossy()
        .to_string();
    let sqlite = SqliteRagRepository::connect(&database_url).unwrap();
    let connection = Connection::open(&database_url).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE rag_backend_dispatch_text_units (
                id INTEGER PRIMARY KEY,
                text_unit_code TEXT NOT NULL UNIQUE,
                source_code TEXT NOT NULL,
                source_version TEXT NOT NULL,
                scope_type TEXT NOT NULL,
                scope_code TEXT NOT NULL,
                text TEXT NOT NULL,
                heading_path TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                token_count INTEGER NOT NULL DEFAULT 1,
                status TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}'
             );
             INSERT INTO rag_backend_dispatch_text_units (
                text_unit_code, source_code, source_version, scope_type, scope_code,
                text, heading_path, ordinal, status
             )
             VALUES (
                'graph-backend-unit-001', 'graph-backend-source', 'v1',
                'SPACE', 'TRON_CORE', 'graph backend should not use sqlite',
                '', 0, 'ACTIVE'
             );
             INSERT INTO rag_corpora (
                corpus_id, display_name, status, description, capabilities_json,
                filter_schema_json, field_mapping_json, storage_binding_json, created_at, updated_at
             )
             VALUES (
                'graph-backend-custom', 'Graph Backend Custom', 'enabled',
                'Dispatch fixture.',
                '{\"fulltext\":true,\"vector\":false,\"graph\":true}',
                '{\"fields\":{\"scopeType\":{\"type\":\"string\",\"column\":\"scope_type\"}}}',
                '{\"source\":{\"sourceCode\":\"source_code\",\"sourceVersion\":\"source_version\",\"scopeType\":\"scope_type\",\"scopeCode\":\"scope_code\"},\"textUnit\":{\"text\":\"text\"}}',
                '{\"mode\":\"TARGETED\",\"bindings\":{\"source\":\"graph-backend-store\",\"textUnit\":\"graph-backend-store\",\"fulltext\":\"graph-backend-store\",\"vector\":\"graph-backend-store\"},\"targets\":[{\"id\":\"graph-backend-store\",\"backend\":\"GRAPH\",\"layout\":\"GRAPH_SINGLE_NODE\",\"namespace\":\"rag_backend_dispatch\",\"nodeLabel\":\"RagBackendDispatch\",\"codeStrategy\":\"CORPUS_SOURCE_VERSION_SCOPE_HASH\"}]}',
                1780592400000, 1780592400000
             );",
        )
        .unwrap();
    drop(connection);
    sqlite.apply_rag_sqlite_extension_index_migration().unwrap();
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: graph_path,
    })
    .unwrap();
    let repository = GraphBackedRagRepository::new(
        sqlite,
        Arc::new(GrafeoGraphStore::new(core)),
        Arc::new(ScopeResolver::with_workspace_provider(
            FakeWorkspaceProvider,
        )),
    );

    let error = repository
        .search_route(RagRouteSearchQuery {
            corpus_id: "graph-backend-custom".into(),
            route_id: "graph-backend-custom-fulltext".into(),
            retriever: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "graph backend".into(),
            query_embedding: None,
            query_embedding_model_code: None,
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({}),
            policy: json!({}),
            return_fields: Vec::new(),
            limit: 5,
        })
        .unwrap_err();

    assert_eq!(error, RagError::SearchRouteUnavailable);
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
    let expanded_edges = result.hits[0].metadata["relatedGraph"]["edges"]
        .as_array()
        .expect("related graph edges should be returned");
    let requires_tool_edge = expanded_edges
        .iter()
        .find(|edge| edge["relationship"] == json!("REQUIRES_TOOL"))
        .expect("requires tool edge should be present");
    assert!(
        requires_tool_edge["edgeCode"]
            .as_str()
            .is_some_and(|edge_code| edge_code.starts_with("RAG_RELATED_EDGE_")),
        "generic graph related edges should no longer use a skill-specific code prefix"
    );
    assert_eq!(
        requires_tool_edge["properties"]["usage"],
        json!("source-control")
    );
    assert_eq!(requires_tool_edge["properties"]["required"], json!(true));
    assert_eq!(requires_tool_edge["properties"]["weight"], json!(0.9));
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
async fn mixed_graph_carrier_search_hydrates_sqlite_canonical_text_units() {
    let database_url = temp_database_path("rag-skill-graph-mixed-hydrate")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-mixed-hydrate-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    enable_skill_graph_sqlite_canonical_storage(&database_url);
    let ingest_service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime,
    );
    ingest_service
        .ingest(skill_graph_request(
            "rag-mixed",
            "Canonical SQLite text should hydrate graph carrier hits.",
        ))
        .await
        .unwrap();

    let connection = Connection::open(&database_url).unwrap();
    let canonical_text: String = connection
        .query_row(
            "SELECT text FROM rag_skill_text_units
             WHERE source_code = 'skill:TRON_CORE:rag-mixed'
             ORDER BY ordinal LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(canonical_text.contains("Canonical SQLite text"));

    let graph_hits = graph_service
        .search(
            graph_scope(),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "skill_name".into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text("rag-mixed".into()),
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
    assert_eq!(graph_hits.len(), 1);
    let mut carrier_properties = graph_hits[0].node.properties.clone();
    carrier_properties.insert(
        "text".into(),
        GraphPropertyValue::Text("carrier only recall phrase".into()),
    );
    carrier_properties.insert(
        "searchText".into(),
        GraphPropertyValue::Text("carrier only recall phrase".into()),
    );
    graph_service
        .update_node(
            graph_scope(),
            GraphNodeWrite {
                node_code: graph_hits[0].node.node_code.clone(),
                labels: graph_hits[0].node.labels.clone(),
                properties: carrier_properties,
                full_text: Some("carrier only recall phrase".into()),
                embedding_text: None,
                embedding_model_code: None,
            },
        )
        .await
        .unwrap();

    let hits = repository
        .lock()
        .unwrap()
        .search_route(RagRouteSearchQuery {
            corpus_id: "skill-graph".into(),
            route_id: "skill-graph-fulltext".into(),
            retriever: "FULLTEXT_GRAPH_SINGLE_NODE_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "carrier".into(),
            query_embedding: None,
            query_embedding_model_code: None,
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({ "skillName": { "value": "rag-mixed" } }),
            policy: json!({}),
            return_fields: Vec::new(),
            limit: 5,
        })
        .unwrap();

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].search_text, "carrier only recall phrase");
    assert!(hits[0].text.contains("Canonical SQLite text"));
    assert!(!hits[0].text.contains("carrier only recall phrase"));
}

#[tokio::test]
async fn graph_text_units_can_use_sqlite_fulltext_carrier_without_sqlite_text_units() {
    let database_url = temp_database_path("rag-skill-graph-sqlite-fulltext-carrier")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-sqlite-fulltext-carrier-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, _) = build_repository(&database_url, &graph_path);
    enable_skill_graph_sqlite_fulltext_carrier_storage(&database_url);
    let ingest_service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime,
    );
    ingest_service
        .ingest(skill_graph_request(
            "rag-sqlite-carrier",
            "SQLite fulltext carrier works without a SQLite text unit table.",
        ))
        .await
        .unwrap();

    let hits = repository
        .lock()
        .unwrap()
        .search_route(RagRouteSearchQuery {
            corpus_id: "skill-graph".into(),
            route_id: "skill-graph-sqlite-fulltext".into(),
            retriever: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "carrier".into(),
            query_embedding: None,
            query_embedding_model_code: None,
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({}),
            policy: json!({}),
            return_fields: Vec::new(),
            limit: 5,
        })
        .unwrap();

    assert_eq!(hits.len(), 1);
    assert_eq!(
        hits[0].target_code,
        "skill:TRON_CORE:rag-sqlite-carrier:v1:0"
    );
    assert!(hits[0].search_text.contains("SQLite fulltext carrier"));
    assert!(hits[0].text.contains("SQLite fulltext carrier"));
}

#[tokio::test]
async fn graph_text_units_can_use_sqlite_source_with_graph_retrieval_segments() {
    let database_url = temp_database_path("rag-skill-graph-sqlite-source-only")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-sqlite-source-only-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    enable_skill_graph_sqlite_source_with_graph_retrieval_storage(&database_url);
    let ingest_service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime,
    );
    ingest_service
        .ingest(skill_graph_request(
            "rag-source-sqlite",
            "SQLite source persists while retrieval segments stay in graph.",
        ))
        .await
        .unwrap();

    let connection = Connection::open(&database_url).unwrap();
    let persisted_source: String = connection
        .query_row(
            "SELECT title FROM rag_skill_graph_sources
             WHERE source_code = 'skill:TRON_CORE:rag-source-sqlite'
               AND source_version = 'v1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(persisted_source, "rag-source-sqlite");
    drop(connection);

    assert_segment_node(
        &graph_service,
        "rag_skill_text_units",
        "RagSkillTextUnit",
        "unitSkillName",
        "rag-source-sqlite",
    )
    .await;
}

#[tokio::test]
async fn segmented_graph_runtime_writes_and_searches_role_specific_nodes() {
    let database_url = temp_database_path("rag-skill-graph-segmented")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-segmented-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    enable_skill_graph_segmented_graph_storage(&database_url);
    let ingest_service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime,
    );
    ingest_service
        .ingest(skill_graph_request(
            "rag-segmented",
            "Segmented graph storage keeps text units and vectors in separate nodes.",
        ))
        .await
        .unwrap();

    assert_segment_node(
        &graph_service,
        "rag_skill_source",
        "RagSkillSource",
        "sourceSkillName",
        "rag-segmented",
    )
    .await;
    assert_segment_node(
        &graph_service,
        "rag_skill_text_units",
        "RagSkillTextUnit",
        "unitSkillName",
        "rag-segmented",
    )
    .await;
    assert_segment_node(
        &graph_service,
        "rag_skill_fulltext",
        "RagSkillFulltext",
        "unitSkillName",
        "rag-segmented",
    )
    .await;
    assert_segment_node(
        &graph_service,
        "rag_skill_vectors",
        "RagSkillVector",
        "unitSkillName",
        "rag-segmented",
    )
    .await;

    let fulltext_hits = repository
        .lock()
        .unwrap()
        .search_route(RagRouteSearchQuery {
            corpus_id: "skill-graph".into(),
            route_id: "skill-graph-fulltext".into(),
            retriever: "FULLTEXT_GRAPH_SINGLE_NODE_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "separate nodes".into(),
            query_embedding: None,
            query_embedding_model_code: None,
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({ "skillName": { "value": "rag-segmented" } }),
            policy: json!({}),
            return_fields: Vec::new(),
            limit: 5,
        })
        .unwrap();
    assert_eq!(fulltext_hits.len(), 1);
    assert!(fulltext_hits[0].search_text.contains("separate nodes"));

    let vector_hits = repository
        .lock()
        .unwrap()
        .search_route(RagRouteSearchQuery {
            corpus_id: "skill-graph".into(),
            route_id: "skill-graph-vector".into(),
            retriever: "VECTOR_GRAPH_SINGLE_NODE_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "semantic segmented lookup".into(),
            query_embedding: Some(embedding_4(1.0, 0.0)),
            query_embedding_model_code: Some("FAKE_EMBEDDING".into()),
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({ "skillName": { "value": "rag-segmented" } }),
            policy: json!({}),
            return_fields: Vec::new(),
            limit: 5,
        })
        .unwrap();
    assert_eq!(vector_hits.len(), 1);
    assert_eq!(
        vector_hits[0].field_values["skillName"],
        json!("rag-segmented")
    );
}

#[tokio::test]
async fn graph_tree_route_with_sqlite_text_units_returns_covered_text_unit_rows() {
    let database_url = temp_database_path("rag-skill-graph-tree-sqlite-text")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-tree-sqlite-text-store")
        .to_string_lossy()
        .to_string();
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    enable_skill_graph_tree_with_sqlite_text_units(&database_url);

    repository
        .lock()
        .unwrap()
        .persist_ingest(tree_route_command(
            "rag-tree-sqlite",
            "Graph summary for the first branch.",
            vec![
                (
                    "skill:TRON_CORE:rag-tree-sqlite:v1:0",
                    "Covered SQLite unit only.",
                ),
                (
                    "skill:TRON_CORE:rag-tree-sqlite:v1:1",
                    "Uncovered SQLite unit must stay out.",
                ),
            ],
            vec!["skill:TRON_CORE:rag-tree-sqlite:v1:0"],
        ))
        .unwrap();

    let tree_nodes = graph_service
        .search(
            graph_scope_for_namespace("rag_skill_tree"),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "treeNodeCode".into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text("rag-tree-sqlite:root".into()),
                }],
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec!["RagSkillTreeNode".into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(tree_nodes.len(), 1);
    assert_eq!(
        tree_nodes[0].node.properties.get("summaryText"),
        Some(&GraphPropertyValue::Text(
            "Graph summary for the first branch.".into()
        ))
    );

    let rows = repository
        .lock()
        .unwrap()
        .search_route(tree_route_query(
            "TREE_FULLTEXT_V1",
            "first branch",
            None,
            "rag-tree-sqlite",
        ))
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].target_code, "skill:TRON_CORE:rag-tree-sqlite:v1:0");
    assert_eq!(rows[0].field_values["matchedBy"], json!("TREE_FULLTEXT_V1"));
    assert_eq!(
        rows[0].field_values["treeNodeCode"],
        json!("rag-tree-sqlite:root")
    );
    assert_eq!(
        rows[0].field_values["treeNodeTitle"],
        json!("rag-tree-sqlite summary")
    );
    assert!(rows[0].field_values.get("treeScore").is_some());
    assert!(rows[0].text.contains("Covered SQLite unit only."));
    assert!(
        !rows[0]
            .text
            .contains("Uncovered SQLite unit must stay out.")
    );
}

#[tokio::test]
async fn graph_tree_pipeline_with_sqlite_segments_persists_when_embeddings_degrade() {
    let database_url = temp_database_path("rag-skill-graph-tree-sqlite-degraded")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-tree-sqlite-degraded-store")
        .to_string_lossy()
        .to_string();
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    enable_skill_graph_tree_with_ui_sqlite_segments(&database_url);
    insert_skill_graph_tree_pipeline_ingest_profile(
        &database_url,
        "skill-graph-tree-pipeline-degraded-v1",
        "SKIP_VECTOR_ON_UNAVAILABLE",
    );
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FailingEmbeddingRuntime::default(),
    );

    let result = service
        .ingest(skill_graph_tree_text_request(
            "skill-graph-tree-pipeline-degraded-v1",
            "rag-tree-degraded",
            "# Runtime Workbench Summary\n\nThe runtime workbench stores root and section summaries.\n\n## Section A\n\nGraph tree persistence should survive unavailable embeddings.",
        ))
        .await
        .unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.embedding_status, "FAILED");
    assert!(result.text_unit_count >= 2);

    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_ui_tree_graph_live_sources WHERE source_code = 'skill:TRON_CORE:rag-tree-degraded'"
        ),
        1
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_ui_tree_graph_live_text_units WHERE source_code = 'skill:TRON_CORE:rag-tree-degraded'"
        ),
        result.text_unit_count as i64
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_ui_tree_graph_live_vectors WHERE source_code = 'skill:TRON_CORE:rag-tree-degraded'"
        ),
        result.text_unit_count as i64,
        "degraded vector carrier rows should still preserve source/text metadata for retry"
    );

    let tree_nodes = graph_service
        .search(
            graph_scope_for_namespace("rag_ui_tree_graph_live_tree"),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "sourceCode".into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text("skill:TRON_CORE:rag-tree-degraded".into()),
                }],
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec!["RagUiTreeGraphLiveNode".into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();
    assert!(
        tree_nodes.len() >= 2,
        "markdown headings should persist root and section tree nodes"
    );
    assert!(
        tree_nodes.iter().all(|hit| hit.node.full_text.is_some()),
        "tree fulltext should remain available when embedding degrades"
    );
    assert!(
        tree_nodes
            .iter()
            .all(|hit| hit.node.embedding_text.is_none()),
        "failed tree-vector indexing must not write semantic text without a projection"
    );
}

#[tokio::test]
async fn ui_graph_tree_memory_card_profile_persists_with_fulltext_text_unit_fallback_and_legacy_edge()
 {
    let database_url = temp_database_path("rag-ui-tree-graph-live-degraded")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-ui-tree-graph-live-degraded-store")
        .to_string_lossy()
        .to_string();
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    insert_ui_tree_graph_live_corpus(&database_url);
    insert_ui_tree_graph_live_pipeline_ingest_profile(
        &database_url,
        "ui-tree-graph-live-ingest-v1",
        false,
        "SKIP_VECTOR_ON_UNAVAILABLE",
    );
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FailingEmbeddingRuntime::default(),
    );

    let result = service
        .ingest(ui_tree_graph_live_memory_card_request(
            "ui-tree-graph-live-ingest-v1",
            "# Runtime Workbench Summary\n\nThe workbench stores a runtime summary.\n\n## Retrieval Detail\n\nGraph tree persistence should keep source, text units, degraded vector carrier rows, and tree nodes.",
        ))
        .await
        .unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.embedding_status, "FAILED");
    assert!(result.text_unit_count >= 2);

    let connection = Connection::open(&database_url).unwrap();
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_ui_tree_graph_live_sources WHERE source_code = 'ui-tree-graph-live-source-fixed'"
        ),
        1
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_ui_tree_graph_live_text_units WHERE source_code = 'ui-tree-graph-live-source-fixed'"
        ),
        result.text_unit_count as i64
    );
    assert_eq!(
        query_i64(
            &connection,
            "SELECT COUNT(*) FROM rag_ui_tree_graph_live_vectors WHERE source_code = 'ui-tree-graph-live-source-fixed'"
        ),
        result.text_unit_count as i64
    );

    let tree_nodes = graph_service
        .search(
            graph_scope_for_namespace("rag_ui_tree_graph_live_tree"),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "sourceCode".into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text("ui-tree-graph-live-source-fixed".into()),
                }],
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec!["RagUiTreeGraphLiveNode".into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();
    assert!(
        tree_nodes.len() >= 2,
        "markdown headings should persist root and section tree nodes"
    );
    assert!(
        tree_nodes.iter().all(|hit| hit.node.full_text.is_some()),
        "tree fulltext should remain available when embedding degrades"
    );
    assert!(
        tree_nodes
            .iter()
            .all(|hit| hit.node.embedding_text.is_none()),
        "failed tree-vector indexing must not write semantic text without a projection"
    );

    let related_text_units = graph_service
        .search(
            graph_scope_for_namespace("rag_ui_tree_graph_live_tree"),
            GraphSearchQuery {
                filters: Vec::new(),
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec!["RAG_TEXT_UNIT".into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();
    assert!(
        !related_text_units.is_empty(),
        "extract-tree generated relatedGraph should be persisted beside tree nodes"
    );
}

#[tokio::test]
async fn graph_tree_persist_accepts_tree_nodes_without_summary_embedding() {
    let database_url = temp_database_path("rag-skill-graph-tree-no-summary-embedding")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-tree-no-summary-embedding-store")
        .to_string_lossy()
        .to_string();
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    enable_skill_graph_tree_with_sqlite_text_units(&database_url);
    let mut command = tree_route_command(
        "rag-tree-no-summary-embedding",
        "Graph fulltext summary without semantic projection.",
        vec![(
            "skill:TRON_CORE:rag-tree-no-summary-embedding:v1:0",
            "Covered SQLite unit.",
        )],
        vec!["skill:TRON_CORE:rag-tree-no-summary-embedding:v1:0"],
    );
    command.tree_nodes[0].summary_embedding_status = "FAILED".into();
    command.tree_nodes[0].summary_embedding_model_code = None;
    command.tree_nodes[0].summary_embedding_version = None;
    command.tree_nodes[0].summary_embedding_text_hash = None;
    command.tree_nodes[0].summary_embedding_dimension = None;
    command.tree_nodes[0].summary_embedding = None;

    repository.lock().unwrap().persist_ingest(command).unwrap();

    let tree_nodes = graph_service
        .search(
            graph_scope_for_namespace("rag_skill_tree"),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: "treeNodeCode".into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text("rag-tree-no-summary-embedding:root".into()),
                }],
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec!["RagSkillTreeNode".into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(tree_nodes.len(), 1);
    assert_eq!(
        tree_nodes[0].node.full_text.as_deref(),
        Some("Graph fulltext summary without semantic projection.")
    );
    assert_eq!(tree_nodes[0].node.embedding_text, None);
}

#[tokio::test]
async fn graph_tree_route_with_graph_text_units_returns_covered_text_unit_rows() {
    let database_url = temp_database_path("rag-skill-graph-tree-graph-text")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-tree-graph-text-store")
        .to_string_lossy()
        .to_string();
    let (repository, _) = build_repository(&database_url, &graph_path);
    enable_skill_graph_tree_with_graph_text_units(&database_url);

    repository
        .lock()
        .unwrap()
        .persist_ingest(tree_route_command(
            "rag-tree-graph",
            "Graph vector summary for a covered branch.",
            vec![
                (
                    "skill:TRON_CORE:rag-tree-graph:v1:0",
                    "Covered graph unit only.",
                ),
                (
                    "skill:TRON_CORE:rag-tree-graph:v1:1",
                    "Uncovered graph unit must stay out.",
                ),
            ],
            vec!["skill:TRON_CORE:rag-tree-graph:v1:0"],
        ))
        .unwrap();

    let rows = repository
        .lock()
        .unwrap()
        .search_route(tree_route_query(
            "TREE_VECTOR_V1",
            "covered branch",
            Some(embedding_4(1.0, 0.0)),
            "rag-tree-graph",
        ))
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].target_code, "skill:TRON_CORE:rag-tree-graph:v1:0");
    assert_eq!(rows[0].field_values["matchedBy"], json!("TREE_VECTOR_V1"));
    assert_eq!(
        rows[0].field_values["treeNodeCode"],
        json!("rag-tree-graph:root")
    );
    assert!(rows[0].field_values.get("treeScore").is_some());
    assert!(rows[0].text.contains("Covered graph unit only."));
    assert!(!rows[0].text.contains("Uncovered graph unit must stay out."));
}

#[tokio::test]
async fn graph_tree_hydration_deduplicates_parent_child_hits_by_text_unit() {
    let database_url = temp_database_path("rag-skill-graph-tree-dedup")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-tree-dedup-store")
        .to_string_lossy()
        .to_string();
    let (repository, _) = build_repository(&database_url, &graph_path);
    enable_skill_graph_tree_with_graph_text_units(&database_url);

    let mut command = tree_route_command(
        "rag-tree-dedup",
        "Duplicate branch summary root.",
        vec![(
            "skill:TRON_CORE:rag-tree-dedup:v1:0",
            "Covered graph unit should appear once.",
        )],
        vec!["skill:TRON_CORE:rag-tree-dedup:v1:0"],
    );
    let mut child = command.tree_nodes[0].clone();
    child.tree_node_code = "rag-tree-dedup:child".into();
    child.parent_node_code = "rag-tree-dedup:root".into();
    child.title = "rag-tree-dedup child summary".into();
    child.summary_text = "Duplicate branch summary child.".into();
    child.ordinal = 1;
    command.tree_nodes.push(child);
    repository.lock().unwrap().persist_ingest(command).unwrap();

    let mut query = tree_route_query(
        "TREE_FULLTEXT_V1",
        "Duplicate branch summary",
        None,
        "rag-tree-dedup",
    );
    query.limit = 2;
    let rows = repository.lock().unwrap().search_route(query).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].target_code, "skill:TRON_CORE:rag-tree-dedup:v1:0");
}

#[tokio::test]
async fn graph_tree_route_rejects_in_filter_instead_of_ignoring_it() {
    let database_url = temp_database_path("rag-skill-graph-tree-filter-in")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-tree-filter-in-store")
        .to_string_lossy()
        .to_string();
    let (repository, _) = build_repository(&database_url, &graph_path);
    enable_skill_graph_tree_with_graph_text_units(&database_url);
    repository
        .lock()
        .unwrap()
        .persist_ingest(tree_route_command(
            "rag-tree-filter",
            "Graph filter branch summary.",
            vec![(
                "skill:TRON_CORE:rag-tree-filter:v1:0",
                "Covered graph filter unit.",
            )],
            vec!["skill:TRON_CORE:rag-tree-filter:v1:0"],
        ))
        .unwrap();

    let mut query = tree_route_query(
        "TREE_FULLTEXT_V1",
        "Graph filter branch",
        None,
        "rag-tree-filter",
    );
    query.filters = json!({
        "skillName": { "op": "in", "value": ["rag-tree-filter"] }
    });
    let error = repository.lock().unwrap().search_route(query).unwrap_err();

    assert_eq!(error, RagError::SearchFilterInvalid);
}

#[test]
fn graph_backed_write_rejects_sqlite_tree_without_sqlite_text_units() {
    let database_url = temp_database_path("rag-skill-graph-tree-sqlite-no-text")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-tree-sqlite-no-text-store")
        .to_string_lossy()
        .to_string();
    let (repository, _) = build_repository(&database_url, &graph_path);
    enable_skill_graph_tree_with_sqlite_tree_and_graph_text_units(&database_url);

    let error = repository
        .lock()
        .unwrap()
        .persist_ingest(tree_route_command(
            "rag-tree-sqlite-no-text",
            "SQLite tree without SQLite text units cannot run.",
            vec![(
                "skill:TRON_CORE:rag-tree-sqlite-no-text:v1:0",
                "Covered graph unit.",
            )],
            vec!["skill:TRON_CORE:rag-tree-sqlite-no-text:v1:0"],
        ))
        .unwrap_err();

    assert_eq!(error, RagError::StorageTopologyUnsupported);
}

#[test]
fn graph_tree_storage_exposes_tree_retrieval_strategy_options() {
    let database_url = temp_database_path("rag-skill-graph-tree-options")
        .to_string_lossy()
        .to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    enable_skill_graph_tree_with_graph_text_units(&database_url);
    let service = RagManagementService::new(repository, DefaultRagStrategyRegistry::default());

    let catalog = service.profile_catalog().unwrap();
    let tree_options = catalog
        .retrieval_strategies
        .iter()
        .filter(|option| {
            option.corpus_id == "skill-graph"
                && matches!(option.route_type.as_str(), "TREE_FULLTEXT" | "TREE_VECTOR")
        })
        .collect::<Vec<_>>();

    assert!(tree_options.iter().any(|option| {
        option.route_type == "TREE_FULLTEXT"
            && option.strategy_ids == vec!["TREE_FULLTEXT_V1".to_string()]
    }));
    assert!(tree_options.iter().any(|option| {
        option.route_type == "TREE_VECTOR"
            && option.strategy_ids == vec!["TREE_VECTOR_V1".to_string()]
    }));
}

#[tokio::test]
async fn graph_related_write_keeps_legacy_skill_anchor_compatible() {
    let database_url = temp_database_path("rag-skill-graph-legacy-anchor")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-legacy-anchor-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, _) = build_repository(&database_url, &graph_path);
    let service = RagIngestService::new(repository, DefaultRagStrategyRegistry::default(), runtime);
    let mut request = skill_graph_request(
        "rag-legacy-anchor",
        "Legacy related graph payloads may still use the old skill anchor.",
    );
    request.payload["relatedGraph"]["edges"][0]["from"] = json!("$skill");
    request.payload["relatedGraph"]["edges"][1]["from"] = json!("$skill");

    let result = service.ingest(request).await.unwrap();

    assert_eq!(result.status, "ACTIVE");
    assert_eq!(result.text_unit_count, 1);
}

#[tokio::test]
async fn skill_graph_default_write_policy_allows_custom_kind_relationship_and_edge_properties() {
    let database_url = temp_database_path("rag-skill-graph-custom")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-custom-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, _) = build_repository(&database_url, &graph_path);
    let service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    let mut request = skill_graph_request(
        "rag-prompter",
        "Builds prompt templates for RAG graph searches.",
    );
    request.payload["relatedGraph"]["nodes"][0]["ref"] = json!("prompt.rag");
    request.payload["relatedGraph"]["nodes"][0]["kind"] = json!("PROMPT_TEMPLATE");
    request.payload["relatedGraph"]["nodes"][0]["label"] = json!("RagPromptTemplate");
    request.payload["relatedGraph"]["nodes"][0]["code"] = json!("RAG_PROMPT_TEMPLATE");
    request.payload["relatedGraph"]["nodes"][0]["title"] = json!("RAG Prompt Template");
    request.payload["relatedGraph"]["edges"][0]["to"] = json!("prompt.rag");
    request.payload["relatedGraph"]["edges"][0]["relationship"] = json!("USES_PROMPT_TEMPLATE");
    request.payload["relatedGraph"]["edges"][0]["properties"] = json!({
        "required": true,
        "weight": 0.7,
        "usage": "agent-selected"
    });

    service.ingest(request).await.unwrap();

    let search_service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    let mut request = search_request("skill-graph-related-v1", "rag-prompter");
    request.filters = json!({ "skillName": "rag-prompter" });
    let result = search_service.search(request).await.unwrap();
    assert_eq!(result.hits.len(), 1);
    let nodes = result.hits[0].metadata["relatedGraph"]["nodes"]
        .as_array()
        .expect("related graph nodes should be returned");
    assert!(nodes.iter().any(|node| {
        node["relatedKind"] == json!("PROMPT_TEMPLATE")
            && node["labels"][0] == json!("RagPromptTemplate")
    }));
    let edges = result.hits[0].metadata["relatedGraph"]["edges"]
        .as_array()
        .expect("related graph edges should be returned");
    let prompt_edge = edges
        .iter()
        .find(|edge| edge["relationship"] == json!("USES_PROMPT_TEMPLATE"))
        .expect("custom relationship edge should be returned");
    assert_eq!(prompt_edge["properties"]["required"], json!(true));
    assert_eq!(prompt_edge["properties"]["weight"], json!(0.7));
    assert_eq!(prompt_edge["properties"]["usage"], json!("agent-selected"));
}

#[tokio::test]
async fn skill_graph_related_write_links_new_nodes_to_existing_nodes_without_updating_existing_nodes()
 {
    let database_url = temp_database_path("rag-skill-graph-existing-node")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-existing-node-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    graph_service
        .create_node(
            graph_scope(),
            GraphNodeWrite {
                node_code: "EXISTING_TOOL".into(),
                labels: vec!["RagTool".into()],
                properties: BTreeMap::from([
                    ("provider".into(), GraphPropertyValue::Text("git".into())),
                    ("priority".into(), GraphPropertyValue::Integer(10)),
                ]),
                full_text: Some("Existing Git tool".into()),
                embedding_text: None,
                embedding_model_code: None,
            },
        )
        .await
        .unwrap();
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    let mut request = skill_graph_request(
        "rag-existing-linker",
        "Links new graph data to existing graph nodes.",
    );
    request.payload["relatedGraph"] = json!({
        "nodes": [{
            "ref": "new.topic",
            "kind": "TOPIC",
            "label": "RagTopic",
            "code": "NEW_TOPIC",
            "title": "New Topic",
            "text": "New topic graph node",
            "properties": { "priority": 3 }
        }],
        "existingNodes": [{
            "ref": "existing.tool",
            "code": "EXISTING_TOOL"
        }],
        "edges": [{
            "from": "new.topic",
            "to": "existing.tool",
            "relationship": "RELATES_TO",
            "properties": { "weight": 0.8 }
        }]
    });

    service.ingest(request).await.unwrap();

    let existing = graph_service
        .get_node(graph_scope(), "EXISTING_TOOL".into())
        .unwrap();
    assert_eq!(
        existing.properties.get("provider"),
        Some(&GraphPropertyValue::Text("git".into()))
    );
    assert_eq!(
        existing.properties.get("priority"),
        Some(&GraphPropertyValue::Integer(10))
    );
    assert_eq!(existing.full_text.as_deref(), Some("Existing Git tool"));
    let created = graph_service
        .get_node(graph_scope(), "NEW_TOPIC".into())
        .unwrap();
    assert_eq!(created.labels, vec!["RagTopic".to_string()]);
    let edge_code = test_stable_graph_code(
        "RAG_RELATED_EDGE",
        &["NEW_TOPIC", "RELATES_TO", "EXISTING_TOOL"],
    );
    let edge = graph_service.get_edge(graph_scope(), edge_code).unwrap();
    assert_eq!(edge.source_node_code, "NEW_TOPIC");
    assert_eq!(edge.target_node_code, "EXISTING_TOOL");
    assert_eq!(edge.relationship, "RELATES_TO");
    assert_eq!(
        edge.properties.get("weight"),
        Some(&GraphPropertyValue::Decimal(0.8))
    );
}

#[tokio::test]
async fn skill_graph_related_write_rejects_missing_existing_nodes_before_writing_new_nodes() {
    let database_url = temp_database_path("rag-skill-graph-missing-existing-node")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-missing-existing-node-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, graph_service) = build_repository(&database_url, &graph_path);
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    let mut request = skill_graph_request(
        "rag-missing-existing-linker",
        "Fails when an existing graph node reference does not resolve.",
    );
    request.payload["relatedGraph"] = json!({
        "nodes": [{
            "ref": "new.topic",
            "kind": "TOPIC",
            "label": "RagTopic",
            "code": "NEW_TOPIC",
            "title": "New Topic"
        }],
        "existingNodes": [{
            "ref": "missing.tool",
            "code": "MISSING_TOOL"
        }],
        "edges": [{
            "from": "new.topic",
            "to": "missing.tool",
            "relationship": "RELATES_TO"
        }]
    });

    assert_eq!(
        service.ingest(request).await.unwrap_err(),
        RagError::IngestRequestInvalid
    );
    assert!(
        graph_service
            .get_node(graph_scope(), "NEW_TOPIC".into())
            .is_err(),
        "new related graph nodes should not be written when existing node preflight fails"
    );
}

#[tokio::test]
async fn skill_graph_write_policy_rejects_values_only_when_whitelists_are_configured() {
    let database_url = temp_database_path("rag-skill-graph-reject")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-reject-store")
        .to_string_lossy()
        .to_string();
    let (repository, _) = build_repository(&database_url, &graph_path);
    set_skill_graph_write_policy(
        &database_url,
        json!({
            "allowedNodeKinds": ["TOOL", "TAG"],
            "allowedRelationships": ["REQUIRES_TOOL", "HAS_TAG"],
            "maxRelatedNodes": 50,
            "maxRelatedEdges": 100
        }),
    );
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

#[tokio::test]
async fn skill_graph_main_node_business_fields_follow_corpus_field_mapping() {
    let database_url = temp_database_path("rag-skill-graph-field-mapping")
        .to_string_lossy()
        .to_string();
    let graph_path = temp_database_path("rag-skill-graph-field-mapping-store")
        .to_string_lossy()
        .to_string();
    let runtime = FakeRuntime::new(embedding_4(1.0, 0.0));
    let (repository, _) = build_repository(&database_url, &graph_path);
    add_skill_alias_field_mapping(&database_url);
    let service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    let mut request = skill_graph_request(
        "rag-mapper",
        "Verifies graph fields are driven by corpus mapping.",
    );
    request.payload["domainFields"]["skillAlias"] = json!("mapped-skill-alias");
    service.ingest(request).await.unwrap();

    let search_service =
        RagSearchService::new(repository, DefaultRagStrategyRegistry::default(), runtime);
    let mut request = search_request("skill-graph-related-v1", "rag-mapper");
    request.filters = json!({ "skillName": "rag-mapper" });
    request.context.return_fields = vec!["skillName".into(), "skillAlias".into()];
    let result = search_service.search(request).await.unwrap();
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].fields["skillName"], json!("rag-mapper"));
    assert_eq!(
        result.hits[0].fields["skillAlias"],
        json!("mapped-skill-alias")
    );
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
            source_type: String::new(),
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
                        "from": "$target",
                        "to": "tool.git",
                        "relationship": "REQUIRES_TOOL",
                        "properties": {
                            "required": true,
                            "usage": "source-control",
                            "weight": 0.9
                        }
                    },
                    {
                        "from": "$target",
                        "to": "tag.review",
                        "relationship": "HAS_TAG"
                    }
                ]
            }
        }),
    }
}

fn skill_graph_tree_text_request(
    profile_id: &str,
    skill_name: &str,
    text: &str,
) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: "skill-graph".into(),
        ingest_profile_id: profile_id.into(),
        workspace_code: "TRON_CORE".into(),
        access_context: access_context(),
        target_scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
        },
        source: RagIngestSource {
            code: format!("skill:TRON_CORE:{skill_name}"),
            version: "v1".into(),
            source_type: String::new(),
        },
        payload: json!({
            "source": {
                "code": format!("skill:TRON_CORE:{skill_name}"),
                "version": "v1",
                "type": "SKILL"
            },
            "title": skill_name,
            "text": text,
            "metadata": {}
        }),
    }
}

fn ui_tree_graph_live_memory_card_request(profile_id: &str, text: &str) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: "ui-tree-graph-live".into(),
        ingest_profile_id: profile_id.into(),
        workspace_code: "TRON_CORE".into(),
        access_context: RagIngestAccessContext {
            user_code: "USR-ADMIN".into(),
            agent_code: "AGENT_CODEX".into(),
            workspace_code: "TRON_CORE".into(),
        },
        target_scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
        },
        source: RagIngestSource {
            code: "ui-tree-graph-live-source-fixed".into(),
            version: "v1".into(),
            source_type: String::new(),
        },
        payload: json!({
            "source": {
                "code": "ui-tree-graph-live-source-fixed",
                "version": "v1",
                "type": "MEMORY_CARD"
            },
            "title": "Runtime Workbench Summary",
            "text": text,
            "metadata": {}
        }),
    }
}

fn set_skill_graph_write_policy(database_url: &str, policy: Value) {
    let connection = Connection::open(database_url).unwrap();
    let config_json: String = connection
        .query_row(
            "SELECT config_json FROM rag_profiles WHERE profile_id = 'skill-graph-ingest-v1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut config: Value = serde_json::from_str(&config_json).unwrap();
    config["graphWritePolicy"] = policy;
    connection
        .execute(
            "UPDATE rag_profiles SET config_json = ?1 WHERE profile_id = 'skill-graph-ingest-v1'",
            params![serde_json::to_string(&config).unwrap()],
        )
        .unwrap();
}

fn insert_skill_graph_tree_pipeline_ingest_profile(
    database_url: &str,
    profile_id: &str,
    degrade_policy: &str,
) {
    let connection = Connection::open(database_url).unwrap();
    let config = json!({
        "sourceInputPolicy": {
            "supportedSourceTypes": ["SKILL"]
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
                    "strategyId": "RAG_SPLIT_MARKDOWN_HEADING_V1"
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
                    "nodeId": "tree-fulltext-index",
                    "stageType": "TREE_FULLTEXT_INDEX",
                    "strategyId": "RAG_TREE_FULLTEXT_INDEX"
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
            VALUES (?1, 'INGEST', 'enabled', 'skill-graph', '[\"skill-graph\"]',
                'v1', ?2, '{}', ?3, 1, 1)",
            params![profile_id, config.to_string(), degrade_policy],
        )
        .unwrap();
}

fn insert_ui_tree_graph_live_pipeline_ingest_profile(
    database_url: &str,
    profile_id: &str,
    include_persist_tree_edge: bool,
    degrade_policy: &str,
) {
    let connection = Connection::open(database_url).unwrap();
    let mut edges = vec![
        json!({ "from": "validate-source", "to": "prepare-payload" }),
        json!({ "from": "prepare-payload", "to": "split-text" }),
        json!({ "from": "split-text", "to": "extract-tree" }),
        json!({ "from": "split-text", "to": "fulltext-index" }),
        json!({ "from": "split-text", "to": "vector-index" }),
        json!({ "from": "extract-tree", "to": "summarize-tree" }),
        json!({ "from": "summarize-tree", "to": "tree-fulltext-index" }),
        json!({ "from": "tree-fulltext-index", "to": "tree-vector-index" }),
        json!({ "from": "tree-vector-index", "to": "persist-tree" }),
        json!({ "from": "fulltext-index", "to": "persist-source" }),
        json!({ "from": "vector-index", "to": "persist-source" }),
    ];
    if include_persist_tree_edge {
        edges.push(json!({ "from": "persist-tree", "to": "persist-source" }));
    }
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
                    "strategyId": "RAG_SPLIT_MARKDOWN_HEADING_V1"
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
                    "nodeId": "tree-fulltext-index",
                    "stageType": "TREE_FULLTEXT_INDEX",
                    "strategyId": "RAG_TREE_FULLTEXT_INDEX"
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
            "edges": edges
        }
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'INGEST', 'enabled', 'ui-tree-graph-live', '[\"ui-tree-graph-live\"]',
                'v1', ?2, '{}', ?3, 1, 1)",
            params![profile_id, config.to_string(), degrade_policy],
        )
        .unwrap();
}

fn add_skill_alias_field_mapping(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    let field_mapping_json: String = connection
        .query_row(
            "SELECT field_mapping_json FROM rag_corpora WHERE corpus_id = 'skill-graph'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut field_mapping: Value = serde_json::from_str(&field_mapping_json).unwrap();
    field_mapping["source"]["skillAlias"] = json!("skillAlias");
    field_mapping["textUnit"]["skillAlias"] = json!("skillAlias");
    connection
        .execute(
            "UPDATE rag_corpora SET field_mapping_json = ?1 WHERE corpus_id = 'skill-graph'",
            params![serde_json::to_string(&field_mapping).unwrap()],
        )
        .unwrap();
}

fn enable_skill_graph_sqlite_canonical_storage(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    let sqlite_field_mapping_json: String = connection
        .query_row(
            "SELECT field_mapping_json FROM rag_corpora WHERE corpus_id = 'skill'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE rag_corpora
             SET field_mapping_json = ?1,
                 storage_binding_json = ?2
             WHERE corpus_id = 'skill-graph'",
            params![
                sqlite_field_mapping_json,
                json!({
                    "mode": "TARGETED",
                    "bindings": {
                        "source": "skill-sqlite-store",
                        "textUnit": "skill-sqlite-store",
                        "fulltext": "skill-graph-store",
                        "vector": "skill-graph-store",
                        "diagnostics": "diagnostics-store"
                    },
                    "targets": [
                        {
                            "id": "skill-sqlite-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_skill_text_units"
                        },
                        {
                            "id": "skill-graph-store",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill",
                            "nodeLabel": "RagSkill",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "diagnostics-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_diagnostics"
                        }
                    ]
                })
                .to_string()
            ],
        )
        .unwrap();
}

fn enable_skill_graph_sqlite_fulltext_carrier_storage(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "UPDATE rag_corpora
             SET storage_binding_json = ?1
             WHERE corpus_id = 'skill-graph'",
            params![
                json!({
                    "mode": "TARGETED",
                    "bindings": {
                        "source": "skill-graph-store",
                        "textUnit": "skill-graph-store",
                        "fulltext": "skill-fulltext-store",
                        "vector": "skill-graph-store",
                        "diagnostics": "diagnostics-store"
                    },
                    "targets": [
                        {
                            "id": "skill-graph-store",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill",
                            "nodeLabel": "RagSkill",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-fulltext-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_skill_graph_fulltext"
                        },
                        {
                            "id": "diagnostics-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_diagnostics"
                        }
                    ]
                })
                .to_string()
            ],
        )
        .unwrap();
}

fn enable_skill_graph_sqlite_source_with_graph_retrieval_storage(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "UPDATE rag_corpora
             SET field_mapping_json = ?1,
                 storage_binding_json = ?2
             WHERE corpus_id = 'skill-graph'",
            params![
                json!({
                    "source": {
                        "sourceCode": "source_code",
                        "sourceVersion": "source_version",
                        "sourceType": "source_type",
                        "scopeType": "scope_type",
                        "scopeCode": "scope_code",
                        "status": "status",
                        "title": "title",
                        "sourceMetadata": "metadata",
                        "skillName": "skill_name",
                        "skillDescription": "skill_description"
                    },
                    "textUnit": {
                        "sourceCode": "sourceCode",
                        "sourceVersion": "sourceVersion",
                        "scopeType": "scopeType",
                        "scopeCode": "scopeCode",
                        "targetCode": "targetCode",
                        "textUnitStatus": "textUnitStatus",
                        "text": "text",
                        "skillName": "unitSkillName",
                        "skillDescription": "unitSkillDescription"
                    }
                })
                .to_string(),
                json!({
                    "mode": "TARGETED",
                    "targets": [
                        {
                            "id": "skill-source-sqlite",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_skill_graph_sources"
                        },
                        {
                            "id": "skill-text-unit-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_text_units",
                            "nodeLabel": "RagSkillTextUnit",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-fulltext-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_fulltext",
                            "nodeLabel": "RagSkillFulltext",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-vector-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_vectors",
                            "nodeLabel": "RagSkillVector",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        }
                    ],
                    "bindings": {
                        "source": "skill-source-sqlite",
                        "textUnit": "skill-text-unit-graph",
                        "fulltext": "skill-fulltext-graph",
                        "vector": "skill-vector-graph"
                    }
                })
                .to_string()
            ],
        )
        .unwrap();
}

fn enable_skill_graph_segmented_graph_storage(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "UPDATE rag_corpora
             SET field_mapping_json = ?1,
                 storage_binding_json = ?2
             WHERE corpus_id = 'skill-graph'",
            params![
                json!({
                    "source": {
                        "sourceCode": "sourceCode",
                        "sourceVersion": "sourceVersion",
                        "sourceType": "sourceType",
                        "scopeType": "scopeType",
                        "scopeCode": "scopeCode",
                        "status": "sourceStatus",
                        "sourceStatus": "sourceStatus",
                        "title": "title",
                        "sourceMetadata": "sourceMetadataJson",
                        "skillName": "sourceSkillName",
                        "skillDescription": "sourceSkillDescription"
                    },
                    "textUnit": {
                        "sourceCode": "sourceCode",
                        "sourceVersion": "sourceVersion",
                        "scopeType": "scopeType",
                        "scopeCode": "scopeCode",
                        "targetCode": "targetCode",
                        "textUnitStatus": "textUnitStatus",
                        "text": "text",
                        "skillName": "unitSkillName",
                        "skillDescription": "unitSkillDescription"
                    }
                })
                .to_string(),
                json!({
                    "mode": "TARGETED",
                    "targets": [
                        {
                            "id": "skill-source-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_source",
                            "nodeLabel": "RagSkillSource",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-text-unit-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_text_units",
                            "nodeLabel": "RagSkillTextUnit",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-fulltext-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_fulltext",
                            "nodeLabel": "RagSkillFulltext",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-vector-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_vectors",
                            "nodeLabel": "RagSkillVector",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        }
                    ],
                    "bindings": {
                        "source": "skill-source-graph",
                        "textUnit": "skill-text-unit-graph",
                        "fulltext": "skill-fulltext-graph",
                        "vector": "skill-vector-graph"
                    }
                })
                .to_string()
            ],
        )
        .unwrap();
}

fn enable_skill_graph_tree_with_sqlite_text_units(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    let sqlite_field_mapping_json: String = connection
        .query_row(
            "SELECT field_mapping_json FROM rag_corpora WHERE corpus_id = 'skill'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE rag_corpora
             SET capabilities_json = ?1,
                 field_mapping_json = ?2,
                 storage_binding_json = ?3
             WHERE corpus_id = 'skill-graph'",
            params![
                json!({
                    "fulltext": true,
                    "vector": true,
                    "tree": true,
                    "graph": true
                })
                .to_string(),
                sqlite_field_mapping_json,
                json!({
                    "mode": "TARGETED",
                    "bindings": {
                        "source": "skill-sqlite-store",
                        "textUnit": "skill-sqlite-store",
                        "fulltext": "skill-sqlite-store",
                        "vector": "skill-sqlite-store",
                        "tree": "skill-tree-graph",
                        "diagnostics": "diagnostics-store"
                    },
                    "targets": [
                        {
                            "id": "skill-sqlite-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_skill_text_units"
                        },
                        {
                            "id": "skill-tree-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_TREE",
                            "namespace": "rag_skill_tree",
                            "nodeLabel": "RagSkillTreeNode",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "diagnostics-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_diagnostics"
                        }
                    ]
                })
                .to_string()
            ],
        )
        .unwrap();
}

fn insert_ui_tree_graph_live_corpus(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "INSERT INTO rag_corpora (
                corpus_id, display_name, status, description, capabilities_json,
                filter_schema_json, field_mapping_json, storage_binding_json,
                created_at, updated_at
            )
            VALUES (?1, ?2, 'enabled', ?3, ?4, ?5, ?6, ?7, 1, 1)",
            params![
                "ui-tree-graph-live",
                "UI Tree Graph Live",
                "Workbench-created graph tree corpus fixture.",
                json!({
                    "fulltext": true,
                    "vector": true,
                    "tree": true,
                    "graph": true
                })
                .to_string(),
                json!({
                    "fields": {
                        "sourceCode": {"type": "string", "column": "source_code"},
                        "sourceVersion": {"type": "string", "column": "source_version"},
                        "scopeType": {"type": "string", "column": "scope_type"},
                        "scopeCode": {"type": "string", "column": "scope_code"},
                        "status": {"type": "string", "column": "status"},
                        "textUnitStatus": {"type": "string", "column": "status"}
                    }
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
                        "sourceMetadata": "metadata"
                    },
                    "textUnit": {
                        "sourceCode": "source_code",
                        "sourceVersion": "source_version",
                        "scopeType": "scope_type",
                        "scopeCode": "scope_code",
                        "textUnitStatus": "status",
                        "text": "text"
                    }
                })
                .to_string(),
                json!({
                    "mode": "TARGETED",
                    "bindings": {
                        "source": "source-store",
                        "textUnit": "text-unit-store",
                        "fulltext": "text-unit-store",
                        "vector": "vector-store",
                        "tree": "tree-graph",
                        "diagnostics": "diagnostics-store"
                    },
                    "targets": [
                        {
                            "id": "source-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_ui_tree_graph_live_sources"
                        },
                        {
                            "id": "text-unit-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_ui_tree_graph_live_text_units"
                        },
                        {
                            "id": "vector-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_ui_tree_graph_live_vectors"
                        },
                        {
                            "id": "tree-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_TREE",
                            "namespace": "rag_ui_tree_graph_live_tree",
                            "nodeLabel": "RagUiTreeGraphLiveNode",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "diagnostics-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_diagnostics"
                        }
                    ]
                })
                .to_string()
            ],
        )
        .unwrap();
}

fn enable_skill_graph_tree_with_ui_sqlite_segments(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "UPDATE rag_corpora
             SET capabilities_json = ?1,
                 field_mapping_json = ?2,
                 filter_schema_json = ?3,
                 storage_binding_json = ?4
             WHERE corpus_id = 'skill-graph'",
            params![
                json!({
                    "fulltext": true,
                    "vector": true,
                    "tree": true,
                    "graph": true
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
                        "sourceMetadata": "metadata"
                    },
                    "textUnit": {
                        "sourceCode": "source_code",
                        "sourceVersion": "source_version",
                        "scopeType": "scope_type",
                        "scopeCode": "scope_code",
                        "textUnitStatus": "status",
                        "text": "text"
                    }
                })
                .to_string(),
                json!({
                    "fields": {
                        "sourceCode": {"type": "string", "column": "source_code"},
                        "sourceVersion": {"type": "string", "column": "source_version"},
                        "scopeType": {"type": "string", "column": "scope_type"},
                        "scopeCode": {"type": "string", "column": "scope_code"},
                        "status": {"type": "string", "column": "status"},
                        "textUnitStatus": {"type": "string", "column": "status"}
                    }
                })
                .to_string(),
                json!({
                    "mode": "TARGETED",
                    "bindings": {
                        "source": "ui-source-sqlite",
                        "textUnit": "ui-text-unit-sqlite",
                        "fulltext": "ui-fulltext-sqlite",
                        "vector": "ui-vector-sqlite",
                        "tree": "ui-tree-graph",
                        "diagnostics": "diagnostics-store"
                    },
                    "targets": [
                        {
                            "id": "ui-source-sqlite",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_ui_tree_graph_live_sources"
                        },
                        {
                            "id": "ui-text-unit-sqlite",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_ui_tree_graph_live_text_units"
                        },
                        {
                            "id": "ui-fulltext-sqlite",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_ui_tree_graph_live_fulltext"
                        },
                        {
                            "id": "ui-vector-sqlite",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_ui_tree_graph_live_vectors"
                        },
                        {
                            "id": "ui-tree-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_TREE",
                            "namespace": "rag_ui_tree_graph_live_tree",
                            "nodeLabel": "RagUiTreeGraphLiveNode",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "diagnostics-store",
                            "backend": "SQLITE",
                            "layout": "TABLE",
                            "table": "rag_diagnostics"
                        }
                    ]
                })
                .to_string()
            ],
        )
        .unwrap();
}

fn enable_skill_graph_tree_with_graph_text_units(database_url: &str) {
    let connection = Connection::open(database_url).unwrap();
    connection
        .execute(
            "UPDATE rag_corpora
             SET capabilities_json = ?1,
                 field_mapping_json = ?2,
                 storage_binding_json = ?3
             WHERE corpus_id = 'skill-graph'",
            params![
                json!({
                    "fulltext": true,
                    "vector": true,
                    "tree": true,
                    "graph": true
                })
                .to_string(),
                json!({
                    "source": {
                        "sourceCode": "sourceCode",
                        "sourceVersion": "sourceVersion",
                        "sourceType": "sourceType",
                        "scopeType": "scopeType",
                        "scopeCode": "scopeCode",
                        "status": "sourceStatus",
                        "sourceStatus": "sourceStatus",
                        "title": "title",
                        "sourceMetadata": "sourceMetadataJson",
                        "skillName": "skillName",
                        "skillDescription": "skillDescription"
                    },
                    "textUnit": {
                        "sourceCode": "sourceCode",
                        "sourceVersion": "sourceVersion",
                        "scopeType": "scopeType",
                        "scopeCode": "scopeCode",
                        "targetCode": "targetCode",
                        "textUnitStatus": "textUnitStatus",
                        "text": "text",
                        "skillName": "skillName",
                        "skillDescription": "skillDescription"
                    }
                })
                .to_string(),
                json!({
                    "mode": "TARGETED",
                    "bindings": {
                        "source": "skill-source-graph",
                        "textUnit": "skill-text-unit-graph",
                        "fulltext": "skill-fulltext-graph",
                        "vector": "skill-vector-graph",
                        "tree": "skill-tree-graph"
                    },
                    "targets": [
                        {
                            "id": "skill-source-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_tree_source",
                            "nodeLabel": "RagSkillTreeSource",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-text-unit-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_tree_text_units",
                            "nodeLabel": "RagSkillTreeTextUnit",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-fulltext-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_tree_fulltext",
                            "nodeLabel": "RagSkillTreeFulltext",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-vector-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_SINGLE_NODE",
                            "namespace": "rag_skill_tree_vectors",
                            "nodeLabel": "RagSkillTreeVector",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        },
                        {
                            "id": "skill-tree-graph",
                            "backend": "GRAPH",
                            "layout": "GRAPH_TREE",
                            "namespace": "rag_skill_tree",
                            "nodeLabel": "RagSkillTreeNode",
                            "codeStrategy": "CORPUS_SOURCE_VERSION_SCOPE_HASH"
                        }
                    ]
                })
                .to_string()
            ],
        )
        .unwrap();
}

fn enable_skill_graph_tree_with_sqlite_tree_and_graph_text_units(database_url: &str) {
    enable_skill_graph_tree_with_graph_text_units(database_url);
    let connection = Connection::open(database_url).unwrap();
    let binding_json: String = connection
        .query_row(
            "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'skill-graph'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut binding = serde_json::from_str::<Value>(&binding_json).unwrap();
    binding["bindings"]["tree"] = json!("skill-tree-sqlite");
    binding["targets"]
        .as_array_mut()
        .expect("storage targets should be an array")
        .push(json!({
            "id": "skill-tree-sqlite",
            "backend": "SQLITE",
            "layout": "TABLE",
            "table": "rag_skill_tree_nodes"
        }));
    connection
        .execute(
            "UPDATE rag_corpora
             SET storage_binding_json = ?1
             WHERE corpus_id = 'skill-graph'",
            params![binding.to_string()],
        )
        .unwrap();
}

async fn assert_segment_node(
    graph_service: &GraphService<ScopeResolver<FakeWorkspaceProvider>>,
    namespace: &str,
    label: &str,
    skill_name_property: &str,
    skill_name: &str,
) {
    let nodes = graph_service
        .search(
            graph_scope_for_namespace(namespace),
            GraphSearchQuery {
                filters: vec![GraphPropertyFilter {
                    property_name: skill_name_property.into(),
                    operator: GraphFilterOperator::Equals,
                    value: GraphPropertyValue::Text(skill_name.into()),
                }],
                full_text: None,
                vector_text: None,
                embedding_model_code: None,
                traversal: None,
                labels: vec![label.into()],
                limit: GraphQueryLimit::new(10).unwrap(),
                min_score: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(nodes.len(), 1, "{label} should contain one segment node");
}

fn tree_route_command(
    skill_name: &str,
    summary_text: &str,
    units: Vec<(&str, &str)>,
    covered_text_unit_codes: Vec<&str>,
) -> RagPersistIngestCommand {
    let source_code = format!("skill:TRON_CORE:{skill_name}");
    let covered_text_unit_count = covered_text_unit_codes.len() as i64;
    RagPersistIngestCommand {
        corpus_id: "skill-graph".into(),
        profile_id: "skill-graph-tree-ingest-v1".into(),
        source_code: source_code.clone(),
        source_version: "v1".into(),
        scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
        },
        source_status: "ACTIVE".into(),
        title: Some(skill_name.into()),
        content_hash: format!("{skill_name}-content-hash"),
        body_hash: Some(format!("{skill_name}-body-hash")),
        metadata: json!({}),
        domain_fields: std::collections::BTreeMap::from([
            ("skillName".into(), json!(skill_name)),
            ("skillDescription".into(), json!(summary_text)),
        ]),
        related_graph: None,
        graph_write_policy: json!({}),
        text_units: units
            .into_iter()
            .enumerate()
            .map(|(ordinal, (code, text))| RagTextUnitWrite {
                text_unit_code: code.into(),
                unit_type: "LEAF".into(),
                text: text.into(),
                search_text: text.into(),
                heading_path: "Skill".into(),
                ordinal,
                token_count: 4,
                content_hash: format!("{code}-hash"),
                status: "ACTIVE".into(),
                fulltext_status: "INDEXED".into(),
                embedding_model_code: Some("FAKE_EMBEDDING".into()),
                embedding_version: Some("v1".into()),
                embedding_text_hash: Some(format!("{code}-embedding-hash")),
                embedding_dimension: Some(4),
                embedding: Some(embedding_bytes_4(1.0, 0.0)),
                embedding_status: "INDEXED".into(),
                metadata: json!({}),
            })
            .collect(),
        tree_nodes: vec![RagTreeNodeWrite {
            tree_node_code: format!("{skill_name}:root"),
            parent_node_code: "0".into(),
            source_code,
            source_version: "v1".into(),
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
            path_key: "/".into(),
            level: 0,
            ordinal: 0,
            title: format!("{skill_name} summary"),
            summary_text: summary_text.into(),
            summary_token_count: 6,
            summary_content_hash: format!("{skill_name}-summary-hash"),
            summary_fulltext_status: "INDEXED".into(),
            summary_embedding_status: "INDEXED".into(),
            summary_embedding_model_code: Some("FAKE_EMBEDDING".into()),
            summary_embedding_version: Some("v1".into()),
            summary_embedding_text_hash: Some(format!("{skill_name}-summary-embedding-hash")),
            summary_embedding_dimension: Some(4),
            summary_embedding: Some(embedding_bytes_4(1.0, 0.0)),
            covered_text_unit_codes: covered_text_unit_codes
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            covered_text_unit_count,
            child_start_ordinal: 0,
            child_end_ordinal: 0,
            metadata: json!({ "summaryStrategy": "TEST" }),
            status: "ACTIVE".into(),
        }],
        warnings: Vec::new(),
        workspace_code: "TRON_CORE".into(),
        user_code: "USR-AUTH-0001".into(),
        agent_code: "AGENT_CODEX".into(),
    }
}

fn tree_route_query(
    retriever: &str,
    query: &str,
    query_embedding: Option<Vec<f32>>,
    skill_name: &str,
) -> RagRouteSearchQuery {
    RagRouteSearchQuery {
        corpus_id: "skill-graph".into(),
        route_id: format!("{skill_name}-tree"),
        retriever: retriever.into(),
        workspace_code: "TRON_CORE".into(),
        query: query.into(),
        query_embedding,
        query_embedding_model_code: Some("FAKE_EMBEDDING".into()),
        source_version: "v1".into(),
        scopes: vec![RagSearchScope {
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
        }],
        filters: json!({}),
        policy: json!({ "includeRoot": true }),
        return_fields: Vec::new(),
        limit: 5,
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
    graph_scope_for_namespace("rag_skill")
}

fn graph_scope_for_namespace(namespace: &str) -> GraphScopeCommand {
    GraphScopeCommand {
        namespace: namespace.into(),
        workspace_code: "TRON_CORE".into(),
        app_code: None,
        user_code: None,
    }
}

fn test_stable_graph_code(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    format!("{prefix}_{:016X}", hasher.finish())
}

fn embedding_4(left: f32, right: f32) -> Vec<f32> {
    vec![left, right, 0.0, 0.0]
}

fn embedding_bytes_4(left: f32, right: f32) -> Vec<u8> {
    embedding_4(left, right)
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect()
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

#[derive(Clone, Default)]
struct FailingEmbeddingRuntime;

impl RagEmbeddingRuntime for FailingEmbeddingRuntime {
    fn embed<'a>(
        &'a self,
        _call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async { Err("embedding provider unavailable".into()) })
    }
}

impl RagDocumentSourceRuntime for FailingEmbeddingRuntime {
    fn resolve_project_doc_markdown<'a>(
        &'a self,
        _call: RagDocumentSourceCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async { Err(RagError::SourceResolveFailed) })
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

fn query_i64(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}
