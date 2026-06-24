use std::{
    collections::{BTreeMap, VecDeque},
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository,
    application::rag::{
        DefaultRagStrategyRegistry, RagConfigRepository, RagDocumentSourceCall,
        RagDocumentSourceRuntime, RagEmbeddingCall, RagEmbeddingResult, RagEmbeddingRuntime,
        RagIngestAccessContext, RagIngestRepository, RagIngestRequest, RagIngestScope,
        RagIngestService, RagIngestSource, RagPersistIngestCommand, RagRerankCall,
        RagResolvedDocument, RagRetryIndexRequest, RagRetryIndexService, RagRouteSearchQuery,
        RagSearchContextRequest, RagSearchRepository, RagSearchRequest, RagSearchRow,
        RagSearchRuntime, RagSearchScope, RagSearchService, RagSearchTraceRequest,
        RagStrategyRegistry, RagTextUnitWrite, RagTreeNodeWrite, StrategyRegistrationStatus,
        TopicFallback, TopicPolicy, compile_retrieval_profile, merge_search_filters,
    },
    domain::rag::{RagCorpusRecord, RagError, RagProfileRecord, RagProfileType, RagStatus},
    shared::sqlite_extensions::default_sqlite_extension_context,
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
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
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
    assert!(
        result
            .stage_results
            .iter()
            .any(|stage| stage.stage_type == "ROUTE_SEARCH"
                && stage.output_artifact_keys == vec!["hits.route"])
    );
    assert!(
        result
            .stage_results
            .iter()
            .any(|stage| stage.stage_type == "BUILD_CONTEXT"
                && stage.output_artifact_keys == vec!["context.pack"])
    );
    assert_eq!(result.hits[0].source_version, "v1");
    assert_eq!(result.hits[0].title.as_deref(), Some("RAG citation rule"));
    assert_eq!(
        result.hits[0].metadata,
        json!({ "seed": "search-contract" })
    );
    assert!(result.hits[0].fields.is_empty());
    let context_pack = result.context_pack.unwrap();
    assert!(context_pack.token_count <= 32);
    assert_eq!(
        context_pack.items[0].metadata["metadata"],
        json!({ "seed": "search-contract" })
    );
    assert_eq!(context_pack.items[0].metadata["returnFields"], json!({}));
    assert!(context_pack.items[0].metadata.get("targetCode").is_none());
    assert!(context_pack.items[0].metadata.get("textUnit").is_none());
}

#[tokio::test]
async fn memory_search_projects_requested_logical_return_fields_only() {
    let database_url = temp_database_path("rag-search-memory-return-fields")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );

    let result = service
        .search(search_request_with_return_fields(
            "memory-related-v1",
            "cite source titles",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
            &["memoryType"],
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].fields["memoryType"], json!("RULE"));
    assert!(result.hits[0].fields.get("memory_type").is_none());
    assert_eq!(
        result.hits[0].metadata,
        json!({ "seed": "search-contract" })
    );
}

#[tokio::test]
async fn pipeline_only_memory_retrieval_profile_preserves_existing_search_contract() {
    let database_url = temp_database_path("rag-search-pipeline-only-memory")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    insert_pipeline_only_retrieval_profile(&database_url, "memory-pipeline-only-v1");
    let repository = CapturingSearchRepository::connect(&database_url);
    let route_calls = repository.route_calls.clone();
    let runtime = FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0));
    let service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let result = service
        .search(search_request(
            "memory-pipeline-only-v1",
            "cite source titles",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
        ))
        .await
        .unwrap();

    assert_eq!(result.context_strategy, "RESULT_LIST");
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert_eq!(runtime.embedding_call_count(), 1);
    assert_eq!(
        route_calls
            .lock()
            .unwrap()
            .iter()
            .map(|call| call.retriever.as_str())
            .collect::<Vec<_>>(),
        vec![
            "STRUCTURED_FILTER_SQLITE_V1",
            "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
            "VECTOR_SQLITE_VEC0_COSINE_V1"
        ]
    );
    assert_eq!(
        result
            .stage_results
            .iter()
            .map(|stage| stage.node_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "structured-route",
            "fulltext-route",
            "vector-route",
            "fuse-results",
            "dedup-results",
            "build-context"
        ]
    );
}

#[tokio::test]
async fn tree_fulltext_retriever_matches_summary_and_hydrates_text_units() {
    let database_url = temp_database_path("rag-search-tree-fulltext")
        .to_string_lossy()
        .to_string();
    let repository = Arc::new(Mutex::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
    ));
    set_memory_card_tree_table(&database_url, "rag_memory_tree_nodes");
    insert_tree_pipeline_ingest_profile(&database_url, "memory-card-tree-pipeline-v1");
    let service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(0.25, 0.75)),
    );
    let mut request = memory_request("memory-rag-rule", "v1", "SPACE", "TRON_CORE");
    request.ingest_profile_id = "memory-card-tree-pipeline-v1".into();
    service.ingest(request).await.unwrap();

    let rows = repository
        .search_route(RagRouteSearchQuery {
            corpus_id: "memory-card".into(),
            route_id: "tree-fulltext".into(),
            retriever: "TREE_FULLTEXT_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "Always cite source titles".into(),
            query_embedding: None,
            query_embedding_model_code: None,
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({}),
            policy: json!({ "includeRoot": true }),
            return_fields: vec![],
            limit: 5,
        })
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].target_code, "memory-rag-rule:v1:0");
    assert_eq!(rows[0].field_values["matchedBy"], json!("TREE_FULLTEXT_V1"));
    assert_eq!(
        rows[0].field_values["treeNodeTitle"],
        json!("RAG citation rule")
    );
    assert!(rows[0].field_values.get("treeNodeCode").is_some());
    assert!(
        rows[0]
            .text
            .contains("Always cite source titles in RAG answers")
    );
}

#[tokio::test]
async fn tree_fulltext_retriever_filters_tree_node_scope_before_hydration() {
    let database_url = temp_database_path("rag-search-tree-scope-filter")
        .to_string_lossy()
        .to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    insert_tree_scope_corpus(
        &database_url,
        "tree-scope-corpus",
        "rag_tree_scope_tree_nodes",
    );
    repository
        .persist_ingest(tree_scope_filter_command(
            "tree-scope-corpus",
            "SPACE",
            "TRON_CORE",
            "scope-space-unit",
            "Ordinary note without matching terms.",
            vec!["scope-space-unit"],
        ))
        .unwrap();
    repository
        .persist_ingest(tree_scope_filter_command(
            "tree-scope-corpus",
            "USER",
            "USR-AUTH-0001",
            "scope-user-unit",
            "Leaked branch summary should not hydrate through another scope.",
            vec!["scope-space-unit"],
        ))
        .unwrap();

    let rows = repository
        .search_route(RagRouteSearchQuery {
            corpus_id: "tree-scope-corpus".into(),
            route_id: "tree-fulltext".into(),
            retriever: "TREE_FULLTEXT_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "Leaked branch summary".into(),
            query_embedding: None,
            query_embedding_model_code: None,
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({}),
            policy: json!({}),
            return_fields: vec![],
            limit: 5,
        })
        .unwrap();

    assert!(
        rows.is_empty(),
        "tree rows from a different scope must not hydrate target-scope text units: {rows:?}"
    );
}

#[tokio::test]
async fn tree_vector_retriever_filters_tree_rows_by_source_and_scope_before_hydrating_units() {
    let database_url = temp_database_path("rag-search-tree-scope-isolation")
        .to_string_lossy()
        .to_string();
    let repository = Arc::new(Mutex::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
    ));
    set_memory_card_tree_table(&database_url, "rag_memory_tree_nodes");
    insert_tree_pipeline_ingest_profile(&database_url, "memory-card-tree-pipeline-v1");
    let service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(0.25, 0.75)),
    );
    let core_request = memory_request("memory-rag-rule", "v1", "SPACE", "TRON_CORE");
    service.ingest(core_request).await.unwrap();
    let mut alternate_request = memory_request("memory-rerank-second", "v1", "SPACE", "TRON_ALT");
    alternate_request.ingest_profile_id = "memory-card-tree-pipeline-v1".into();
    alternate_request.access_context.workspace_code = "TRON_ALT".into();
    alternate_request.workspace_code = "TRON_ALT".into();
    service.ingest(alternate_request).await.unwrap();
    let connection = open_prepared_connection(&database_url);
    connection
        .execute(
            "UPDATE rag_memory_tree_nodes
             SET covered_text_unit_codes = '[\"memory-rag-rule:v1:0\"]',
                 covered_text_unit_count = 1
             WHERE source_code = 'memory-rerank-second'
               AND scope_code = 'TRON_ALT'",
            [],
        )
        .unwrap();
    drop(connection);

    let core_rows = repository
        .search_route(RagRouteSearchQuery {
            corpus_id: "memory-card".into(),
            route_id: "tree-vector".into(),
            retriever: "TREE_VECTOR_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "alternate vector tree".into(),
            query_embedding: Some(embedding_4096(0.25, 0.75)),
            query_embedding_model_code: Some("fake-embedding".into()),
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({}),
            policy: json!({ "includeRoot": true }),
            return_fields: vec![],
            limit: 5,
        })
        .unwrap();
    assert!(
        core_rows.is_empty(),
        "tree rows from another scope must not hydrate same-code core text units"
    );
}

