use crate::application::knowledge_base::ingestion::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::Value;
use sha2::{Digest, Sha256};
mod availability;
mod publication;
mod publication_indexes;
mod publication_cleanup;

struct SqliteKnowledgeIngestionQueueSql<'a> {
    connection: &'a mut Connection,
}
#[derive(Clone)]
pub struct SqliteKnowledgeIngestionQueue {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl SqliteKnowledgeIngestionQueueSql<'_> {}

fn storage(e: impl std::fmt::Display) -> IngestionQueueError {
    IngestionQueueError::Storage(e.to_string())
}
fn encode(task: &IngestionTask) -> Result<String, IngestionQueueError> {
    serde_json::to_string(task).map_err(storage)
}
fn read(conn: &Connection, id: &str) -> Result<IngestionTask, IngestionQueueError> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM knowledge_base_ingestion_tasks WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    serde_json::from_str(&raw.ok_or(IngestionQueueError::NotFound)?).map_err(storage)
}
fn write(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    task: &IngestionTask,
) -> Result<(), IngestionQueueError> {
    tx.execute("UPDATE knowledge_base_ingestion_tasks SET state=?1,stage=?2,attempt=?3,lease_owner=?4,lease_until_ms=?5,retry_after_ms=?6,endpoint_id=?7,payload_json=?8 WHERE id=?9",
        params![task.state,format!("{:?}",task.stage),task.attempt,task.lease_owner,task.lease_until_ms,task.retry_after_ms,task.endpoint_id,encode(task)?,task.id]).map_err(storage)?;
    Ok(())
}
fn leased(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    lease: &IngestionLease,
    now: i64,
) -> Result<IngestionTask, IngestionQueueError> {
    let task = read(tx, &lease.task_id)?;
    if task.state != "RUNNING"
        || task.attempt != lease.attempt
        || task.lease_owner.as_deref() != Some(lease.owner.as_str())
        || task.lease_until_ms <= now
    {
        return Err(IngestionQueueError::Conflict);
    }
    Ok(task)
}
fn document(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    task: &IngestionTask,
) -> Result<Value, IngestionQueueError> {
    let value: Option<(String,u64)> = tx.query_row("SELECT payload_json,revision FROM knowledge_base_documents WHERE id=?1 AND knowledge_base_id=?2",
        params![task.document_id,task.knowledge_base_id], |r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage)?;
    let (raw, revision) = value.ok_or(IngestionQueueError::NotFound)?;
    if revision != task.document_revision {
        return Err(IngestionQueueError::Conflict);
    }
    let value: Value = serde_json::from_str(&raw).map_err(storage)?;
    // Use the immutable identity already captured at enqueue, including original
    // resource code. Document id and revision alone can repeat after deletion.
    let resource_code = value["resourceCode"]
        .as_str()
        .ok_or(IngestionQueueError::Conflict)?;
    let identity = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "{}:{}:{}",
                resource_code, task.resource_revision, task.configuration
            )
            .as_bytes()
        )
    );
    if value["resourceRevision"].as_u64() != Some(task.resource_revision)
        || identity != task.input_config_hash
    {
        return Err(IngestionQueueError::Conflict);
    }
    Ok(value)
}
fn document_state(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    task: &IngestionTask,
    state: &str,
    error: Option<&str>,
) -> Result<(), IngestionQueueError> {
    let mut doc = document(tx, task)?;
    doc["status"] = state.into();
    doc["error"] = error.map(Value::from).unwrap_or(Value::Null);
    doc["waitReason"] = serde_json::to_value(&task.wait_reason).map_err(storage)?;
    tx.execute("UPDATE knowledge_base_documents SET payload_json=?1 WHERE id=?2 AND knowledge_base_id=?3 AND revision=?4",
        params![doc.to_string(),task.document_id,task.knowledge_base_id,task.document_revision]).map_err(storage)?;
    Ok(())
}
fn clear_lease(task: &mut IngestionTask) {
    task.lease_owner = None;
    task.lease_until_ms = 0;
}

