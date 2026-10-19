//! The public record-ingestion path must atomically publish real graph generations.
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tron_backend::{
    adapters::outbound::persistence::{
        GrafeoAdapterConfig, GrafeoRagStore, GrafeoStoreCore, SqliteRagRepository,
    },
    application::rag::{graph::*, *},
    domain::rag::graph::GraphPartition,
};

#[derive(Clone, Default)]
struct Runtime {
    fail: Arc<AtomicBool>,
    batches: Arc<Mutex<Vec<Vec<String>>>>,
    query_calls: Arc<AtomicUsize>,
    revoke_on_query: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}
impl RagEmbeddingRuntime for Runtime {
    fn embedding_fingerprint(
        &self,
        _: &str,
        _: Option<&str>,
        _: usize,
    ) -> Result<Option<String>, String> {
        Ok(Some("mock-space-2".into()))
    }
    fn embed<'a>(
        &'a self,
        _: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async { panic!("indexing uses the batch adapter") })
    }
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        Box::pin(async move {
            if self.fail.load(Ordering::SeqCst) {
                return Err("node embedding unavailable".into());
            }
            self.batches.lock().unwrap().push(call.inputs.clone());
            Ok(RagEmbeddingBatchResult {
                model_code: call.model_code.unwrap(),
                provider_model: Some("mock-provider".into()),
                vectors: call
                    .inputs
                    .iter()
                    .enumerate()
                    .map(|(index, _)| eliteforge_agent_framework::EmbeddingVector {
                        index,
                        values: vec![1., 0.],
                    })
                    .collect(),
            })
        })
    }
}
impl RagGraphRuntime for Runtime {
    fn extract_graph<'a>(
        &'a self,
        _: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>> {
        Box::pin(async { panic!("the request supplies validated structured evidence") })
    }
}
impl RagSearchRuntime for Runtime {
    fn embedding_fingerprint(
        &self,
        workspace: &str,
        model: Option<&str>,
        dimension: usize,
    ) -> Result<Option<String>, String> {
        RagEmbeddingRuntime::embedding_fingerprint(self, workspace, model, dimension)
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.query_calls.fetch_add(1, Ordering::SeqCst);
            if let Some(revoked) = self.revoke_on_query.lock().unwrap().as_ref() {
                revoked.store(true, Ordering::SeqCst);
            }
            if self.fail.load(Ordering::SeqCst) {
                return Err("query embedding unavailable".into());
            }
            Ok(RagEmbeddingResult {
                model_code: call.model_code.unwrap(),
                provider_model: Some("mock-provider".into()),
                embedding: vec![1., 0.],
            })
        })
    }
    fn rerank<'a>(
        &'a self,
        _: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async { Err("rerank is not requested in this test".into()) })
    }
}
fn request(text: &str) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: "skill".into(),
        workspace_code: "SPACE-A".into(),
        access_context: RagIngestAccessContext {
            user_code: "USER-A".into(),
            agent_code: "AGENT-A".into(),
            workspace_code: "SPACE-A".into(),
        },
        target_scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "SPACE-A".into(),
        },
        source: RagIngestSource {
            code: "source".into(),
            version: "v1".into(),
        },
        payload: json!({"text":text,"title":"Service manual","domainFields":{"skillName":"manual","skillDescription":"Source evidence"},
            "relatedGraph":{"nodes":[{"ref":"a","name":"Alpha","type":"Service","evidence":"Alpha calls Beta."},
                {"ref":"b","name":"Beta","type":"Service","evidence":"Alpha calls Beta."}],
                "edges":[{"from":"a","to":"b","relationship":"CALLS","evidence":"Alpha calls Beta."}]}}),
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn graph_only_record_indexes_nodes_and_failed_replacement_preserves_published_graph() {
    let repo = Arc::new(Mutex::new(
        SqliteRagRepository::connect(":memory:").unwrap(),
    ));
    let index = Arc::new(repo.lock().unwrap().graph_index_repository().unwrap());
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: ":memory:".into(),
    })
    .unwrap();
    let store = Arc::new(GrafeoRagStore::new(core));
    let runtime = Runtime::default();
    let service = RagService::new(repo.clone(), runtime.clone())
        .with_graph_infrastructure(GraphInfrastructure::new(store, index.clone()));
    service
        .configure_corpus(
            "skill",
            RagSettings {
                fulltext: false,
                vector: false,
                graph: true,
                embedding_dimension: 2,
                embedding_model_code: Some("mock-embedding".into()),
                graph_model_code: Some("mock-extractor".into()),
                rerank_model_code: None,
                ..Default::default()
            },
        )
        .unwrap();
    service.ingest(request("Alpha calls Beta.")).await.unwrap();
    let partition = GraphPartition {
        corpus_id: "skill".into(),
        scope_type: "SPACE".into(),
        scope_code: "SPACE-A".into(),
    };
    let active = index.active_generations(&[partition.clone()]).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].expected_projections, 2);
    let (_, projections) = index.export_generation(&active[0].id).unwrap().unwrap();
    assert_eq!(projections.len(), 2);
    assert!(projections.iter().all(|p| p.embedding == [1., 0.]));
    assert_eq!(
        runtime
            .batches
            .lock()
            .unwrap()
            .iter()
            .map(Vec::len)
            .sum::<usize>(),
        2,
        "only graph nodes are embedded when chunk vectors are off"
    );
    let range = AuthorizedRagRange {
        corpus_id: "skill".into(),
        scopes: vec![RagSearchScope {
            scope_type: "SPACE".into(),
            scope_code: "SPACE-A".into(),
        }],
        source_version: "v1".into(),
        filters: json!({}),
        embedding_model_code: None,
        embedding_dimension: None,
        provider_model: None,
        embedding_fingerprint: None,
        return_fields: vec![],
        fulltext: false,
        vector: false,
        graph: true,
    };
    let recall = service
        .retrieve_authorized("SPACE-A", "Alpha", &[range.clone()], 8, None, None)
        .await
        .unwrap();
    assert_eq!(
        recall.hits.len(),
        1,
        "graph-only recall must return its original source chunk"
    );
    assert!(recall.hits[0].channels.contains("graph"));
    assert!(recall.hits[0].row.text.contains("Alpha calls Beta."));
    assert_eq!(recall.graph_context.len(), 1);
    assert_eq!(recall.graph_context[0].nodes.len(), 2);
    assert_eq!(recall.graph_context[0].edges.len(), 1);
    assert_eq!(recall.graph_context[0].contributions.len(), 2);
    assert_eq!(recall.graph_context[0].provenance.len(), 4);
    assert!(recall.warnings.is_empty(), "{:?}", recall.warnings);
    assert_eq!(runtime.query_calls.load(Ordering::SeqCst), 1);
    runtime.fail.store(true, Ordering::SeqCst);
    let replacement_text = "Alpha calls Beta. Additional original text.";
    let mut replacement = request(replacement_text);
    for node in replacement.payload["relatedGraph"]["nodes"]
        .as_array_mut()
        .unwrap()
    {
        node["evidence"] = json!(replacement_text);
    }
    replacement.payload["relatedGraph"]["edges"][0]["evidence"] = json!(replacement_text);
    assert!(service.ingest(replacement).await.is_err());
    let after = index.active_generations(&[partition]).unwrap();
    assert_eq!(after[0].id, active[0].id);
    repo.invalidate_vector_cache(RagRouteSearchQuery {
        corpus_id: "skill".into(),
        route_id: "graph-invalidation".into(),
        retriever: "STRUCTURED_FILTER_SQLITE_V1".into(),
        workspace_code: "SPACE-A".into(),
        query: String::new(),
        query_embedding: None,
        query_embedding_model_code: None,
        source_version: "v1".into(),
        scopes: range.scopes.clone(),
        filters: json!({}),
        policy: json!({"graphGenerationIds":[active[0].id]}),
        return_fields: vec![],
        limit: 100,
    })
    .unwrap();
    assert!(
        service.ingest(request("Alpha calls Beta.")).await.is_err(),
        "an observed invalidation must prevent old node-vector reuse even with unchanged input"
    );
    let partial = service
        .retrieve_authorized("SPACE-A", "Alpha", &[range.clone()], 8, None, None)
        .await
        .unwrap();
    assert_eq!(
        partial.hits.len(),
        1,
        "node fulltext remains usable if query embedding fails"
    );
    assert!(
        partial
            .stages
            .iter()
            .any(|s| s.channel == "graph.vector" && s.status == "FAILED")
    );
    let mut foreign = range.clone();
    foreign.scopes[0].scope_code = "OTHER-SPACE".into();
    let private = service
        .retrieve_authorized("SPACE-A", "Alpha", &[foreign], 8, None, None)
        .await
        .unwrap();
    assert!(private.hits.is_empty());
    assert!(private.graph_context.is_empty());
    let tasks = repo.record_tasks("USER-A").unwrap();
    assert!(tasks.iter().any(|t| t.state == "FAILED"));
    assert!(tasks.iter().any(|t| t.state == "COMPLETED"));
    service
        .deactivate_source(RagDeactivateSourceCommand {
            corpus_id: "skill".into(),
            source_code: "source".into(),
            source_version: "v1".into(),
            workspace_code: "SPACE-A".into(),
            scope: RagIngestScope {
                scope_type: "SPACE".into(),
                scope_code: "SPACE-A".into(),
            },
        })
        .unwrap();
    assert!(
        index
            .active_generations(&[GraphPartition {
                corpus_id: "skill".into(),
                scope_type: "SPACE".into(),
                scope_code: "SPACE-A".into()
            }])
            .unwrap()
            .is_empty()
    );
    let deleted = service
        .retrieve_authorized("SPACE-A", "Alpha", &[range], 8, None, None)
        .await
        .unwrap();
    assert!(deleted.hits.is_empty());
    assert!(deleted.graph_context.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn mixed_recall_reuses_query_embedding_and_disabling_graph_retires_nodes_immediately() {
    let repo = Arc::new(Mutex::new(
        SqliteRagRepository::connect(":memory:").unwrap(),
    ));
    let index = Arc::new(repo.lock().unwrap().graph_index_repository().unwrap());
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: ":memory:".into(),
    })
    .unwrap();
    let runtime = Runtime::default();
    let service = RagService::new(repo.clone(), runtime.clone()).with_graph_infrastructure(
        GraphInfrastructure::new(Arc::new(GrafeoRagStore::new(core)), index.clone()),
    );
    let mut settings = RagSettings {
        fulltext: true,
        vector: true,
        graph: true,
        embedding_dimension: 2,
        embedding_model_code: Some("mock-embedding".into()),
        graph_model_code: Some("mock-extractor".into()),
        rerank_model_code: None,
        ..Default::default()
    };
    service.configure_corpus("skill", settings.clone()).unwrap();
    service.ingest(request("Alpha calls Beta.")).await.unwrap();
    let node_batches = || {
        runtime
            .batches
            .lock()
            .unwrap()
            .iter()
            .filter(|batch| batch.iter().any(|text| text.contains("\nService\n")))
            .count()
    };
    let paid_batches = node_batches();
    for fulltext in [false, true] {
        settings.fulltext = fulltext;
        service.configure_corpus("skill", settings.clone()).unwrap();
        service.ingest(request("Alpha calls Beta.")).await.unwrap();
        assert_eq!(
            node_batches(),
            paid_batches,
            "two consecutive supplementary generations must reuse unchanged node vectors"
        );
    }
    let range = AuthorizedRagRange {
        corpus_id: "skill".into(),
        scopes: vec![RagSearchScope {
            scope_type: "SPACE".into(),
            scope_code: "SPACE-A".into(),
        }],
        source_version: "v1".into(),
        filters: json!({}),
        embedding_model_code: Some("mock-embedding".into()),
        embedding_dimension: Some(2),
        provider_model: Some("mock-provider".into()),
        embedding_fingerprint: Some("mock-space-2".into()),
        return_fields: vec![],
        fulltext: true,
        vector: true,
        graph: true,
    };
    let result = service
        .retrieve_authorized("SPACE-A", "Alpha", &[range.clone()], 8, None, None)
        .await
        .unwrap();
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].channels.len(), 3);
    assert_eq!(
        runtime.query_calls.load(Ordering::SeqCst),
        1,
        "body and node recall share one query vector in the same model space"
    );
    settings.graph = false;
    service.configure_corpus("skill", settings).unwrap();
    let partition = GraphPartition {
        corpus_id: "skill".into(),
        scope_type: "SPACE".into(),
        scope_code: "SPACE-A".into(),
    };
    assert!(
        index.active_generations(&[partition]).unwrap().is_empty(),
        "no reindex is required to revoke graph visibility"
    );
    let result = service
        .retrieve_authorized("SPACE-A", "Alpha", &[range], 8, None, None)
        .await
        .unwrap();
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].channels.len(), 2);
    assert!(result.graph_context.is_empty());
}