#[tokio::test]
async fn tree_vector_retriever_matches_summary_embedding_and_hydrates_text_units() {
    let database_url = temp_database_path("rag-search-tree-vector")
        .to_string_lossy()
        .to_string();
    let repository = Arc::new(Mutex::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
    ));
    set_memory_card_tree_table(&database_url, "rag_memory_tree_nodes");
    insert_tree_pipeline_ingest_profile(&database_url, "memory-card-tree-pipeline-v1");
    let service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );
    let mut request = memory_request("memory-rag-rule", "v1", "SPACE", "TRON_CORE");
    request.ingest_profile_id = "memory-card-tree-pipeline-v1".into();
    service.ingest(request).await.unwrap();

    let rows = repository
        .search_route(RagRouteSearchQuery {
            corpus_id: "memory-card".into(),
            route_id: "tree-vector".into(),
            retriever: "TREE_VECTOR_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "source title citation".into(),
            query_embedding: Some(embedding_4096(1.0, 0.0)),
            query_embedding_model_code: Some("fake-embedding".into()),
            source_version: "v1".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            }],
            filters: json!({}),
            policy: json!({ "includeRoot": true }),
            return_fields: vec![],
            limit: 5,
        })
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].target_code, "memory-rag-rule:v1:0");
    assert_eq!(rows[0].field_values["matchedBy"], json!("TREE_VECTOR_V1"));
    assert!(rows[0].field_values.get("treeScore").is_some());
}

#[tokio::test]
async fn tree_fulltext_route_policy_excludes_root_limits_expansion_and_prefers_deeper_nodes() {
    let database_url = temp_database_path("rag-search-tree-route-policy")
        .to_string_lossy()
        .to_string();
    let repository = Arc::new(Mutex::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
    ));
    set_memory_card_tree_table(&database_url, "rag_memory_tree_nodes");
    insert_markdown_tree_pipeline_ingest_profile(&database_url, "memory-card-heading-tree-v1");
    let service = RagIngestService::new(
        repository.clone(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );
    let mut request = memory_request("memory-rag-rule", "v1", "SPACE", "TRON_CORE");
    request.ingest_profile_id = "memory-card-heading-tree-v1".into();
    request.payload["text"] = json!(
        "# Overview\nRoot overview.\n## Deep Leaf\nAlways cite source titles in deep leaf.\n## Another Leaf\nAlways cite source titles in another leaf."
    );
    service.ingest(request).await.unwrap();
    rewrite_tree_summaries_for_tie(&database_url, "policy tie phrase");

    let query = |policy: Value| RagRouteSearchQuery {
        corpus_id: "memory-card".into(),
        route_id: "tree-fulltext".into(),
        retriever: "TREE_FULLTEXT_V1".into(),
        workspace_code: "TRON_CORE".into(),
        query: "policy tie phrase".into(),
        query_embedding: None,
        query_embedding_model_code: None,
        source_version: "v1".into(),
        scopes: vec![RagSearchScope {
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
        }],
        filters: json!({}),
        policy,
        return_fields: vec![],
        limit: 10,
    };

    let default_rows = repository.search_route(query(json!({}))).unwrap();
    assert!(
        default_rows
            .iter()
            .all(|row| row.field_values["treeNodeTitle"] != json!("RAG citation rule")),
        "includeRoot should default to false for tree routes"
    );

    let limited_rows = repository
        .search_route(query(json!({
            "includeRoot": true,
            "maxCoveredUnitsPerNode": 1
        })))
        .unwrap();
    let limited_titles = limited_rows
        .iter()
        .map(|row| row.field_values["treeNodeTitle"].clone())
        .collect::<Vec<_>>();
    assert_eq!(limited_rows.len(), 3);
    assert!(limited_titles.contains(&json!("RAG citation rule")));
    assert!(
        limited_titles.contains(&json!("Deep Leaf"))
            || limited_titles.contains(&json!("Another Leaf")),
        "maxCoveredUnitsPerNode should let child tree nodes hydrate units not consumed by root"
    );

    let deeper_rows = repository
        .search_route(query(json!({
            "includeRoot": true,
            "maxCoveredUnitsPerNode": 1,
            "preferDeeperTreeNode": true
        })))
        .unwrap();
    assert_eq!(
        deeper_rows[0].field_values["treeNodeTitle"],
        json!("Deep Leaf")
    );
}

#[tokio::test]
async fn retrieval_pipeline_generalizes_and_rewrites_query_before_route_execution() {
    let database_url = temp_database_path("rag-search-phase3-query-rewrite")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    insert_advanced_query_retrieval_profile(&database_url, "memory-advanced-query-v1");
    let repository = CapturingSearchRepository::connect(&database_url);
    let route_calls = repository.route_calls.clone();
    let service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new(),
    );

    let result = service
        .search(search_request(
            "memory-advanced-query-v1",
            "cite?",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
        ))
        .await
        .unwrap();

    assert_eq!(result.query, "cite?");
    assert_eq!(result.hits.len(), 1);
    let route_calls = route_calls.lock().unwrap();
    assert_eq!(route_calls.len(), 1);
    assert_eq!(
        route_calls[0].query,
        "cite source titles retrieval ordering"
    );
    drop(route_calls);
    assert_eq!(
        result
            .stage_results
            .iter()
            .map(|stage| stage.node_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "generalize-question",
            "rewrite-query",
            "fulltext-route",
            "fuse-results",
            "dedup-results",
            "build-context"
        ]
    );
    assert!(
        result.stage_results.iter().any(|stage| {
            stage.node_id == "generalize-question"
                && stage.input_artifact_keys == vec!["query.original"]
                && stage.output_artifact_keys == vec!["query.generalized", "query.effective"]
        }),
        "QUESTION_GENERALIZATION should report actual query artifact handoff"
    );
    assert!(
        result.stage_results.iter().any(|stage| {
            stage.node_id == "rewrite-query"
                && stage.input_artifact_keys == vec!["query.effective"]
                && stage.output_artifact_keys == vec!["query.rewritten", "query.effective"]
        }),
        "QUERY_REWRITE should report actual query artifact handoff"
    );
    assert!(
        result.stage_results.iter().any(|stage| {
            stage.node_id == "fulltext-route"
                && stage.input_artifact_keys == vec!["query.effective"]
                && stage.output_artifact_keys == vec!["hits.route"]
        }),
        "route search should consume the effective query artifact"
    );
}

#[tokio::test]
async fn search_rejects_physical_return_field_names() {
    let database_url = temp_database_path("rag-search-physical-return-field")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );

    let error = service
        .search(search_request_with_return_fields(
            "memory-related-v1",
            "cite source titles",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
            &["memory_type"],
        ))
        .await
        .unwrap_err();

    assert_eq!(error, RagError::SearchRequestInvalid);
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
        FakeSearchRuntime::new().with_embedding(embedding_4096(0.5, 0.25)),
    );

    let result = service
        .search(search_request_with_return_fields(
            "skill-related-v1",
            "rag reviewer",
            "RESULT_LIST",
            json!({ "skillName": "rag-reviewer" }),
            &["skillName", "skillDescription"],
        ))
        .await
        .unwrap();

    assert_eq!(result.context_strategy, "RESULT_LIST");
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].corpus_id, "skill");
    assert_eq!(result.hits[0].source_code, "skill:TRON_CORE:rag-reviewer");
    assert_eq!(result.hits[0].source_version, "v1");
    assert_eq!(result.hits[0].title.as_deref(), Some("rag-reviewer"));
    assert_eq!(result.hits[0].metadata, json!({}));
    assert_eq!(result.hits[0].fields["skillName"], json!("rag-reviewer"));
    assert_eq!(
        result.hits[0].fields["skillDescription"],
        json!("Reviews RAG retrieval changes before release.")
    );
    assert!(result.context_pack.is_none());
}

