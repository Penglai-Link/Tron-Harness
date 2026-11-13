use super::*;
use crate::application::{
    memory::*,
    rag::{RagIdentity, RagRecordTask},
};
use rusqlite::OptionalExtension;
use std::sync::Arc;

struct SqliteMemoryRecords {
    database: super::super::framework_storage::SharedDatabase,
}
impl SqliteRagRepository {
    pub fn memory_repository(&self) -> Arc<dyn MemoryRecordRepository> {
        Arc::new(SqliteMemoryRecords {
            database: self.connection.clone(),
        })
    }
}
const SELECT: &str = "SELECT m.memory_code,m.user_code,m.scope_type,m.scope_code,m.semantic_key,m.title,m.text,m.enabled,m.version,m.source_refs_json,m.source_label,m.expires_at,m.created_at,m.updated_at,
 (SELECT t.state FROM rag_record_tasks t WHERE t.corpus_id='memory-card' AND json_extract(t.payload_json,'$.sourceCode')=m.memory_code AND json_extract(t.payload_json,'$.payload.metadata.recordVersion')=m.version ORDER BY t.rowid DESC LIMIT 1),
 (SELECT json_extract(t.payload_json,'$.error') FROM rag_record_tasks t WHERE t.corpus_id='memory-card' AND json_extract(t.payload_json,'$.sourceCode')=m.memory_code AND json_extract(t.payload_json,'$.payload.metadata.recordVersion')=m.version ORDER BY t.rowid DESC LIMIT 1)
 FROM memory_records m";
