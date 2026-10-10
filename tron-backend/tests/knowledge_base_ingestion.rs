mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tron_backend::{
    adapters::outbound::persistence::sqlite::{
        SqliteKnowledgeBaseRepository, SqliteKnowledgeIngestionQueue,
    },
    application::{
        knowledge_base::{KnowledgeBaseService, ingestion::*},
        llm::LlmRuntimeService,
    },
    config::KnowledgeIngestionConfig,
};
fn fast() -> KnowledgeIngestionConfig {
    KnowledgeIngestionConfig {
        parse_capacity: 1,
        embedding_capacity: 1,
        lease_ms: 600,
        poll_ms: 10,
        max_attempts: 3,
        retry_backoff_ms: 10,
        ..Default::default()
    }
}
fn reopen(f: &Fixture) -> KnowledgeBaseService {
    let llm = LlmRuntimeService::new(Arc::new(EmbeddingResolver), Arc::new(f.client.clone()));
    KnowledgeBaseService::new(
        SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap(),
        tron_backend::application::rag::RagService::new(
            Arc::new(std::sync::Mutex::new(tron_backend::adapters::outbound::persistence::sqlite::SqliteRagRepository::connect(&f.state.config.database_url).unwrap())),
            tron_backend::application::rag::DefaultRagRuntime::new(llm.clone(), f.state.llm_router.clone()),
        ),
        f.state.resources.clone(),
        f.state.agent_management.clone(),
        llm,
    )
    .with_ingestion_config(fast())
}
async fn wait_document(
    service: &KnowledgeBaseService,
    kb: &str,
    doc: &str,
    status: &str,
) -> tron_backend::application::knowledge_base::KnowledgeBaseDocument {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let document = service
                .detail(USER, kb)
                .unwrap()
                .documents
                .into_iter()
                .find(|d| d.id == doc)
                .unwrap();
            if document.status == status {
                break document;
            }
            assert_ne!(document.status, "FAILED", "{:?}", document.error);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn repeated_upload_and_clicks_have_one_document_and_live_task_with_scoped_request_replay() {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Duplicate protection", "").unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    let first = service
        .upload_with_request(
            USER,
            &kb.id,
            "guide.md",
            b"orchid durable",
            Some("request-a"),
        )
        .unwrap();
    let replay = service
        .upload_with_request(
            USER,
            &kb.id,
            "guide.md",
            b"orchid durable",
            Some("request-a"),
        )
        .unwrap();
    let duplicate = service
        .upload_and_start_index(USER, &kb.id, "guide.md", b"orchid durable")
        .unwrap();
    assert_eq!(first.id, replay.id);
    assert_eq!(first.id, duplicate.id);
    assert!(
        service
            .upload_with_request(
                USER,
                &kb.id,
                "guide.md",
                b"different bytes",
                Some("request-a")
            )
            .is_err()
    );
    for _ in 0..5 {
        service.start_index(USER, &kb.id, &first.id).unwrap();
    }
    assert_eq!(service.detail(USER, &kb.id).unwrap().documents.len(), 1);
    assert_eq!(
        service
            .ingestion_tasks(USER, &kb.id, &first.id)
            .unwrap()
            .len(),
        1
    );
    assert!(
        service
            .ingestion_tasks("OTHER-USER", &kb.id, &first.id)
            .is_err()
    );
    service.stop_ingestion_workers();
}
#[tokio::test]
async fn completed_parses_release_capacity_while_embedding_is_blocked_and_cancel_fences_publication()
 {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Independent capacities", "").unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    let mut documents = Vec::new();
    for n in 0..3 {
        documents.push(
            service
                .upload_and_start_index(USER, &kb.id, &format!("guide-{n}.md"), b"orchid staged")
                .unwrap(),
        );
    }
    wait_document(&service, &kb.id, &documents[0].id, "EMBEDDING").await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let tasks: Vec<_> = documents
                .iter()
                .map(|d| {
                    service
                        .ingestion_tasks(USER, &kb.id, &d.id)
                        .unwrap()
                        .remove(0)
                })
                .collect();
            if tasks.iter().all(|t| t.stage == IngestionStage::Embedding) {
                assert_eq!(tasks.iter().filter(|t| t.state == "RUNNING").count(), 1);
                assert_eq!(tasks.iter().filter(|t| t.state == "QUEUED").count(), 2);
                assert!(tasks.iter().all(|t| t.chunks_resource_code.is_some()));
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    for doc in &documents {
        let cancelled = service.cancel_index(USER, &kb.id, &doc.id).await.unwrap();
        assert_eq!(cancelled.status, "CANCELLED");
        assert!(cancelled.active_source_code.is_none());
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        service
            .detail(USER, &kb.id)
            .unwrap()
            .documents
            .iter()
            .all(|d| d.status == "CANCELLED" && d.index_revision == 0)
    );
    service.stop_ingestion_workers();
}
#[tokio::test]
async fn cancelled_rebuild_keeps_old_whole_index_and_manual_retry_uses_saved_chunks() {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Publication", "").unwrap();
    let doc = service
        .upload_and_index(USER, &kb.id, "guide.md", b"orchid published")
        .await
        .unwrap();
    service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    service.start_index(USER, &kb.id, &doc.id).unwrap();
    wait_document(&service, &kb.id, &doc.id, "EMBEDDING").await;
    let cancelled = service.cancel_index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(cancelled.active_source_code, doc.active_source_code);
    // Full-text remains available without letting the blocked query embed stall this test.
    f.client.blocking.store(false, Ordering::SeqCst);
    let hits = service
        .search_for_agent(USER, WORKSPACE, &f.agent, "orchid", 1200)
        .await
        .unwrap()
        .hits;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].source_code, doc.active_source_code.clone().unwrap());
    let task = service
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    let checkpoint = task.chunks_resource_code.clone();
    service.retry_index(USER, &kb.id, &doc.id).unwrap();
    let ready = wait_document(&service, &kb.id, &doc.id, "READY").await;
    assert_eq!(ready.index_revision, 2);
    assert_ne!(ready.active_source_code, doc.active_source_code);
    let after = service
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    assert_eq!(after.id, task.id);
    assert_eq!(
        ready.active_source_code.as_deref(),
        Some(after.generation_source_code().as_str())
    );
    assert_ne!(
        after.generation_source_code(),
        task.generation_source_code()
    );
    assert!(checkpoint.is_some());
    assert!(after.chunks_resource_code.is_none());
    service.stop_ingestion_workers();
}
#[tokio::test]
async fn reopening_database_preserves_live_lease_and_resumes_expired_embedding_without_reparse() {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Restart", "").unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    let doc = service
        .upload_and_start_index(USER, &kb.id, "guide.md", b"orchid checkpoint")
        .unwrap();
    wait_document(&service, &kb.id, &doc.id, "EMBEDDING").await;
    let before = service
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    let repository = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    assert_eq!(
        repository.document(&kb.id, &doc.id).unwrap().status,
        "EMBEDDING"
    );
    service.stop_ingestion_workers();
    f.client.blocking.store(false, Ordering::SeqCst);
    let resumed = reopen(&f);
    resumed.start_ingestion_workers().unwrap();
    let ready = wait_document(&resumed, &kb.id, &doc.id, "READY").await;
    assert_eq!(ready.index_revision, 1);
    let after = resumed
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    assert_eq!(after.id, before.id);
    assert!(after.attempt > before.attempt);
    assert!(before.chunks_resource_code.is_some());
    assert!(after.chunks_resource_code.is_none());
    resumed.stop_ingestion_workers();
}
#[tokio::test]
async fn automatic_retry_backoff_is_bounded_and_partial_embedding_never_publishes() {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Retry limit", "").unwrap();
    f.client.failing.store(true, Ordering::SeqCst);
    let doc = service
        .upload_and_start_index(USER, &kb.id, "guide.md", b"orchid fail")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let task = service
                .ingestion_tasks(USER, &kb.id, &doc.id)
                .unwrap()
                .remove(0);
            if task.state == "FAILED" {
                assert_eq!(task.failures, 3);
                assert_eq!(task.attempt, 4);
                assert!(task.chunks_resource_code.is_some());
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let doc = service.detail(USER, &kb.id).unwrap().documents.remove(0);
    assert!(doc.active_source_code.is_none());
    assert_eq!(doc.index_revision, 0);
    f.client.failing.store(false, Ordering::SeqCst);
    service.retry_index(USER, &kb.id, &doc.id).unwrap();
    wait_document(&service, &kb.id, &doc.id, "READY").await;
    service.stop_ingestion_workers();
}
#[test]
fn queue_and_upload_initialization_is_idempotent_on_reconnect() {
    let f = Fixture::new();
    let one = SqliteKnowledgeIngestionQueue::connect(&f.state.config.database_url).unwrap();
    let two = SqliteKnowledgeIngestionQueue::connect(&f.state.config.database_url).unwrap();
    assert!(!one.pending(IngestionStage::Parsing, 0).unwrap());
    assert!(!two.pending(IngestionStage::Embedding, 0).unwrap());
}

#[tokio::test]
async fn embedding_batch_restart_reuses_each_completed_batch_checkpoint() {
    use tron_backend::application::knowledge_base::local_documents::{ChunkConfig, ChunkMode};
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Vector checkpoints", "").unwrap();
    let content = format!("# Guide\n{}", "orchid durable fact. ".repeat(400));
    let doc = service
        .upload(USER, &kb.id, "guide.md", content.as_bytes())
        .unwrap();
    service
        .configure(
            USER,
            &kb.id,
            &doc.id,
            Some(ChunkConfig {
                mode: ChunkMode::Fixed,
                max_chars: 200,
                overlap_chars: 0,
                heading_depth: 2,
            }),
        )
        .unwrap();
    f.client.block_after_calls.store(1, Ordering::SeqCst);
    service.start_index(USER, &kb.id, &doc.id).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while service.ingestion_tasks(USER, &kb.id, &doc.id).unwrap()[0]
            .embedding_resource_code
            .is_none()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(f.client.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        service.ingestion_tasks(USER, &kb.id, &doc.id).unwrap()[0].completed_units,
        32
    );
    service.stop_ingestion_workers();
    f.client
        .block_after_calls
        .store(usize::MAX, Ordering::SeqCst);
    let resumed = reopen(&f);
    resumed.start_ingestion_workers().unwrap();
    let ready = wait_document(&resumed, &kb.id, &doc.id, "READY").await;
    assert!(ready.chunk_count > 32);
    assert_eq!(
        f.client.calls.load(Ordering::SeqCst),
        ready.chunk_count.div_ceil(32)
    );
    resumed.stop_ingestion_workers();
}

#[tokio::test]
async fn publishing_restart_reuses_vectors_and_atomically_retires_old_generation() {
    use serde_json::json;
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Publish recovery", "").unwrap();
    let doc = service
        .upload_and_index(USER, &kb.id, "guide.md", b"orchid complete publication")
        .await
        .unwrap();
    let initial = service
        .ingestion_tasks(USER, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    let q = SqliteKnowledgeIngestionQueue::connect(&f.state.config.database_url).unwrap();
    let task = q
        .enqueue(IngestionTask::queued(
            "recovered-publication".into(),
            kb.id.clone(),
            doc.id.clone(),
            USER.into(),
            "PLATFORM".into(),
            doc.revision,
            doc.resource_revision,
            initial.input_config_hash.clone(),
            initial.configuration.clone(),
        ))
        .unwrap();
    let claim = |stage| IngestionClaim {
        task_id: Some(task.id.clone()),
        stage,
        owner: "crashed".into(),
        now_ms: 1,
        lease_ms: 100,
        global_capacity: 1,
        embedding_resource_group: None,
        parse_endpoints: vec![ParseEndpointCapacity {
            supported_tiers: vec![],
            endpoint_id: "local-markdown".into(),
            capacity: 1,
            resource_group: "local-markdown".into(),
            resource_group_capacity: 1,
            weight: 1,
        }],
    };
    let mut parse = q.claim(claim(IngestionStage::Parsing)).unwrap().unwrap();
    let owned = |t: &IngestionTask| IngestionLease {
        task_id: t.id.clone(),
        attempt: t.attempt,
        owner: t.lease_owner.clone().unwrap(),
    };
    parse.stage = IngestionStage::Chunking;
    q.checkpoint(&parse, &owned(&parse), 2).unwrap();
    // A completed generation no longer retains checkpoints. Create the durable
    // pre-publication snapshot that a crashed rebuild would have written.
    let repository = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let current = repository.document(&kb.id, &doc.id).unwrap();
    let preview = service.preview(USER, &kb.id, &doc.id).await.unwrap();
    let chunks = checkpoint_resource(
        &f,
        &current,
        "ingestion-chunks.json",
        &json!({"document":current,"chunks":preview.chunks,"warnings":[]}),
    );
    parse.parsed_resource_code = Some(chunks.clone());
    parse.chunks_resource_code = Some(chunks);
    q.wait_for_embedding(&parse, &owned(&parse), 2).unwrap();
    let mut c = claim(IngestionStage::Embedding);
    c.now_ms = 3;
    let mut publish = q.claim(c).unwrap().unwrap();
    publish.stage = IngestionStage::Publishing;
    let mut vector = vec![0.0f32; 1024];
    vector[0] = 1.0;
    publish.embedding_resource_code = Some(checkpoint_resource(
        &f,
        &current,
        "ingestion-embeddings.json",
        &json!({"vectors":[vector],"vectorResources":[],"providerModel":null}),
    ));
    q.checkpoint(&publish, &owned(&publish), 4).unwrap();
    assert_eq!(
        service.detail(USER, &kb.id).unwrap().documents[0].active_source_code,
        doc.active_source_code
    );
    let before = f.client.calls.load(Ordering::SeqCst);
    let resumed = reopen(&f);
    resumed.start_ingestion_workers().unwrap();
    let ready = wait_document(&resumed, &kb.id, &doc.id, "READY").await;
    assert_eq!(f.client.calls.load(Ordering::SeqCst), before);
    assert_eq!(ready.index_revision, 2);
    assert_eq!(q.get(&task.id).unwrap().state, "READY");
    let completed = q.get(&task.id).unwrap();
    assert_eq!(
        ready.active_source_code.as_deref(),
        Some(completed.generation_source_code().as_str())
    );
    assert_ne!(
        completed.generation_source_code(),
        publish.generation_source_code()
    );
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    tron_backend::shared::sqlite_extensions::default_sqlite_extension_context()
        .prepare_connection(&conn)
        .unwrap();
    let table = format!("\"rag_{}_units\"", kb.id);
    let active: usize = conn
        .query_row(
            &format!("SELECT count(*) FROM {table} WHERE source_status='ACTIVE'"),
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(active, ready.chunk_count);
    assert_eq!(
        conn.query_row(
            &format!("SELECT count(*) FROM {table} WHERE source_code=?1"),
            [doc.active_source_code.unwrap()],
            |r| r.get::<_, usize>(0)
        )
        .unwrap(),
        0
    );
    for index in [
        format!("rag_{}_units_fts", kb.id),
        format!("rag_{}_units_vec_d1024", kb.id),
    ] {
        assert_eq!(
            conn.query_row(&format!("SELECT count(*) FROM \"{index}\""), [], |r| r
                .get::<_, usize>(0))
                .unwrap(),
            ready.chunk_count
        );
    }
    let mut invalid_ready = serde_json::to_value(&ready).unwrap();
    invalid_ready["activeSourceCode"] = json!("late-generation");
    assert_eq!(
        q.publish(&owned(&publish), 5, &invalid_ready),
        Err(IngestionQueueError::Conflict)
    );
    resumed.stop_ingestion_workers();
}

fn checkpoint_resource(
    f: &Fixture,
    doc: &tron_backend::application::knowledge_base::KnowledgeBaseDocument,
    name: &str,
    value: &serde_json::Value,
) -> String {
    use std::io::Write;
    use tron_backend::application::resource::{ResourceCommitBinary, ResourceCreateCommand};
    use tron_backend::domain::resource::ResourceKind;
    let code = format!("KB_{}", format!("{:032x}", rand::random::<u128>()));
    let repo = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    repo.register_derived_resource(doc, "PLATFORM", &code)
        .unwrap();
    repo.mark_private(USER, "PLATFORM", &code).unwrap();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
    f.state
        .resources
        .create_file_resource(
            USER,
            ResourceCreateCommand {
                workspace_code: "PLATFORM".into(),
                resource_code: code.clone(),
                kind: ResourceKind::File,
                name: name.into(),
                description: "知识库解析附件".into(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: file.reopen().unwrap(),
                file_name: name.into(),
            }),
        )
        .unwrap();
    code
}

#[tokio::test]
async fn settings_and_task_enqueue_are_atomic_and_stale_settings_cannot_overwrite() {
    use sha2::{Digest, Sha256};
    use tron_backend::application::knowledge_base::local_documents::{ChunkConfig, ChunkMode};
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Atomic settings", "").unwrap();
    let before = service
        .upload_and_index(USER, &kb.id, "guide.md", b"orchid atomic settings")
        .await
        .unwrap();
    let repo = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let original = repo.document(&kb.id, &before.id).unwrap();
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_rebuild_task BEFORE INSERT ON knowledge_base_ingestion_tasks BEGIN SELECT RAISE(ABORT,'test enqueue failure'); END;").unwrap();
    let config = ChunkConfig {
        mode: ChunkMode::Fixed,
        max_chars: 200,
        overlap_chars: 0,
        heading_depth: 2,
    };
    assert!(
        service
            .configure_and_start_index(USER, &kb.id, &before.id, Some(config.clone()), None, None)
            .await
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(repo.document(&kb.id, &before.id).unwrap()).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    assert_eq!(
        service
            .ingestion_tasks(USER, &kb.id, &before.id)
            .unwrap()
            .len(),
        1
    );
    conn.execute_batch("DROP TRIGGER reject_rebuild_task;")
        .unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    service
        .configure_and_start_index(USER, &kb.id, &before.id, Some(config.clone()), None, None)
        .await
        .unwrap();
    let queued = repo.document(&kb.id, &before.id).unwrap();
    let task = service
        .ingestion_tasks(USER, &kb.id, &before.id)
        .unwrap()
        .remove(0);
    assert_eq!(queued.revision, original.revision + 1);
    assert_eq!(task.document_revision, queued.revision);
    assert_eq!(
        serde_json::to_value(queued.chunk_config).unwrap(),
        serde_json::to_value(&config).unwrap()
    );
    assert_eq!(
        task.configuration["chunkConfig"],
        serde_json::to_value(&config).unwrap()
    );
    service
        .cancel_index(USER, &kb.id, &before.id)
        .await
        .unwrap();
    service.stop_ingestion_workers();
    let mut stale = repo.document(&kb.id, &before.id).unwrap();
    stale.chunk_config = None;
    let mut config_snapshot = task.configuration;
    config_snapshot["chunkConfig"] = serde_json::to_value(&kb.default_chunk_config).unwrap();
    let hash = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "{}:{}:{config_snapshot}",
                stale.resource_code, stale.resource_revision
            )
            .as_bytes()
        )
    );
    let proposal = IngestionTask::queued(
        "stale-configuration".into(),
        kb.id.clone(),
        stale.id.clone(),
        USER.into(),
        "PLATFORM".into(),
        stale.revision,
        stale.resource_revision,
        hash,
        config_snapshot,
    );
    service
        .configure(USER, &kb.id, &before.id, Some(config))
        .unwrap();
    let latest = repo.document(&kb.id, &before.id).unwrap();
    let queue = SqliteKnowledgeIngestionQueue::connect(&f.state.config.database_url).unwrap();
    assert_eq!(
        queue.enqueue_configured(proposal, stale).unwrap_err(),
        IngestionQueueError::Conflict
    );
    assert_eq!(
        serde_json::to_value(repo.document(&kb.id, &before.id).unwrap()).unwrap(),
        serde_json::to_value(latest).unwrap()
    );
}

#[tokio::test]
async fn successful_reparse_deletes_old_chunks_history_and_files_but_retains_live_cache_and_original()
 {
    use tron_backend::application::knowledge_base::parsing::ParsingTier;
    use tron_backend::application::resource::ResourceBinaryDownloadCommand;
    let f = Fixture::new();
    let service = f
        .service
        .clone()
        .with_document_parser(Arc::new(CleanupImageParser))
        .with_ingestion_config(fast());
    let kb = service.create(USER, "Rebuild cleanup", "").unwrap();
    let first = service
        .upload_and_index(USER, &kb.id, "guide.pdf", &pdf())
        .await
        .unwrap();
    let repo = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let old = repo.document(&kb.id, &first.id).unwrap();
    let old_cache = old.parsing_artifact.as_ref().unwrap().resource_code.clone();
    let old_image = old.parsing_artifact.as_ref().unwrap().image_resource_codes[0].clone();
    let obsolete =
        checkpoint_resource(&f, &old, "ingestion-vector.json", &serde_json::json!([1.0]));
    let (_, old_file) = f
        .state
        .resources
        .download_binary_to_file(
            USER,
            ResourceBinaryDownloadCommand {
                workspace_code: "PLATFORM".into(),
                resource_code: obsolete.clone(),
            },
        )
        .unwrap();
    let other = service
        .upload_and_index(USER, &kb.id, "other.md", b"orchid independent")
        .await
        .unwrap();
    let shared = checkpoint_resource(&f, &old, "shared-cache.json", &serde_json::json!({}));
    repo.register_derived_resource(&other, "PLATFORM", &shared)
        .unwrap();
    service
        .configure_and_start_index(
            USER,
            &kb.id,
            &first.id,
            None,
            Some(Some(ParsingTier::Basic)),
            None,
        )
        .await
        .unwrap();
    let ready = wait_document(&service, &kb.id, &first.id, "READY").await;
    let new_cache = ready
        .parsing_artifact
        .as_ref()
        .unwrap()
        .resource_code
        .clone();
    assert_ne!(new_cache, old_cache);
    let tasks = service.ingestion_tasks(USER, &kb.id, &first.id).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].state, "READY");
    assert!(
        tasks[0].parsed_resource_code.is_none()
            && tasks[0].chunks_resource_code.is_none()
            && tasks[0].embedding_resource_code.is_none()
    );
    assert_eq!(
        repo.derived_resources(&first.id).unwrap(),
        ready.derived_resource_codes
    );
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    tron_backend::shared::sqlite_extensions::default_sqlite_extension_context()
        .prepare_connection(&conn)
        .unwrap();
    let table = format!("rag_{}_units", kb.id);
    assert_eq!(
        conn.query_row(
            &format!("SELECT count(*) FROM \"{table}\" WHERE source_code=?1"),
            [first.active_source_code.unwrap()],
            |r| r.get::<_, usize>(0)
        )
        .unwrap(),
        0
    );
    for index in [
        table,
        format!("rag_{}_units_fts", kb.id),
        format!("rag_{}_units_vec_d1024", kb.id),
    ] {
        assert_eq!(
            conn.query_row(&format!("SELECT count(*) FROM \"{index}\""), [], |r| r
                .get::<_, usize>(0))
                .unwrap(),
            ready.chunk_count + other.chunk_count
        );
    }
    for code in [&obsolete, &old_cache, &old_image] {
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM resource_files WHERE workspace_code=?1 AND resource_code=?2",
                rusqlite::params!["PLATFORM", code],
                |r| r.get::<_, usize>(0)
            )
            .unwrap(),
            0
        );
        for table in ["resource_code_registry", "knowledge_base_resource_privacy"] {
            assert_eq!(
                conn.query_row(
                    &format!(
                        "SELECT count(*) FROM {table} WHERE workspace_code=?1 AND resource_code=?2"
                    ),
                    rusqlite::params!["PLATFORM", code],
                    |r| r.get::<_, usize>(0)
                )
                .unwrap(),
                0
            );
        }
    }
    // Physical removal follows commit; wait for the worker's post-commit cleanup.
    tokio::time::timeout(Duration::from_secs(5), async {
        while f
            .state
            .resources
            .download_binary_storage(&old_file, &mut Vec::new())
            .is_ok()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let current_image = &ready
        .parsing_artifact
        .as_ref()
        .unwrap()
        .image_resource_codes[0];
    assert_ne!(current_image, &old_image);
    for code in [&ready.resource_code, &new_cache, current_image] {
        f.state
            .resources
            .download_binary_to_file(
                USER,
                ResourceBinaryDownloadCommand {
                    workspace_code: "PLATFORM".into(),
                    resource_code: code.clone(),
                },
            )
            .unwrap();
    }
    service.preview(USER, &kb.id, &ready.id).await.unwrap();
    assert_eq!(
        repo.document(&kb.id, &ready.id)
            .unwrap()
            .parsing_artifact
            .unwrap()
            .resource_code,
        new_cache
    );
    assert!(repo.derived_resources(&other.id).unwrap().contains(&shared));
    f.state
        .resources
        .download_binary_to_file(
            USER,
            ResourceBinaryDownloadCommand {
                workspace_code: "PLATFORM".into(),
                resource_code: shared,
            },
        )
        .unwrap();
    service.stop_ingestion_workers();
}

#[tokio::test]
async fn publication_cleanup_failure_rolls_back_old_index_and_can_retry() {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, "Publication rollback", "").unwrap();
    let first = service
        .upload_and_index(USER, &kb.id, "guide.md", b"orchid keep active")
        .await
        .unwrap();
    let repo = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let before = repo.document(&kb.id, &first.id).unwrap();
    let obsolete = checkpoint_resource(&f, &before, "unused.json", &serde_json::json!({}));
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    conn.execute_batch(&format!("CREATE TRIGGER reject_resource_cleanup BEFORE DELETE ON resource_files WHEN OLD.resource_code='{obsolete}' BEGIN SELECT RAISE(ABORT,'test cleanup failure'); END;")).unwrap();
    let failed = service.index(USER, &kb.id, &first.id).await.unwrap();
    assert_eq!(failed.status, "FAILED");
    assert_eq!(failed.active_source_code, before.active_source_code);
    assert_eq!(failed.index_revision, before.index_revision);
    tron_backend::shared::sqlite_extensions::default_sqlite_extension_context()
        .prepare_connection(&conn)
        .unwrap();
    let old_count = conn.query_row(&format!("SELECT count(*) FROM \"rag_{}_units\" WHERE source_code=?1 AND source_status='ACTIVE'",kb.id),[before.active_source_code.unwrap()],|r|r.get::<_,usize>(0)).unwrap();
    assert_eq!(old_count, before.chunk_count);
    assert_eq!(
        service
            .ingestion_tasks(USER, &kb.id, &first.id)
            .unwrap()
            .len(),
        2
    );
    assert!(
        repo.derived_resources(&first.id)
            .unwrap()
            .contains(&obsolete)
    );
    conn.execute_batch("DROP TRIGGER reject_resource_cleanup;")
        .unwrap();
    service.retry_index(USER, &kb.id, &first.id).unwrap();
    let ready = wait_document(&service, &kb.id, &first.id, "READY").await;
    assert_eq!(ready.index_revision, before.index_revision + 1);
    assert_eq!(
        service
            .ingestion_tasks(USER, &kb.id, &first.id)
            .unwrap()
            .len(),
        1
    );
    assert!(
        !repo
            .derived_resources(&first.id)
            .unwrap()
            .contains(&obsolete)
    );
    for table in [
        format!("rag_{}_units", kb.id),
        format!("rag_{}_units_fts", kb.id),
        format!("rag_{}_units_vec_d1024", kb.id),
    ] {
        assert_eq!(
            conn.query_row(&format!("SELECT count(*) FROM \"{table}\""), [], |r| r
                .get::<_, usize>(0))
                .unwrap(),
            ready.chunk_count
        );
    }
    service.stop_ingestion_workers();
}