#[tokio::test]
async fn default_fts5_search_tokenizes_mixed_identifier_and_chinese_query() {
    let database_url = temp_database_path("rag-search-mixed-query")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_1536(1.0, 0.0)),
    );

    let result = service
        .search(search_request(
            "memory-related-v1",
            "retrieval是什么？",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert_eq!(result.hits[0].title.as_deref(), Some("RAG citation rule"));
}

#[tokio::test]
async fn project_doc_search_projects_requested_document_fields_and_context_pack() {
    let database_url = temp_database_path("rag-search-doc")
        .to_string_lossy()
        .to_string();
    seed_project_doc_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(0.0, 1.0)),
    );

    let result = service
        .search(search_request_with_return_fields(
            "project-doc-related-v1",
            "sync dag persist",
            "DOC_QA_CONTEXT_PACK",
            json!({ "documentType": ["DESIGN"], "repoCode": "REPO_TRON" }),
            &["documentCode", "path"],
        ))
        .await
        .unwrap();

    let hit = result
        .hits
        .iter()
        .find(|hit| hit.text.contains("synchronous DAG"))
        .expect("write path text unit should be returned");
    assert_eq!(hit.source_code, "doc-rag-mvp");
    assert_eq!(hit.title.as_deref(), Some("RAG MVP"));
    assert_eq!(hit.metadata, json!({}));
    assert_eq!(hit.fields["documentCode"], json!("DOC_RAG_MVP"));
    assert_eq!(hit.fields["path"], json!("docs/rag-mvp.md"));
    let context_pack = result.context_pack.unwrap();
    let item = context_pack
        .items
        .iter()
        .find(|item| item.text.contains("synchronous DAG"))
        .unwrap();
    assert_eq!(item.metadata["metadata"], json!({}));
    assert_eq!(
        item.metadata["returnFields"]["path"],
        json!("docs/rag-mvp.md")
    );
    assert!(item.metadata.get("targetCode").is_none());
    assert!(item.metadata.get("textUnit").is_none());
}

#[tokio::test]
async fn project_doc_vector_route_reads_distinct_vector_storage_table_when_configured() {
    let database_url = temp_database_path("rag-search-doc-separate-vector")
        .to_string_lossy()
        .to_string();
    seed_project_doc_fixture(&database_url).await;
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_project_doc_vector_table(&database_url, "rag_project_doc_vectors");
    backfill_project_doc_vector_storage(&database_url).await;

    let connection = open_prepared_connection(&database_url);
    connection
        .execute(
            "DROP TABLE IF EXISTS rag_project_doc_text_units_vec_d4096",
            [],
        )
        .unwrap();
    drop(connection);

    let hits = repository
        .search_route(RagRouteSearchQuery {
            corpus_id: "project-doc".into(),
            route_id: "project-doc-vector".into(),
            retriever: "VECTOR_SQLITE_VEC0_COSINE_V1".into(),
            workspace_code: "TRON_CORE".into(),
            query: "sync dag persist".into(),
            query_embedding: Some(project_doc_embedding()),
            query_embedding_model_code: Some("workspace-default-embedding".into()),
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

    assert!(!table_exists(
        &repository,
        "rag_project_doc_text_units_vec_d4096"
    ));
    assert!(table_exists(
        &repository,
        "rag_project_doc_vectors_vec_d4096"
    ));
    assert!(hits.iter().any(|hit| hit.text.contains("synchronous DAG")));
}

#[tokio::test]
async fn project_doc_fulltext_route_reads_distinct_carrier_and_hydrates_text_units() {
    let database_url = temp_database_path("rag-search-doc-separate-fulltext")
        .to_string_lossy()
        .to_string();
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_project_doc_fulltext_table(&database_url, "rag_project_doc_fulltext");
    let service = RagIngestService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new()
            .with_embedding(project_doc_embedding())
            .with_embedding(project_doc_embedding())
            .with_document(project_doc_markdown()),
    );
    service.ingest(project_doc_request()).await.unwrap();
    let connection = open_prepared_connection(&database_url);
    connection
        .execute(
            "UPDATE rag_project_doc_fulltext
             SET search_text = 'carrier recall phrase'
             WHERE ordinal = 1",
            [],
        )
        .unwrap();
    connection
        .execute("DELETE FROM rag_project_doc_fulltext_fts", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_project_doc_fulltext_fts(rowid, search_text)
             SELECT id, search_text FROM rag_project_doc_fulltext WHERE status = 'ACTIVE'",
            [],
        )
        .unwrap();
    drop(connection);
    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    set_project_doc_fulltext_table(&database_url, "rag_project_doc_fulltext");

    let hits = repository
        .search_route(RagRouteSearchQuery {
            corpus_id: "project-doc".into(),
            route_id: "project-doc-fulltext".into(),
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

    assert!(table_exists(&repository, "rag_project_doc_fulltext_fts"));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].target_code, "doc-rag-mvp:v1:1");
    assert_eq!(hits[0].search_text, "carrier recall phrase");
    assert!(hits[0].text.contains("synchronous DAG"));
}

#[tokio::test]
async fn fusion_dedup_and_fake_rerank_produce_stable_ordering() {
    let database_url = temp_database_path("rag-search-rerank")
        .to_string_lossy()
        .to_string();
    seed_two_memory_fixture(&database_url).await;
    let runtime = FakeSearchRuntime::new()
        .with_embedding(embedding_4096(1.0, 0.0))
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
    assert!(result.context_pack.is_none());
    assert_eq!(runtime.last_rerank_model_code(), None);
}

#[tokio::test]
async fn rrf_embedding_mmr_promotes_diverse_cross_corpus_candidate() {
    let database_url = temp_database_path("rag-search-rrf-mmr")
        .to_string_lossy()
        .to_string();
    seed_two_memory_fixture(&database_url).await;
    seed_project_doc_fixture(&database_url).await;
    insert_rrf_mmr_profile(&database_url, "multi-corpus-rrf-mmr-v1", true);
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );

    let result = service
        .search(search_request(
            "multi-corpus-rrf-mmr-v1",
            "retrieval ordering",
            "RESULT_LIST",
            json!({}),
        ))
        .await
        .unwrap();

    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert_eq!(result.hits[1].source_code, "doc-rag-mvp");
    assert_eq!(result.hits[2].source_code, "memory-rerank-second");
}

#[tokio::test]
async fn seeded_memory_project_doc_fusion_profile_searches_both_corpora() {
    let database_url = temp_database_path("rag-search-seeded-fusion")
        .to_string_lossy()
        .to_string();
    seed_two_memory_fixture(&database_url).await;
    seed_project_doc_fixture(&database_url).await;
    let runtime = FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0));
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let result = service
        .search(search_request(
            "memory-project-doc-fusion-v1",
            "retrieval ordering",
            "RESULT_LIST",
            json!({}),
        ))
        .await
        .unwrap();

    let corpus_ids = result
        .hits
        .iter()
        .map(|hit| hit.corpus_id.as_str())
        .collect::<Vec<_>>();
    assert!(corpus_ids.contains(&"memory-card"));
    assert!(corpus_ids.contains(&"project-doc"));
    assert!(
        result
            .hits
            .iter()
            .any(|hit| hit.source_code == "memory-rag-rule")
    );
    assert!(
        result
            .hits
            .iter()
            .any(|hit| hit.source_code == "doc-rag-mvp")
    );
    assert_eq!(runtime.last_rerank_model_code(), None);
}

#[tokio::test]
async fn request_filters_match_each_route_filter_schema() {
    let database_url = temp_database_path("rag-search-route-filter-schema")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    seed_project_doc_fixture(&database_url).await;
    seed_skill_fixture(&database_url).await;
    insert_mixed_filter_profile(&database_url, "mixed-filter-routing-v1");
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );

    let result = service
        .search(search_request(
            "mixed-filter-routing-v1",
            "rag retrieval",
            "RESULT_LIST",
            json!({
                "memoryType": "RULE",
                "documentType": "DESIGN",
                "skillName": "rag-reviewer"
            }),
        ))
        .await
        .unwrap();
    let source_codes = result
        .hits
        .iter()
        .map(|hit| hit.source_code.as_str())
        .collect::<Vec<_>>();
    assert!(source_codes.contains(&"memory-rag-rule"));
    assert!(source_codes.contains(&"doc-rag-mvp"));
    assert!(source_codes.contains(&"skill:TRON_CORE:rag-reviewer"));

    let invalid_known_field = service
        .search(search_request(
            "mixed-filter-routing-v1",
            "rag retrieval",
            "RESULT_LIST",
            json!({
                "memoryType": 7,
                "documentType": "DESIGN",
                "skillName": "rag-reviewer"
            }),
        ))
        .await
        .unwrap_err();
    assert_eq!(invalid_known_field, RagError::SearchFilterInvalid);
}

