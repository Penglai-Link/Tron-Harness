use super::*;
use crate::application::knowledge_base::ingestion::IngestionWaitReason;
use rusqlite::TransactionBehavior;

impl SqliteKnowledgeBaseRepositorySql<'_> {
    pub fn insert_document_with_intent(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
        requested: bool,
    ) -> Result<(), KnowledgeBaseError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        tx.execute("INSERT INTO knowledge_base_documents(id,knowledge_base_id,payload_json,revision) VALUES(?1,?2,?3,?4)", params![doc.id,kb,encode(doc)?,doc.revision]).map_err(storage)?;
        if requested {
            tx.execute("INSERT INTO knowledge_base_ingestion_intents(document_id,knowledge_base_id,document_revision) VALUES(?1,?2,?3)", params![doc.id,kb,doc.revision]).map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }

    pub fn request_ingestion(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        tx.execute("INSERT OR IGNORE INTO knowledge_base_ingestion_intents(document_id,knowledge_base_id,document_revision) SELECT id,knowledge_base_id,revision FROM knowledge_base_documents d WHERE id=?1 AND knowledge_base_id=?2 AND revision=?3 AND json_extract(payload_json,'$.resourceCode')=?4 AND json_extract(payload_json,'$.resourceRevision')=?5 AND json_extract(payload_json,'$.status')='UPLOADED' AND NOT EXISTS(SELECT 1 FROM knowledge_base_ingestion_tasks t WHERE t.document_id=d.id)", params![doc.id,kb,doc.revision,doc.resource_code,doc.resource_revision]).map_err(storage)?;
        tx.execute("UPDATE knowledge_base_documents SET payload_json=json_set(payload_json,'$.status','QUEUED','$.error',NULL) WHERE id=?1 AND revision=?2 AND json_extract(payload_json,'$.resourceCode')=?3 AND json_extract(payload_json,'$.resourceRevision')=?4 AND id IN (SELECT document_id FROM knowledge_base_ingestion_intents)", params![doc.id,doc.revision,doc.resource_code,doc.resource_revision]).map_err(storage)?;
        tx.commit().map_err(storage)
    }

    pub fn pending_ingestion_intents(
        &self,
        now: i64,
    ) -> Result<Vec<(String, String, String, String, u64, String)>, KnowledgeBaseError> {
        let conn = &*self.connection;
        let mut stmt = conn.prepare("SELECT k.owner_user_code,k.workspace_code,i.knowledge_base_id,i.document_id,i.document_revision,json_extract(d.payload_json,'$.resourceCode') FROM knowledge_base_ingestion_intents i JOIN knowledge_base_documents d ON d.id=i.document_id JOIN knowledge_bases k ON k.id=i.knowledge_base_id WHERE i.retry_after_ms<=?1 ORDER BY i.retry_after_ms,i.rowid LIMIT 32").map_err(storage)?;
        stmt.query_map([now], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)
    }

    pub fn finish_ingestion_intent(
        &self,
        doc: &str,
        revision: u64,
    ) -> Result<(), KnowledgeBaseError> {
        self.connection.execute("DELETE FROM knowledge_base_ingestion_intents WHERE document_id=?1 AND document_revision=?2", params![doc,revision]).map_err(storage)?;
        Ok(())
    }

    pub fn fail_ingestion_intent(
        &self,
        doc: &str,
        revision: u64,
        error: &str,
        reason: &IngestionWaitReason,
        retry_after: Option<i64>,
    ) -> Result<(), KnowledgeBaseError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        tx.execute("UPDATE knowledge_base_documents SET payload_json=json_set(payload_json,'$.status',?1,'$.error',?2,'$.waitReason',json(?5)) WHERE id=?3 AND revision=?4 AND EXISTS(SELECT 1 FROM knowledge_base_ingestion_intents i WHERE i.document_id=?3 AND i.document_revision=?4) AND NOT EXISTS(SELECT 1 FROM knowledge_base_ingestion_tasks t WHERE t.document_id=?3)", params![if retry_after.is_some() {"QUEUED"} else {"FAILED"},error,doc,revision,encode(reason)?]).map_err(storage)?;
        if let Some(after) = retry_after {
            tx.execute("UPDATE knowledge_base_ingestion_intents SET retry_after_ms=?1 WHERE document_id=?2 AND document_revision=?3", params![after,doc,revision]).map_err(storage)?;
        } else {
            tx.execute("DELETE FROM knowledge_base_ingestion_intents WHERE document_id=?1 AND document_revision=?2", params![doc,revision]).map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }

    pub fn cancel_ingestion_intent(&self, kb: &str, doc: &str) -> Result<(), KnowledgeBaseError> {
        let conn = &*self.connection;
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&conn, TransactionBehavior::Immediate)
            .map_err(storage)?;
        let changed = tx.execute("UPDATE knowledge_base_documents SET revision=revision+1,payload_json=json_set(payload_json,'$.status','CANCELLED','$.error',NULL,'$.waitReason',NULL,'$.revision',revision+1) WHERE id=?1 AND knowledge_base_id=?2 AND EXISTS(SELECT 1 FROM knowledge_base_ingestion_intents i WHERE i.document_id=?1 AND i.document_revision=knowledge_base_documents.revision) AND NOT EXISTS(SELECT 1 FROM knowledge_base_ingestion_tasks t WHERE t.document_id=?1)", params![doc,kb]).map_err(storage)?;
        if changed != 1 {
            return Err(KnowledgeBaseError::Conflict);
        }
        tx.execute(
            "DELETE FROM knowledge_base_ingestion_intents WHERE document_id=?1",
            [doc],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)
    }
}

impl SqliteKnowledgeBaseRepository {
    pub fn insert_document_with_intent(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
        requested: bool,
    ) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }
                    .insert_document_with_intent(&kb, &doc, requested)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn request_ingestion(
        &self,
        kb: &str,
        doc: &KnowledgeBaseDocument,
    ) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.request_ingestion(&kb, &doc)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn pending_ingestion_intents(
        &self,
        now: i64,
    ) -> Result<Vec<(String, String, String, String, u64, String)>, KnowledgeBaseError> {
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.pending_ingestion_intents(now)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn finish_ingestion_intent(
        &self,
        doc: &str,
        revision: u64,
    ) -> Result<(), KnowledgeBaseError> {
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }
                    .finish_ingestion_intent(&doc, revision)
            },
            || storage("database worker stopped"),
        )
    }
    pub fn fail_ingestion_intent(
        &self,
        doc: &str,
        revision: u64,
        error: &str,
        reason: &IngestionWaitReason,
        retry_after: Option<i64>,
    ) -> Result<(), KnowledgeBaseError> {
        let doc = doc.to_owned();
        let error = error.to_owned();
        let reason = reason.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.fail_ingestion_intent(
                    &doc,
                    revision,
                    &error,
                    &reason,
                    retry_after,
                )
            },
            || storage("database worker stopped"),
        )
    }
    pub fn cancel_ingestion_intent(&self, kb: &str, doc: &str) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let doc = doc.to_owned();
        self.connection.call(
            move |connection| {
                SqliteKnowledgeBaseRepositorySql { connection }.cancel_ingestion_intent(&kb, &doc)
            },
            || storage("database worker stopped"),
        )
    }
}