fn map_record(r: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryRecord> {
    let state: Option<String> = r.get(14)?;
    let enabled: bool = r.get(7)?;
    Ok(MemoryRecord {
        memory_code: r.get(0)?,
        user_code: r.get(1)?,
        scope: match r.get::<_, String>(2)?.as_str() {
            "USER" => MemoryScope::User,
            "SPACE" => MemoryScope::Space,
            _ => MemoryScope::WorkDomain,
        },
        scope_code: r.get(3)?,
        semantic_key: r.get(4)?,
        title: r.get(5)?,
        text: r.get(6)?,
        enabled,
        version: r.get(8)?,
        source_refs: serde_json::from_str(&r.get::<_, String>(9)?).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e))
        })?,
        source_label: r.get(10)?,
        expires_at: r.get(11)?,
        created_at: r.get(12)?,
        updated_at: r.get(13)?,
        index_status: if !enabled {
            "Disabled".into()
        } else {
            match state.as_deref() {
                Some("COMPLETED") => "Ready",
                Some("FAILED") => "Failed",
                Some("CANCELLED") => "Cancelled",
                _ => "Pending",
            }
            .into()
        },
        index_error: r.get(15)?,
    })
}
fn err(e: rusqlite::Error) -> MemoryError {
    MemoryError::Storage(e.to_string())
}
fn unavailable() -> MemoryError {
    MemoryError::Storage("memory database unavailable".into())
}
impl MemoryRecordRepository for SqliteMemoryRecords {
    fn get(&self, user: &str, code: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        let (user, code) = (user.to_owned(), code.to_owned());
        self.database.call(
            move |c| {
                c.query_row(
                    &format!("{SELECT} WHERE m.user_code=?1 AND m.memory_code=?2"),
                    params![user, code],
                    map_record,
                )
                .optional()
                .map_err(err)
            },
            unavailable,
        )
    }
    fn find_semantic(
        &self,
        user: &str,
        scope: MemoryScope,
        scope_code: &str,
        key: &str,
    ) -> Result<Option<MemoryRecord>, MemoryError> {
        let (user, code, key) = (user.to_owned(), scope_code.to_owned(), key.to_owned());
        self.database.call(move |c|c.query_row(&format!("{SELECT} WHERE m.user_code=?1 AND m.scope_type=?2 AND m.scope_code=?3 AND m.semantic_key=?4"),params![user,scope.rag_type(),code,key],map_record).optional().map_err(err),unavailable)
    }
    fn domain_exists(&self, code: &str) -> Result<bool, MemoryError> {
        let code = code.to_owned();
        self.database.call(
            move |c| {
                c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM memory_work_domains WHERE domain_code=?1)",
                    [code],
                    |r| r.get(0),
                )
                .map_err(err)
            },
            unavailable,
        )
    }
    fn query(&self, user: &str, q: MemoryRecordQuery) -> Result<MemoryRecordPage, MemoryError> {
        let user = user.to_owned();
        self.database.call(move |c| {
            let tx=c.transaction().map_err(err)?;
            let filter="m.user_code=?1 AND m.scope_type=?2 AND m.scope_code=?3 AND (?4='' OR instr(lower(m.title || char(10) || m.text),lower(?4))>0) AND (?5 IS NULL OR m.enabled=?5)";
            let args=params![user,q.scope.rag_type(),q.scope_code,q.keyword,q.enabled];
            let total=tx.query_row(&format!("SELECT COUNT(*) FROM memory_records m WHERE {filter}"),args,|r|r.get(0)).map_err(err)?;
            let records={let mut stmt=tx.prepare(&format!("{SELECT} WHERE {filter} ORDER BY m.updated_at DESC,m.memory_code ASC LIMIT ?6 OFFSET ?7")).map_err(err)?;
            stmt.query_map(params![user,q.scope.rag_type(),q.scope_code,q.keyword,q.enabled,q.size,(q.current-1)*q.size],map_record).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?};
            tx.commit().map_err(err)?;
            Ok(MemoryRecordPage{current:q.current,size:q.size,total,records})
        },unavailable)
    }
    fn recallable(&self, i: &RagIdentity) -> Result<Vec<MemoryRecord>, MemoryError> {
        let i = i.clone();
        self.database.call(move |c|{
            let mut stmt=c.prepare(&format!("{SELECT} WHERE m.user_code=?1 AND m.enabled=1 AND (m.expires_at IS NULL OR m.expires_at>?3) AND ((m.scope_type='USER' AND m.scope_code=?1) OR (m.scope_type='SPACE' AND ?2<>'' AND m.scope_code=?2) OR (m.scope_type='AGENT' AND EXISTS(SELECT 1 FROM memory_work_domains d WHERE d.domain_code=m.scope_code AND d.status='ACTIVE'))) ")).map_err(err)?;
            stmt.query_map(params![i.user_code,i.workspace_code,chrono::Utc::now().timestamp_millis()],map_record).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)
        },unavailable)
    }
    fn recallable_by_codes(
        &self,
        identity: &RagIdentity,
        codes: &[String],
    ) -> Result<Vec<MemoryRecord>, MemoryError> {
        if codes.is_empty() || identity.user_code.is_empty() {
            return Ok(vec![]);
        }
        let identity = identity.clone();
        let codes =
            serde_json::to_string(codes).map_err(|e| MemoryError::Storage(e.to_string()))?;
        self.database.call(move |c| {
            let mut statement = c.prepare(&format!(
                "{SELECT} WHERE m.user_code=?1 AND m.memory_code IN (SELECT value FROM json_each(?4)) \
                 AND m.enabled=1 AND (m.expires_at IS NULL OR m.expires_at>?3) \
                 AND ((m.scope_type='USER' AND m.scope_code=?1) \
                 OR (m.scope_type='SPACE' AND ?2<>'' AND m.scope_code=?2) \
                 OR (m.scope_type='AGENT' AND EXISTS(SELECT 1 FROM memory_work_domains d \
                     WHERE d.domain_code=m.scope_code AND d.status='ACTIVE')))"
            )).map_err(err)?;
            statement.query_map(
                params![identity.user_code, identity.workspace_code, chrono::Utc::now().timestamp_millis(), codes],
                map_record,
            ).map_err(err)?.collect::<Result<Vec<_>, _>>().map_err(err)
        }, unavailable)
    }
    fn save(
        &self,
        r: &MemoryRecord,
        expected: Option<u32>,
        task: Option<&RagRecordTask>,
    ) -> Result<(), MemoryError> {
        let (r, task) = (r.clone(), task.cloned());
        self.database.call(move |c|{
            let tx=super::super::framework_storage::business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(err)?;
            let refs=serde_json::to_string(&r.source_refs).map_err(|e|MemoryError::Storage(e.to_string()))?;
            if let Some(v)=expected {
                let n=tx.execute("UPDATE memory_records SET title=?4,text=?5,enabled=?6,version=?7,updated_at=?8 WHERE memory_code=?1 AND user_code=?2 AND version=?3",params![r.memory_code,r.user_code,v,r.title,r.text,r.enabled,r.version,r.updated_at]).map_err(err)?;
                if n!=1{return Err(MemoryError::Conflict("memory.version_conflict".into()))}
            } else {
                tx.execute("INSERT INTO memory_records(memory_code,user_code,scope_type,scope_code,semantic_key,title,text,enabled,version,source_refs_json,source_label,expires_at,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",params![r.memory_code,r.user_code,r.scope.rag_type(),r.scope_code,r.semantic_key,r.title,r.text,r.enabled,r.version,refs,r.source_label,r.expires_at,r.created_at,r.updated_at]).map_err(|e|if matches!(e,rusqlite::Error::SqliteFailure(ref c,_) if c.code==rusqlite::ErrorCode::ConstraintViolation){MemoryError::Conflict("memory.duplicate".into())}else{err(e)})?;
            }
            invalidate(&tx,&r.memory_code)?;
            if let Some(task)=task {
                tx.execute("INSERT INTO rag_record_tasks(id,user_code,corpus_id,state,payload_json,updated_at) VALUES (?1,?2,'memory-card','QUEUED',?3,?4)",params![task.id,task.user_code,serde_json::to_string(&task).map_err(|e|MemoryError::Storage(e.to_string()))?,task.updated_at]).map_err(err)?;
            }
            tx.commit().map_err(err)
        },unavailable)
    }
    fn delete(&self, user: &str, code: &str, version: u32) -> Result<(), MemoryError> {
        let (user, code) = (user.to_owned(), code.to_owned());
        self.database.call(move |c|{
            let tx=super::super::framework_storage::business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(err)?;
            if tx.execute("DELETE FROM memory_records WHERE user_code=?1 AND memory_code=?2 AND version=?3",params![user,code,version]).map_err(err)?!=1{return Err(MemoryError::Conflict("memory.version_conflict".into()))}
            invalidate(&tx,&code)?;
            tx.commit().map_err(err)
        },unavailable)
    }
}
fn invalidate(
    tx: &super::super::framework_storage::BusinessTransaction<'_>,
    code: &str,
) -> Result<(), MemoryError> {
    tx.execute("UPDATE rag_record_tasks SET state='CANCELLED' WHERE corpus_id='memory-card' AND json_extract(payload_json,'$.sourceCode')=?1 AND state IN ('RUNNING','QUEUED','FAILED')",[code]).map_err(err)?;
    let repo = SqliteRagRepositorySql { connection: tx };
    let corpus = repo
        .get_corpus("memory-card")
        .map_err(|e| MemoryError::Rag(e.to_string()))?
        .ok_or_else(|| MemoryError::Storage("memory corpus missing".into()))?;
    let plan = sql_plan::compile_sql_corpus_plan(&repo, &corpus)
        .map_err(|e| MemoryError::Rag(e.to_string()))?;
    let rows: Vec<(String, String, String)> = {
        let mut s=tx.prepare("SELECT DISTINCT source_version,scope_type,scope_code FROM rag_memory_text_units WHERE source_code=?1").map_err(err)?;
        s.query_map([code], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?
    };
    for (version, scope, scope_code) in rows {
        indexes::delete_text_unit_indexes_for_source(
            tx,
            &plan,
            code,
            &version,
            &scope,
            &scope_code,
        )
        .map_err(|e| MemoryError::Rag(e.to_string()))?;
        super::super::rag_graph_index::retire_graph_source_version(
            tx,
            &crate::domain::rag::graph::GraphPartition {
                corpus_id: "memory-card".into(),
                scope_type: scope,
                scope_code,
            },
            code,
            &version,
        )
        .map_err(MemoryError::Storage)?;
    }
    tx.execute(
        "DELETE FROM rag_memory_text_units WHERE source_code=?1",
        [code],
    )
    .map_err(err)?;
    Ok(())
}
