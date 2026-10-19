use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository, application::rag::*,
    domain::rag::RagError,
};
#[derive(Clone)]
struct Runtime {
    fail: bool,
}
impl RagEmbeddingRuntime for Runtime {
    fn embedding_fingerprint(
        &self,
        _: &str,
        model: Option<&str>,
        _: usize,
    ) -> Result<Option<String>, String> {
        Ok(Some(format!("space:{}", model.unwrap_or("embedding-a"))))
    }
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let result = RagEmbeddingRuntime::embed(
                self,
                RagEmbeddingCall {
                    workspace_code: call.workspace_code,
                    model_code: call.model_code,
                    input: call.inputs[0].clone(),
                },
            )
            .await?;
            Ok(RagEmbeddingBatchResult {
                model_code: result.model_code,
                provider_model: result.provider_model,
                vectors: call
                    .inputs
                    .iter()
                    .enumerate()
                    .map(|(index, _)| eliteforge_agent_framework::EmbeddingVector {
                        index,
                        values: result.embedding.clone(),
                    })
                    .collect(),
            })
        })
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            if self.fail {
                return Err("embedding outage".into());
            }
            Ok(RagEmbeddingResult {
                model_code: call.model_code.clone().unwrap_or("embedding-a".into()),
                provider_model: Some("provider-a".into()),
                embedding: vec![
                    0.1;
                    if call.model_code.as_deref() == Some("embedding-b") {
                        1536
                    } else {
                        1024
                    }
                ],
            })
        })
    }
}
impl RagGraphRuntime for Runtime {
    fn extract_graph<'a>(
        &'a self,
        _: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>> {
        Box::pin(async { Err("unexpected graph extraction".into()) })
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
        RagEmbeddingRuntime::embed(self, call)
    }
    fn rerank<'a>(
        &'a self,
        _: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async { Err("reranker outage".into()) })
    }
}
fn repository() -> Arc<Mutex<SqliteRagRepository>> {
    let repo = Arc::new(Mutex::new(
        SqliteRagRepository::connect(":memory:").unwrap(),
    ));
    // Ingestion requires an explicit fixed model or automatic route selection.
    RagService::new(repo.clone(), Runtime { fail: false })
        .configure_corpus(
            "skill",
            RagSettings {
                embedding_model_code: Some("embedding-a".into()),
                rerank_model_code: None,
                graph: true,
                ..Default::default()
            },
        )
        .unwrap();
    repo
}
fn service(
    repo: Arc<Mutex<SqliteRagRepository>>,
    runtime: Runtime,
) -> RagService<Arc<Mutex<SqliteRagRepository>>, Runtime> {
    use tron_backend::adapters::outbound::persistence::{
        GrafeoAdapterConfig, GrafeoRagStore, GrafeoStoreCore,
    };
    use tron_backend::application::rag::graph::GraphInfrastructure;
    let index = Arc::new(repo.lock().unwrap().graph_index_repository().unwrap());
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: ":memory:".into(),
    })
    .unwrap();
    RagService::new(repo, runtime).with_graph_infrastructure(GraphInfrastructure::new(
        Arc::new(GrafeoRagStore::new(core)),
        index,
    ))
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
            code: "skill-evidence".into(),
            version: "v1".into(),
        },
        payload: json!({"text":text,"title":"Orchid","domainFields":{"skillName":"orchid","skillDescription":"flower evidence"},"relatedGraph":{"nodes":[{"ref":"orchid","name":"orchid","type":"Plant","evidence":"orchid"}],"edges":[{"from":"orchid","to":"orchid","relationship":"HAS_EVIDENCE","evidence":"orchid"}]}}),
    }
}
fn range(scope: &str) -> AuthorizedRagRange {
    AuthorizedRagRange {
        corpus_id: "skill".into(),
        scopes: vec![RagSearchScope {
            scope_type: "SPACE".into(),
            scope_code: scope.into(),
        }],
        source_version: "v1".into(),
        filters: json!({}),
        embedding_model_code: Some("embedding-a".into()),
        embedding_dimension: Some(1024),
        provider_model: Some("provider-a".into()),
        embedding_fingerprint: None,
        return_fields: vec![],
        fulltext: true,
        vector: true,
        graph: true,
    }
}
#[test]
fn fresh_schema_initializes_only_two_corpora() {
    let repo = repository();
    let ids: Vec<_> = repo
        .list_corpora()
        .unwrap()
        .into_iter()
        .map(|row| row.corpus_id)
        .collect();
    assert_eq!(ids, vec!["memory-card", "skill"]);
}

