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
    KnowledgeBaseService::new(
        SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap(),
        f.state.rag.clone(),
        f.state.resources.clone(),
        f.state.workspaces.clone(),
        f.state.agent_management.clone(),
        LlmRuntimeService::new(Arc::new(EmbeddingResolver), Arc::new(f.client.clone())),
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
                .detail(USER, WORKSPACE, kb)
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
    let kb = service
        .create(USER, WORKSPACE, "Duplicate protection", "")
        .unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    let first = service
        .upload_with_request(
            USER,
            WORKSPACE,
            &kb.id,
            "guide.md",
            b"orchid durable",
            Some("request-a"),
        )
        .unwrap();
    let replay = service
        .upload_with_request(
            USER,
            WORKSPACE,
            &kb.id,
            "guide.md",
            b"orchid durable",
            Some("request-a"),
        )
        .unwrap();
    let duplicate = service
        .upload_and_start_index(USER, WORKSPACE, &kb.id, "guide.md", b"orchid durable")
        .unwrap();
    assert_eq!(first.id, replay.id);
    assert_eq!(first.id, duplicate.id);
    assert!(
        service
            .upload_with_request(
                USER,
                WORKSPACE,
                &kb.id,
                "guide.md",
                b"different bytes",
                Some("request-a")
            )
            .is_err()
    );
    for _ in 0..5 {
        service
            .start_index(USER, WORKSPACE, &kb.id, &first.id)
            .unwrap();
    }
    assert_eq!(
        service
            .detail(USER, WORKSPACE, &kb.id)
            .unwrap()
            .documents
            .len(),
        1
    );
    assert_eq!(
        service
            .ingestion_tasks(USER, WORKSPACE, &kb.id, &first.id)
            .unwrap()
            .len(),
        1
    );
    assert!(
        service
            .ingestion_tasks("OTHER-USER", WORKSPACE, &kb.id, &first.id)
            .is_err()
    );
    service.stop_ingestion_workers();
}
#[tokio::test]
async fn completed_parses_release_capacity_while_embedding_is_blocked_and_cancel_fences_publication()
 {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service
        .create(USER, WORKSPACE, "Independent capacities", "")
        .unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    let mut documents = Vec::new();
    for n in 0..3 {
        documents.push(
            service
                .upload_and_start_index(
                    USER,
                    WORKSPACE,
                    &kb.id,
                    &format!("guide-{n}.md"),
                    b"orchid staged",
                )
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
                        .ingestion_tasks(USER, WORKSPACE, &kb.id, &d.id)
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
        let cancelled = service
            .cancel_index(USER, WORKSPACE, &kb.id, &doc.id)
            .await
            .unwrap();
        assert_eq!(cancelled.status, "CANCELLED");
        assert!(cancelled.active_source_code.is_none());
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        service
            .detail(USER, WORKSPACE, &kb.id)
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
    let kb = service.create(USER, WORKSPACE, "Publication", "").unwrap();
    let doc = service
        .upload_and_index(USER, WORKSPACE, &kb.id, "guide.md", b"orchid published")
        .await
        .unwrap();
    service
        .set_grants(USER, WORKSPACE, &kb.id, vec![f.agent.clone()])
        .unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    service
        .start_index(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap();
    wait_document(&service, &kb.id, &doc.id, "EMBEDDING").await;
    let cancelled = service
        .cancel_index(USER, WORKSPACE, &kb.id, &doc.id)
        .await
        .unwrap();
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
        .ingestion_tasks(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    let checkpoint = task.chunks_resource_code.clone();
    service
        .retry_index(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap();
    let ready = wait_document(&service, &kb.id, &doc.id, "READY").await;
    assert_eq!(ready.index_revision, 2);
    assert_ne!(ready.active_source_code, doc.active_source_code);
    let after = service
        .ingestion_tasks(USER, WORKSPACE, &kb.id, &doc.id)
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
    assert_eq!(after.chunks_resource_code, checkpoint);
    service.stop_ingestion_workers();
}
#[tokio::test]
async fn reopening_database_preserves_live_lease_and_resumes_expired_embedding_without_reparse() {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, WORKSPACE, "Restart", "").unwrap();
    f.client.blocking.store(true, Ordering::SeqCst);
    let doc = service
        .upload_and_start_index(USER, WORKSPACE, &kb.id, "guide.md", b"orchid checkpoint")
        .unwrap();
    wait_document(&service, &kb.id, &doc.id, "EMBEDDING").await;
    let before = service
        .ingestion_tasks(USER, WORKSPACE, &kb.id, &doc.id)
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
        .ingestion_tasks(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    assert_eq!(after.id, before.id);
    assert!(after.attempt > before.attempt);
    assert_eq!(after.chunks_resource_code, before.chunks_resource_code);
    resumed.stop_ingestion_workers();
}
#[tokio::test]
async fn automatic_retry_backoff_is_bounded_and_partial_embedding_never_publishes() {
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service.create(USER, WORKSPACE, "Retry limit", "").unwrap();
    f.client.failing.store(true, Ordering::SeqCst);
    let doc = service
        .upload_and_start_index(USER, WORKSPACE, &kb.id, "guide.md", b"orchid fail")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let task = service
                .ingestion_tasks(USER, WORKSPACE, &kb.id, &doc.id)
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
    let doc = service
        .detail(USER, WORKSPACE, &kb.id)
        .unwrap()
        .documents
        .remove(0);
    assert!(doc.active_source_code.is_none());
    assert_eq!(doc.index_revision, 0);
    f.client.failing.store(false, Ordering::SeqCst);
    service
        .retry_index(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap();
    wait_document(&service, &kb.id, &doc.id, "READY").await;
    service.stop_ingestion_workers();
}
#[test]
fn queue_and_upload_migration_are_additive_and_reconnect_is_idempotent() {
    let f = Fixture::new();
    let one = SqliteKnowledgeIngestionQueue::connect(&f.state.config.database_url).unwrap();
    let two = SqliteKnowledgeIngestionQueue::connect(&f.state.config.database_url).unwrap();
    assert!(!one.pending(IngestionStage::Parsing, 0).unwrap());
    assert!(!two.pending(IngestionStage::Embedding, 0).unwrap());
}

#[tokio::test]
async fn restart_reuses_each_completed_vector_checkpoint() {
    use tron_backend::application::knowledge_base::local_documents::{ChunkConfig, ChunkMode};
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service
        .create(USER, WORKSPACE, "Vector checkpoints", "")
        .unwrap();
    let content = format!("# Guide\n{}", "orchid durable fact. ".repeat(80));
    let doc = service
        .upload(USER, WORKSPACE, &kb.id, "guide.md", content.as_bytes())
        .unwrap();
    service
        .configure(
            USER,
            WORKSPACE,
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
    service
        .start_index(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while service
            .ingestion_tasks(USER, WORKSPACE, &kb.id, &doc.id)
            .unwrap()[0]
            .embedding_resource_code
            .is_none()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(f.client.calls.load(Ordering::SeqCst), 1);
    service.stop_ingestion_workers();
    f.client
        .block_after_calls
        .store(usize::MAX, Ordering::SeqCst);
    let resumed = reopen(&f);
    resumed.start_ingestion_workers().unwrap();
    let ready = wait_document(&resumed, &kb.id, &doc.id, "READY").await;
    assert!(ready.chunk_count > 1);
    assert_eq!(f.client.calls.load(Ordering::SeqCst), ready.chunk_count);
    resumed.stop_ingestion_workers();
}

#[tokio::test]
async fn publishing_restart_reuses_vectors_and_atomically_retires_old_generation() {
    use serde_json::json;
    let f = Fixture::new();
    let service = f.service.clone().with_ingestion_config(fast());
    let kb = service
        .create(USER, WORKSPACE, "Publish recovery", "")
        .unwrap();
    let doc = service
        .upload_and_index(
            USER,
            WORKSPACE,
            &kb.id,
            "guide.md",
            b"orchid complete publication",
        )
        .await
        .unwrap();
    let initial = service
        .ingestion_tasks(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap()
        .remove(0);
    let q = SqliteKnowledgeIngestionQueue::connect(&f.state.config.database_url).unwrap();
    let task = q
        .enqueue(IngestionTask::queued(
            "recovered-publication".into(),
            kb.id.clone(),
            doc.id.clone(),
            USER.into(),
            WORKSPACE.into(),
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
    parse.parsed_resource_code = initial.parsed_resource_code.clone();
    parse.chunks_resource_code = initial.chunks_resource_code.clone();
    q.wait_for_embedding(&parse, &owned(&parse), 2).unwrap();
    let mut c = claim(IngestionStage::Embedding);
    c.now_ms = 3;
    let mut publish = q.claim(c).unwrap().unwrap();
    publish.stage = IngestionStage::Publishing;
    publish.embedding_resource_code = initial.embedding_resource_code;
    q.checkpoint(&publish, &owned(&publish), 4).unwrap();
    assert_eq!(
        service.detail(USER, WORKSPACE, &kb.id).unwrap().documents[0].active_source_code,
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
            &format!(
                "SELECT count(*) FROM {table} WHERE source_code=?1 AND source_status='ACTIVE'"
            ),
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
