use super::*;
use crate::application::knowledge_base::{deletion::DeletionCleanupPlan, ingestion::IngestionTask};
use rusqlite::TransactionBehavior;

impl SqliteKnowledgeBaseRepositorySql<'_> {
    pub fn register_derived_resource(
        &self,
        document: &KnowledgeBaseDocument,
        workspace: &str,
        code: &str,
    ) -> Result<(), KnowledgeBaseError> {
        let conn = &*self.connection;
        let changed = conn.execute("INSERT INTO knowledge_base_derived_resources(document_id,workspace_code,resource_code) SELECT id,?2,?3 FROM knowledge_base_documents WHERE id=?1 AND json_extract(payload_json,'$.resourceCode')=?4 AND json_extract(payload_json,'$.resourceRevision')=?5 AND revision=?6", params![document.id,workspace,code,document.resource_code,document.resource_revision,document.revision]).map_err(storage)?;
        if changed != 1 {
            return Err(KnowledgeBaseError::Conflict);
        }
        Ok(())
    }

    pub fn derived_resources(&self, doc: &str) -> Result<Vec<String>, KnowledgeBaseError> {
        let conn = &*self.connection;
        let mut stmt = conn
            .prepare(
                "SELECT resource_code FROM knowledge_base_derived_resources WHERE document_id=?1",
            )
            .map_err(storage)?;
        stmt.query_map([doc], |r| r.get(0))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)
    }

    /// Fence every writer and remove every generation and its indexes in one transaction.
    /// Removing the document and tasks fences late writers; missing records cannot publish.
    pub fn delete_document_transaction(
        &self,
        mut job: DeletionCleanupPlan,
    ) -> Result<DeletionCleanupPlan, KnowledgeBaseError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let (raw, revision): (String,u64) = tx.query_row("SELECT payload_json,revision FROM knowledge_base_documents WHERE id=?1 AND knowledge_base_id=?2", params![job.document_id,job.knowledge_base_id], |r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage)?.ok_or(KnowledgeBaseError::NotFound)?;
        let doc: KnowledgeBaseDocument = decode(&raw)?;
        if revision != job.document_revision || doc.resource_code != job.original_resource_code {
            return Err(KnowledgeBaseError::Conflict);
        }
        let task_rows = {
            let mut stmt = tx
                .prepare(
                    "SELECT payload_json FROM knowledge_base_ingestion_tasks WHERE document_id=?1",
                )
                .map_err(storage)?;
            stmt.query_map([&job.document_id], |r| r.get::<_, String>(0))
                .map_err(storage)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for raw in task_rows {
            let task: IngestionTask = decode(&raw)?;
            if task.cancellation_pending
                || (matches!(task.state.as_str(), "RUNNING" | "QUEUED")
                    && (task.remote_job_id.is_some() || task.reconciling))
            {
                job.cancellations.push(task);
            }
        }
        let kb = &job.knowledge_base_id;
        if !kb.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            return Err(KnowledgeBaseError::Invalid("知识库ID无效".into()));
        }
        let units = format!("rag_{kb}_units");
        if table_exists(&tx, &units)? {
            // Match document metadata, all attempt-specific source IDs, and the active pointer.
            // A source explicitly shared by another document is retained.
            let sql = format!(
                "SELECT rowid FROM \"{units}\" WHERE (json_extract(metadata,'$.documentId')=?1 OR substr(source_code,1,length(?1)+1)=?1||'_' OR source_code=?2) AND scope_code=?3 AND source_code NOT IN (SELECT coalesce(json_extract(payload_json,'$.activeSourceCode'),'') FROM knowledge_base_documents WHERE id!=?1)"
            );
            let ids = {
                let mut stmt = tx.prepare(&sql).map_err(storage)?;
                stmt.query_map(
                    params![
                        job.document_id,
                        doc.active_source_code.as_deref().unwrap_or(""),
                        job.workspace_code
                    ],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(storage)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage)?
            };
            for name in [format!("{units}_fts"), format!("{units}_vec_d1024"), units] {
                if table_exists(&tx, &name)? {
                    for id in &ids {
                        tx.execute(&format!("DELETE FROM \"{name}\" WHERE rowid=?1"), [id])
                            .map_err(storage)?;
                    }
                }
            }
        }
        for code in &job.resources {
            if let Some(locator) = plan_derived_cleanup(&tx, &job, code)? {
                job.objects.push(locator);
            }
        }
        job.objects.sort();
        job.objects.dedup();
        for table in [
            "knowledge_base_ingestion_tasks",
            "knowledge_base_ingestion_intents",
            "knowledge_base_upload_requests",
            "knowledge_base_derived_resources",
        ] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE document_id=?1"),
                [&job.document_id],
            )
            .map_err(storage)?;
        }
        tx.execute(
            "DELETE FROM knowledge_base_documents WHERE id=?1 AND knowledge_base_id=?2",
            params![job.document_id, job.knowledge_base_id],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
        Ok(job)
    }
}

fn table_exists(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    name: &str,
) -> Result<bool, KnowledgeBaseError> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )
    .map_err(storage)
}