#[tokio::test]
async fn one_ingest_publishes_all_channels_with_original_evidence_and_scope_filtering() {
    let repo = repository();
    let rag = service(repo.clone(), Runtime { fail: false });
    rag.ingest(request("orchid original evidence"))
        .await
        .unwrap();
    let result = rag
        .retrieve_authorized("SPACE-A", "orchid", &[range("SPACE-A")], 8, None, None)
        .await
        .unwrap();
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].channels.len(), 3);
    assert_eq!(result.hits[0].row.text, "orchid original evidence");
    let denied = rag
        .retrieve_authorized("SPACE-B", "orchid", &[range("SPACE-B")], 8, None, None)
        .await
        .unwrap();
    assert!(denied.hits.is_empty());
    let counts = repo.index_counts("skill").unwrap();
    assert_eq!(
        (
            counts.source_count,
            counts.chunk_count,
            counts.vector_count,
            counts.graph_count
        ),
        (1, 1, 1, 1)
    );
    rag.deactivate_source(RagDeactivateSourceCommand {
        corpus_id: "skill".into(),
        source_code: "skill-evidence".into(),
        source_version: "v1".into(),
        workspace_code: "SPACE-A".into(),
        scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "SPACE-A".into(),
        },
    })
    .unwrap();
    assert!(
        rag.retrieve_authorized("SPACE-A", "orchid", &[range("SPACE-A")], 8, None, None)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(repo.index_counts("skill").unwrap().graph_count, 0);
}
#[tokio::test]
async fn failed_rebuild_keeps_old_version_and_returns_explicit_channel_failures() {
    let repo = repository();
    let rag = service(repo.clone(), Runtime { fail: false });
    rag.ingest(request("orchid published evidence"))
        .await
        .unwrap();
    let failing = rag.with_runtime(Runtime { fail: true });
    assert!(
        failing
            .ingest(request("replacement must not publish"))
            .await
            .is_err()
    );
    let result = failing
        .retrieve_authorized(
            "SPACE-A",
            "orchid",
            &[range("SPACE-A")],
            8,
            Some("reranker"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.hits[0].row.text, "orchid published evidence");
    assert!(!result.warnings.is_empty());
    assert!(
        result
            .stages
            .iter()
            .any(|stage| stage.channel == "vector" && stage.status == "FAILED")
    );
    assert!(
        repo.record_tasks("USER-A")
            .unwrap()
            .iter()
            .any(|task| task.state == "FAILED")
    );
}

#[tokio::test]
async fn published_models_and_dimensions_are_separate_query_spaces() {
    let repo = repository();
    let rag = service(repo.clone(), Runtime { fail: false });
    let mut settings = RagSettings::default();
    settings.graph = false;
    settings.embedding_model_code = Some("embedding-a".into());
    rag.configure_corpus("skill", settings.clone()).unwrap();
    rag.ingest(request("orchid first model evidence"))
        .await
        .unwrap();
    settings.embedding_model_code = Some("embedding-b".into());
    settings.embedding_dimension = 1536;
    rag.configure_corpus("skill", settings).unwrap();
    let mut second = request("orchid second model evidence");
    second.source.code = "second-skill".into();
    rag.ingest(second).await.unwrap();
    let mut baseline = range("SPACE-A");
    baseline.embedding_model_code = None;
    baseline.provider_model = None;
    baseline.graph = false;
    let ranges = rag.partition_ranges(baseline).unwrap();
    assert_eq!(ranges.iter().filter(|range| range.vector).count(), 2);
    let result = rag
        .retrieve_authorized("SPACE-A", "orchid", &ranges, 8, None, None)
        .await
        .unwrap();
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    assert_eq!(result.hits.len(), 2);
    assert!(
        result
            .hits
            .iter()
            .all(|hit| hit.channels.contains("vector"))
    );
    assert_eq!(repo.index_counts("skill").unwrap().vector_count, 2);
}

struct Access {
    revoked: Arc<std::sync::atomic::AtomicBool>,
}
impl RagAccessResolver for Access {
    fn resolve(&self, identity: &RagIdentity) -> Result<Vec<AuthorizedRagRange>, RagError> {
        if identity.user_code != "USER-A"
            || identity.workspace_code != "SPACE-A"
            || identity.agent_code != "AGENT-A"
        {
            return Err(RagError::SearchScopeInvalid);
        }
        Ok(vec![range("SPACE-A")])
    }
    fn recheck(&self, _: &RagIdentity, result: &mut RagEvidenceResult) -> Result<(), RagError> {
        if self.revoked.load(std::sync::atomic::Ordering::SeqCst) {
            result.hits.clear();
        }
        Ok(())
    }
}
fn identity() -> RagIdentity {
    RagIdentity {
        user_code: "USER-A".into(),
        workspace_code: "SPACE-A".into(),
        agent_code: "AGENT-A".into(),
    }
}
fn query() -> RagQueryInput {
    RagQueryInput {
        query: "orchid".into(),
        corpus_ids: vec!["skill".into()],
        filters: json!({}),
        limit: 8,
    }
}

#[derive(Clone)]
struct ContextRankRuntime(Arc<Mutex<Vec<String>>>);
impl RagSearchRuntime for ContextRankRuntime {
    fn embedding_fingerprint(
        &self,
        workspace: &str,
        model: Option<&str>,
        dimension: usize,
    ) -> Result<Option<String>, String> {
        RagSearchRuntime::embedding_fingerprint(
            &Runtime { fail: false },
            workspace,
            model,
            dimension,
        )
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move { RagEmbeddingRuntime::embed(&Runtime { fail: false }, call).await })
    }
    fn rerank<'a>(
        &'a self,
        call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async move {
            *self.0.lock().unwrap() = call.documents.clone();
            let mut order = call
                .documents
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    (
                        i,
                        if text == "Complete stronger source context" {
                            1.
                        } else {
                            0.
                        },
                    )
                })
                .collect::<Vec<_>>();
            order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            order.truncate(call.top_n.unwrap_or(order.len()));
            Ok(order)
        })
    }
}
struct ContextAccess;
impl RagAccessResolver for ContextAccess {
    fn resolve(&self, _: &RagIdentity) -> Result<Vec<AuthorizedRagRange>, RagError> {
        Ok(vec![range("SPACE-A")])
    }
    fn recheck(&self, _: &RagIdentity, _: &mut RagEvidenceResult) -> Result<(), RagError> {
        Ok(())
    }
    fn assemble_context(
        &self,
        _: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        for hit in &mut result.hits {
            hit.context_text = Some(
                if hit.row.text.contains("stronger") {
                    "Complete stronger source context"
                } else {
                    "Complete weaker source context"
                }
                .into(),
            );
        }
        Ok(())
    }
}
#[tokio::test]
async fn context_is_assembled_before_rerank_and_top_n_without_mutating_publication_rows() {
    let repo = repository();
    let rag = service(repo.clone(), Runtime { fail: false });
    rag.ingest(request("orchid weaker source")).await.unwrap();
    let mut stronger = request("orchid stronger source");
    stronger.source.code = "skill-stronger".into();
    rag.ingest(stronger).await.unwrap();
    rag.configure_corpus(
        "skill",
        RagSettings {
            embedding_model_code: Some("embedding-a".into()),
            rerank_model_code: Some("ranker".into()),
            graph: true,
            ..Default::default()
        },
    )
    .unwrap();
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let mut input = query();
    input.limit = 1;
    let result = rag
        .with_runtime(ContextRankRuntime(recorded.clone()))
        .search(&ContextAccess, &identity(), input, None)
        .await
        .unwrap();
    let documents = recorded.lock().unwrap();
    assert!(
        documents
            .iter()
            .any(|text| text == "Complete weaker source context")
    );
    assert!(
        documents
            .iter()
            .any(|text| text == "Complete stronger source context")
    );
    assert_eq!(result.evidence.len(), 1);
    assert_eq!(result.evidence[0].text, "Complete stronger source context");
    assert_eq!(result.evidence[0].source_code, "skill-stronger");
    assert!(
        result
            .warnings
            .iter()
            .all(|warning| !warning.contains("撤销发布"))
    );
}
#[derive(Clone)]
struct RevokingRuntime {
    revoked: Arc<std::sync::atomic::AtomicBool>,
}
impl RagSearchRuntime for RevokingRuntime {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let result = RagEmbeddingRuntime::embed(&Runtime { fail: false }, call).await;
            self.revoked
                .store(true, std::sync::atomic::Ordering::SeqCst);
            result
        })
    }
    fn rerank<'a>(
        &'a self,
        _: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async { Ok(vec![]) })
    }
}
#[tokio::test]
async fn client_filters_cannot_widen_scope_and_revocation_is_rechecked_after_recall() {
    let repo = repository();
    let rag = service(repo.clone(), Runtime { fail: false });
    rag.ingest(request("orchid original evidence"))
        .await
        .unwrap();
    let revoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let access = Access {
        revoked: revoked.clone(),
    };
    let mut restricted = query();
    restricted.filters = json!({"scopeCode":"SPACE-B"});
    assert!(
        rag.search(&access, &identity(), restricted, None)
            .await
            .unwrap()
            .evidence
            .is_empty()
    );
    let mut unauthorized = query();
    unauthorized.corpus_ids = vec!["not-granted".into()];
    assert!(matches!(
        rag.search(&access, &identity(), unauthorized, None).await,
        Err(RagError::SearchScopeInvalid)
    ));
    assert!(
        serde_json::from_value::<RagQueryInput>(json!({"query":"orchid","userCode":"OTHER-USER"}))
            .is_err()
    );
    let revoking = rag.with_runtime(RevokingRuntime { revoked });
    let result = revoking
        .search(&access, &identity(), query(), None)
        .await
        .unwrap();
    assert!(result.evidence.is_empty());
    assert!(
        result.graph_context.is_empty(),
        "revoked graph aliases and relationship evidence must not escape"
    );
    assert!(
        result
            .stages
            .iter()
            .filter(|s| s.channel.starts_with("graph"))
            .all(|s| s.hit_count == 0),
        "revoked sources must not escape through graph diagnostic counts"
    );
    assert!(!result.warnings.is_empty());
}
#[derive(Clone)]
struct SlowRuntime {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
impl RagEmbeddingRuntime for SlowRuntime {
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.started.notify_one();
            self.release.notified().await;
            RagEmbeddingRuntime::embed_batch(&Runtime { fail: false }, call).await
        })
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.started.notify_one();
            self.release.notified().await;
            RagEmbeddingRuntime::embed(&Runtime { fail: false }, call).await
        })
    }
}
impl RagGraphRuntime for SlowRuntime {
    fn extract_graph<'a>(
        &'a self,
        _: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>> {
        Box::pin(async { Err("unexpected graph extraction".into()) })
    }
}
#[tokio::test]
async fn cancellation_fences_publication_and_cancelled_task_can_be_retried() {
    let repo = repository();
    let fast = service(repo.clone(), Runtime { fail: false });
    fast.ingest(request("orchid prior published evidence"))
        .await
        .unwrap();
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let slow = fast.with_runtime(SlowRuntime {
        started: started.clone(),
        release: release.clone(),
    });
    let work =
        tokio::spawn(async move { slow.ingest(request("orchid replacement evidence")).await });
    started.notified().await;
    let task = repo
        .record_tasks("USER-A")
        .unwrap()
        .into_iter()
        .find(|task| task.state == "RUNNING")
        .unwrap();
    fast.cancel_record_task("USER-A", &task.id).unwrap();
    release.notify_one();
    assert!(work.await.unwrap().is_err());
    assert_eq!(
        repo.record_tasks("USER-A")
            .unwrap()
            .into_iter()
            .find(|row| row.id == task.id)
            .unwrap()
            .state,
        "CANCELLED"
    );
    let result = fast
        .retrieve_authorized("SPACE-A", "orchid", &[range("SPACE-A")], 8, None, None)
        .await
        .unwrap();
    assert_eq!(result.hits[0].row.text, "orchid prior published evidence");
    assert!(
        fast.retry_record_task("OTHER-USER", &task.id)
            .await
            .is_err()
    );
    fast.retry_record_task("USER-A", &task.id).await.unwrap();
    let result = fast
        .retrieve_authorized("SPACE-A", "orchid", &[range("SPACE-A")], 8, None, None)
        .await
        .unwrap();
    assert_eq!(result.hits[0].row.text, "orchid replacement evidence");
}