// Synthetic image/resource lifecycle fixture; this does not claim OCR accuracy.
struct CleanupImageParser;
#[async_trait::async_trait]
impl tron_backend::application::knowledge_base::parsing::KnowledgeDocumentParser
    for CleanupImageParser
{
    fn default_tier(&self) -> tron_backend::application::knowledge_base::parsing::ParsingTier {
        Default::default()
    }
    async fn capabilities(
        &self,
    ) -> Result<
        tron_backend::application::knowledge_base::parsing::ParsingCapabilities,
        tron_backend::application::knowledge_base::KnowledgeBaseError,
    > {
        use tron_backend::application::knowledge_base::parsing::*;
        Ok(ParsingCapabilities {
            default_tier: ParsingTier::Standard,
            default_ocr_mode: OcrMode::Auto,
            available_tiers: vec![ParsingTier::Standard, ParsingTier::Basic],
            parser_version: "4.0.10-fixture".into(),
            office_tier: ParsingTier::Flash,
        })
    }
    async fn parse(
        &self,
        _: &str,
        _: &[u8],
        tier: Option<tron_backend::application::knowledge_base::parsing::ParsingTier>,
    ) -> Result<
        tron_backend::application::knowledge_base::parsing::StructuredParse,
        tron_backend::application::knowledge_base::KnowledgeBaseError,
    > {
        use serde_json::json;
        use std::io::{Cursor, Write};
        let tier = tier.unwrap_or_default();
        let middle = json!({"schema":"docvortex.middle","schema_version":"2.0","metadata":{"file_suffix":"pdf"},"extensions":{"mineru":{"tier":tier}},"pages":[{"page_idx":0,"blocks":[{"index":0,"type":"text","content":"orchid PDF evidence"},{"index":1,"type":"image","content":[{"index":2,"type":"image_body","image_path":"images/figure.jpg","content":""},{"index":3,"type":"image_caption","content":"orchid diagram"}]}]}]});
        let mut image = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 2)
            .write_to(&mut image, image::ImageFormat::Jpeg)
            .unwrap();
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in [
            ("middle_json.json", serde_json::to_vec(&middle).unwrap()),
            ("structured_content.json", b"{}".to_vec()),
            ("markdown.md", b"orchid PDF evidence".to_vec()),
            ("images/figure.jpg", image.into_inner()),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&bytes).unwrap();
        }
        let archive = zip.finish().unwrap().into_inner();
        tron_backend::adapters::outbound::mineru_documents::decode_archive(
            &archive,
            tier,
            "4.0.10-fixture",
            "cleanup-fixture",
        )
    }
}
