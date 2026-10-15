//! Integration layer: knowledge-library parsing output enters the generic RAG indexer.
//! Neither domain imports the other from inside its own module.
use crate::application::rag::*;
use crate::domain::resource::ResourceKind;
use crate::{
    app::AppRagService,
    application::knowledge_base::{
        hash,
        ingestion::*,
        ingestion_worker::{
            RemoteCheckpoint, StoredChunks, StoredEmbeddings, lease, now_ms,
            pipeline::VectorCheckpoint, queue_error,
        },
        invalid_storage, *,
    },
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
impl AppRagService {
    pub(crate) async fn execute_document_task(
        &self,
        host: &KnowledgeBaseService,
        mut task: IngestionTask,
    ) -> Result<(), KnowledgeBaseError> {
        let owned = lease(&task);
        let mut kb = host.owned(&task.user_code, &task.knowledge_base_id)?;
        let mut doc = host.repository.document(&kb.id, &task.document_id)?;
        // The existing task identity includes the original resource code. A recreated
        // same-content document may reuse id/revision, but never its original resource.
        let identity = hash(&format!(
            "{}:{}:{}",
            doc.resource_code, doc.resource_revision, task.configuration
        ));
        if doc.revision != task.document_revision
            || doc.resource_revision != task.resource_revision
            || identity != task.input_config_hash
        {
            return Err(KnowledgeBaseError::Conflict);
        }
        kb.default_chunk_config = serde_json::from_value(task.configuration["chunkConfig"].clone())
            .map_err(invalid_storage)?;
        doc.chunk_config = Some(kb.default_chunk_config.clone());
        doc.parsing_tier = serde_json::from_value(task.configuration["parsingTier"].clone())
            .map_err(invalid_storage)?;
        doc.ocr_mode = serde_json::from_value(task.configuration["ocrMode"].clone())
            .map_err(invalid_storage)?;
        if matches!(
            task.stage,
            IngestionStage::Parsing | IngestionStage::Chunking
        ) {
            let task_parser = host
                .parser
                .as_ref()
                .map(|parser| parser.task_snapshot(&task.configuration["parser"]))
                .transpose()?
                .flatten();
            if !doc.file_name.to_ascii_lowercase().ends_with(".md")
                && task_parser.is_none()
                && host
                    .document_parser(&doc)?
                    .as_ref()
                    .map(|p| p.profile_identity())
                    != Some(task.configuration["parser"].clone())
            {
                return Err(KnowledgeBaseError::Invalid(
                    "解析节点或配置已改变，请重新入库".into(),
                ));
            }
            let checkpoint = Arc::new(RemoteCheckpoint {
                queue: host.queue.clone(),
                lease: owned.clone(),
                task: Mutex::new(task.clone()),
            });
            let mut service = host.clone();
            if let Some(parser) = task_parser {
                service.parser = Some(parser);
            }
            service.runtime_config = None;
            service.parse_checkpoint = Some(checkpoint);
            service.image_description =
                serde_json::from_value(task.configuration["imageDescription"].clone())
                    .map_err(invalid_storage)?;
            let (chunks, warnings, parsed) = service
                .chunks(&task.user_code, &task.workspace_code, &kb, &doc, true)
                .await?;
            if chunks.is_empty() {
                return Err(KnowledgeBaseError::Invalid("资料没有可入库内容".into()));
            }
            task = host.queue.get(&task.id).map_err(queue_error)?;
            task.stage = IngestionStage::Chunking;
            task.total_units = chunks.len();
            task.completed_units = 0;
            task.updated_at = now_ms();
            host.queue
                .checkpoint(&task, &owned, now_ms())
                .map_err(queue_error)?;
            let stored = StoredChunks {
                document: parsed,
                chunks,
                warnings,
            };
            let code = host.derived_resource(
                &task.user_code,
                &task.workspace_code,
                &doc,
                "ingestion-chunks.json",
                ResourceKind::File,
                &serde_json::to_vec(&stored).map_err(invalid_storage)?,
            )?;
            task.parsed_resource_code = Some(code.clone());
            task.chunks_resource_code = Some(code);
            host.queue
                .wait_for_embedding(&task, &owned, now_ms())
                .map_err(queue_error)?;
            return Ok(());
        }
        let stored: StoredChunks = host.read_checkpoint(
            &task,
            task.chunks_resource_code
                .as_deref()
                .ok_or_else(|| KnowledgeBaseError::Invalid("入库切分检查点缺失".into()))?,
        )?;
        let configuration = task.configuration.clone();
        let settings: RagSettings = serde_json::from_value(configuration["ragSettings"].clone())
            .map_err(invalid_storage)?;
        let model = configuration["embeddingModel"]
            .as_str()
            .or_else(|| (!settings.vector).then_some(""))
            .ok_or_else(|| {
                KnowledgeBaseError::Invalid(
                    task.configuration["embeddingError"]
                        .as_str()
                        .unwrap_or("Embedding模型未配置")
                        .into(),
                )
            })?;
        let expected = configuration["embeddingFingerprint"]
            .as_str()
            .or_else(|| (!settings.vector).then_some(""))
            .ok_or_else(|| KnowledgeBaseError::Invalid("Embedding空间未配置".into()))?;
        let mut embedded: StoredEmbeddings = match &task.embedding_resource_code {
            Some(code) => host.read_checkpoint(&task, code)?,
            None => Default::default(),
        };
        if !embedded.vector_resources.is_empty() {
            if !embedded.vectors.is_empty() || embedded.completed_units > stored.chunks.len() {
                return Err(KnowledgeBaseError::Invalid("向量检查点不匹配".into()));
            }
            for code in &embedded.vector_resources {
                let batch: Vec<Vec<f32>> = host.read_checkpoint(&task, code)?;
                embedded.vectors.extend(batch);
            }
            if embedded.vectors.len() != embedded.completed_units {
                return Err(KnowledgeBaseError::Invalid("向量检查点数量不匹配".into()));
            }
        }
        if embedded.vectors.len() > stored.chunks.len() {
            return Err(KnowledgeBaseError::Invalid("向量检查点不匹配".into()));
        }
        if embedded
            .vectors
            .iter()
            .any(|v| v.len() != settings.embedding_dimension || v.iter().any(|n| !n.is_finite()))
        {
            return Err(KnowledgeBaseError::Invalid(
                "向量检查点维度或数值无效，整份索引未发布".into(),
            ));
        }
        task.total_units = stored.chunks.len();
        task.completed_units = embedded.vectors.len();
        let checkpoint = VectorCheckpoint {
            service: host,
            task: Mutex::new(task.clone()),
            lease: &owned,
            model,
            fingerprint: expected,
            resources: Mutex::new(embedded.vector_resources.clone()),
            dimension: settings.embedding_dimension,
            provider_model: Mutex::new(embedded.provider_model.clone()),
            document: &doc,
        };
        let source = task.generation_source_code();
        let units: Vec<RagTextUnitWrite> = stored.chunks.iter().map(|chunk| RagTextUnitWrite {
            text_unit_code: format!("{}_{}",source,chunk.ordinal), unit_type:"LEAF".into(),
            text:chunk.text.clone(), search_text:chunk.text.clone(),
            heading_path:serde_json::to_string(&chunk.heading_path).unwrap_or_default(), ordinal:chunk.ordinal,
            token_count:chunk.text.chars().count().div_ceil(4) as i64, content_hash:hash(&chunk.text), status:"ACTIVE".into(),
            fulltext_status:String::new(), embedding_model_code:None, embedding_version:None, embedding_text_hash:None,
            embedding_dimension:None, embedding:None, embedding_status:String::new(),
            metadata:json!({"location":chunk.location,"documentId":doc.id,"resourceCode":doc.resource_code,"resourceRevision":doc.resource_revision}),
        }).collect();
        let mut index_settings = settings.clone();
        index_settings.embedding_model_code = settings.vector.then(|| model.to_owned());
        let prepared = self
            .prepare_index(
                &task.workspace_code,
                &index_settings,
                units,
                RagPreparedVectors {
                    vectors: embedded.vectors,
                    provider_model: embedded.provider_model,
                    model_code: settings.vector.then(|| model.to_owned()),
                    fingerprint: settings.vector.then(|| expected.to_owned()),
                },
                None,
                &checkpoint,
            )
            .await
            .map_err(|error| {
                if error.contains("取消") || error.contains("租约") {
                    KnowledgeBaseError::Conflict
                } else if [
                    "维度",
                    "模型配置",
                    "模型或",
                    "模型发生",
                    "检查点",
                    "没有",
                    "请先配置",
                    "没有有效原文",
                    "图谱抽取返回格式",
                ]
                .iter()
                .any(|word| error.contains(word))
                {
                    KnowledgeBaseError::Invalid(error)
                } else {
                    KnowledgeBaseError::Storage(error)
                }
            })?;
        task = checkpoint.task.lock().map_err(invalid_storage)?.clone();
        embedded.provider_model = prepared.vectors.provider_model;
        let units = prepared.units;
        let graph = prepared.graph;
        task.stage = IngestionStage::Publishing;
        host.queue
            .checkpoint(&task, &owned, now_ms())
            .map_err(queue_error)?;
        // Synchronize the final staging write/publication with deletion fencing.
        // No await occurs while this synchronous gate is held.
        let mut ready = stored.document;
        host.retain_publication_resources(
            &task.user_code,
            &task.workspace_code,
            &mut ready,
            &stored.chunks,
        )?;
        let cleanup = {
            let _guard = host.cleanup_gate.lock().map_err(invalid_storage)?;
            let current_task = host.queue.get(&task.id).map_err(queue_error)?;
            if current_task.state != "RUNNING"
                || current_task.attempt != owned.attempt
                || current_task.lease_owner.as_deref() != Some(&owned.owner)
            {
                return Err(KnowledgeBaseError::Conflict);
            }
            self.repository_handle().persist_ingest(RagPersistIngestCommand { corpus_id: kb.id.clone(), task_id: task.id.clone(), source_code: source.clone(), source_version: "v1".into(), scope: RagIngestScope { scope_type: "SPACE".into(), scope_code: task.workspace_code.clone() }, source_status: "STAGING".into(), title: Some(doc.name.clone()), content_hash: hash(&source), body_hash: None, metadata: json!({"documentId":doc.id,"resourceCode":doc.resource_code,"resourceRevision":doc.resource_revision,"embeddingProviderModel":embedded.provider_model,"embeddingFingerprint":expected,"indexSettings":settings}), domain_fields: BTreeMap::new(), related_graph: graph, graph_write_policy: json!({}), text_units: units, tree_nodes: vec![], warnings: vec![], workspace_code: task.workspace_code.clone(), user_code: task.user_code.clone(), agent_code: String::new() })?;
            if settings.vector {
                host.verify_embedding_space(
                    &task.workspace_code,
                    model,
                    expected,
                    settings.embedding_dimension,
                )?;
            }
            ready.active_source_code = Some(source);
            ready.index_revision = doc.index_revision + 1;
            ready.chunk_count = stored.chunks.len();
            ready.embedding_model_code = settings.vector.then(|| model.into());
            ready.embedding_dimension = settings.vector.then_some(settings.embedding_dimension);
            ready.embedding_provider_model = embedded.provider_model;
            ready.embedding_space_fingerprint = settings.vector.then(|| expected.into());
            ready.warnings = stored.warnings;
            host.queue
                .publish(
                    &owned,
                    now_ms(),
                    &serde_json::to_value(ready).map_err(invalid_storage)?,
                )
                .map_err(queue_error)?
        };
        let plan = crate::application::knowledge_base::deletion::DeletionCleanupPlan {
            document_id: task.document_id,
            knowledge_base_id: task.knowledge_base_id,
            workspace_code: task.workspace_code,
            document_revision: task.document_revision,
            original_resource_code: doc.resource_code,
            resources: vec![],
            objects: cleanup.objects,
            cancellations: cleanup.cancellations,
        };
        // Publication clears the lease. Cleanup must outlive the ingestion lease
        // monitor, which may stop polling this now-completed attempt at any time.
        let service = host.clone();
        tokio::spawn(async move {
            service.clean_document_deletion(plan).await;
        });
        Ok(())
    }
}

impl KnowledgeBaseService {
    pub fn rag_task_views(&self, user: &str) -> Result<Vec<RagTaskView>, KnowledgeBaseError> {
        self.require_user(user)?;
        Ok(self
            .queue
            .index_tasks_for_user(user)
            .map_err(queue_error)?
            .into_iter()
            .map(|(task, title)| RagTaskView {
                file_size_bytes: task.file_size_bytes,
                id: task.id,
                document_id: Some(task.document_id.clone()),
                corpus_id: task.knowledge_base_id,
                source_code: task.document_id,
                title,
                state: match task.state.as_str() {
                    "READY" => "COMPLETED",
                    "WAITING_EMBEDDING" | "WAITING_RETRY" => "QUEUED",
                    other => other,
                }
                .into(),
                stage: task
                    .index_stage
                    .unwrap_or_else(|| format!("{:?}", task.stage).to_ascii_uppercase()),
                completed_units: task.completed_units,
                total_units: task.total_units,
                error: task.error,
                created_at: task.created_at,
                updated_at: task.updated_at,
            })
            .collect())
    }
}
