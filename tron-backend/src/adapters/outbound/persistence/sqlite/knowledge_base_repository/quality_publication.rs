use super::quality::OptimizedDocumentPublication;
use super::*;
use crate::application::document::ChunkConfig;
use crate::application::knowledge_quality::QualityOptimizationRun;

impl SqliteKnowledgeBaseRepository {
    /// Publish the tested candidate for all affected documents in one transaction.
    /// No old index versions are retained; parsing artifacts remain reusable.
    pub(crate) fn publish_quality_candidate(
        &self,
        user: &str,
        kb: &str,
        run: QualityOptimizationRun,
        config: ChunkConfig,
        expected_capabilities: serde_json::Value,
        documents: Vec<OptimizedDocumentPublication>,
    ) -> Result<
        Vec<crate::application::knowledge_base::deletion::DeletionCleanupPlan>,
        KnowledgeBaseError,
    > {
        let user = user.to_owned();
        let kb = kb.to_owned();
        self.connection.call(move|conn|{
            if !kb.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_'){return Err(KnowledgeBaseError::Invalid("Invalid corpus ID".into()));}
            let tx=crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(conn,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let raw:Option<String>=tx.query_row("SELECT payload_json FROM knowledge_bases WHERE id=?1 AND owner_user_code=?2",params![kb,user],|r|r.get(0)).optional().map_err(storage)?;
            let mut library:KnowledgeBase=decode(&raw.ok_or(KnowledgeBaseError::NotFound)?)?;
            let state:Option<String>=tx.query_row("SELECT state FROM knowledge_quality_runs WHERE id=?1 AND knowledge_base_id=?2 AND owner_user_code=?3",params![run.id,kb,user],|r|r.get(0)).optional().map_err(storage)?;
            if state.as_deref()!=Some("PUBLISHING"){return Err(KnowledgeBaseError::Conflict);}
            let capabilities:String=tx.query_row("SELECT capabilities_json FROM rag_corpora WHERE corpus_id=?1",[&kb],|r|r.get(0)).map_err(storage)?;
            if serde_json::from_str::<serde_json::Value>(&capabilities).map_err(storage)? != expected_capabilities {return Err(KnowledgeBaseError::Conflict);}
            let count:usize=tx.query_row("SELECT count(*) FROM knowledge_base_documents WHERE knowledge_base_id=?1 AND json_extract(payload_json,'$.activeSourceCode') IS NOT NULL",[&kb],|r|r.get(0)).map_err(storage)?;
            if count!=documents.len(){return Err(KnowledgeBaseError::Conflict);}
            let revision:u64=tx.query_row("SELECT revision FROM knowledge_evaluation_datasets WHERE knowledge_base_id=?1",[&kb],|r|r.get(0)).map_err(storage)?;
            if revision != run.dataset_revision {return Err(KnowledgeBaseError::Conflict);}
            let table=format!("rag_{kb}_units");
            let selected = encode(&documents.iter().map(|document|&document.original.id).collect::<Vec<_>>())?;
            let originals = {
                let mut q=tx.prepare("SELECT id,revision,payload_json FROM knowledge_base_documents WHERE knowledge_base_id=?1 AND id IN (SELECT value FROM json_each(?2))").map_err(storage)?;
                q.query_map(params![kb,selected],|r|Ok((r.get::<_,String>(0)?,r.get::<_,u64>(1)?,r.get::<_,String>(2)?))).map_err(storage)?.map(|row| {
                    let (id,revision,raw)=row.map_err(storage)?;
                    Ok((id,(revision,decode::<KnowledgeBaseDocument>(&raw)?)))
                }).collect::<Result<std::collections::BTreeMap<_,_>,KnowledgeBaseError>>()?
            };
            let busy = {
                let mut q=tx.prepare("SELECT DISTINCT document_id FROM knowledge_base_ingestion_tasks WHERE knowledge_base_id=?1 AND state IN ('RUNNING','QUEUED','CANCEL_REQUESTED')").map_err(storage)?;
                q.query_map([&kb],|r|r.get::<_,String>(0)).map_err(storage)?.collect::<Result<std::collections::BTreeSet<_>,_>>().map_err(storage)?
            };
            let sources=documents.iter().map(|document|document.ready.active_source_code.as_deref().ok_or(KnowledgeBaseError::Conflict)).collect::<Result<Vec<_>,_>>()?;
            let counts = {
                let mut q=tx.prepare(&format!("SELECT source_code,count(*) FROM \"{table}\" WHERE source_status='STAGING' AND source_code IN (SELECT value FROM json_each(?1)) GROUP BY source_code")).map_err(storage)?;
                q.query_map([encode(&sources)?],|r|Ok((r.get::<_,String>(0)?,r.get::<_,usize>(1)?))).map_err(storage)?.collect::<Result<std::collections::BTreeMap<_,_>,_>>().map_err(storage)?
            };
            let mut dimensions=std::collections::BTreeMap::<usize,Vec<&str>>::new();
            for document in &documents {
                if let Some(dimension)=document.ready.embedding_dimension {
                    dimensions.entry(dimension).or_default().push(document.ready.active_source_code.as_deref().ok_or(KnowledgeBaseError::Conflict)?);
                }
            }
            let vector_tables={
                let mut q=tx.prepare("SELECT name FROM sqlite_schema WHERE type='table'").map_err(storage)?;
                q.query_map([],|r|r.get::<_,String>(0)).map_err(storage)?.collect::<Result<std::collections::BTreeSet<_>,_>>().map_err(storage)?
            };
            let mut vector_counts=std::collections::BTreeMap::new();
            for (dimension,sources) in dimensions {
                let vector=format!("rag_{kb}_units_vec_d{dimension}");
                if !vector_tables.contains(&vector) { return Err(KnowledgeBaseError::Conflict); }
                let mut q=tx.prepare(&format!("SELECT u.source_code,count(*) FROM \"{table}\" u JOIN \"{vector}\" v ON v.rowid=u.id WHERE u.source_status='STAGING' AND u.source_code IN (SELECT value FROM json_each(?1)) GROUP BY u.source_code")).map_err(storage)?;
                for row in q.query_map([encode(&sources)?],|r|Ok((r.get::<_,String>(0)?,r.get::<_,usize>(1)?))).map_err(storage)? {
                    let (source,count)=row.map_err(storage)?; vector_counts.insert((dimension,source),count);
                }
            }
            let graph_sources=documents.iter().filter(|d|d.task.configuration["ragSettings"]["graph"]==true).map(|d|d.ready.active_source_code.as_deref().ok_or(KnowledgeBaseError::Conflict)).collect::<Result<Vec<_>,_>>()?;
            let graph_ids={
                let mut q=tx.prepare(&format!("SELECT source_code,source_metadata FROM \"{table}\" WHERE id IN (SELECT min(id) FROM \"{table}\" WHERE source_status='STAGING' AND source_code IN (SELECT value FROM json_each(?1)) GROUP BY source_code)")).map_err(storage)?;
                q.query_map([encode(&graph_sources)?],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(storage)?.map(|row| {
                    let (source,raw)=row.map_err(storage)?; let metadata:serde_json::Value=serde_json::from_str(&raw).map_err(storage)?;
                    Ok((source,metadata["graphGenerationId"].as_str().ok_or(KnowledgeBaseError::Conflict)?.to_owned()))
                }).collect::<Result<std::collections::BTreeMap<_,_>,KnowledgeBaseError>>()?
            };
            let partition=crate::domain::rag::graph::GraphPartition{corpus_id:kb.clone(),scope_type:"SPACE".into(),scope_code:"PLATFORM".into()};
            let bindings=documents.iter().filter(|d|d.task.configuration["ragSettings"]["graph"]==true).map(|d| {
                let source=d.ready.active_source_code.as_deref().ok_or(KnowledgeBaseError::Conflict)?;
                Ok((graph_ids.get(source).ok_or(KnowledgeBaseError::Conflict)?.clone(),partition.clone(),d.ready.id.clone(),source.to_owned(),d.task.id.clone(),d.task.input_config_hash.clone()))
            }).collect::<Result<Vec<_>,KnowledgeBaseError>>()?;
            super::super::rag_graph_index::validate_graph_generation_bindings(&tx,&bindings).map_err(storage)?;
            for document in &documents {
                let (revision,current)=originals.get(&document.original.id).ok_or(KnowledgeBaseError::Conflict)?;
                if *revision!=document.original.revision || current.resource_code!=document.original.resource_code || current.resource_revision!=document.original.resource_revision || current.active_source_code!=document.original.active_source_code || current.status!="READY" || busy.contains(&current.id) {return Err(KnowledgeBaseError::Conflict);}
                let source=document.ready.active_source_code.as_deref().ok_or(KnowledgeBaseError::Conflict)?;
                let count=counts.get(source).copied().unwrap_or(0);
                if count==0 || count!=document.ready.chunk_count{return Err(KnowledgeBaseError::Conflict);}
                if let Some(dimension)=document.ready.embedding_dimension {
                    if vector_counts.get(&(dimension,source.to_owned())).copied().unwrap_or(0)!=count { return Err(KnowledgeBaseError::Conflict); }
                }
            }
            super::super::rag_graph_index::activate_graph_generations(&tx,&bindings.iter().map(|binding|binding.0.clone()).collect::<Vec<_>>()).map_err(storage)?;
            let mut cleanups=Vec::new();
            for document in documents {
                let source=document.ready.active_source_code.as_deref().ok_or(KnowledgeBaseError::Conflict)?;
                for context in document.contexts {
                    tx.execute("INSERT INTO knowledge_chunk_contexts(knowledge_base_id,document_id,source_code,context_id,payload_json) VALUES(?1,?2,?3,?4,?5)",params![kb,document.ready.id,source,context.id,encode(&context)?]).map_err(storage)?;
                }
                tx.execute(&format!("UPDATE \"{table}\" SET source_status='ACTIVE',status='ACTIVE' WHERE source_code=?1 AND source_status='STAGING'"),[source]).map_err(storage)?;
                let partition=crate::domain::rag::graph::GraphPartition{corpus_id:kb.clone(),scope_type:"SPACE".into(),scope_code:"PLATFORM".into()};
                if document.task.configuration["ragSettings"]["graph"]!=true {
                    super::super::rag_graph_index::retire_graph_source(&tx,&partition,&document.ready.id).map_err(storage)?;
                }
                let ready=encode(&document.ready)?;
                if tx.execute("UPDATE knowledge_base_documents SET payload_json=?1,revision=?2 WHERE id=?3 AND knowledge_base_id=?4 AND revision=?5",params![ready,document.ready.revision,document.ready.id,kb,document.original.revision]).map_err(storage)?!=1{return Err(KnowledgeBaseError::Conflict);}
                let task=document.task;
                tx.execute("INSERT INTO knowledge_base_ingestion_tasks(id,document_id,knowledge_base_id,input_config_hash,state,stage,attempt,lease_owner,lease_until_ms,retry_after_ms,endpoint_id,resource_group,payload_json) VALUES(?1,?2,?3,?4,'READY','Publishing',?5,NULL,0,0,NULL,NULL,?6)",params![task.id,task.document_id,kb,task.input_config_hash,task.attempt,encode(&task)?]).map_err(storage)?;
                let cleanup=super::super::knowledge_ingestion_queue::optimized_publication_cleanup(&tx,&task,&serde_json::to_value(&document.original).map_err(storage)?,&serde_json::to_value(&document.ready).map_err(storage)?).map_err(|e|storage(format!("{e:?}")))?;
                cleanups.push(crate::application::knowledge_base::deletion::DeletionCleanupPlan{document_id:document.ready.id,knowledge_base_id:kb.clone(),workspace_code:"PLATFORM".into(),document_revision:document.ready.revision,original_resource_code:document.ready.resource_code,resources:vec![],objects:cleanup.objects,cancellations:cleanup.cancellations});
            }
            library.default_chunk_config=config;
            tx.execute("UPDATE knowledge_bases SET payload_json=?1 WHERE id=?2 AND owner_user_code=?3",params![encode(&library)?,kb,user]).map_err(storage)?;
            if tx.execute("UPDATE knowledge_quality_runs SET state='PUBLISHED',payload_json=?1 WHERE id=?2 AND state='PUBLISHING'",params![encode(&run)?,run.id]).map_err(storage)?!=1{return Err(KnowledgeBaseError::Conflict);}
            tx.commit().map_err(storage)?;Ok(cleanups)
        },||storage("database worker stopped"))
    }
}