impl SqliteKnowledgeIngestionQueueSql<'_> {
    fn for_document(&self, document: &str) -> Result<Vec<IngestionTask>, IngestionQueueError> {
        let conn = &*self.connection;
        let mut stmt = conn.prepare("SELECT payload_json FROM knowledge_base_ingestion_tasks WHERE document_id=?1 ORDER BY rowid DESC").map_err(storage)?;
        stmt.query_map([document], |r| r.get::<_, String>(0))
            .map_err(storage)?
            .map(|r| serde_json::from_str(&r.map_err(storage)?).map_err(storage))
            .collect()
    }
    fn pending(&self, stage: IngestionStage, now: i64) -> Result<bool, IngestionQueueError> {
        let stages = if stage == IngestionStage::Parsing {
            "('Parsing','Chunking')"
        } else {
            "('Embedding','Publishing')"
        };
        self.connection.query_row(&format!("SELECT EXISTS(SELECT 1 FROM knowledge_base_ingestion_tasks WHERE stage IN {stages} AND COALESCE(json_extract(payload_json,'$.cancellationPending'),0)=0 AND ((state='QUEUED' AND retry_after_ms<=?1) OR (state='RUNNING' AND lease_until_ms<=?1)))"), [now], |r| r.get(0)).map_err(storage)
    }
    fn pending_cancellations(&self) -> Result<Vec<IngestionTask>, IngestionQueueError> {
        let conn = &*self.connection;
        let mut stmt = conn.prepare("SELECT payload_json FROM knowledge_base_ingestion_tasks WHERE state IN ('CANCELLED','QUEUED') AND json_extract(payload_json,'$.cancellationPending')=1 ORDER BY rowid LIMIT 32").map_err(storage)?;
        stmt.query_map([], |r| r.get::<_, String>(0))
            .map_err(storage)?
            .map(|r| serde_json::from_str(&r.map_err(storage)?).map_err(storage))
            .collect()
    }
    fn complete_cancellation(
        &self,
        expected: &IngestionTask,
        job_id: Option<String>,
    ) -> Result<(), IngestionQueueError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut task = read(&tx, &expected.id)?;
        if task.attempt != expected.attempt
            || task.document_revision != expected.document_revision
            || !matches!(task.state.as_str(), "CANCELLED" | "QUEUED")
        {
            return Err(IngestionQueueError::Conflict);
        }
        if let Some(job) = job_id {
            task.remote_job_id = Some(job);
        }
        task.cancellation_pending = false;
        task.reconciling = false;
        task.wait_reason = None;
        if task.state == "QUEUED" {
            document_state(&tx, &task, "QUEUED", task.error.as_deref())?;
        }
        write(&tx, &task)?;
        tx.commit().map_err(storage)
    }
    fn enqueue(&self, task: IngestionTask) -> Result<IngestionTask, IngestionQueueError> {
        self.enqueue_document(task, None)
    }
    fn enqueue_document(&self, mut task: IngestionTask, configured: Option<crate::application::knowledge_base::KnowledgeBaseDocument>) -> Result<IngestionTask, IngestionQueueError> {
        if task.id.is_empty() || task.input_config_hash.is_empty() {
            return Err(IngestionQueueError::Invalid(
                "Missing task/config identity".into(),
            ));
        }
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        // Task ID is an application-scoped request identity. Replaying the same
        // request returns its result; a new explicit rebuild receives a fresh ID.
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM knowledge_base_ingestion_tasks WHERE id=?1",
                [&task.id],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        if let Some(id) = existing {
            if configured.is_some() { return Err(IngestionQueueError::Conflict); }
            let prior = read(&tx, &id)?;
            if prior.document_id != task.document_id
                || prior.knowledge_base_id != task.knowledge_base_id
                || prior.user_code != task.user_code
                || prior.workspace_code != task.workspace_code
                || prior.input_config_hash != task.input_config_hash
            {
                return Err(IngestionQueueError::Conflict);
            }
            return Ok(prior);
        }
        let active: Option<String> = tx.query_row("SELECT id FROM knowledge_base_ingestion_tasks WHERE document_id=?1 AND state IN ('QUEUED','RUNNING')",[&task.document_id],|r|r.get(0)).optional().map_err(storage)?;
        if let Some(id) = active {
            if configured.is_some() { return Err(IngestionQueueError::Conflict); }
            let prior = read(&tx, &id)?;
            if prior.input_config_hash == task.input_config_hash
                && prior.user_code == task.user_code
                && prior.workspace_code == task.workspace_code
                && prior.knowledge_base_id == task.knowledge_base_id
            {
                return Ok(prior);
            }
            return Err(IngestionQueueError::Conflict);
        }
        let mut doc = document(&tx, &task)?;
        if let Some(configured) = configured {
            let owned: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_bases WHERE id=?1 AND owner_user_code=?2 AND workspace_code=?3)", params![task.knowledge_base_id,task.user_code,task.workspace_code], |r|r.get(0)).map_err(storage)?;
            if !owned || configured.id != task.document_id || configured.revision != task.document_revision
                || configured.resource_code != doc["resourceCode"].as_str().unwrap_or("")
                || configured.resource_revision != task.resource_revision {
                return Err(IngestionQueueError::Conflict);
            }
            // No intermediate configuration write: this payload and task become visible together.
            doc = serde_json::to_value(configured).map_err(storage)?;
        }
        if doc["resourceRevision"].as_u64() != Some(task.resource_revision) {
            return Err(IngestionQueueError::Conflict);
        }
        task.state = "QUEUED".into();
        task.stage = IngestionStage::Parsing;
        task.attempt = 0;
        clear_lease(&mut task);
        task.endpoint_id = None;
        task.remote_file_id = None;
        task.remote_job_id = None;
        task.reconciling = false;
        task.document_revision += 1;
        doc["revision"] = task.document_revision.into();
        doc["status"] = "QUEUED".into();
        doc["error"] = Value::Null;
        doc["waitReason"] = Value::Null;
        let changed = tx.execute("UPDATE knowledge_base_documents SET payload_json=?1,revision=revision+1 WHERE id=?2 AND revision=?3",params![doc.to_string(),task.document_id,task.document_revision-1]).map_err(storage)?;
        if changed != 1 { return Err(IngestionQueueError::Conflict); }
        tx.execute("INSERT INTO knowledge_base_ingestion_tasks(id,document_id,knowledge_base_id,input_config_hash,state,stage,attempt,lease_until_ms,retry_after_ms,payload_json) VALUES(?1,?2,?3,?4,'QUEUED','Parsing',0,0,0,?5)",
            params![task.id,task.document_id,task.knowledge_base_id,task.input_config_hash,encode(&task)?]).map_err(storage)?;
        tx.execute("DELETE FROM knowledge_base_ingestion_intents WHERE document_id=?1 AND document_revision=?2", params![task.document_id,task.document_revision-1]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        Ok(task)
    }
    fn get(&self, id: &str) -> Result<IngestionTask, IngestionQueueError> {
        let conn = &*self.connection;
        read(&conn, id)
    }
    fn report_parser_availability(
        &self,
        now: i64,
        endpoints: &[ParseEndpointCapacity],
        unavailable: &[(Option<String>, IngestionWaitReason)],
    ) -> Result<(), IngestionQueueError> {
        availability::report(self, now, endpoints, unavailable)
    }
    fn claim(&self, claim: IngestionClaim) -> Result<Option<IngestionTask>, IngestionQueueError> {
        if claim.global_capacity == 0 || claim.lease_ms <= 0 || claim.owner.is_empty() {
            return Err(IngestionQueueError::Invalid(
                "Invalid worker capacity/lease".into(),
            ));
        }
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let parse = claim.stage == IngestionStage::Parsing;
        if !parse && claim.stage != IngestionStage::Embedding {
            return Err(IngestionQueueError::Invalid(
                "Only parse/embed workers claim tasks".into(),
            ));
        }
        let stages = if parse {
            "('Parsing','Chunking')"
        } else {
            "('Embedding','Publishing')"
        };
        let active: usize = tx.query_row(&format!("SELECT count(*) FROM knowledge_base_ingestion_tasks WHERE state='RUNNING' AND lease_until_ms>?1 AND stage IN {stages}"),[claim.now_ms],|r|r.get(0)).map_err(storage)?;
        let global_busy = active >= claim.global_capacity;
        let group_busy = if !parse && let Some((group, capacity)) = &claim.embedding_resource_group
        {
            let count: usize = tx.query_row("SELECT count(*) FROM knowledge_base_ingestion_tasks WHERE state='RUNNING' AND lease_until_ms>?1 AND resource_group=?2",params![claim.now_ms,group],|r|r.get(0)).map_err(storage)?;
            count >= *capacity
        } else {
            false
        };
        let ids = {
            let mut stmt = tx.prepare(&format!("SELECT id FROM knowledge_base_ingestion_tasks WHERE stage IN {stages} AND COALESCE(json_extract(payload_json,'$.cancellationPending'),0)=0 AND ((state='QUEUED' AND retry_after_ms<=?1) OR (state='RUNNING' AND lease_until_ms<=?1)) ORDER BY rowid")).map_err(storage)?;
            stmt.query_map([claim.now_ms], |r| r.get::<_, String>(0))
                .map_err(storage)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for id in ids {
            if claim
                .task_id
                .as_ref()
                .is_some_and(|requested| requested != &id)
            {
                continue;
            }
            let mut task = read(&tx, &id)?;
            match document(&tx, &task) {
                Ok(_) => {}
                Err(IngestionQueueError::Conflict | IngestionQueueError::NotFound) => {
                    task.cancellation_pending = matches!(
                        task.stage,
                        IngestionStage::Parsing | IngestionStage::Chunking
                    ) && (task.remote_job_id.is_some()
                        || task.reconciling);
                    task.state = "CANCELLED".into();
                    task.error = Some("Document generation changed".into());
                    clear_lease(&mut task);
                    write(&tx, &task)?;
                    continue;
                }
                Err(error) => return Err(error),
            }
            if global_busy || group_busy {
                if !parse
                    || claim
                        .parse_endpoints
                        .iter()
                        .any(|e| availability::eligible(&task, e))
                {
                    availability::waiting(
                        &tx,
                        &mut task,
                        IngestionWaitReason::new(if group_busy {
                            WaitReasonCode::EmbeddingGroupCapacity
                        } else if parse {
                            WaitReasonCode::ParseGlobalCapacity
                        } else {
                            WaitReasonCode::EmbeddingGlobalCapacity
                        }),
                    )?;
                }
                continue;
            }
            if parse {
                let mut available = Vec::new();
                for endpoint in &claim.parse_endpoints {
                    if !availability::eligible(&task, endpoint) {
                        continue;
                    }
                    let node_count: usize = tx.query_row("SELECT count(*) FROM knowledge_base_ingestion_tasks WHERE state='RUNNING' AND lease_until_ms>?1 AND endpoint_id=?2 AND stage IN ('Parsing','Chunking')",params![claim.now_ms,endpoint.endpoint_id],|r|r.get(0)).map_err(storage)?;
                    let group_count: usize = tx.query_row("SELECT count(*) FROM knowledge_base_ingestion_tasks WHERE state='RUNNING' AND lease_until_ms>?1 AND resource_group=?2",params![claim.now_ms,endpoint.resource_group],|r|r.get(0)).map_err(storage)?;
                    if node_count < endpoint.capacity
                        && group_count < endpoint.resource_group_capacity
                    {
                        available.push((node_count as f64 / endpoint.weight as f64, endpoint));
                    }
                }
                available.sort_by(|a, b| a.0.total_cmp(&b.0));
                let Some((_, endpoint)) = available.first() else {
                    if claim
                        .parse_endpoints
                        .iter()
                        .any(|e| availability::eligible(&task, e))
                    {
                        availability::waiting(
                            &tx,
                            &mut task,
                            IngestionWaitReason::new(WaitReasonCode::NodeCapacity),
                        )?;
                    }
                    continue;
                };
                task.endpoint_id = Some(endpoint.endpoint_id.clone());
                tx.execute(
                    "UPDATE knowledge_base_ingestion_tasks SET resource_group=?1 WHERE id=?2",
                    params![endpoint.resource_group, id],
                )
                .map_err(storage)?;
            } else {
                tx.execute(
                    "UPDATE knowledge_base_ingestion_tasks SET resource_group=?1 WHERE id=?2",
                    params![
                        claim
                            .embedding_resource_group
                            .as_ref()
                            .map(|(group, _)| group),
                        id
                    ],
                )
                .map_err(storage)?;
            }
            task.attempt = task
                .attempt
                .checked_add(1)
                .ok_or_else(|| IngestionQueueError::Invalid("Attempt overflow".into()))?;
            task.state = "RUNNING".into();
            task.lease_owner = Some(claim.owner.clone());
            task.lease_until_ms = claim.now_ms.saturating_add(claim.lease_ms);
            task.error = None;
            task.wait_reason = None;
            write(&tx, &task)?;
            document_state(
                &tx,
                &task,
                match task.stage {
                    IngestionStage::Parsing => "PARSING",
                    IngestionStage::Chunking => "CHUNKING",
                    IngestionStage::Embedding => "EMBEDDING",
                    IngestionStage::Publishing => "PUBLISHING",
                },
                None,
            )?;
            tx.commit().map_err(storage)?;
            return Ok(Some(task));
        }
        tx.commit().map_err(storage)?;
        Ok(None)
    }
    fn checkpoint(
        &self,
        update: &IngestionTask,
        lease: &IngestionLease,
        now: i64,
    ) -> Result<(), IngestionQueueError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut task = leased(&tx, lease, now)?;
        if update.id != task.id || update.endpoint_id != task.endpoint_id {
            return Err(IngestionQueueError::Conflict);
        }
        let allowed = task.stage == update.stage
            || matches!(
                (task.stage, update.stage),
                (IngestionStage::Parsing, IngestionStage::Chunking)
                    | (IngestionStage::Embedding, IngestionStage::Publishing)
            );
        if !allowed {
            return Err(IngestionQueueError::Conflict);
        }
        task.stage = update.stage;
        task.remote_file_id = update.remote_file_id.clone();
        task.remote_job_id = update.remote_job_id.clone();
        task.reconciling = update.reconciling;
        task.parsed_resource_code = update.parsed_resource_code.clone();
        task.chunks_resource_code = update.chunks_resource_code.clone();
        task.embedding_resource_code = update.embedding_resource_code.clone();
        document_state(
            &tx,
            &task,
            match task.stage {
                IngestionStage::Parsing => "PARSING",
                IngestionStage::Chunking => "CHUNKING",
                IngestionStage::Embedding => "EMBEDDING",
                IngestionStage::Publishing => "PUBLISHING",
            },
            None,
        )?;
        write(&tx, &task)?;
        tx.commit().map_err(storage)
    }
    fn heartbeat(
        &self,
        lease: &IngestionLease,
        now: i64,
        lease_ms: i64,
    ) -> Result<(), IngestionQueueError> {
        if lease_ms <= 0 {
            return Err(IngestionQueueError::Invalid("Invalid lease".into()));
        }
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut task = leased(&tx, lease, now)?;
        task.lease_until_ms = now.saturating_add(lease_ms);
        write(&tx, &task)?;
        tx.commit().map_err(storage)
    }
    fn wait_for_embedding(
        &self,
        update: &IngestionTask,
        lease: &IngestionLease,
        now: i64,
    ) -> Result<(), IngestionQueueError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut task = leased(&tx, lease, now)?;
        if update.id != task.id
            || update.endpoint_id != task.endpoint_id
            || task.stage != IngestionStage::Chunking
            || update.parsed_resource_code.is_none()
            || update.chunks_resource_code.is_none()
        {
            return Err(IngestionQueueError::Conflict);
        }
        task.parsed_resource_code = update.parsed_resource_code.clone();
        task.chunks_resource_code = update.chunks_resource_code.clone();
        task.stage = IngestionStage::Embedding;
        task.state = "QUEUED".into();
        task.retry_after_ms = now;
        clear_lease(&mut task);
        document_state(&tx, &task, "QUEUED", None)?;
        write(&tx, &task)?;
        tx.commit().map_err(storage)
    }
    fn cancel(&self, id: &str) -> Result<IngestionTask, IngestionQueueError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut task = read(&tx, id)?;
        if task.state == "READY" {
            return Err(IngestionQueueError::Conflict);
        }
        if task.state == "CANCELLED" {
            return Ok(task);
        }
        let mut doc = document(&tx, &task)?;
        task.document_revision += 1;
        doc["revision"] = task.document_revision.into();
        tx.execute(
            "UPDATE knowledge_base_documents SET revision=?1,payload_json=?2 WHERE id=?3",
            params![task.document_revision, doc.to_string(), task.document_id],
        )
        .map_err(storage)?;
        task.state = "CANCELLED".into();
        task.error = None;
        task.wait_reason = None;
        clear_lease(&mut task);
        task.cancellation_pending = matches!(
            task.stage,
            IngestionStage::Parsing | IngestionStage::Chunking
        ) && (task.remote_job_id.is_some() || task.reconciling);
        document_state(&tx, &task, "CANCELLED", None)?;
        write(&tx, &task)?;
        tx.commit().map_err(storage)?;
        Ok(task)
    }
    fn fail(
        &self,
        lease: &IngestionLease,
        now: i64,
        error: &str,
        retry_after: Option<i64>,
    ) -> Result<(), IngestionQueueError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut task = leased(&tx, lease, now)?;
        task.failures = task.failures.saturating_add(1);
        task.state = if retry_after.is_some() {
            "QUEUED"
        } else {
            "FAILED"
        }
        .into();
        task.retry_after_ms = retry_after.unwrap_or(0).max(now);
        task.error = Some(error.into());
        task.wait_reason =
            retry_after.map(|_| IngestionWaitReason::new(WaitReasonCode::RetryBackoff));
        clear_lease(&mut task);
        document_state(&tx, &task, &task.state, Some(error))?;
        write(&tx, &task)?;
        tx.commit().map_err(storage)
    }
    fn retry(&self, id: &str, now: i64) -> Result<IngestionTask, IngestionQueueError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut task = read(&tx, id)?;
        if !matches!(task.state.as_str(), "FAILED" | "CANCELLED") {
            return Err(IngestionQueueError::Conflict);
        }
        document(&tx, &task)?;
        // Keep upstream identity. The transport confirms its terminal state before
        // creating another job; an unavailable DELETE must not cause duplicates.
        task.state = "QUEUED".into();
        task.error = None;
        task.wait_reason = task
            .cancellation_pending
            .then(|| IngestionWaitReason::new(WaitReasonCode::CancellationPending));
        task.failures = 0;
        task.retry_after_ms = now;
        clear_lease(&mut task);
        document_state(&tx, &task, "QUEUED", None)?;
        write(&tx, &task)?;
        tx.commit().map_err(storage)?;
        Ok(task)
    }
    fn publish(
        &self,
        lease: &IngestionLease,
        now: i64,
        ready: &Value,
    ) -> Result<PublicationCleanup, IngestionQueueError> {
        self.publish_generation(lease, now, ready)
    }
}