#[tokio::test]
async fn embedding_mmr_missing_embeddings_falls_back_to_fused_order() {
    let database_url = temp_database_path("rag-search-mmr-missing-embedding")
        .to_string_lossy()
        .to_string();
    seed_two_memory_fixture(&database_url).await;
    insert_rrf_mmr_profile(&database_url, "memory-fulltext-mmr-v1", false);
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );

    let result = service
        .search(search_request(
            "memory-fulltext-mmr-v1",
            "retrieval ordering",
            "RESULT_LIST",
            json!({}),
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 2);
    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert_eq!(result.hits[1].source_code, "memory-rerank-second");
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
fn retrieval_profile_compiler_accepts_explicit_structured_filter_sqlite_route() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();
    let corpus = repository.get_corpus("memory-card").unwrap().unwrap();
    let registry = DefaultRagStrategyRegistry::default();

    assert_eq!(
        registry
            .resolve_retriever("STRUCTURED_FILTER_SQLITE_V1")
            .status,
        StrategyRegistrationStatus::Enabled
    );

    let compiled = compile_retrieval_profile(
        &retrieval_profile(
            "structured-filter",
            Some("v1"),
            json!({
                "routes": [{
                    "retriever": "STRUCTURED_FILTER_SQLITE_V1",
                    "topK": 3,
                    "filters": {"injectable": true}
                }],
                "contextPolicy": {"supportedStrategies": ["RESULT_LIST"]}
            }),
        ),
        &corpus,
        &registry,
    )
    .unwrap();

    assert_eq!(compiled.routes.len(), 1);
    assert_eq!(compiled.routes[0].route_id, "memory-card-structured-filter");
    assert_eq!(compiled.routes[0].retriever, "STRUCTURED_FILTER_SQLITE_V1");
}