/// Detach only exclusive derived metadata and capture its locator for best-effort cleanup.
/// This runs in the same transaction as the document fence and all index removal.
/// No storage I/O or external HTTP occurs here.
fn plan_derived_cleanup(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    job: &DeletionCleanupPlan,
    code: &str,
) -> Result<Option<String>, KnowledgeBaseError> {
    let mut object = None;
    let needle = code;
    let shared: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_base_documents WHERE id!=?1 AND instr(payload_json,?2)>0) OR EXISTS(SELECT 1 FROM knowledge_base_ingestion_tasks WHERE document_id!=?1 AND instr(payload_json,?2)>0) OR EXISTS(SELECT 1 FROM knowledge_base_derived_resources WHERE document_id!=?1 AND workspace_code=?3 AND resource_code=?4)", params![job.document_id,needle,job.workspace_code,code], |r|r.get(0)).map_err(storage)?;
    let record: Option<(String,String)> = tx.query_row("SELECT storage_locator,description FROM resource_files WHERE workspace_code=?1 AND resource_code=?2", params![job.workspace_code,code], |r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage)?;
    // References in another KB's chunks include Markdown image links and locations.
    let mut chunk_reference = false;
    let tables = {
        let mut stmt = tx
            .prepare(
                "SELECT name FROM sqlite_schema WHERE type='table' AND name GLOB 'rag_kb_*_units'",
            )
            .map_err(storage)?;
        stmt.query_map([], |r| r.get::<_, String>(0))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?
    };
    for table in tables {
        if !table
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err(KnowledgeBaseError::Invalid("知识库索引表名无效".into()));
        }
        if tx.query_row(&format!("SELECT EXISTS(SELECT 1 FROM \"{table}\" WHERE instr(metadata,?1)>0 OR instr(text,?1)>0 OR instr(source_metadata,?1)>0)"), [code], |r|r.get::<_,bool>(0)).map_err(storage)? { chunk_reference = true; break; }
    }
    let attachment_reference = if let Some((locator, _)) = &record {
        if table_exists(tx, "resource_chat_session_attachments")? {
            tx.query_row("SELECT EXISTS(SELECT 1 FROM resource_chat_session_attachments WHERE storage_locator=?1)", [locator], |r|r.get::<_,bool>(0)).map_err(storage)?
        } else {
            false
        }
    } else {
        false
    };
    let object_reference = if let Some((locator, _)) = &record {
        tx.query_row("SELECT EXISTS(SELECT 1 FROM resource_files WHERE storage_locator=?1 AND NOT(workspace_code=?2 AND resource_code=?3))", params![locator,job.workspace_code,code], |r|r.get::<_,bool>(0)).map_err(storage)?
    } else {
        false
    };
    if let Some((locator, description)) = record {
        // Only this product's derived attachments are candidates; every original is retained.
        if !shared
            && !chunk_reference
            && !attachment_reference
            && !object_reference
            && description == "知识库解析附件"
            && code != job.original_resource_code
        {
            object = Some(locator);
            tx.execute(
                "DELETE FROM resource_files WHERE workspace_code=?1 AND resource_code=?2",
                params![job.workspace_code, code],
            )
            .map_err(storage)?;
            tx.execute(
                "DELETE FROM resource_code_registry WHERE workspace_code=?1 AND resource_code=?2",
                params![job.workspace_code, code],
            )
            .map_err(storage)?;
            tx.execute("DELETE FROM knowledge_base_resource_privacy WHERE workspace_code=?1 AND resource_code=?2", params![job.workspace_code,code]).map_err(storage)?;
        }
    } else if !shared && !chunk_reference {
        // A failed creation can leave only privacy/registry rows; remove them atomically.
        tx.execute(
            "DELETE FROM resource_code_registry WHERE workspace_code=?1 AND resource_code=?2",
            params![job.workspace_code, code],
        )
        .map_err(storage)?;
        tx.execute("DELETE FROM knowledge_base_resource_privacy WHERE workspace_code=?1 AND resource_code=?2", params![job.workspace_code,code]).map_err(storage)?;
    }
    tx.execute("DELETE FROM knowledge_base_derived_resources WHERE document_id=?1 AND workspace_code=?2 AND resource_code=?3", params![job.document_id,job.workspace_code,code]).map_err(storage)?;
    Ok(object)
}

impl SqliteKnowledgeBaseRepository {
    pub fn register_derived_resource(
        &self,
        document: &KnowledgeBaseDocument,
        workspace: &str,
        code: &str,
    ) -> Result<(), KnowledgeBaseError> {
        let document = document.to_owned();
        let workspace = workspace.to_owned();
        let code = code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }
                    .register_derived_resource(&document, &workspace, &code)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn derived_resources(&self, doc: &str) -> Result<Vec<String>, KnowledgeBaseError> {
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.derived_resources(&doc)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn delete_document_transaction(
        &self,
        job: DeletionCleanupPlan,
    ) -> Result<DeletionCleanupPlan, KnowledgeBaseError> {
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.delete_document_transaction(job)
            },
            || storage("database worker stopped"),
        )
    }
}
