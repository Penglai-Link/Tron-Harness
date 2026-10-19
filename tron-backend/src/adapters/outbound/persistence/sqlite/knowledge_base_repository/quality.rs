use super::*;
use crate::application::document::LocatedChunk;
use crate::application::knowledge_quality::{KnowledgeEvaluationDataset, QualityOptimizationRun};
use eliteforge_agent_core::{ChunkContext, ChunkSourceSpan};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredChunkContext {
    pub id: String,
    pub text: String,
    pub spans: Vec<ChunkSourceSpan>,
    pub source_locations: Vec<Value>,
}

pub(crate) struct OptimizedDocumentPublication {
    pub original: KnowledgeBaseDocument,
    pub ready: KnowledgeBaseDocument,
    pub contexts: Vec<StoredChunkContext>,
    pub task: crate::application::knowledge_base::ingestion::IngestionTask,
}

pub(crate) fn stored_contexts(
    contexts: &[ChunkContext],
    chunks: &[LocatedChunk],
) -> Vec<StoredChunkContext> {
    contexts
        .iter()
        .map(|context| {
            let mut locations = Vec::new();
            for location in chunks
                .iter()
                .filter(|chunk| chunk.context_id == context.id)
                .flat_map(|chunk| &chunk.source_locations)
            {
                if !locations.contains(location) {
                    locations.push(location.clone());
                }
            }
            StoredChunkContext {
                id: context.id.clone(),
                text: context.text.clone(),
                spans: context.spans.clone(),
                source_locations: locations,
            }
        })
        .collect()
}