#[test]
fn retrieval_profile_compiler_accepts_rrf_and_embedding_mmr_policy() {
    let repository = SqliteRagRepository::connect(":memory:").unwrap();
    let corpus = repository.get_corpus("memory-card").unwrap().unwrap();
    let registry = DefaultRagStrategyRegistry::default();

    let compiled = compile_retrieval_profile(
        &retrieval_profile(
            "rrf-mmr",
            Some("v1"),
            json!({
                "routes": [{"routeId": "memory-vector", "retriever": "VECTOR"}],
                "fusionPolicy": {"type": "RRF", "rankConstant": 60},
                "dedupPolicy": {"type": "CANONICAL_TEXT_UNIT"},
                "topicPolicy": {
                    "enabled": true,
                    "type": "EMBEDDING_MMR_V1",
                    "candidateWindow": 3,
                    "lambda": 0.5,
                    "fallback": "KEEP_ORDER"
                },
                "contextPolicy": {"supportedStrategies": ["RESULT_LIST"]}
            }),
        ),
        &corpus,
        &registry,
    )
    .unwrap();

    assert_eq!(compiled.fusion_policy.type_name(), "RRF");
    assert_eq!(compiled.dedup_policy.type_name(), "CANONICAL_TEXT_UNIT");
    match compiled.topic_policy {
        TopicPolicy::EmbeddingMmr(policy) => {
            assert_eq!(policy.candidate_window, 3);
            assert_eq!(policy.fallback, TopicFallback::KeepOrder);
        }
        TopicPolicy::Disabled => panic!("topic policy should be enabled"),
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
async fn blank_query_executes_only_structured_filter_routes_without_embedding_or_rerank() {
    let database_url = temp_database_path("rag-structured-filter-blank")
        .to_string_lossy()
        .to_string();
    seed_structured_filter_fixture(&database_url).await;
    insert_structured_filter_profile(
        &database_url,
        "memory-structured-filter-mixed-v1",
        true,
        true,
        5,
    );
    let repository = CapturingSearchRepository::connect(&database_url);
    let route_calls = repository.route_calls.clone();
    let runtime = FakeSearchRuntime::new();
    let service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );
    let mut request = search_request_with_return_fields(
        "memory-structured-filter-mixed-v1",
        "   ",
        "RESULT_LIST",
        json!({ "importanceScore": { "op": "gte", "value": 80 } }),
        &["memoryType"],
    );
    request.limit = 5;

    let result = service.search(request).await.unwrap();

    assert_eq!(
        result
            .hits
            .iter()
            .map(|hit| hit.source_code.as_str())
            .collect::<Vec<_>>(),
        vec!["memory-rag-rule", "memory-rerank-second"]
    );
    assert!(result.warnings.is_empty());
    assert_eq!(runtime.embedding_call_count(), 0);
    assert_eq!(runtime.rerank_call_count(), 0);
    let route_calls = route_calls.lock().unwrap();
    assert_eq!(route_calls.len(), 1);
    assert_eq!(route_calls[0].retriever, "STRUCTURED_FILTER_SQLITE_V1");
    assert!(route_calls[0].query.trim().is_empty());
    assert_eq!(
        result
            .stage_results
            .iter()
            .filter(|stage| stage.stage_type == "ROUTE_SEARCH")
            .map(|stage| stage.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["memory-structured"]
    );
}

#[tokio::test]
async fn blank_query_rejects_profiles_without_structured_filter_route() {
    let database_url = temp_database_path("rag-structured-filter-blank-reject")
        .to_string_lossy()
        .to_string();
    seed_project_doc_fixture(&database_url).await;
    let repository = CapturingSearchRepository::connect(&database_url);
    let route_calls = repository.route_calls.clone();
    let runtime = FakeSearchRuntime::new();
    let service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let error = service
        .search(search_request(
            "project-doc-related-v1",
            "",
            "RESULT_LIST",
            json!({ "documentType": "DESIGN" }),
        ))
        .await
        .unwrap_err();

    assert_eq!(error, RagError::SearchRequestInvalid);
    assert_eq!(runtime.embedding_call_count(), 0);
    assert!(route_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn seeded_memory_related_profile_uses_structured_fulltext_and_vector_routes() {
    let database_url = temp_database_path("rag-seeded-memory-related-three-routes")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    let repository = CapturingSearchRepository::connect(&database_url);
    let route_calls = repository.route_calls.clone();
    let runtime = FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0));
    let service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let result = service
        .search(search_request(
            "memory-related-v1",
            "cite source titles",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert_eq!(runtime.embedding_call_count(), 1);
    let route_calls = route_calls.lock().unwrap();
    assert_eq!(
        route_calls
            .iter()
            .map(|call| call.retriever.as_str())
            .collect::<Vec<_>>(),
        vec![
            "STRUCTURED_FILTER_SQLITE_V1",
            "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
            "VECTOR_SQLITE_VEC0_COSINE_V1"
        ]
    );
}

#[tokio::test]
async fn seeded_memory_related_profile_allows_blank_query_structured_filter_only() {
    let database_url = temp_database_path("rag-seeded-memory-related-blank-structured")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    let repository = CapturingSearchRepository::connect(&database_url);
    let route_calls = repository.route_calls.clone();
    let runtime = FakeSearchRuntime::new();
    let service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let result = service
        .search(search_request_with_return_fields(
            "memory-related-v1",
            "",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
            &["memoryType"],
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert_eq!(result.hits[0].fields["memoryType"], json!("RULE"));
    assert_eq!(runtime.embedding_call_count(), 0);
    assert_eq!(runtime.rerank_call_count(), 0);
    let route_calls = route_calls.lock().unwrap();
    assert_eq!(route_calls.len(), 1);
    assert_eq!(route_calls[0].retriever, "STRUCTURED_FILTER_SQLITE_V1");
    assert!(route_calls[0].query.is_empty());
}

#[tokio::test]
async fn seeded_memory_query_profile_uses_only_query_routes_and_access_context_scopes() {
    let database_url = temp_database_path("rag-seeded-memory-query-two-routes")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    let repository = CapturingSearchRepository::connect(&database_url);
    let route_calls = repository.route_calls.clone();
    let runtime = FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0));
    let service = RagSearchService::new(
        repository,
        DefaultRagStrategyRegistry::default(),
        runtime.clone(),
    );

    let result = service
        .search(search_request(
            "memory-query-v1",
            "cite source titles",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert_eq!(runtime.embedding_call_count(), 1);
    let route_calls = route_calls.lock().unwrap();
    assert_eq!(
        route_calls
            .iter()
            .map(|call| call.retriever.as_str())
            .collect::<Vec<_>>(),
        vec![
            "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
            "VECTOR_SQLITE_VEC0_COSINE_V1"
        ]
    );
    for call in route_calls.iter() {
        assert_eq!(call.query, "cite source titles");
        assert_eq!(
            call.scopes
                .iter()
                .map(|scope| (scope.scope_type.as_str(), scope.scope_code.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("USER", "USR-LARGE-ID"),
                ("AGENT", "AGENT_CODEX"),
                ("SPACE", "TRON_CORE")
            ]
        );
    }
}

#[tokio::test]
async fn sqlite_structured_filter_returns_active_scoped_versioned_limited_rows() {
    let database_url = temp_database_path("rag-structured-filter-sqlite")
        .to_string_lossy()
        .to_string();
    seed_structured_filter_fixture(&database_url).await;
    insert_structured_filter_profile(
        &database_url,
        "memory-structured-filter-v1",
        false,
        false,
        10,
    );
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new(),
    );

    let mut request = search_request_with_return_fields(
        "memory-structured-filter-v1",
        "",
        "RESULT_LIST",
        json!({ "importanceScore": { "op": "gte", "value": 80 } }),
        &["memoryType", "importanceScore"],
    );
    request.limit = 10;
    let result = service.search(request).await.unwrap();

    assert_eq!(
        result
            .hits
            .iter()
            .map(|hit| hit.source_code.as_str())
            .collect::<Vec<_>>(),
        vec!["memory-rag-rule", "memory-rerank-second"]
    );
    assert!(result.hits.iter().all(|hit| hit.source_version == "v1"));
    assert!(result.hits.iter().all(|hit| hit.score > 0.0));
    assert_eq!(result.hits[0].fields["memoryType"], json!("RULE"));
    assert_eq!(result.hits[0].fields["importanceScore"], json!(90));

    let limited_service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new(),
    );
    let mut limited_request = search_request_with_return_fields(
        "memory-structured-filter-v1",
        "",
        "RESULT_LIST",
        json!({ "importanceScore": { "op": "gte", "value": 80 } }),
        &["memoryType"],
    );
    limited_request.limit = 1;
    let limited = limited_service.search(limited_request).await.unwrap();
    assert_eq!(limited.hits.len(), 1);
    assert_eq!(limited.hits[0].source_code, "memory-rag-rule");
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
            FakeSearchRuntime::new().with_embedding(embedding_4096(0.0, 1.0)),
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

#[tokio::test]
async fn vector_dimension_mismatch_degrades_to_fulltext_warning_without_route_error() {
    let database_url = temp_database_path("rag-search-vector-dimension-warning")
        .to_string_lossy()
        .to_string();
    seed_project_doc_fixture(&database_url).await;
    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_1536(0.0, 1.0)),
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
    assert_eq!(result.hits[0].source_code, "doc-rag-mvp");
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning["code"] == json!("RAG_VECTOR_ROUTE_UNAVAILABLE"))
    );
}

#[tokio::test]
async fn default_and_custom_extension_retrievers_return_storage_scored_hits() {
    let database_url = temp_database_path("rag-search-extension")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;
    insert_extension_memory_profile(&database_url, "memory-extension-v1");

    let default_service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );
    let default_result = default_service
        .search(search_request(
            "memory-related-v1",
            "cite source titles",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
        ))
        .await
        .unwrap();
    assert_eq!(default_result.hits[0].source_code, "memory-rag-rule");
    assert!(default_result.hits[0].score > 0.0);

    let extension_service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );
    let extension_result = extension_service
        .search(search_request(
            "memory-extension-v1",
            "cite source titles",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
        ))
        .await
        .unwrap();

    assert_eq!(extension_result.hits.len(), 1);
    assert_eq!(extension_result.hits[0].source_code, "memory-rag-rule");
    assert!(extension_result.hits[0].score > 0.0);
}

#[tokio::test]
async fn sqlite_extension_indexes_survive_reconnect_without_main_index_payload() {
    let database_url = temp_database_path("rag-search-extension-backfill")
        .to_string_lossy()
        .to_string();
    seed_memory_fixture(&database_url).await;

    let repository = SqliteRagRepository::connect(&database_url).unwrap();
    assert!(
        !repository
            .table_columns("rag_memory_text_units")
            .unwrap()
            .contains("embedding")
    );
    drop(repository);

    let connection = open_prepared_connection(&database_url);
    let sidecar_count = connection
        .query_row(
            "SELECT COUNT(*)
             FROM sqlite_schema
             WHERE name = 'rag_memory_text_units_vec_d4096'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap();
    assert_eq!(sidecar_count, 1);
    drop(connection);
    insert_extension_memory_profile(&database_url, "memory-extension-backfill-v1");

    let service = RagSearchService::new(
        SqliteRagRepository::connect(&database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
    );
    let result = service
        .search(search_request(
            "memory-extension-backfill-v1",
            "cite source titles",
            "RESULT_LIST",
            json!({ "memoryType": "RULE" }),
        ))
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].source_code, "memory-rag-rule");
    assert!(result.hits[0].score > 0.0);
}

#[test]
fn repository_search_boundary_keeps_graph_summary_and_memory_flow_behavior_out() {
    let rag_http_routes = include_str!("../src/adapters/inbound/http/rag/routes.rs");
    let rag_application = [
        include_str!("../src/application/rag/search.rs"),
        include_str!("../src/application/rag/search/split/contracts.rs"),
        include_str!("../src/application/rag/search/split/service_runtime.rs"),
    ]
    .join("\n");
    let repository_search = [
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/search.rs"),
        include_str!(
            "../src/adapters/outbound/persistence/sqlite/rag_repository/search/split/query_repository.rs"
        ),
        include_str!(
            "../src/adapters/outbound/persistence/sqlite/rag_repository/search/split/row_projection.rs"
        ),
    ]
    .join("\n");

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
    let repository_ingest = [
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/ingest.rs"),
        include_str!(
            "../src/adapters/outbound/persistence/sqlite/rag_repository/ingest/write_repository.rs"
        ),
        include_str!(
            "../src/adapters/outbound/persistence/sqlite/rag_repository/ingest/write_helpers.rs"
        ),
    ]
    .join("\n");
    let repository_search = [
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/search.rs"),
        include_str!(
            "../src/adapters/outbound/persistence/sqlite/rag_repository/search/split/query_repository.rs"
        ),
        include_str!(
            "../src/adapters/outbound/persistence/sqlite/rag_repository/search/split/row_projection.rs"
        ),
    ]
    .join("\n");
    let repository_retry =
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/retry.rs");
    let repository_diagnostics =
        include_str!("../src/adapters/outbound/persistence/sqlite/rag_repository/diagnostics.rs");

    for source in [
        repository_ingest.as_str(),
        repository_search.as_str(),
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

#[test]
fn search_runtime_uses_pipeline_stage_results_instead_of_posthoc_synthesis() {
    let search_source = [
        include_str!("../src/application/rag/search.rs"),
        include_str!("../src/application/rag/search/split/contracts.rs"),
        include_str!("../src/application/rag/search/split/service_runtime.rs"),
    ]
    .join("\n");

    assert!(
        !search_source.contains("fn search_stage_results("),
        "search stageResults should be emitted by the generic DAG executor for actually executed nodes"
    );
    assert!(
        search_source.contains("execute_retrieval_pipeline"),
        "search runtime should execute the compiled retrieval pipeline"
    );
}

struct CapturingSearchRepository {
    inner: SqliteRagRepository,
    route_calls: Arc<Mutex<Vec<RagRouteSearchQuery>>>,
}

impl CapturingSearchRepository {
    fn connect(database_url: &str) -> Self {
        Self {
            inner: SqliteRagRepository::connect(database_url).unwrap(),
            route_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl RagConfigRepository for CapturingSearchRepository {
    fn list_corpora(&self) -> Result<Vec<RagCorpusRecord>, RagError> {
        self.inner.list_corpora()
    }

    fn get_corpus(&self, corpus_id: &str) -> Result<Option<RagCorpusRecord>, RagError> {
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

impl RagSearchRepository for CapturingSearchRepository {
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError> {
        self.route_calls.lock().unwrap().push(query.clone());
        self.inner.search_route(query)
    }

    fn persist_search_trace(
        &self,
        trace: tron_backend::application::rag::RagSearchTraceWrite,
    ) -> Result<(), RagError> {
        self.inner.persist_search_trace(trace)
    }

    fn expand_related_graph(
        &self,
        query: tron_backend::application::rag::RagGraphExpansionQuery,
    ) -> Result<Option<Value>, RagError> {
        self.inner.expand_related_graph(query)
    }
}

#[derive(Clone, Default)]
struct FakeSearchRuntime {
    embeddings: Arc<Mutex<VecDeque<Result<Vec<f32>, String>>>>,
    reranks: Arc<Mutex<VecDeque<Result<Vec<RagRerankItem>, String>>>>,
    documents: Arc<Mutex<VecDeque<Result<RagResolvedDocument, RagError>>>>,
    last_rerank_model_code: Arc<Mutex<Option<String>>>,
    embedding_call_count: Arc<Mutex<usize>>,
    rerank_call_count: Arc<Mutex<usize>>,
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

    fn embedding_call_count(&self) -> usize {
        *self.embedding_call_count.lock().unwrap()
    }

    fn rerank_call_count(&self) -> usize {
        *self.rerank_call_count.lock().unwrap()
    }
}

impl RagSearchRuntime for FakeSearchRuntime {
    fn embed<'a>(
        &'a self,
        _call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            *self.embedding_call_count.lock().unwrap() += 1;
            self.embeddings
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(embedding_4096(1.0, 0.0)))
                .map(|embedding| RagEmbeddingResult {
                    model_code: "workspace-default-embedding".into(),
                    embedding,
                })
        })
    }

    fn rerank<'a>(
        &'a self,
        call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RagRerankItem>, String>> + Send + 'a>> {
        Box::pin(async move {
            *self.rerank_call_count.lock().unwrap() += 1;
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
        Box::pin(async move { <Self as RagSearchRuntime>::embed(self, call).await })
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
            return_fields: Vec::new(),
            graph_expansion: Default::default(),
        },
        trace: RagSearchTraceRequest { enabled: false },
    }
}

fn search_request_with_return_fields(
    profile_id: &str,
    query: &str,
    strategy: &str,
    filters: Value,
    return_fields: &[&str],
) -> RagSearchRequest {
    let mut request = search_request(profile_id, query, strategy, filters);
    request.context.return_fields = return_fields.iter().map(|field| (*field).into()).collect();
    request
}

async fn seed_memory_fixture(database_url: &str) {
    let service = RagIngestService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(1.0, 0.0)),
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
        FakeSearchRuntime::new().with_embedding(embedding_4096(0.9, 0.1)),
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
            .with_embedding(project_doc_embedding())
            .with_embedding(project_doc_embedding())
            .with_document(project_doc_markdown()),
    );
    service.ingest(project_doc_request()).await.unwrap();
}

async fn seed_skill_fixture(database_url: &str) {
    let service = RagIngestService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new().with_embedding(embedding_4096(0.5, 0.25)),
    );
    service.ingest(skill_request()).await.unwrap();
}

async fn seed_structured_filter_fixture(database_url: &str) {
    seed_two_memory_fixture(database_url).await;
    let service = RagIngestService::new(
        SqliteRagRepository::connect(database_url).unwrap(),
        DefaultRagStrategyRegistry::default(),
        FakeSearchRuntime::new(),
    );
    service
        .ingest(memory_request(
            "memory-inactive-filter-match",
            "v1",
            "SPACE",
            "TRON_CORE",
        ))
        .await
        .unwrap();
    service
        .ingest(memory_request(
            "memory-wrong-scope-filter-match",
            "v1",
            "SPACE",
            "TRON_CORE",
        ))
        .await
        .unwrap();
    let connection = open_prepared_connection(database_url);
    connection
        .execute(
            "UPDATE rag_memory_text_units
             SET status = 'DISABLED'
             WHERE source_code = ?1",
            params!["memory-inactive-filter-match"],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE rag_memory_text_units
             SET scope_type = 'USER',
                 scope_code = 'OTHER-USER'
             WHERE source_code = ?1",
            params!["memory-wrong-scope-filter-match"],
        )
        .unwrap();
}

fn memory_request(
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> RagIngestRequest {
    let text = if source_code == "memory-rerank-second" {
        "Retrieval ordering should keep related but distinct RAG ranking notes available."
    } else {
        "Always cite source titles in RAG answers for retrieval ordering."
    };
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
        source: RagIngestSource {
            code: source_code.into(),
            version: source_version.into(),
            source_type: "MEMORY_CARD".into(),
        },
        payload: json!({
            "source": {
                "code": source_code,
                "version": source_version,
                "type": "MEMORY_CARD"
            },
            "title": "RAG citation rule",
            "text": text,
            "metadata": {
                "seed": "search-contract"
            },
            "domainFields": {
                "version": source_version,
                "memoryType": "RULE",
                "memoryStatus": "ACTIVE",
                "reviewStatus": "APPROVED",
                "importanceScore": 90,
                "injectable": true
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
        source: RagIngestSource {
            code: "doc-rag-mvp".into(),
            version: "v1".into(),
            source_type: "PROJECT_DOC_REF".into(),
        },
        payload: json!({
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
        source: RagIngestSource {
            code: "skill:TRON_CORE:rag-reviewer".into(),
            version: "v1".into(),
            source_type: "SKILL".into(),
        },
        payload: json!({
            "source": {
                "code": "skill:TRON_CORE:rag-reviewer",
                "version": "v1",
                "type": "SKILL"
            },
            "title": "rag-reviewer",
            "text": "Skill: rag-reviewer\nDescription: Reviews RAG retrieval changes before release.",
            "domainFields": {
                "skillName": "rag-reviewer",
                "skillDescription": "Reviews RAG retrieval changes before release."
            }
        }),
    }
}

fn project_doc_markdown() -> &'static str {
    "# RAG MVP\nIntro.\n\n## Write Path\nThe write path uses a synchronous DAG to persist source and text units."
}

fn insert_structured_filter_profile(
    database_url: &str,
    profile_id: &str,
    include_query_routes: bool,
    rerank_enabled: bool,
    route_top_k: usize,
) {
    let connection = open_prepared_connection(database_url);
    let mut routes = vec![json!({
        "routeId": "memory-structured",
        "retriever": "STRUCTURED_FILTER_SQLITE_V1",
        "topK": route_top_k,
        "filters": {"injectable": true}
    })];
    if include_query_routes {
        routes.extend([
            json!({
                "routeId": "memory-fulltext",
                "retriever": "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"
            }),
            json!({
                "routeId": "memory-vector",
                "retriever": "VECTOR_SQLITE_VEC0_COSINE_V1"
            }),
        ]);
    }
    let config = json!({
        "routes": routes,
        "fusionPolicy": {"type": "SUM_SCORE"},
        "dedupPolicy": {"type": "TEXT_UNIT"},
        "rerankPolicy": {"enabled": rerank_enabled},
        "hydratePolicy": {"type": "TEXT_UNIT"},
        "contextPolicy": {"supportedStrategies": ["RESULT_LIST", "MEMORY_CONTEXT_PACK"]}
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'RETRIEVAL', 'enabled', 'memory-card', '[\"memory-card\"]',
                'v1', ?2, '{\"memoryType\":\"RULE\"}', 'SKIP_VECTOR_ON_UNAVAILABLE', 1, 1)
            ON CONFLICT (profile_id) DO UPDATE SET
                config_json = excluded.config_json,
                filters_json = excluded.filters_json,
                updated_at = excluded.updated_at",
            params![profile_id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();
}

fn insert_pipeline_only_retrieval_profile(database_url: &str, profile_id: &str) {
    let connection = open_prepared_connection(database_url);
    let config = json!({
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {
                    "nodeId": "structured-route",
                    "stageType": "ROUTE_SEARCH",
                    "strategyId": "STRUCTURED_FILTER_SQLITE_V1",
                    "config": {
                        "corpusId": "memory-card",
                        "topK": 5,
                        "weight": 1.0,
                        "filters": {"injectable": true}
                    }
                },
                {
                    "nodeId": "fulltext-route",
                    "stageType": "ROUTE_SEARCH",
                    "strategyId": "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
                    "config": {
                        "corpusId": "memory-card",
                        "topK": 5,
                        "weight": 1.0
                    }
                },
                {
                    "nodeId": "vector-route",
                    "stageType": "ROUTE_SEARCH",
                    "strategyId": "VECTOR_SQLITE_VEC0_COSINE_V1",
                    "config": {
                        "corpusId": "memory-card",
                        "topK": 5,
                        "weight": 1.0
                    }
                },
                {
                    "nodeId": "fuse-results",
                    "stageType": "FUSE_RESULTS",
                    "strategyId": "SUM_SCORE"
                },
                {
                    "nodeId": "dedup-results",
                    "stageType": "DEDUP_RESULTS",
                    "strategyId": "TEXT_UNIT"
                },
                {
                    "nodeId": "build-context",
                    "stageType": "BUILD_CONTEXT",
                    "strategyId": "RESULT_LIST"
                }
            ],
            "edges": [
                { "from": "structured-route", "to": "fuse-results" },
                { "from": "fulltext-route", "to": "fuse-results" },
                { "from": "vector-route", "to": "fuse-results" },
                { "from": "fuse-results", "to": "dedup-results" },
                { "from": "dedup-results", "to": "build-context" }
            ]
        },
        "contextPolicy": {
            "supportedStrategies": ["RESULT_LIST"]
        },
        "rerankPolicy": {"enabled": false}
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'RETRIEVAL', 'enabled', 'memory-card', '[\"memory-card\"]',
                'v1', ?2, '{}', 'SKIP_VECTOR_ON_UNAVAILABLE', 1, 1)",
            params![profile_id, config.to_string()],
        )
        .unwrap();
}

fn insert_tree_pipeline_ingest_profile(database_url: &str, profile_id: &str) {
    let connection = open_prepared_connection(database_url);
    let config = json!({
        "sourceInputPolicy": {
            "supportedSourceTypes": ["MEMORY_CARD"]
        },
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {"nodeId": "validate-source", "stageType": "VALIDATE_SOURCE", "strategyId": "RAG_VALIDATE_SOURCE"},
                {"nodeId": "prepare-payload", "stageType": "PREPARE_PAYLOAD", "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1"},
                {"nodeId": "split-text", "stageType": "SPLIT_TEXT", "strategyId": "RAG_SPLIT_TEXT"},
                {"nodeId": "extract-tree", "stageType": "EXTRACT_TREE", "strategyId": "RAG_MARKDOWN_TREE_EXTRACT_V1"},
                {"nodeId": "summarize-tree", "stageType": "SUMMARIZE_TREE", "strategyId": "RAG_TREE_EXTRACTIVE_SUMMARY_V1"},
                {"nodeId": "tree-fulltext-index", "stageType": "TREE_FULLTEXT_INDEX", "strategyId": "RAG_TREE_FULLTEXT_INDEX"},
                {"nodeId": "fulltext-index", "stageType": "FULLTEXT_INDEX", "strategyId": "RAG_FULLTEXT_INDEX"},
                {"nodeId": "vector-index", "stageType": "VECTOR_INDEX", "strategyId": "RAG_VECTOR_INDEX"},
                {"nodeId": "tree-vector-index", "stageType": "TREE_VECTOR_INDEX", "strategyId": "RAG_TREE_VECTOR_INDEX"},
                {"nodeId": "persist-tree", "stageType": "PERSIST_TREE", "strategyId": "RAG_PERSIST_TREE_GRAPH_V1"},
                {"nodeId": "persist-source", "stageType": "PERSIST_SOURCE", "strategyId": "RAG_PERSIST_SOURCE"}
            ],
            "edges": [
                { "from": "validate-source", "to": "prepare-payload" },
                { "from": "prepare-payload", "to": "split-text" },
                { "from": "split-text", "to": "extract-tree" },
                { "from": "extract-tree", "to": "summarize-tree" },
                { "from": "summarize-tree", "to": "tree-fulltext-index" },
                { "from": "tree-fulltext-index", "to": "tree-vector-index" },
                { "from": "tree-vector-index", "to": "persist-tree" },
                { "from": "split-text", "to": "fulltext-index" },
                { "from": "split-text", "to": "vector-index" },
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

fn insert_markdown_tree_pipeline_ingest_profile(database_url: &str, profile_id: &str) {
    insert_tree_pipeline_ingest_profile(database_url, profile_id);
    let connection = open_prepared_connection(database_url);
    let raw_config: String = connection
        .query_row(
            "SELECT config_json FROM rag_profiles WHERE profile_id = ?1",
            params![profile_id],
            |row| row.get(0),
        )
        .unwrap();
    let mut config: Value = serde_json::from_str(&raw_config).unwrap();
    for node in config["pipeline"]["nodes"]
        .as_array_mut()
        .expect("pipeline nodes should be an array")
    {
        if node["nodeId"] == json!("split-text") {
            node["strategyId"] = json!("RAG_SPLIT_MARKDOWN_HEADING_V1");
            node["config"] = json!({
                "splitter": {
                    "headingDepth": 2,
                    "includeHeadingInIndexText": true
                }
            });
        }
    }
    connection
        .execute(
            "UPDATE rag_profiles SET config_json = ?1 WHERE profile_id = ?2",
            params![config.to_string(), profile_id],
        )
        .unwrap();
}

fn rewrite_tree_summaries_for_tie(database_url: &str, summary_text: &str) {
    let connection = open_prepared_connection(database_url);
    connection
        .execute(
            "UPDATE rag_memory_tree_nodes
             SET summary_text = ?1,
                 summary_fulltext_status = 'INDEXED'",
            params![summary_text],
        )
        .unwrap();
    connection
        .execute("DELETE FROM rag_memory_tree_nodes_fts", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO rag_memory_tree_nodes_fts(rowid, search_text)
             SELECT id, summary_text
             FROM rag_memory_tree_nodes
             WHERE status = 'ACTIVE'",
            [],
        )
        .unwrap();
}

fn insert_advanced_query_retrieval_profile(database_url: &str, profile_id: &str) {
    let connection = open_prepared_connection(database_url);
    let config = json!({
        "pipeline": {
            "mode": "DAG",
            "nodes": [
                {
                    "nodeId": "generalize-question",
                    "stageType": "QUESTION_GENERALIZATION",
                    "strategyId": "RAG_QUESTION_GENERALIZATION_SIMPLE_V1",
                    "config": {
                        "appendTerms": ["source titles"]
                    }
                },
                {
                    "nodeId": "rewrite-query",
                    "stageType": "QUERY_REWRITE",
                    "strategyId": "RAG_QUERY_REWRITE_TEMPLATE_V1",
                    "config": {
                        "template": "{query} retrieval ordering"
                    }
                },
                {
                    "nodeId": "fulltext-route",
                    "stageType": "ROUTE_SEARCH",
                    "strategyId": "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
                    "config": {
                        "corpusId": "memory-card",
                        "topK": 5,
                        "weight": 1.0
                    }
                },
                {
                    "nodeId": "fuse-results",
                    "stageType": "FUSE_RESULTS",
                    "strategyId": "SUM_SCORE"
                },
                {
                    "nodeId": "dedup-results",
                    "stageType": "DEDUP_RESULTS",
                    "strategyId": "TEXT_UNIT"
                },
                {
                    "nodeId": "build-context",
                    "stageType": "BUILD_CONTEXT",
                    "strategyId": "RESULT_LIST"
                }
            ],
            "edges": [
                { "from": "generalize-question", "to": "rewrite-query" },
                { "from": "rewrite-query", "to": "fulltext-route" },
                { "from": "fulltext-route", "to": "fuse-results" },
                { "from": "fuse-results", "to": "dedup-results" },
                { "from": "dedup-results", "to": "build-context" }
            ]
        },
        "contextPolicy": {
            "supportedStrategies": ["RESULT_LIST"]
        },
        "rerankPolicy": {"enabled": false}
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'RETRIEVAL', 'enabled', 'memory-card', '[\"memory-card\"]',
                'v1', ?2, '{\"memoryType\":\"RULE\"}', 'PARTIAL_ALLOWED', 1, 1)",
            params![profile_id, config.to_string()],
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
    let connection = open_prepared_connection(database_url);
    let binding_json = connection
        .query_row(
            "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'project-doc'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap();
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

async fn backfill_project_doc_vector_storage(database_url: &str) {
    let repository = SqliteRagRepository::connect(database_url).unwrap();
    set_project_doc_vector_table(database_url, "rag_project_doc_vectors");
    let retry = RagRetryIndexService::new(
        repository,
        FakeSearchRuntime::new().with_embedding(project_doc_embedding()),
    );
    retry
        .retry_index(RagRetryIndexRequest {
            workspace_code: "TRON_CORE".into(),
            corpus_id: "project-doc".into(),
            source_code: "doc-rag-mvp".into(),
            source_version: "v1".into(),
            scope_type: "SPACE".into(),
            scope_code: "TRON_CORE".into(),
            stages: vec!["vectorIndex".into()],
            embedding_model_code: Some("workspace-default-embedding".into()),
            user_code: "USR-LARGE-ID".into(),
            agent_code: "AGENT_CODEX".into(),
        })
        .await
        .unwrap();
}

fn open_prepared_connection(database_url: &str) -> Connection {
    let connection = Connection::open(database_url).unwrap();
    default_sqlite_extension_context()
        .prepare_connection(&connection)
        .unwrap();
    connection
}

fn insert_extension_memory_profile(database_url: &str, profile_id: &str) {
    let connection = open_prepared_connection(database_url);
    let config = json!({
        "routes": [
            {
                "routeId": "memory-fts5",
                "retriever": "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"
            },
            {
                "routeId": "memory-vec0",
                "retriever": "VECTOR_SQLITE_VEC0_COSINE_V1"
            }
        ],
        "fusionPolicy": {"type": "SUM_SCORE"},
        "dedupPolicy": {"type": "TEXT_UNIT"},
        "rerankPolicy": {"enabled": false},
        "hydratePolicy": {"type": "TEXT_UNIT"},
        "contextPolicy": {"supportedStrategies": ["RESULT_LIST", "MEMORY_CONTEXT_PACK"]}
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'RETRIEVAL', 'enabled', 'memory-card', '[\"memory-card\"]',
                'v1', ?2, '{}', 'SKIP_VECTOR_ON_UNAVAILABLE', 1, 1)
            ON CONFLICT (profile_id) DO UPDATE SET
                config_json = excluded.config_json,
                updated_at = excluded.updated_at",
            params![profile_id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();
}

fn insert_rrf_mmr_profile(database_url: &str, profile_id: &str, include_project_doc: bool) {
    let connection = open_prepared_connection(database_url);
    let (primary_corpus, corpus_ids, routes) = if include_project_doc {
        (
            "memory-card",
            json!(["memory-card", "project-doc"]),
            json!([
                {
                    "routeId": "memory-vector",
                    "corpusId": "memory-card",
                    "retriever": "VECTOR_SQLITE_VEC0_COSINE_V1",
                    "topK": 2,
                    "weight": 1.0
                },
                {
                    "routeId": "project-doc-vector",
                    "corpusId": "project-doc",
                    "retriever": "VECTOR_SQLITE_VEC0_COSINE_V1",
                    "topK": 1,
                    "weight": 0.5
                }
            ]),
        )
    } else {
        (
            "memory-card",
            json!(["memory-card"]),
            json!([
                {
                    "routeId": "memory-fulltext",
                    "corpusId": "memory-card",
                    "retriever": "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
                    "topK": 2,
                    "weight": 1.0
                }
            ]),
        )
    };
    let config = json!({
        "routes": routes,
        "fusionPolicy": {"type": "RRF", "rankConstant": 60},
        "dedupPolicy": {"type": "CANONICAL_TEXT_UNIT"},
        "topicPolicy": {
            "enabled": true,
            "type": "EMBEDDING_MMR_V1",
            "candidateWindow": 3,
            "lambda": 0.5,
            "similarityThreshold": 0.80,
            "maxPerTopic": 1,
            "fallback": "KEEP_ORDER"
        },
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
            VALUES (?1, 'RETRIEVAL', 'enabled', ?2, ?3,
                'v1', ?4, '{}', 'SKIP_VECTOR_ON_UNAVAILABLE', 1, 1)
            ON CONFLICT (profile_id) DO UPDATE SET
                config_json = excluded.config_json,
                corpus_ids_json = excluded.corpus_ids_json,
                updated_at = excluded.updated_at",
            params![
                profile_id,
                primary_corpus,
                serde_json::to_string(&corpus_ids).unwrap(),
                serde_json::to_string(&config).unwrap()
            ],
        )
        .unwrap();
}

fn insert_mixed_filter_profile(database_url: &str, profile_id: &str) {
    let connection = open_prepared_connection(database_url);
    let config = json!({
        "routes": [
            {
                "routeId": "memory-vector",
                "corpusId": "memory-card",
                "retriever": "VECTOR_SQLITE_VEC0_COSINE_V1",
                "topK": 4,
                "weight": 1.0
            },
            {
                "routeId": "project-doc-vector",
                "corpusId": "project-doc",
                "retriever": "VECTOR_SQLITE_VEC0_COSINE_V1",
                "topK": 4,
                "weight": 1.0
            },
            {
                "routeId": "skill-vector",
                "corpusId": "skill",
                "retriever": "VECTOR_SQLITE_VEC0_COSINE_V1",
                "topK": 4,
                "weight": 1.0
            }
        ],
        "fusionPolicy": {"type": "SUM_SCORE"},
        "dedupPolicy": {"type": "TEXT_UNIT"},
        "rerankPolicy": {"enabled": false},
        "hydratePolicy": {"type": "TEXT_UNIT"},
        "contextPolicy": {"supportedStrategies": ["RESULT_LIST"]}
    });
    connection
        .execute(
            "INSERT INTO rag_profiles (
                profile_id, profile_type, status, primary_corpus_id, corpus_ids_json,
                required_source_version, config_json, filters_json, degrade_policy,
                created_at, updated_at
            )
            VALUES (?1, 'RETRIEVAL', 'enabled', 'memory-card', '[\"memory-card\",\"project-doc\",\"skill\"]',
                'v1', ?2, '{}', 'SKIP_VECTOR_ON_UNAVAILABLE', 1, 1)
            ON CONFLICT (profile_id) DO UPDATE SET
                config_json = excluded.config_json,
                corpus_ids_json = excluded.corpus_ids_json,
                updated_at = excluded.updated_at",
            params![profile_id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();
}

fn embedding_4096(first: f32, second: f32) -> Vec<f32> {
    let mut embedding = vec![0.0; 4096];
    embedding[0] = first;
    embedding[1] = second;
    embedding
}

fn project_doc_embedding() -> Vec<f32> {
    embedding_4096(0.2, 0.98)
}

fn embedding_1536(first: f32, second: f32) -> Vec<f32> {
    let mut embedding = vec![0.0; 1536];
    embedding[0] = first;
    embedding[1] = second;
    embedding
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

fn table_exists(repository: &SqliteRagRepository, table_name: &str) -> bool {
    repository.rag_table_names().unwrap().contains(table_name)
}

fn set_memory_card_tree_table(database_url: &str, table_name: &str) {
    let connection = open_prepared_connection(database_url);
    let binding_json = connection
        .query_row(
            "SELECT storage_binding_json FROM rag_corpora WHERE corpus_id = 'memory-card'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap();
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
                    "graph": false,
                    "reserved": false
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

fn tree_scope_filter_command(
    corpus_id: &str,
    scope_type: &str,
    scope_code: &str,
    text_unit_code: &str,
    summary_text: &str,
    covered_text_unit_codes: Vec<&str>,
) -> RagPersistIngestCommand {
    RagPersistIngestCommand {
        corpus_id: corpus_id.into(),
        profile_id: "memory-card-tree-pipeline-v1".into(),
        source_code: "scope-filter-source".into(),
        source_version: "v1".into(),
        source_type: "MEMORY_CARD".into(),
        scope: RagIngestScope {
            scope_type: scope_type.into(),
            scope_code: scope_code.into(),
        },
        source_status: "ACTIVE".into(),
        title: Some("Scope filter source".into()),
        content_hash: format!("{scope_type}-{scope_code}-content"),
        body_hash: None,
        metadata: json!({}),
        domain_fields: BTreeMap::new(),
        related_graph: None,
        graph_write_policy: json!({}),
        text_units: vec![RagTextUnitWrite {
            text_unit_code: text_unit_code.into(),
            unit_type: "LEAF".into(),
            text: format!("Hydratable text for {scope_type}:{scope_code}."),
            search_text: format!("Hydratable text for {scope_type}:{scope_code}."),
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
            tree_node_code: format!("scope-filter-source:v1:{scope_type}:tree:root"),
            parent_node_code: "0".into(),
            source_code: "scope-filter-source".into(),
            source_version: "v1".into(),
            scope_type: scope_type.into(),
            scope_code: scope_code.into(),
            path_key: "/".into(),
            level: 0,
            ordinal: 0,
            title: format!("{scope_type} summary"),
            summary_text: summary_text.into(),
            summary_token_count: 8,
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
            covered_text_unit_codes: covered_text_unit_codes
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
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
