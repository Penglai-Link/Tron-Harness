//! Atomic activation of one complete product generation.
use super::*;

impl SqliteKnowledgeIngestionQueueSql<'_> {
    pub(super) fn publish_generation(
        &self,
        lease: &IngestionLease,
        now: i64,
        ready: &Value,
    ) -> Result<PublicationCleanup, IngestionQueueError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut task = leased(&tx, lease, now)?;
        if task.stage != IngestionStage::Publishing
            || ready["id"].as_str() != Some(task.document_id.as_str())
            || ready["resourceRevision"].as_u64() != Some(task.resource_revision)
            || ready["activeSourceCode"].as_str() != Some(task.generation_source_code().as_str())
        {
            return Err(IngestionQueueError::Conflict);
        }
        let current = document(&tx, &task)?;
        if ready["resourceCode"] != current["resourceCode"] {
            return Err(IngestionQueueError::Conflict);
        }
        // The product owns this fixed embedded layout. Promote all rows and retire
        // the previous whole generation in the same transaction as the pointer.
        if !task
            .knowledge_base_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err(IngestionQueueError::Invalid("Invalid corpus ID".into()));
        }
        let table = format!("\"rag_{}_units\"", task.knowledge_base_id);
        let source = ready["activeSourceCode"].as_str().unwrap();
        let count: usize = tx
            .query_row(
                &format!(
                    "SELECT count(*) FROM {table} WHERE source_code=?1 AND source_status='STAGING'"
                ),
                [source],
                |r| r.get(0),
            )
            .map_err(storage)?;
        if count == 0 || ready["chunkCount"].as_u64() != Some(count as u64) {
            return Err(IngestionQueueError::Conflict);
        }
        publication_indexes::validate_vectors(&tx, &task.knowledge_base_id, source, count)?;
        publication_cleanup::remove_old_generations(&tx, &task, &current, source)?;
        tx.execute(&format!("UPDATE {table} SET source_status='ACTIVE',status='ACTIVE' WHERE source_code=?1 AND source_status='STAGING'"), [source]).map_err(storage)?;
        let mut ready = ready.clone();
        ready["status"] = "READY".into();
        ready["error"] = Value::Null;
        ready["revision"] = (task.document_revision + 1).into();
        let changed = tx.execute("UPDATE knowledge_base_documents SET payload_json=?1,revision=revision+1 WHERE id=?2 AND knowledge_base_id=?3 AND revision=?4",
            params![ready.to_string(),task.document_id,task.knowledge_base_id,task.document_revision]).map_err(storage)?;
        if changed != 1 {
            return Err(IngestionQueueError::Conflict);
        }
        task.state = "READY".into();
        clear_lease(&mut task);
        task.parsed_resource_code = None;
        task.chunks_resource_code = None;
        task.embedding_resource_code = None;
        task.error = None;
        task.wait_reason = None;
        write(&tx, &task)?;
        let cleanup = publication_cleanup::detach_obsolete_resources(&tx, &task, &ready)?;
        tx.commit().map_err(storage)?;
        Ok(cleanup)
    }
}

impl SqliteKnowledgeIngestionQueue {
    pub(super) fn publish_generation(
        &self,
        lease: &IngestionLease,
        now: i64,
        ready: &Value,
    ) -> Result<PublicationCleanup, IngestionQueueError> {
        let lease = lease.to_owned();
        let ready = ready.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeIngestionQueueSql { connection }
                    .publish_generation(&lease, now, &ready)
            },
            || storage("database worker stopped"),
        )
    }
}
