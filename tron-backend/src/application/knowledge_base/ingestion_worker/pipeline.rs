//! Parse/chunk checkpoints and whole-generation vector publication.
use super::*;

impl KnowledgeBaseService {
    pub(super) async fn process_task(
        &self,
        mut task: IngestionTask,
    ) -> Result<(), KnowledgeBaseError> {
        let owned = lease(&task);
        let mut kb = self.owned(
            &task.user_code,
            &task.workspace_code,
            &task.knowledge_base_id,
        )?;
        let mut doc = self.repository.document(&kb.id, &task.document_id)?;
        // The existing task identity includes the original resource code. A recreated
        // same-content document may reuse id/revision, but never its original resource.
        let identity = hash(&format!("{}:{}:{}", doc.resource_code, doc.resource_revision, task.configuration));
        if doc.revision != task.document_revision || doc.resource_revision != task.resource_revision
            || identity != task.input_config_hash
        {
            return Err(KnowledgeBaseError::Conflict);
        }
        kb.default_chunk_config = serde_json::from_value(task.configuration["chunkConfig"].clone())
            .map_err(invalid_storage)?;
        doc.chunk_config = Some(kb.default_chunk_config.clone());
        doc.parsing_tier = serde_json::from_value(task.configuration["parsingTier"].clone())
            .map_err(invalid_storage)?;
        if matches!(
            task.stage,
            IngestionStage::Parsing | IngestionStage::Chunking
        ) {
            let task_parser = self
                .parser
                .as_ref()
                .map(|parser| parser.task_snapshot(&task.configuration["parser"]))
                .transpose()?
                .flatten();
            if !doc.file_name.to_ascii_lowercase().ends_with(".md")
                && task_parser.is_none()
                && self.parser.as_ref().map(|p| p.profile_identity())
                    != Some(task.configuration["parser"].clone())
            {
                return Err(KnowledgeBaseError::Invalid(
                    "解析节点或配置已改变，请重新入库".into(),
                ));
            }
            let checkpoint = Arc::new(RemoteCheckpoint {
                queue: self.queue.clone(),
                lease: owned.clone(),
                task: Mutex::new(task.clone()),
            });
            let mut service = self.clone();
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
            task = self.queue.get(&task.id).map_err(queue_error)?;
            task.stage = IngestionStage::Chunking;
            self.queue
                .checkpoint(&task, &owned, now_ms())
                .map_err(queue_error)?;
            let stored = StoredChunks {
                document: parsed,
                chunks,
                warnings,
            };
            let code = self.derived_resource(
                &task.user_code,
                &task.workspace_code,
                &doc,
                "ingestion-chunks.json",
                ResourceKind::File,
                &serde_json::to_vec(&stored).map_err(invalid_storage)?,
            )?;
            task.parsed_resource_code = Some(code.clone());
            task.chunks_resource_code = Some(code);
            self.queue
                .wait_for_embedding(&task, &owned, now_ms())
                .map_err(queue_error)?;
            return Ok(());
        }
        let stored: StoredChunks = self.read_checkpoint(
            &task,
            task.chunks_resource_code
                .as_deref()
                .ok_or_else(|| KnowledgeBaseError::Invalid("入库切分检查点缺失".into()))?,
        )?;
        let model = task.configuration["embeddingModel"]
            .as_str()
            .ok_or_else(|| {
                KnowledgeBaseError::Invalid(
                    task.configuration["embeddingError"]
                        .as_str()
                        .unwrap_or("Embedding模型未配置")
                        .into(),
                )
            })?;
        let expected = task.configuration["embeddingFingerprint"]
            .as_str()
            .ok_or_else(|| KnowledgeBaseError::Invalid("Embedding空间未配置".into()))?;
        let mut embedded: StoredEmbeddings = match &task.embedding_resource_code {
            Some(code) => self.read_checkpoint(&task, code)?,
            None => Default::default(),
        };
        if !embedded.vector_resources.is_empty() {
            if !embedded.vectors.is_empty() || embedded.vector_resources.len() > stored.chunks.len()
            {
                return Err(KnowledgeBaseError::Invalid("向量检查点不匹配".into()));
            }
            for code in &embedded.vector_resources {
                embedded.vectors.push(self.read_checkpoint(&task, code)?);
            }
        }
        if embedded.vectors.len() > stored.chunks.len() {
            return Err(KnowledgeBaseError::Invalid("向量检查点不匹配".into()));
        }
        if embedded
            .vectors
            .iter()
            .any(|v| v.len() != TRON_VECTOR_INDEX_DIMENSION || v.iter().any(|n| !n.is_finite()))
        {
            return Err(KnowledgeBaseError::Invalid(
                "向量检查点维度或数值无效，整份索引未发布".into(),
            ));
        }
        for chunk in stored.chunks.iter().skip(embedded.vectors.len()) {
            self.verify_embedding_space(&task.workspace_code, model, expected)?;
            let value = self
                .llm
                .embedding_call(LlmEmbeddingCallCommand {
                    workspace_code: task.workspace_code.clone(),
                    model_code: Some(model.into()),
                    input: chunk.text.clone(),
                })
                .await
                .map_err(invalid_storage)?;
            if value.model_code != model
                || value.embedding.len() != TRON_VECTOR_INDEX_DIMENSION
                || value.embedding.iter().any(|n| !n.is_finite())
            {
                return Err(KnowledgeBaseError::Invalid(format!(
                    "Embedding模型或维度不匹配（返回{}维，要求1024维），整份索引未发布",
                    value.embedding.len()
                )));
            }
            if embedded
                .provider_model
                .as_ref()
                .is_some_and(|old| value.provider_model.as_ref().is_some_and(|new| old != new))
            {
                return Err(KnowledgeBaseError::Invalid(
                    "实际Embedding模型发生变化，整份索引未发布".into(),
                ));
            }
            if value.provider_model.is_some() {
                embedded.provider_model = value.provider_model;
            }
            embedded.vectors.push(value.embedding);
            // Each vector is immutable and written once. Only its small manifest
            // grows; do not archive another complete vector array per chunk.
            for vector in embedded
                .vectors
                .iter()
                .skip(embedded.vector_resources.len())
            {
                embedded.vector_resources.push(self.derived_resource(
                    &task.user_code,
                    &task.workspace_code,
                    &doc,
                    "ingestion-vector.json",
                    ResourceKind::File,
                    &serde_json::to_vec(vector).map_err(invalid_storage)?,
                )?);
            }
            let manifest = StoredEmbeddings {
                vectors: vec![],
                vector_resources: embedded.vector_resources.clone(),
                provider_model: embedded.provider_model.clone(),
            };
            task.embedding_resource_code = Some(self.derived_resource(
                &task.user_code,
                &task.workspace_code,
                &doc,
                "ingestion-embeddings.json",
                ResourceKind::File,
                &serde_json::to_vec(&manifest).map_err(invalid_storage)?,
            )?);
            self.queue
                .checkpoint(&task, &owned, now_ms())
                .map_err(queue_error)?;
        }
        self.verify_embedding_space(&task.workspace_code, model, expected)?;
        let source = task.generation_source_code();
        let units = stored.chunks.iter().zip(embedded.vectors).map(|(chunk, vector)| RagTextUnitWrite {
            text_unit_code: format!("{}_{}", source, chunk.ordinal), unit_type: "LEAF".into(), text: chunk.text.clone(), search_text: chunk.text.clone(), heading_path: serde_json::to_string(&chunk.heading_path).unwrap_or_default(), ordinal: chunk.ordinal, token_count: chunk.text.chars().count().div_ceil(4) as i64, content_hash: hash(&chunk.text), status: "ACTIVE".into(), fulltext_status: "INDEXED".into(), embedding_model_code: Some(model.into()), embedding_version: Some("v1".into()), embedding_text_hash: Some(hash(&chunk.text)), embedding_dimension: Some(TRON_VECTOR_INDEX_DIMENSION as i64), embedding: Some(vector.into_iter().flat_map(f32::to_le_bytes).collect()), embedding_status: "INDEXED".into(), metadata: json!({"location":chunk.location,"documentId":doc.id,"resourceCode":doc.resource_code,"resourceRevision":doc.resource_revision})
        }).collect();
        task.stage = IngestionStage::Publishing;
        self.queue
            .checkpoint(&task, &owned, now_ms())
            .map_err(queue_error)?;
        // Synchronize the final staging write/publication with deletion fencing.
        // No await occurs while this synchronous gate is held.
        let _guard = self.cleanup_gate.lock().map_err(invalid_storage)?;
        let current_task = self.queue.get(&task.id).map_err(queue_error)?;
        if current_task.state != "RUNNING" || current_task.attempt != owned.attempt || current_task.lease_owner.as_deref() != Some(&owned.owner) { return Err(KnowledgeBaseError::Conflict); }
        self.rag.repository_handle().persist_ingest(RagPersistIngestCommand { corpus_id: kb.id.clone(), profile_id: format!("{}_local", kb.id), source_code: source.clone(), source_version: "v1".into(), scope: RagIngestScope { scope_type: "SPACE".into(), scope_code: task.workspace_code.clone() }, source_status: "STAGING".into(), title: Some(doc.name.clone()), content_hash: hash(&source), body_hash: None, metadata: json!({"documentId":doc.id,"resourceCode":doc.resource_code,"resourceRevision":doc.resource_revision}), domain_fields: BTreeMap::new(), related_graph: None, graph_write_policy: json!({}), text_units: units, tree_nodes: vec![], warnings: vec![], workspace_code: task.workspace_code.clone(), user_code: task.user_code.clone(), agent_code: String::new() })?;
        self.verify_embedding_space(&task.workspace_code, model, expected)?;
        let mut ready = stored.document;
        ready.active_source_code = Some(source);
        ready.index_revision = doc.index_revision + 1;
        ready.chunk_count = stored.chunks.len();
        ready.embedding_model_code = Some(model.into());
        ready.embedding_dimension = Some(TRON_VECTOR_INDEX_DIMENSION);
        ready.embedding_provider_model = embedded.provider_model;
        ready.embedding_space_fingerprint = Some(expected.into());
        ready.warnings = stored.warnings;
        self.queue
            .publish(
                &owned,
                now_ms(),
                &serde_json::to_value(ready).map_err(invalid_storage)?,
            )
            .map_err(queue_error)
    }
    fn verify_embedding_space(
        &self,
        workspace: &str,
        model: &str,
        expected: &str,
    ) -> Result<(), KnowledgeBaseError> {
        let actual = self
            .llm
            .embedding_space_fingerprint(workspace, Some(model), TRON_VECTOR_INDEX_DIMENSION)
            .map_err(invalid_storage)?;
        if actual != expected {
            return Err(KnowledgeBaseError::Invalid(
                "Embedding模型配置已改变，整份索引未发布；请重建".into(),
            ));
        }
        Ok(())
    }
    fn read_checkpoint<T: DeserializeOwned>(
        &self,
        task: &IngestionTask,
        code: &str,
    ) -> Result<T, KnowledgeBaseError> {
        let (_, record) = self.resources.download_binary_to_file(
            &task.user_code,
            ResourceBinaryDownloadCommand {
                workspace_code: task.workspace_code.clone(),
                resource_code: code.into(),
            },
        )?;
        let mut bytes = Vec::new();
        self.resources
            .download_binary_storage(&record, &mut bytes)?;
        if bytes.len() > local_documents::MAX_EXPANDED_BYTES {
            return Err(KnowledgeBaseError::Invalid("入库检查点超过上限".into()));
        }
        serde_json::from_slice(&bytes).map_err(invalid_storage)
    }
}