impl KnowledgeIngestionQueue for SqliteKnowledgeIngestionQueue {
    fn for_document(&self, document: &str) -> Result<Vec<IngestionTask>, IngestionQueueError> {
        let document = document.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }.for_document(&document)
            },
            || storage("database worker stopped"),
        )
    }
    fn pending(&self, stage: IngestionStage, now: i64) -> Result<bool, IngestionQueueError> {
        self.connection.call(
            move |connection| SqliteKnowledgeIngestionQueueSql { connection }.pending(stage, now),
            || storage("database worker stopped"),
        )
    }
    fn pending_cancellations(&self) -> Result<Vec<IngestionTask>, IngestionQueueError> {
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }.pending_cancellations()
            },
            || storage("database worker stopped"),
        )
    }
    fn complete_cancellation(
        &self,
        expected: &IngestionTask,
        job_id: Option<String>,
    ) -> Result<(), IngestionQueueError> {
        let expected = expected.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }
                    .complete_cancellation(&expected, job_id)
            },
            || storage("database worker stopped"),
        )
    }
    fn enqueue(&self, task: IngestionTask) -> Result<IngestionTask, IngestionQueueError> {
        self.connection.call(
            move |connection| SqliteKnowledgeIngestionQueueSql { connection }.enqueue(task),
            || storage("database worker stopped"),
        )
    }
    fn enqueue_configured(&self, task: IngestionTask, document: crate::application::knowledge_base::KnowledgeBaseDocument) -> Result<IngestionTask, IngestionQueueError> {
        self.connection.call(
            move |connection| SqliteKnowledgeIngestionQueueSql { connection }.enqueue_document(task, Some(document)),
            || storage("database worker stopped"),
        )
    }
    fn get(&self, id: &str) -> Result<IngestionTask, IngestionQueueError> {
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeIngestionQueueSql { connection }.get(&id),
            || storage("database worker stopped"),
        )
    }
    fn report_parser_availability(
        &self,
        now: i64,
        endpoints: &[ParseEndpointCapacity],
        unavailable: &[(Option<String>, IngestionWaitReason)],
    ) -> Result<(), IngestionQueueError> {
        let endpoints = endpoints.to_owned();
        let unavailable = unavailable.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }.report_parser_availability(
                    now,
                    &endpoints,
                    &unavailable,
                )
            },
            || storage("database worker stopped"),
        )
    }
    fn claim(&self, claim: IngestionClaim) -> Result<Option<IngestionTask>, IngestionQueueError> {
        self.connection.call(
            move |connection| SqliteKnowledgeIngestionQueueSql { connection }.claim(claim),
            || storage("database worker stopped"),
        )
    }
    fn checkpoint(
        &self,
        update: &IngestionTask,
        lease: &IngestionLease,
        now: i64,
    ) -> Result<(), IngestionQueueError> {
        let update = update.to_owned();
        let lease = lease.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }.checkpoint(&update, &lease, now)
            },
            || storage("database worker stopped"),
        )
    }
    fn heartbeat(
        &self,
        lease: &IngestionLease,
        now: i64,
        lease_ms: i64,
    ) -> Result<(), IngestionQueueError> {
        let lease = lease.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }.heartbeat(&lease, now, lease_ms)
            },
            || storage("database worker stopped"),
        )
    }
    fn wait_for_embedding(
        &self,
        update: &IngestionTask,
        lease: &IngestionLease,
        now: i64,
    ) -> Result<(), IngestionQueueError> {
        let update = update.to_owned();
        let lease = lease.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }
                    .wait_for_embedding(&update, &lease, now)
            },
            || storage("database worker stopped"),
        )
    }
    fn cancel(&self, id: &str) -> Result<IngestionTask, IngestionQueueError> {
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeIngestionQueueSql { connection }.cancel(&id),
            || storage("database worker stopped"),
        )
    }
    fn fail(
        &self,
        lease: &IngestionLease,
        now: i64,
        error: &str,
        retry_after: Option<i64>,
    ) -> Result<(), IngestionQueueError> {
        let lease = lease.to_owned();
        let error = error.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }.fail(
                    &lease,
                    now,
                    &error,
                    retry_after,
                )
            },
            || storage("database worker stopped"),
        )
    }
    fn retry(&self, id: &str, now: i64) -> Result<IngestionTask, IngestionQueueError> {
        let id = id.to_owned();
        self.connection.call(
            move |connection| SqliteKnowledgeIngestionQueueSql { connection }.retry(&id, now),
            || storage("database worker stopped"),
        )
    }
    fn publish(
        &self,
        lease: &IngestionLease,
        now: i64,
        ready: &Value,
    ) -> Result<PublicationCleanup, IngestionQueueError> {
        let lease = lease.to_owned();
        let ready = ready.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }.publish(&lease, now, &ready)
            },
            || storage("database worker stopped"),
        )
    }
}

impl SqliteKnowledgeIngestionQueue {
    pub(crate) fn from_connection(
        connection:crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
    ) -> Result<Self, IngestionQueueError> {
        connection.call(
            |c| {
                crate::shared::sqlite_extensions::default_sqlite_extension_context()
                    .prepare_connection(c)
                    .map_err(storage)?;
                c.execute_batch(include_str!(
                    "../../../../../migrations/20261005_0001_knowledge_ingestion.sql"
                ))
                .map_err(storage)
            },
            || storage("database worker stopped"),
        )?;
        Ok(Self { connection })
    }
    pub fn connect(path: &str) -> Result<Self, IngestionQueueError> {
        Self::from_connection(crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(path).map_err(storage)?)
    }
}