#[tokio::test]
async fn chinese_question_uses_segmented_fulltext_and_source_graph_evidence() {
    let rag = service(repository(), Runtime { fail: false });
    rag.configure_corpus(
        "skill",
        RagSettings {
            vector: false,
            graph: true,
            embedding_model_code: Some("embedding-a".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let mut input = request("术后胸痛需要评估，警惕严重并发症。");
    input.payload["relatedGraph"] = json!({"nodes":[{"ref":"胸痛","name":"胸痛","type":"Symptom","evidence":"术后胸痛"}],"edges":[]});
    rag.ingest(input).await.unwrap();
    let mut authorized = range("SPACE-A");
    authorized.vector = false;
    let found = rag
        .retrieve_authorized(
            "SPACE-A",
            "术后出现胸痛怎么处理",
            &[authorized],
            8,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(found.warnings.is_empty(), "{:?}", found.warnings);
    assert_eq!(found.hits.len(), 1);
    assert!(found.hits[0].channels.contains("fulltext"));
    assert!(found.hits[0].channels.contains("graph"));
}

#[derive(Clone)]
struct BatchFailureRuntime {
    calls: Arc<Mutex<Vec<Vec<String>>>>,
}
impl RagEmbeddingRuntime for BatchFailureRuntime {
    fn embed<'a>(
        &'a self,
        _: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async { panic!("single input ingestion is forbidden") })
    }
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let attempt = {
                let mut calls = self.calls.lock().unwrap();
                calls.push(call.inputs.clone());
                calls.len()
            };
            if attempt == 2 {
                return Err("second batch outage".into());
            }
            RagEmbeddingRuntime::embed_batch(&Runtime { fail: false }, call).await
        })
    }
}
impl RagGraphRuntime for BatchFailureRuntime {
    fn extract_graph<'a>(
        &'a self,
        _: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>> {
        Box::pin(async { Err("unexpected graph extraction".into()) })
    }
}
#[tokio::test]
async fn embedding_batch_record_checkpoint_survives_failure_and_retry_without_reembedding_prefix() {
    let repo = repository();
    let runtime = BatchFailureRuntime {
        calls: Arc::new(Mutex::new(vec![])),
    };
    let rag = RagService::new(repo.clone(), runtime.clone()).with_embedding_batch_limits(
        RagEmbeddingBatchLimits {
            max_inputs: 2,
            max_tokens: 32_768,
        },
    );
    let mut settings = rag.settings_for_corpus("skill").unwrap();
    settings.graph = false;
    rag.configure_corpus("skill", settings).unwrap();
    assert!(
        rag.ingest(request(
            &(0..200)
                .map(|i| format!("orchid evidence {i:04} "))
                .collect::<String>()
        ))
        .await
        .is_err()
    );
    let task = repo.record_tasks("USER-A").unwrap().remove(0);
    assert!(task.total_units > 2);
    let stored = repo.record_task("USER-A", &task.id).unwrap().unwrap();
    assert_eq!(stored.state, "FAILED");
    assert_eq!(stored.completed_units, 2);
    assert_eq!(stored.checkpoint.as_ref().unwrap().vectors.vectors.len(), 2);
    assert_eq!(repo.index_counts("skill").unwrap().source_count, 0);
    let retry = rag
        .prepare_retry_record_task("USER-A", &task.id, false)
        .unwrap();
    let result = rag.execute_record_task(retry.clone()).await.unwrap();
    assert_eq!(result.text_unit_count, task.total_units);
    let calls = runtime.calls.lock().unwrap();
    assert_eq!(calls[1], calls[2]);
    assert!(
        calls[0]
            .iter()
            .all(|input| !calls[2..].iter().flatten().any(|later| later == input)),
        "completed chunks must never be resent"
    );
    let stored = repo.record_task("USER-A", &retry.id).unwrap().unwrap();
    assert_eq!(stored.state, "COMPLETED");
    assert_eq!(stored.completed_units, task.total_units);
}
