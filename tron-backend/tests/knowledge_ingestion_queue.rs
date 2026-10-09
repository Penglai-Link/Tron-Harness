use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteKnowledgeIngestionQueue,
    application::knowledge_base::ingestion::*,
};

fn fixture() -> (tempfile::TempDir, SqliteKnowledgeIngestionQueue) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("queue.sqlite");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE knowledge_base_documents(id TEXT PRIMARY KEY,knowledge_base_id TEXT,payload_json TEXT,revision INTEGER NOT NULL);").unwrap();
    conn.execute_batch("CREATE TABLE rag_kb_units(source_code TEXT, source_status TEXT,status TEXT,embedding_status TEXT,embedding_dimension INTEGER,metadata TEXT,scope_code TEXT);").unwrap();
    for source in ["fully-staged", "complete"] {
        for _ in 0..12 {
            conn.execute(
                "INSERT INTO rag_kb_units VALUES(?1,'STAGING','ACTIVE','INDEXED',1024,'{}','workspace')",
                [source],
            )
            .unwrap();
        }
    }
    for id in ["a", "b", "c", "d"] {
        let doc = json!({"id":id,"resourceCode":format!("resource-{id}"),"resourceRevision":1,"revision":0,"status":"READY","activeSourceCode":format!("old-{id}")});
        conn.execute(
            "INSERT INTO knowledge_base_documents VALUES(?1,'kb',?2,0)",
            params![id, doc.to_string()],
        )
        .unwrap();
    }
    (
        dir,
        SqliteKnowledgeIngestionQueue::connect(db.to_str().unwrap()).unwrap(),
    )
}
fn queued(id: &str, hash: &str) -> IngestionTask {
    use sha2::{Digest, Sha256};
    let configuration = json!({"tier":"standard","ocrMode":"ocr","parserVersion":"4.0.10","embeddingRevision":"fixed","profile":hash});
    let identity = format!(
        "{:x}",
        Sha256::digest(format!("resource-{id}:1:{configuration}").as_bytes())
    );
    IngestionTask::queued(
        format!("task-{id}-{hash}"),
        "kb".into(),
        id.into(),
        "user".into(),
        "workspace".into(),
        0,
        1,
        identity,
        configuration,
    )
}
fn nodes() -> Vec<ParseEndpointCapacity> {
    ["node-a", "node-b"]
        .into_iter()
        .map(|id| ParseEndpointCapacity {
            supported_tiers: vec![],
            endpoint_id: id.into(),
            capacity: 1,
            resource_group: id.into(),
            resource_group_capacity: 1,
            weight: 1,
        })
        .collect()
}
fn claim(stage: IngestionStage, owner: &str, now: i64, capacity: usize) -> IngestionClaim {
    IngestionClaim {
        task_id: None,
        stage,
        owner: owner.into(),
        now_ms: now,
        lease_ms: 100,
        global_capacity: capacity,
        embedding_resource_group: None,
        parse_endpoints: nodes(),
    }
}
fn lease(task: &IngestionTask) -> IngestionLease {
    IngestionLease {
        task_id: task.id.clone(),
        attempt: task.attempt,
        owner: task.lease_owner.clone().unwrap(),
    }
}
fn complete_parse(
    q: &SqliteKnowledgeIngestionQueue,
    mut task: IngestionTask,
    now: i64,
) -> IngestionTask {
    let l = lease(&task);
    task.stage = IngestionStage::Chunking;
    task.parsed_resource_code = Some(format!("parsed-{}", task.id));
    q.checkpoint(&task, &l, now).unwrap();
    task.chunks_resource_code = Some(format!("chunks-{}", task.id));
    q.wait_for_embedding(&task, &l, now).unwrap();
    q.get(&task.id).unwrap()
}
fn doc(dir: &tempfile::TempDir, id: &str) -> Value {
    let conn = Connection::open(dir.path().join("queue.sqlite")).unwrap();
    let raw: String = conn
        .query_row(
            "SELECT payload_json FROM knowledge_base_documents WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn smaller_files_are_claimed_first_in_both_stages_with_fifo_ties_and_unknown_sizes_last() {
    for stage in [IngestionStage::Parsing, IngestionStage::Embedding] {
        let (dir, q) = fixture();
        for (id, size) in [
            ("d", None),
            ("a", Some(4096)),
            ("b", Some(1024)),
            ("c", Some(1024)),
        ] {
            let mut task = queued(id, "size-order");
            task.file_size_bytes = size;
            let submitted = q.enqueue(task).unwrap();
            assert_eq!(q.get(&submitted.id).unwrap().file_size_bytes, size);
            if stage == IngestionStage::Embedding {
                let mut request = claim(IngestionStage::Parsing, "parser", 1, 4);
                request.task_id = Some(submitted.id);
                let parsing = q.claim(request).unwrap().unwrap();
                complete_parse(&q, parsing, 2);
            }
        }
        // Tasks written before size tracking still deserialize and run last.
        let conn = Connection::open(dir.path().join("queue.sqlite")).unwrap();
        conn.execute("UPDATE knowledge_base_ingestion_tasks SET payload_json=json_remove(payload_json,'$.fileSizeBytes') WHERE document_id='d'", []).unwrap();
        for expected in ["b", "c", "a", "d"] {
            let task = q.claim(claim(stage, "worker", 3, 4)).unwrap().unwrap();
            assert_eq!(task.document_id, expected);
            if stage == IngestionStage::Parsing {
                complete_parse(&q, task, 4);
            }
        }
    }
}

#[test]
fn smaller_arrivals_do_not_preempt_live_leases_or_bypass_retry_backoff() {
    let (_dir, q) = fixture();
    let mut large = queued("a", "priority");
    large.file_size_bytes = Some(4096);
    q.enqueue(large).unwrap();
    let running = q
        .claim(claim(IngestionStage::Parsing, "large", 1, 1))
        .unwrap()
        .unwrap();
    let mut small = queued("b", "priority");
    small.file_size_bytes = Some(1024);
    q.enqueue(small).unwrap();
    assert!(
        q.claim(claim(IngestionStage::Parsing, "small", 2, 1))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        q.get(&running.id).unwrap().lease_owner.as_deref(),
        Some("large")
    );
    complete_parse(&q, running, 3);
    let small = q
        .claim(claim(IngestionStage::Parsing, "small", 4, 1))
        .unwrap()
        .unwrap();
    q.fail(&lease(&small), 5, "retry later", Some(50)).unwrap();
    let mut medium = queued("c", "priority");
    medium.file_size_bytes = Some(2048);
    q.enqueue(medium).unwrap();
    let next = q
        .claim(claim(IngestionStage::Parsing, "medium", 6, 1))
        .unwrap()
        .unwrap();
    assert_eq!(next.document_id, "c");
    complete_parse(&q, next, 7);
    assert!(
        q.claim(claim(IngestionStage::Parsing, "early", 8, 1))
            .unwrap()
            .is_none()
    );
    let retry = q
        .claim(claim(IngestionStage::Parsing, "retry", 50, 1))
        .unwrap()
        .unwrap();
    assert_eq!(retry.id, small.id);
    assert_eq!(retry.file_size_bytes, Some(1024));
}
#[test]
fn duplicate_enqueue_is_one_task_and_one_generation() {
    let (dir, q) = fixture();
    let one = q.enqueue(queued("a", "same")).unwrap();
    let mut duplicate = queued("a", "same");
    duplicate.id = "different-click".into();
    let two = q.enqueue(duplicate).unwrap();
    assert_eq!(one.id, two.id);
    assert_eq!(doc(&dir, "a")["revision"], 1);
    assert_eq!(
        q.enqueue(queued("a", "different")).unwrap_err(),
        IngestionQueueError::Conflict
    );
    assert_eq!(doc(&dir, "a")["activeSourceCode"], "old-a");
}

#[test]
fn parse_and_embed_caps_are_independent_and_nodes_have_capacity() {
    let (_dir, q) = fixture();
    for id in ["a", "b", "c"] {
        q.enqueue(queued(id, "fixed")).unwrap();
    }
    let first = q
        .claim(claim(IngestionStage::Parsing, "w1", 1, 2))
        .unwrap()
        .unwrap();
    let second = q
        .claim(claim(IngestionStage::Parsing, "w2", 1, 2))
        .unwrap()
        .unwrap();
    assert_ne!(first.endpoint_id, second.endpoint_id);
    assert!(
        q.claim(claim(IngestionStage::Parsing, "w3", 1, 2))
            .unwrap()
            .is_none()
    );
    complete_parse(&q, first, 2);
    let embedding = q
        .claim(claim(IngestionStage::Embedding, "e1", 3, 1))
        .unwrap()
        .unwrap();
    let third = q
        .claim(claim(IngestionStage::Parsing, "w3", 3, 2))
        .unwrap()
        .unwrap();
    assert_ne!(third.endpoint_id, second.endpoint_id);
    complete_parse(&q, second, 4);
    assert!(
        q.claim(claim(IngestionStage::Embedding, "e2", 4, 1))
            .unwrap()
            .is_none()
    );
    q.fail(&lease(&embedding), 5, "temporary", Some(100))
        .unwrap();
    assert!(
        q.claim(claim(IngestionStage::Embedding, "e2", 6, 1))
            .unwrap()
            .is_some()
    );
}

#[test]
fn shared_hardware_group_restricts_parse_nodes() {
    let (_dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    q.enqueue(queued("b", "x")).unwrap();
    let mut c = claim(IngestionStage::Parsing, "w", 1, 4);
    for node in &mut c.parse_endpoints {
        node.resource_group = "same-gpu".into();
        node.resource_group_capacity = 1;
    }
    assert!(q.claim(c.clone()).unwrap().is_some());
    assert!(q.claim(c).unwrap().is_none());
}

#[test]
fn restart_reclaims_expired_attempt_with_same_node_and_unknown_post() {
    let (dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    let mut task = q
        .claim(claim(IngestionStage::Parsing, "old", 1, 1))
        .unwrap()
        .unwrap();
    let old = lease(&task);
    task.remote_file_id = Some("file-a".into());
    task.reconciling = true;
    q.checkpoint(&task, &old, 2).unwrap();
    drop(q);
    let reopened =
        SqliteKnowledgeIngestionQueue::connect(dir.path().join("queue.sqlite").to_str().unwrap())
            .unwrap();
    assert!(
        reopened
            .claim(claim(IngestionStage::Parsing, "new", 50, 1))
            .unwrap()
            .is_none()
    );
    let resumed = reopened
        .claim(claim(IngestionStage::Parsing, "new", 102, 1))
        .unwrap()
        .unwrap();
    assert_eq!(resumed.endpoint_id, task.endpoint_id);
    assert_eq!(resumed.remote_file_id, task.remote_file_id);
    assert!(resumed.reconciling);
    assert!(resumed.remote_job_id.is_none());
    assert_eq!(resumed.attempt, 2);
    assert_eq!(
        reopened.checkpoint(&task, &old, 103),
        Err(IngestionQueueError::Conflict)
    );
    assert_eq!(
        reopened.heartbeat(&old, 103, 100),
        Err(IngestionQueueError::Conflict)
    );
}

#[test]
fn remote_job_and_file_do_not_move_to_another_node() {
    let (_dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    let mut task = q
        .claim(claim(IngestionStage::Parsing, "old", 1, 1))
        .unwrap()
        .unwrap();
    task.remote_file_id = Some("f".into());
    task.remote_job_id = Some("j".into());
    q.checkpoint(&task, &lease(&task), 2).unwrap();
    let mut c = claim(IngestionStage::Parsing, "new", 102, 1);
    c.parse_endpoints
        .retain(|n| Some(&n.endpoint_id) != task.endpoint_id.as_ref());
    assert!(q.claim(c).unwrap().is_none());
    let resumed = q
        .claim(claim(IngestionStage::Parsing, "new", 102, 1))
        .unwrap()
        .unwrap();
    assert_eq!(resumed.remote_job_id.as_deref(), Some("j"));
    assert_eq!(resumed.endpoint_id, task.endpoint_id);
}

#[test]
fn cancel_fences_every_old_callback_and_retry_uses_new_attempt() {
    let (dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    let mut task = q
        .claim(claim(IngestionStage::Parsing, "old", 1, 1))
        .unwrap()
        .unwrap();
    task.remote_job_id = Some("cancel-this-job".into());
    q.checkpoint(&task, &lease(&task), 2).unwrap();
    let cancelled = q.cancel(&task.id).unwrap();
    assert_eq!(cancelled.remote_job_id, task.remote_job_id);
    assert_eq!(doc(&dir, "a")["activeSourceCode"], "old-a");
    assert_eq!(
        q.fail(&lease(&task), 3, "late", None),
        Err(IngestionQueueError::Conflict)
    );
    assert_eq!(
        q.checkpoint(&task, &lease(&task), 3),
        Err(IngestionQueueError::Conflict)
    );
    let retry = q.retry(&task.id, 4).unwrap();
    assert_eq!(retry.remote_job_id, task.remote_job_id);
    assert!(
        q.claim(claim(IngestionStage::Parsing, "new", 4, 1))
            .unwrap()
            .is_none()
    );
    q.complete_cancellation(&retry, retry.remote_job_id.clone())
        .unwrap();
    let restarted = q
        .claim(claim(IngestionStage::Parsing, "new", 4, 1))
        .unwrap()
        .unwrap();
    assert_eq!(restarted.attempt, 2);
    assert_eq!(
        q.complete_cancellation(&retry, retry.remote_job_id.clone()),
        Err(IngestionQueueError::Conflict)
    );
    assert_eq!(
        q.heartbeat(&lease(&task), 5, 100),
        Err(IngestionQueueError::Conflict)
    );
}

#[test]
fn durable_completed_parse_skips_parse_queue_after_restart() {
    let (dir, q) = fixture();
    let queued = q.enqueue(queued("a", "x")).unwrap();
    let parsed = q
        .claim(claim(IngestionStage::Parsing, "p", 1, 1))
        .unwrap()
        .unwrap();
    complete_parse(&q, parsed, 2);
    drop(q);
    let q =
        SqliteKnowledgeIngestionQueue::connect(dir.path().join("queue.sqlite").to_str().unwrap())
            .unwrap();
    assert!(
        q.claim(claim(IngestionStage::Parsing, "p2", 3, 1))
            .unwrap()
            .is_none()
    );
    let embedding = q
        .claim(claim(IngestionStage::Embedding, "e", 3, 1))
        .unwrap()
        .unwrap();
    assert_eq!(embedding.id, queued.id);
    assert!(embedding.parsed_resource_code.is_some());
    assert!(embedding.chunks_resource_code.is_some());
}

#[test]
fn publish_switches_pointer_and_task_in_one_transaction_and_rejects_stale_attempt() {
    let (dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    let parsed = q
        .claim(claim(IngestionStage::Parsing, "p", 1, 1))
        .unwrap()
        .unwrap();
    complete_parse(&q, parsed, 2);
    let mut embedding = q
        .claim(claim(IngestionStage::Embedding, "e", 3, 1))
        .unwrap()
        .unwrap();
    let old = lease(&embedding);
    assert_eq!(doc(&dir, "a")["activeSourceCode"], "old-a");
    embedding.stage = IngestionStage::Publishing;
    q.checkpoint(&embedding, &old, 4).unwrap();
    let mut ready = doc(&dir, "a");
    ready["activeSourceCode"] = "fully-staged".into();
    ready["chunkCount"] = 12.into();
    let newer = q
        .claim(claim(IngestionStage::Embedding, "replacement", 104, 1))
        .unwrap()
        .unwrap();
    // A live lease cannot publish another attempt's staged source either.
    assert_eq!(
        q.publish(&lease(&newer), 105, &ready),
        Err(IngestionQueueError::Conflict)
    );
    let source = newer.generation_source_code();
    Connection::open(dir.path().join("queue.sqlite"))
        .unwrap()
        .execute(
            "UPDATE rag_kb_units SET source_code=?1 WHERE source_code='fully-staged'",
            [&source],
        )
        .unwrap();
    ready["activeSourceCode"] = source.clone().into();
    assert_eq!(
        q.publish(&old, 105, &ready),
        Err(IngestionQueueError::Conflict)
    );
    assert_eq!(doc(&dir, "a")["activeSourceCode"], "old-a");
    q.publish(&lease(&newer), 105, &ready).unwrap();
    assert_eq!(doc(&dir, "a")["activeSourceCode"], source);
    assert_eq!(doc(&dir, "a")["revision"], 2);
    assert_eq!(q.get(&embedding.id).unwrap().state, "READY");
    assert!(q.cancel(&embedding.id).is_err());
}

#[test]
fn failure_keeps_active_generation_and_obeys_persistent_backoff() {
    let (dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    let task = q
        .claim(claim(IngestionStage::Parsing, "p", 1, 1))
        .unwrap()
        .unwrap();
    q.fail(&lease(&task), 2, "transient", Some(1000)).unwrap();
    assert_eq!(doc(&dir, "a")["activeSourceCode"], "old-a");
    assert!(
        q.claim(claim(IngestionStage::Parsing, "p2", 999, 1))
            .unwrap()
            .is_none()
    );
    let task = q
        .claim(claim(IngestionStage::Parsing, "p2", 1000, 1))
        .unwrap()
        .unwrap();
    q.fail(&lease(&task), 1001, "permanent", None).unwrap();
    assert_eq!(doc(&dir, "a")["status"], "FAILED");
    assert!(
        q.claim(claim(IngestionStage::Parsing, "p3", 2000, 1))
            .unwrap()
            .is_none()
    );
}

#[test]
fn stale_document_generation_does_not_block_other_work() {
    let (dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    q.enqueue(queued("b", "x")).unwrap();
    let conn = Connection::open(dir.path().join("queue.sqlite")).unwrap();
    conn.execute(
        "UPDATE knowledge_base_documents SET revision=revision+1 WHERE id='a'",
        [],
    )
    .unwrap();
    let claimed = q
        .claim(claim(IngestionStage::Parsing, "p", 1, 1))
        .unwrap()
        .unwrap();
    assert_eq!(claimed.document_id, "b");
    assert_eq!(q.get("task-a-x").unwrap().state, "CANCELLED");
}

#[test]
fn two_connections_cannot_claim_same_attempt() {
    let (dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    let q2 =
        SqliteKnowledgeIngestionQueue::connect(dir.path().join("queue.sqlite").to_str().unwrap())
            .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let b = barrier.clone();
    let one = std::thread::spawn(move || {
        b.wait();
        q.claim(claim(IngestionStage::Parsing, "one", 1, 1))
            .unwrap()
    });
    let two = std::thread::spawn(move || {
        barrier.wait();
        q2.claim(claim(IngestionStage::Parsing, "two", 1, 1))
            .unwrap()
    });
    assert_eq!(
        [one.join().unwrap(), two.join().unwrap()]
            .into_iter()
            .filter(Option::is_some)
            .count(),
        1
    );
}

#[test]
fn cancel_retry_does_not_erase_unknown_post_acceptance() {
    let (_dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    let mut task = q
        .claim(claim(IngestionStage::Parsing, "p", 1, 1))
        .unwrap()
        .unwrap();
    task.reconciling = true;
    task.remote_file_id = Some("file".into());
    q.checkpoint(&task, &lease(&task), 2).unwrap();
    q.cancel(&task.id).unwrap();
    let retried = q.retry(&task.id, 3).unwrap();
    assert!(retried.reconciling);
    assert_eq!(retried.remote_file_id.as_deref(), Some("file"));
    assert!(retried.cancellation_pending);
    assert!(
        q.claim(claim(IngestionStage::Parsing, "p2", 3, 1))
            .unwrap()
            .is_none()
    );
    let pending = q.pending_cancellations().unwrap().remove(0);
    q.complete_cancellation(&pending, Some("job".into()))
        .unwrap();
    let recovered = q
        .claim(claim(IngestionStage::Parsing, "p2", 3, 1))
        .unwrap()
        .unwrap();
    assert!(!recovered.reconciling);
    assert!(!recovered.cancellation_pending);
    assert_eq!(recovered.remote_job_id.as_deref(), Some("job"));
    assert_eq!(recovered.endpoint_id, task.endpoint_id);
}

#[test]
fn checkpoint_cannot_change_node_or_immutable_configuration() {
    let (_dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    let task = q
        .claim(claim(IngestionStage::Parsing, "p", 1, 1))
        .unwrap()
        .unwrap();
    let mut update = task.clone();
    update.endpoint_id = Some("different-node".into());
    assert_eq!(
        q.checkpoint(&update, &lease(&task), 2),
        Err(IngestionQueueError::Conflict)
    );
    update = task.clone();
    update.configuration = json!({"tier":"flash"});
    update.input_config_hash = "new".into();
    q.checkpoint(&update, &lease(&task), 2).unwrap();
    let persisted = q.get(&task.id).unwrap();
    assert_eq!(persisted.configuration, task.configuration);
    assert_eq!(persisted.input_config_hash, task.input_config_hash);
}

#[test]
fn parser_and_framework_embedding_share_hardware_capacity_without_sharing_clients() {
    let (_dir, q) = fixture();
    q.enqueue(queued("a", "x")).unwrap();
    q.enqueue(queued("b", "x")).unwrap();
    let mut parse = claim(IngestionStage::Parsing, "p", 1, 2);
    for endpoint in &mut parse.parse_endpoints {
        endpoint.resource_group = "gpu".into();
        endpoint.resource_group_capacity = 1;
    }
    let parsed = q.claim(parse.clone()).unwrap().unwrap();
    complete_parse(&q, parsed, 2);
    let mut embed = claim(IngestionStage::Embedding, "e", 3, 2);
    embed.embedding_resource_group = Some(("gpu".into(), 1));
    let embedding = q.claim(embed).unwrap().unwrap();
    parse.now_ms = 4;
    assert!(q.claim(parse.clone()).unwrap().is_none());
    q.fail(&lease(&embedding), 4, "temporary", Some(100))
        .unwrap();
    assert!(q.claim(parse).unwrap().is_some());
}

#[test]
fn explicit_rebuild_same_config_can_follow_completed_run_but_request_replay_is_stable() {
    let (dir, q) = fixture();
    let request = queued("a", "x");
    let initial = q.enqueue(request.clone()).unwrap();
    let parsed = q
        .claim(claim(IngestionStage::Parsing, "p", 1, 1))
        .unwrap()
        .unwrap();
    complete_parse(&q, parsed, 2);
    let mut task = q
        .claim(claim(IngestionStage::Embedding, "e", 3, 1))
        .unwrap()
        .unwrap();
    task.stage = IngestionStage::Publishing;
    q.checkpoint(&task, &lease(&task), 4).unwrap();
    let mut ready = doc(&dir, "a");
    let source = task.generation_source_code();
    Connection::open(dir.path().join("queue.sqlite"))
        .unwrap()
        .execute(
            "UPDATE rag_kb_units SET source_code=?1 WHERE source_code='complete'",
            [&source],
        )
        .unwrap();
    ready["activeSourceCode"] = source.clone().into();
    ready["chunkCount"] = 12.into();
    q.publish(&lease(&task), 5, &ready).unwrap();
    assert_eq!(q.enqueue(request.clone()).unwrap().id, initial.id);
    let mut rebuild = request;
    rebuild.id = "new-explicit-rebuild".into();
    rebuild.document_revision = 2;
    assert_eq!(q.enqueue(rebuild).unwrap().document_revision, 3);
    assert_eq!(doc(&dir, "a")["activeSourceCode"], source);
}

#[test]
fn embedding_batch_checkpoint_commits_manifest_and_counts_together() {
    let (_dir, queue) = fixture();
    queue.enqueue(queued("a", "x")).unwrap();
    let task = queue.claim(claim(IngestionStage::Parsing, "p", 1, 1)).unwrap().unwrap();
    let mut update = task.clone();
    update.total_units = 3;
    update.completed_units = 2;
    update.index_stage = Some("EMBEDDING".into());
    update.embedding_resource_code = Some("batch-manifest".into());
    queue.checkpoint(&update, &lease(&task), 2).unwrap();
    let saved = queue.get(&task.id).unwrap();
    assert_eq!(saved.total_units, 3);
    assert_eq!(saved.completed_units, 2);
    assert_eq!(saved.index_stage.as_deref(), Some("EMBEDDING"));
    assert_eq!(saved.embedding_resource_code.as_deref(), Some("batch-manifest"));
    update.completed_units = 4;
    update.embedding_resource_code = Some("invalid-manifest".into());
    assert_eq!(queue.checkpoint(&update, &lease(&task), 3), Err(IngestionQueueError::Conflict));
    let unchanged = queue.get(&task.id).unwrap();
    assert_eq!(unchanged.completed_units, 2);
    assert_eq!(unchanged.embedding_resource_code, saved.embedding_resource_code);
}