struct Access {
    ranges: Vec<AuthorizedRagRange>,
    revoked: Arc<AtomicBool>,
}
impl RagAccessResolver for Access {
    fn resolve(
        &self,
        _: &RagIdentity,
    ) -> Result<Vec<AuthorizedRagRange>, tron_backend::domain::rag::RagError> {
        Ok(if self.revoked.load(Ordering::SeqCst) {
            vec![]
        } else {
            self.ranges.clone()
        })
    }
    fn recheck(
        &self,
        _: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), tron_backend::domain::rag::RagError> {
        if self.revoked.load(Ordering::SeqCst) {
            result.hits.clear();
        }
        Ok(())
    }
}
type TestService = RagService<Arc<Mutex<SqliteRagRepository>>, Runtime>;
fn graph_fixture() -> (TestService, Runtime, Access, RagIdentity) {
    let repo = Arc::new(Mutex::new(
        SqliteRagRepository::connect(":memory:").unwrap(),
    ));
    let index = Arc::new(repo.lock().unwrap().graph_index_repository().unwrap());
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: ":memory:".into(),
    })
    .unwrap();
    let runtime = Runtime::default();
    let service = RagService::new(repo, runtime.clone()).with_graph_infrastructure(
        GraphInfrastructure::new(Arc::new(GrafeoRagStore::new(core)), index),
    );
    service
        .configure_corpus(
            "skill",
            RagSettings {
                fulltext: false,
                vector: false,
                graph: true,
                embedding_dimension: 2,
                embedding_model_code: Some("mock-embedding".into()),
                graph_model_code: Some("mock-extractor".into()),
                rerank_model_code: None,
                ..Default::default()
            },
        )
        .unwrap();
    let access = Access {
        ranges: vec![AuthorizedRagRange {
            corpus_id: "skill".into(),
            scopes: vec![RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: "SPACE-A".into(),
            }],
            source_version: "v1".into(),
            filters: json!({}),
            embedding_model_code: None,
            embedding_dimension: None,
            provider_model: None,
            embedding_fingerprint: None,
            return_fields: vec![],
            fulltext: false,
            vector: false,
            graph: true,
        }],
        revoked: Arc::new(AtomicBool::new(false)),
    };
    let identity = RagIdentity {
        user_code: "USER-A".into(),
        workspace_code: "SPACE-A".into(),
        agent_code: "AGENT-A".into(),
    };
    (service, runtime, access, identity)
}
fn search_input() -> GraphSearchInput {
    GraphSearchInput {
        query: "Alpha".into(),
        corpus_ids: vec!["skill".into()],
        filters: json!({}),
        options: GraphReadOptions::default(),
    }
}
fn continuation(cursor: String) -> GraphExpandInput {
    GraphExpandInput {
        node_ids: vec![],
        corpus_ids: vec![],
        filters: json!({}),
        options: GraphReadOptions::default(),
        cursor: Some(cursor),
    }
}
fn star_request(children: usize) -> RagIngestRequest {
    let text = (0..children)
        .map(|i| format!("Alpha calls Service{i}. "))
        .collect::<String>();
    let mut input = request(&text);
    let mut nodes =
        vec![json!({"ref":"a","name":"Alpha","type":"Service","evidence":"Alpha calls Service0."})];
    let mut edges = vec![];
    for i in 0..children {
        let quote = format!("Alpha calls Service{i}.");
        nodes.push(json!({"ref":format!("s{i}"),"name":format!("Service{i}"),"type":"Service","evidence":quote}));
        edges
            .push(json!({"from":"a","to":format!("s{i}"),"relationship":"CALLS","evidence":quote}));
    }
    input.payload["relatedGraph"] = json!({"nodes":nodes,"edges":edges});
    input
}

