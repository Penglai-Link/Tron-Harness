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
        if let Some(expected)=task.configuration.get("ragSettings") {
            let capabilities:String=tx.query_row("SELECT capabilities_json FROM rag_corpora WHERE corpus_id=?1",[&task.knowledge_base_id],|r|r.get(0)).map_err(storage)?;
            let current=crate::application::rag::RagSettings::from_capabilities(&serde_json::from_str(&capabilities).map_err(storage)?).map_err(storage)?;
            let expected:crate::application::rag::RagSettings=serde_json::from_value(expected.clone()).map_err(storage)?;
            if current!=expected{return Err(IngestionQueueError::Conflict);}
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
        if let Some(dimension) = ready["embeddingDimension"].as_u64() {
            publication_indexes::validate_vectors(
                &tx,
                &task.knowledge_base_id,
                source,
                count,
                dimension as usize,
            )?;
        }
        let partition=crate::domain::rag::graph::GraphPartition{corpus_id:task.knowledge_base_id.clone(),scope_type:"SPACE".into(),scope_code:task.workspace_code.clone()};
        if task.configuration["ragSettings"]["graph"]==true {
            let metadata:String=tx.query_row(&format!("SELECT source_metadata FROM {table} WHERE source_code=?1 AND source_status='STAGING' LIMIT 1"),[source],|r|r.get(0)).map_err(storage)?;
            let metadata:Value=serde_json::from_str(&metadata).map_err(storage)?;
            let id=metadata["graphGenerationId"].as_str().ok_or(IngestionQueueError::Conflict)?;
            super::super::rag_graph_index::validate_graph_generation_binding(&tx,id,&partition,&task.document_id,source,&task.id,&task.input_config_hash).map_err(|_|IngestionQueueError::Conflict)?;
            super::super::rag_graph_index::activate_graph_generation(&tx,id).map_err(|_|IngestionQueueError::Conflict)?;
        } else {
            super::super::rag_graph_index::retire_graph_source(&tx,&partition,&task.document_id).map_err(storage)?;
        }
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
        task.completed_units = count;
        task.total_units = count;
        task.updated_at = now;
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