impl SqliteKnowledgeBaseRepository {
    pub fn quality_dataset(
        &self,
        user: &str,
        kb: &str,
    ) -> Result<Option<KnowledgeEvaluationDataset>, KnowledgeBaseError> {
        let user = user.to_owned();
        let kb = kb.to_owned();
        self.connection.call_read(move|conn|{
            let raw:Option<String>=conn.query_row("SELECT d.payload_json FROM knowledge_evaluation_datasets d JOIN knowledge_bases k ON k.id=d.knowledge_base_id WHERE d.knowledge_base_id=?1 AND d.owner_user_code=?2 AND k.owner_user_code=?2",params![kb,user],|r|r.get(0)).optional().map_err(storage)?;
            raw.map(|raw|decode(&raw)).transpose()
        },||storage("database worker stopped"))
    }
    pub fn save_quality_dataset(
        &self,
        user: &str,
        dataset: &KnowledgeEvaluationDataset,
    ) -> Result<(), KnowledgeBaseError> {
        let user = user.to_owned();
        let dataset = dataset.clone();
        self.connection.call(move|conn|{
            let owner:Option<String>=conn.query_row("SELECT owner_user_code FROM knowledge_bases WHERE id=?1",[&dataset.library_id],|r|r.get(0)).optional().map_err(storage)?;
            if owner.as_deref()!=Some(user.as_str()){return Err(KnowledgeBaseError::NotFound);}
            conn.execute("INSERT INTO knowledge_evaluation_datasets(knowledge_base_id,owner_user_code,revision,payload_json) VALUES(?1,?2,?3,?4) ON CONFLICT(knowledge_base_id) DO UPDATE SET owner_user_code=excluded.owner_user_code,revision=excluded.revision,payload_json=excluded.payload_json WHERE knowledge_evaluation_datasets.revision+1=excluded.revision",params![dataset.library_id,user,dataset.revision,encode(&dataset)?]).map_err(storage).and_then(|count|if count==1{Ok(())}else{Err(KnowledgeBaseError::Conflict)})
        },||storage("database worker stopped"))
    }
    pub fn quality_run(
        &self,
        user: &str,
        kb: &str,
        id: &str,
    ) -> Result<QualityOptimizationRun, KnowledgeBaseError> {
        let user = user.to_owned();
        let kb = kb.to_owned();
        let id = id.to_owned();
        self.connection.call_read(move|conn|{
            let raw:Option<String>=conn.query_row("SELECT r.payload_json FROM knowledge_quality_runs r JOIN knowledge_bases k ON k.id=r.knowledge_base_id WHERE r.id=?1 AND r.knowledge_base_id=?2 AND r.owner_user_code=?3 AND k.owner_user_code=?3",params![id,kb,user],|r|r.get(0)).optional().map_err(storage)?;
            let mut run:QualityOptimizationRun=decode(&raw.ok_or(KnowledgeBaseError::NotFound)?)?;run.owner_user_code=user;Ok(run)
        },||storage("database worker stopped"))
    }
    pub fn quality_runs(
        &self,
        user: &str,
        kb: &str,
    ) -> Result<Vec<QualityOptimizationRun>, KnowledgeBaseError> {
        let user = user.to_owned();
        let kb = kb.to_owned();
        self.connection.call_read(move|conn|{
            let mut statement=conn.prepare("SELECT r.payload_json FROM knowledge_quality_runs r JOIN knowledge_bases k ON k.id=r.knowledge_base_id WHERE r.knowledge_base_id=?1 AND r.owner_user_code=?2 AND k.owner_user_code=?2 ORDER BY r.created_at DESC LIMIT 20").map_err(storage)?;
            statement.query_map(params![kb,user],|row|row.get::<_,String>(0)).map_err(storage)?.map(|raw|{let mut run:QualityOptimizationRun=decode(&raw.map_err(storage)?)?;run.owner_user_code=user.clone();Ok(run)}).collect()
        },||storage("database worker stopped"))
    }
    pub fn insert_quality_run(
        &self,
        run: &QualityOptimizationRun,
    ) -> Result<QualityOptimizationRun, KnowledgeBaseError> {
        let run = run.clone();
        self.connection.call(move|conn|{
            let tx=crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(conn,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let owner:Option<String>=tx.query_row("SELECT owner_user_code FROM knowledge_bases WHERE id=?1",[&run.library_id],|r|r.get(0)).optional().map_err(storage)?;
            if owner.as_deref()!=Some(run.owner_user_code.as_str()){return Err(KnowledgeBaseError::NotFound);}
            let old:Option<String>=tx.query_row("SELECT payload_json FROM knowledge_quality_runs WHERE owner_user_code=?1 AND knowledge_base_id=?2 AND request_id=?3",params![run.owner_user_code,run.library_id,run.request_id],|r|r.get(0)).optional().map_err(storage)?;
            if let Some(old)=old{let mut old:QualityOptimizationRun=decode(&old)?;if old.source_fingerprint!=run.source_fingerprint || serde_json::to_value(&old.options).map_err(storage)?!=serde_json::to_value(&run.options).map_err(storage)?{return Err(KnowledgeBaseError::Conflict);}old.owner_user_code=run.owner_user_code;return Ok(old);}
            let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_quality_runs WHERE knowledge_base_id=?1 AND state IN ('QUEUED','RUNNING','PUBLISHING','CANCEL_REQUESTED'))",[&run.library_id],|r|r.get(0)).map_err(storage)?;
            if active{return Err(KnowledgeBaseError::Conflict);}
            tx.execute("INSERT INTO knowledge_quality_runs(id,knowledge_base_id,owner_user_code,request_id,state,created_at,payload_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![run.id,run.library_id,run.owner_user_code,run.request_id,run.status,run.created_at,encode(&run)?]).map_err(storage)?;
            tx.commit().map_err(storage)?;Ok(run)
        },||storage("database worker stopped"))
    }
    pub fn save_quality_run(&self, run: &QualityOptimizationRun) -> Result<(), KnowledgeBaseError> {
        let run = run.clone();
        self.connection.call(move|conn|{
            let changed=conn.execute("UPDATE knowledge_quality_runs SET state=?1,payload_json=?2 WHERE id=?3 AND knowledge_base_id=?4 AND owner_user_code=?5 AND (state!='CANCEL_REQUESTED' OR ?1='CANCELLED') AND state NOT IN ('COMPLETED','PUBLISHED','CANCELLED','FAILED','INTERRUPTED','STALE','BUDGET_LIMIT')",params![run.status,encode(&run)?,run.id,run.library_id,run.owner_user_code]).map_err(storage)?;
            if changed!=1{return Err(KnowledgeBaseError::Conflict);}Ok(())
        },||storage("database worker stopped"))
    }
    pub fn cancel_quality_run(
        &self,
        user: &str,
        kb: &str,
        id: &str,
    ) -> Result<QualityOptimizationRun, KnowledgeBaseError> {
        let mut run = self.quality_run(user, kb, id)?;
        if !matches!(
            run.status.as_str(),
            "QUEUED" | "RUNNING" | "PUBLISHING" | "CANCEL_REQUESTED"
        ) {
            return Ok(run);
        }
        if run.status == "CANCEL_REQUESTED" {
            return Ok(run);
        }
        run.status = "CANCEL_REQUESTED".into();
        run.updated_at = chrono::Utc::now().timestamp_millis();
        self.save_quality_run(&run)?;
        Ok(run)
    }

    pub(crate) fn discard_quality_staging(
        &self,
        kb: &str,
        run_id: &str,
    ) -> Result<(), KnowledgeBaseError> {
        if !kb.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            return Err(KnowledgeBaseError::Conflict);
        }
        let kb = kb.to_owned();
        let marker = format!("_quality_{run_id}_");
        self.connection.call(move|conn|{
            let tx=crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(conn,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let table=format!("rag_{kb}_units");
            let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name=?1)",[&table],|r|r.get(0)).map_err(storage)?;
            if exists {
                let mut stmt=tx.prepare(&format!("SELECT DISTINCT source_code FROM \"{table}\" WHERE source_status='STAGING' AND instr(source_code,?1)>0 AND source_code NOT IN (SELECT json_extract(payload_json,'$.activeSourceCode') FROM knowledge_base_documents WHERE knowledge_base_id=?2 AND json_extract(payload_json,'$.activeSourceCode') IS NOT NULL)")).map_err(storage)?;
                let sources=stmt.query_map(params![marker,kb],|r|r.get::<_,String>(0)).map_err(storage)?.collect::<Result<Vec<_>,_>>().map_err(storage)?;
                drop(stmt);
                for source in sources { super::super::knowledge_ingestion_queue::retire_knowledge_index(&tx,&kb,&source).map_err(|e|storage(format!("{e:?}")))?; }
            }
            tx.commit().map_err(storage)?;Ok(())
        },||storage("database worker stopped"))
    }
    pub fn recover_quality_runs(&self) -> Result<(), KnowledgeBaseError> {
        let interrupted = self.connection.call(|conn|{
            conn.execute("UPDATE knowledge_quality_runs SET state='INTERRUPTED',payload_json=json_set(payload_json,'$.status','INTERRUPTED','$.stage','COMPLETED','$.error','优化进程已中断；当前索引未被候选替换') WHERE state IN ('QUEUED','RUNNING','PUBLISHING','CANCEL_REQUESTED')",[]).map_err(storage)?;
            let mut stmt=conn.prepare("SELECT knowledge_base_id,id FROM knowledge_quality_runs WHERE state='INTERRUPTED'").map_err(storage)?;
            stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(storage)?.collect::<Result<Vec<_>,_>>().map_err(storage)
        },||storage("database worker stopped"))?;
        for (kb, id) in interrupted {
            self.discard_quality_staging(&kb, &id)?;
        }
        Ok(())
    }
    pub(crate) fn save_chunk_contexts(
        &self,
        kb: &str,
        document: &str,
        source: &str,
        contexts: Vec<StoredChunkContext>,
    ) -> Result<(), KnowledgeBaseError> {
        let kb = kb.to_owned();
        let document = document.to_owned();
        let source = source.to_owned();
        self.connection.call(move|conn|{let tx=crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(conn,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            for context in contexts{tx.execute("INSERT INTO knowledge_chunk_contexts(knowledge_base_id,document_id,source_code,context_id,payload_json) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(knowledge_base_id,source_code,context_id) DO UPDATE SET payload_json=excluded.payload_json",params![kb,document,source,context.id,encode(&context)?]).map_err(storage)?;}
            tx.commit().map_err(storage)?;Ok(())},||storage("database worker stopped"))
    }
    pub(crate) fn chunk_contexts(
        &self,
        user: &str,
        keys: Vec<(String, String, String)>,
    ) -> Result<Vec<(String, String, StoredChunkContext)>, KnowledgeBaseError> {
        if keys.is_empty() {
            return Ok(vec![]);
        }
        if keys.len() > 100 {
            return Err(KnowledgeBaseError::Invalid("上下文数量超出范围".into()));
        }
        let user = user.to_owned();
        self.connection.call_read(move|conn|{
            let tuples=vec!["(?,?,?)";keys.len()].join(",");let mut values=vec![user];
            for (kb,source,id) in keys{values.extend([kb,source,id]);}
            let sql=format!("SELECT c.knowledge_base_id,c.source_code,c.payload_json FROM knowledge_chunk_contexts c JOIN knowledge_base_documents d ON d.id=c.document_id JOIN knowledge_bases k ON k.id=c.knowledge_base_id WHERE k.owner_user_code=? AND json_extract(d.payload_json,'$.activeSourceCode')=c.source_code AND (c.knowledge_base_id,c.source_code,c.context_id) IN ({tuples})");
            let mut statement=conn.prepare(&sql).map_err(storage)?;
            statement.query_map(rusqlite::params_from_iter(values),|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(storage)?.map(|row|{let (kb,source,raw)=row.map_err(storage)?;Ok((kb,source,decode(&raw)?))}).collect()
        },||storage("database worker stopped"))
    }
}