#[tokio::test(flavor = "multi_thread")]
async fn graph_query_is_strict_but_selected_node_expansion_does_not_call_models() {
    let (service, runtime, access, identity) = graph_fixture();
    service.ingest(request("Alpha calls Beta.")).await.unwrap();
    let page = service
        .graph_search(&access, &identity, search_input(), None)
        .await
        .unwrap();
    assert_eq!(page.contexts.len(), 1);
    assert_eq!(page.references.len(), 1);
    assert_eq!(
        page.references[0].file_name.as_deref(),
        Some("Service manual")
    );
    assert_eq!(page.references[0].quote, "Alpha calls Beta.");
    let alpha = page.contexts[0]
        .nodes
        .iter()
        .find(|n| n.name == "Alpha")
        .unwrap()
        .id
        .clone();
    let calls = runtime.query_calls.load(Ordering::SeqCst);
    runtime.fail.store(true, Ordering::SeqCst);
    assert!(matches!(
        service
            .graph_search(&access, &identity, search_input(), None)
            .await,
        Err(GraphQueryError::Unavailable(_))
    ));
    let after_failure = runtime.query_calls.load(Ordering::SeqCst);
    assert!(after_failure > calls);
    let expanded = service
        .graph_expand(
            &access,
            &identity,
            GraphExpandInput {
                node_ids: vec![alpha],
                corpus_ids: vec!["skill".into()],
                filters: json!({}),
                options: GraphReadOptions {
                    incoming_depth: 0,
                    outgoing_depth: 1,
                    ..Default::default()
                },
                cursor: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(expanded.contexts[0].edges.len(), 1);
    assert_eq!(runtime.query_calls.load(Ordering::SeqCst), after_failure);
    let ordinary = service
        .retrieve_authorized("SPACE-A", "Alpha", &access.ranges, 8, None, None)
        .await
        .unwrap();
    assert_eq!(ordinary.hits.len(), 1);
    assert!(
        ordinary
            .stages
            .iter()
            .any(|s| s.channel == "graph.vector" && s.status == "FAILED")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn graph_cursor_drains_all_relationships_with_small_windows_and_no_more_model_calls() {
    let (service, runtime, access, identity) = graph_fixture();
    service.ingest(star_request(12)).await.unwrap();
    let initial = service
        .graph_search(&access, &identity, search_input(), None)
        .await
        .unwrap();
    let alpha = initial.contexts[0]
        .nodes
        .iter()
        .find(|n| n.name == "Alpha")
        .unwrap()
        .id
        .clone();
    let calls = runtime.query_calls.load(Ordering::SeqCst);
    let mut page = service
        .graph_expand(
            &access,
            &identity,
            GraphExpandInput {
                node_ids: vec![alpha],
                corpus_ids: vec!["skill".into()],
                filters: json!({}),
                options: GraphReadOptions {
                    incoming_depth: 0,
                    outgoing_depth: 1,
                    node_limit: 3,
                    edge_limit: 1,
                    ..Default::default()
                },
                cursor: None,
            },
        )
        .await
        .unwrap();
    let expiry = page.expires_at;
    assert!(expiry.is_some());
    let mut edges = std::collections::BTreeSet::new();
    let mut pages = 0;
    loop {
        pages += 1;
        assert!(pages <= 20, "cursor must progress");
        assert!(page.contexts.iter().map(|c| c.nodes.len()).sum::<usize>() <= 3);
        assert!(page.contexts.iter().map(|c| c.edges.len()).sum::<usize>() <= 1);
        for edge in page.contexts.iter().flat_map(|c| &c.edges) {
            edges.insert(edge.id.clone());
        }
        let Some(cursor) = page.next_cursor else {
            break;
        };
        assert_eq!(page.expires_at, expiry, "continuing must not extend expiry");
        assert!(!page.unfinished.is_empty());
        page = service
            .graph_expand(&access, &identity, continuation(cursor))
            .await
            .unwrap();
    }
    assert_eq!(edges.len(), 12);
    assert_eq!(runtime.query_calls.load(Ordering::SeqCst), calls);
}

#[tokio::test(flavor = "multi_thread")]
async fn graph_cursor_is_bound_to_identity_grants_and_publication() {
    let (service, _, access, identity) = graph_fixture();
    service.ingest(star_request(12)).await.unwrap();
    let initial = service
        .graph_search(&access, &identity, search_input(), None)
        .await
        .unwrap();
    let alpha = initial.contexts[0]
        .nodes
        .iter()
        .find(|n| n.name == "Alpha")
        .unwrap()
        .id
        .clone();
    let page = service
        .graph_expand(
            &access,
            &identity,
            GraphExpandInput {
                node_ids: vec![alpha],
                corpus_ids: vec!["skill".into()],
                filters: json!({}),
                cursor: None,
                options: GraphReadOptions {
                    incoming_depth: 0,
                    outgoing_depth: 1,
                    node_limit: 3,
                    edge_limit: 1,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let cursor = page.next_cursor.unwrap();
    let mut other = identity.clone();
    other.agent_code = "OTHER-AGENT".into();
    assert!(matches!(
        service
            .graph_expand(&access, &other, continuation(cursor.clone()))
            .await,
        Err(GraphQueryError::SnapshotChanged)
    ));
    let mut tampered = continuation(cursor.clone());
    tampered.options.outgoing_depth = 6;
    assert!(matches!(
        service.graph_expand(&access, &identity, tampered).await,
        Err(GraphQueryError::Invalid)
    ));
    access.revoked.store(true, Ordering::SeqCst);
    assert!(matches!(
        service
            .graph_expand(&access, &identity, continuation(cursor.clone()))
            .await,
        Err(GraphQueryError::Forbidden)
    ));
    access.revoked.store(false, Ordering::SeqCst);
    service.ingest(request("Alpha calls Beta.")).await.unwrap();
    assert!(matches!(
        service
            .graph_expand(&access, &identity, continuation(cursor))
            .await,
        Err(GraphQueryError::SnapshotChanged)
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn graph_depth_zero_and_relation_filter_remain_within_source_scope() {
    let (service, _, access, identity) = graph_fixture();
    service.ingest(request("Alpha calls Beta.")).await.unwrap();
    let mut input = search_input();
    input.options.seed_count = 1;
    input.options.incoming_depth = 0;
    input.options.outgoing_depth = 0;
    let page = service
        .graph_search(&access, &identity, input.clone(), None)
        .await
        .unwrap();
    assert_eq!(page.contexts[0].nodes.len(), 1);
    assert!(page.contexts[0].edges.is_empty());
    assert!(page.next_cursor.is_none());
    assert!(!page.references.is_empty());
    input.options.outgoing_depth = 3;
    input
        .options
        .relationship_types
        .insert("DOES_NOT_EXIST".into());
    let page = service
        .graph_search(&access, &identity, input.clone(), None)
        .await
        .unwrap();
    assert_eq!(page.contexts[0].nodes.len(), 1);
    assert!(page.contexts[0].edges.is_empty());
    let mut foreign = Access {
        ranges: access.ranges.clone(),
        revoked: Arc::new(AtomicBool::new(false)),
    };
    foreign.ranges[0].scopes[0].scope_code = "PRIVATE-SPACE".into();
    let page = service
        .graph_search(&foreign, &identity, input, None)
        .await
        .unwrap();
    assert!(page.contexts.is_empty());
    assert!(page.references.is_empty());
    let expanded = service
        .graph_expand(
            &foreign,
            &identity,
            GraphExpandInput {
                node_ids: vec![
                    page.contexts
                        .first()
                        .map(|c| c.seeds[0].id.clone())
                        .unwrap_or("unknown".into()),
                ],
                corpus_ids: vec!["skill".into()],
                filters: json!({}),
                options: GraphReadOptions::default(),
                cursor: None,
            },
        )
        .await;
    assert!(matches!(expanded, Err(GraphQueryError::NodeNotFound)));
}

#[tokio::test(flavor = "multi_thread")]
async fn graph_search_rechecks_grants_after_query_embedding_and_before_returning_any_counts() {
    let (service, runtime, access, identity) = graph_fixture();
    service.ingest(request("Alpha calls Beta.")).await.unwrap();
    *runtime.revoke_on_query.lock().unwrap() = Some(access.revoked.clone());
    assert!(matches!(
        service
            .graph_search(&access, &identity, search_input(), None)
            .await,
        Err(GraphQueryError::Forbidden)
    ));
    assert_eq!(runtime.query_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn graph_search_applies_one_seed_budget_across_authorized_partitions() {
    let (service, runtime, mut access, identity) = graph_fixture();
    service.ingest(request("Alpha calls Beta.")).await.unwrap();
    let mut other = request("Alpha calls Beta.");
    other.source.code = "other-source".into();
    other.workspace_code = "SPACE-B".into();
    other.access_context.workspace_code = "SPACE-B".into();
    other.target_scope.scope_code = "SPACE-B".into();
    service.ingest(other).await.unwrap();
    access.ranges[0].scopes.push(RagSearchScope {
        scope_type: "SPACE".into(),
        scope_code: "SPACE-B".into(),
    });
    let mut query = search_input();
    query.options = GraphReadOptions {
        seed_count: 1,
        incoming_depth: 0,
        outgoing_depth: 0,
        ..Default::default()
    };
    let page = service
        .graph_search(&access, &identity, query, None)
        .await
        .unwrap();
    assert_eq!(
        page.contexts.iter().map(|c| c.seeds.len()).sum::<usize>(),
        1,
        "seed budget is global, not one budget per permission partition"
    );
    assert_eq!(
        page.contexts.iter().map(|c| c.nodes.len()).sum::<usize>(),
        1
    );
    assert_eq!(
        runtime.query_calls.load(Ordering::SeqCst),
        1,
        "same node model space shares one query embedding across all partitions"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn provenance_pagination_preserves_all_originals_and_per_source_descriptions() {
    let (service, runtime, access, identity) = graph_fixture();
    for i in 0..130 {
        let mut input = request("Alpha calls Beta.");
        input.source.code = format!("manual-{i}");
        input.payload["title"] = json!(format!("manual-{i}.md"));
        service.ingest(input).await.unwrap();
    }
    let mut query = search_input();
    query.options = GraphReadOptions {
        seed_count: 2,
        incoming_depth: 0,
        outgoing_depth: 0,
        ..Default::default()
    };
    let mut page = service
        .graph_search(&access, &identity, query, None)
        .await
        .unwrap();
    assert!(page.truncated_by.contains("ProvenanceBudget"));
    let query_calls = runtime.query_calls.load(Ordering::SeqCst);
    let mut originals = std::collections::BTreeSet::new();
    let mut descriptions = std::collections::BTreeSet::new();
    let mut count = 0;
    loop {
        count += 1;
        assert!(
            count <= 4,
            "provenance cursor must drain its saved page before traversing more"
        );
        let view = GraphPresentation::from(page);
        for reference in view.references {
            assert_eq!(reference.quote, "Alpha calls Beta.");
            originals.insert(reference.source_code);
        }
        for node in view.nodes {
            for description in node.descriptions {
                descriptions.insert((node.id.clone(), description.reference_id));
            }
        }
        let Some(cursor) = view.next_cursor else {
            break;
        };
        page = service
            .graph_expand(&access, &identity, continuation(cursor))
            .await
            .unwrap();
    }
    assert_eq!(
        originals.len(),
        130,
        "same-name entities must retain each original source support"
    );
    assert_eq!(
        descriptions.len(),
        260,
        "descriptions must be accumulated per entity and original source"
    );
    assert_eq!(runtime.query_calls.load(Ordering::SeqCst), query_calls);
}
