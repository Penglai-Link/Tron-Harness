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
            RemoteCheckpoint, StoredChunks, lease, now_ms, pipeline::VectorCheckpoint, queue_error,
        },
        invalid_storage, *,
    },
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

// The graph stage includes paid node embeddings. Preserve the SDK's transient
// classification so the durable worker retries from its saved graph/vectors;
// deterministic graph validation and model-space errors still need correction.
fn graph_stage_error(error: String) -> KnowledgeBaseError {
    use eliteforge_agent_core::{ModelError, ProviderErrorCategory};
    let transient = match eliteforge_agent_official_plugin_model_http::decode_model_error(&error) {
        Some(ModelError::Transient(_)) => true,
        Some(ModelError::Provider(detail)) => detail.category == ProviderErrorCategory::Transient,
        _ => false,
    };
    if transient {
        KnowledgeBaseError::Storage(error)
    } else {
        KnowledgeBaseError::Invalid(error)
    }
}

fn index_preparation_error(error: String) -> KnowledgeBaseError {
    if error.starts_with("SDK_MODEL_ERROR:") {
        // A timeout after dispatch is indeterminate, not permission to repeat a
        // potentially billed call. Only typed transient failures auto-retry.
        return graph_stage_error(error);
    }
    if error.contains("取消") || error.contains("租约") {
        KnowledgeBaseError::Conflict
    } else if [
        "维度",
        "模型配置",
        "索引配置",
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
}

#[cfg(test)]
mod retry_classification_tests {
    use super::*;
    use eliteforge_agent_core::{ModelError, ProviderErrorCategory, ProviderModelError};

    #[test]
    fn node_embedding_retries_typed_transient_failures_but_not_invalid_graphs_or_accounts() {
        for (category, status, retryable) in [
            (ProviderErrorCategory::Transient, 503, true),
            (ProviderErrorCategory::Authentication, 401, false),
            (ProviderErrorCategory::Permission, 403, false),
            (ProviderErrorCategory::Quota, 429, false),
        ] {
            let wire = eliteforge_agent_official_plugin_model_http::encode_model_error(
                &ModelError::Provider(Box::new(ProviderModelError {
                    authorization_attempt: None,
                    provider: "test-provider".into(),
                    category,
                    http_status: Some(status),
                    business_code: None,
                    error_type: None,
                    model: None,
                    message: "model request failed".into(),
                })),
            );
            for classify in [graph_stage_error, index_preparation_error] {
                let error = classify(wire.clone());
                assert_eq!(matches!(error, KnowledgeBaseError::Storage(_)), retryable);
                assert_eq!(
                    error.to_string(),
                    wire,
                    "preserve diagnostics for the durable task"
                );
            }
        }
        let timeout = eliteforge_agent_official_plugin_model_http::encode_model_error(
            &ModelError::Indeterminate("model invocation timed out".into()),
        );
        assert!(matches!(
            index_preparation_error(timeout),
            KnowledgeBaseError::Invalid(_)
        ));
        let unexpected_tools = eliteforge_agent_official_plugin_model_http::encode_model_error(
            &ModelError::Fatal("text generation unexpectedly requested tool calls".into()),
        );
        assert!(matches!(
            index_preparation_error(unexpected_tools),
            KnowledgeBaseError::Invalid(_)
        ));
        assert!(matches!(
            index_preparation_error("SDK_MODEL_ERROR:{bad-json}".into()),
            KnowledgeBaseError::Invalid(_)
        ));
        for error in [
            "图谱关系缺少原文依据",
            "图谱节点向量空间已改变",
            "SDK_MODEL_ERROR:{bad-json}",
            "untrusted text says HTTP 503",
        ] {
            assert!(matches!(
                graph_stage_error(error.into()),
                KnowledgeBaseError::Invalid(_)
            ));
        }
    }
}
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
        if !task.configuration["chunkStrategy"].is_null() {
            let expected: eliteforge_agent_core::ChunkStrategyDescriptor =
                serde_json::from_value(task.configuration["chunkStrategy"].clone())
                    .map_err(invalid_storage)?;
            if !host
                .chunk_strategies
                .descriptors()
                .iter()
                .any(|s| s == &expected)
            {
                return Err(KnowledgeBaseError::Invalid(
                    "切片策略实现已改变，请重新提交索引任务".into(),
                ));
            }
        }
        doc.parsing_tier = serde_json::from_value(task.configuration["parsingTier"].clone())
            .map_err(invalid_storage)?;
        doc.ocr_mode = serde_json::from_value(task.configuration["ocrMode"].clone())
            .map_err(invalid_storage)?;
        doc.parse_plan = serde_json::from_value(task.configuration["parsePlan"].clone())
            .map_err(invalid_storage)?;
        if matches!(
            task.stage,
            IngestionStage::Parsing | IngestionStage::Chunking
        ) {
            let task_parser = if task.configuration["reuseParsedOnly"] == true {
                None
            } else {
                host.parser
                    .as_ref()
                    .map(|parser| parser.task_snapshot(&task.configuration["parser"]))
                    .transpose()?
                    .flatten()
            };
            if task.configuration["reuseParsedOnly"] != true
                && !doc
                    .parse_plan
                    .as_ref()
                    .is_some_and(|p| p.features.local_text)
                && !doc.file_name.to_ascii_lowercase().ends_with(".md")
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
            let (chunks, contexts, warnings, parsed) = service
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
                contexts,
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
        let mut stored: StoredChunks = host.read_checkpoint(
            &task,
            task.chunks_resource_code
                .as_deref()
                .ok_or_else(|| KnowledgeBaseError::Invalid("入库切分检查点缺失".into()))?,
        )?;
        let settings: RagSettings =
            serde_json::from_value(task.configuration["ragSettings"].clone())
                .map_err(invalid_storage)?;
        // A failed preflight is not a resolved model identity. Retry resolution
        // under the lease, before any vectors exist, without parsing again.
        // Once pinned, the concrete model and space survive every retry.
        if (settings.vector || settings.graph)
            && (task.embedding_model_code().is_none() || task.embedding_fingerprint().is_none())
        {
            if task.embedding_resource_code.is_some() {
                return Err(KnowledgeBaseError::Invalid(
                    "向量检查点缺少模型身份，不能安全续跑".into(),
                ));
            }
            let requested = task
                .embedding_model_code()
                .or(settings.embedding_model_code.as_deref());
            let model = host
                .llm
                .resolve_embedding_index_model_code(&task.workspace_code, requested)
                .map_err(|error| KnowledgeBaseError::Invalid(error.to_string()))?;
            let fingerprint = host
                .llm
                .embedding_space_fingerprint(
                    &task.workspace_code,
                    Some(&model),
                    settings.embedding_dimension,
                )
                .map_err(|error| KnowledgeBaseError::Invalid(error.to_string()))?;
            task.embedding_identity = Some(IngestionEmbeddingIdentity {
                model_code: model,
                fingerprint,
            });
            host.queue
                .checkpoint(&task, &owned, now_ms())
                .map_err(queue_error)?;
        }
        let pinned_model = task.embedding_model_code().map(str::to_owned);
        let pinned_fingerprint = task.embedding_fingerprint().map(str::to_owned);
        let model = pinned_model
            .as_deref()
            .or_else(|| (!settings.vector && !settings.graph).then_some(""))
            .ok_or_else(|| {
                KnowledgeBaseError::Invalid(
                    task.configuration["embeddingError"]
                        .as_str()
                        .unwrap_or("Embedding模型未配置")
                        .into(),
                )
            })?;
        let expected = pinned_fingerprint
            .as_deref()
            .or_else(|| (!settings.vector && !settings.graph).then_some(""))
            .ok_or_else(|| KnowledgeBaseError::Invalid("Embedding空间未配置".into()))?;
        let image_vectors = if settings.vector {
            match task.configuration["embeddingImages"].as_bool() {
                Some(known) => known,
                None => host
                    .llm
                    .embedding_accepts_images(&task.workspace_code, model)
                    .map_err(|error| KnowledgeBaseError::Invalid(error.to_string()))?,
            }
        } else {
            false
        };
        if !image_vectors
            && stored
                .chunks
                .iter()
                .any(|chunk| chunk.location.block_kind.as_deref() == Some("image_vector"))
        {
            if task.embedding_resource_code.is_some() {
                return Err(KnowledgeBaseError::Invalid(
                    "向量检查点的图片输入方式已改变，不能安全续跑".into(),
                ));
            }
            stored
                .chunks
                .retain(|chunk| chunk.location.block_kind.as_deref() != Some("image_vector"));
            task.total_units = stored.chunks.len();
            task.chunks_resource_code = Some(host.derived_resource(
                &task.user_code,
                &task.workspace_code,
                &doc,
                "ingestion-index-inputs.json",
                ResourceKind::File,
                &serde_json::to_vec(&stored).map_err(invalid_storage)?,
            )?);
            host.queue
                .checkpoint(&task, &owned, now_ms())
                .map_err(queue_error)?;
        }
        let mut embedded = host.restore_embeddings(&task, stored.chunks.len()).await?;
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
            text_unit_code: format!("{}_{}",source,chunk.ordinal), unit_type:if chunk.location.block_kind.as_deref()==Some("image_vector") {"IMAGE"} else {"LEAF"}.into(),
            text:chunk.text.clone(), search_text:if chunk.index_text.is_empty() {chunk.text.clone()} else {chunk.index_text.clone()},
            heading_path:serde_json::to_string(&chunk.heading_path).unwrap_or_default(), ordinal:chunk.ordinal,
            token_count:chunk.text.chars().count().div_ceil(4) as i64, content_hash:hash(&chunk.text), status:"ACTIVE".into(),
            fulltext_status:String::new(), embedding_model_code:None, embedding_version:None, embedding_text_hash:None,
            embedding_dimension:None, embedding:None, embedding_status:String::new(),
            metadata:json!({"location":chunk.location,"documentId":doc.id,"resourceCode":doc.resource_code,"resourceRevision":doc.resource_revision,"fileName":doc.file_name,"sourceFactsVersion":"normalized-blocks-v3","sourceSpans":chunk.source_spans,"sourceLocations":chunk.source_locations,"contextId":chunk.context_id}),
        }).collect();
        let mut index_settings = settings.clone();
        index_settings.embedding_model_code =
            (settings.vector || settings.graph).then(|| model.to_owned());
        let inputs = if settings.vector {
            Some(host.index_embedding_inputs(
                &task.user_code,
                &task.workspace_code,
                &stored.chunks,
                model,
            )?)
        } else {
            None
        };
        let cached =
            if settings.vector && doc.embedding_space_fingerprint.as_deref() == Some(expected) {
                match doc.active_source_code.as_deref() {
                    Some(source) => self.repository_handle().cached_vectors(
                        &kb.id,
                        source,
                        &task.workspace_code,
                        expected,
                        settings.embedding_dimension,
                    )?,
                    None => Default::default(),
                }
            } else {
                Default::default()
            };
        if embedded.provider_model.is_none() && !cached.is_empty() {
            embedded.provider_model = doc.embedding_provider_model.clone();
        }
        let cached_graphs = Default::default();
        let graph_context = crate::application::rag::graph::GraphIndexContext {
            partition: crate::domain::rag::graph::GraphPartition {
                corpus_id: kb.id.clone(),
                scope_type: "SPACE".into(),
                scope_code: task.workspace_code.clone(),
            },
            logical_source_code: doc.id.clone(),
            source_code: source.clone(),
            source_version: "v1".into(),
            task_id: task.id.clone(),
            configuration_hash: task.input_config_hash.clone(),
        };
        let graph_checkpoint = crate::application::rag::graph::GraphComputationCheckpoint::new(
            self.graph_infrastructure(),
            &checkpoint,
            &graph_context,
            &index_settings,
        )
        .map_err(KnowledgeBaseError::Invalid)?;
        let prepared = self
            .prepare_index_with_content(
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
                &graph_checkpoint,
                inputs,
                &cached,
                &cached_graphs,
            )
            .await
            .map_err(index_preparation_error)?;
        let graph_id = self
            .stage_graph_generation(
                &task.workspace_code,
                &index_settings,
                &graph_context,
                &prepared,
                &graph_checkpoint,
            )
            .await
            .map_err(graph_stage_error)?;
        graph_checkpoint
            .stage("PUBLISHING")
            .map_err(KnowledgeBaseError::Invalid)?;
        task = checkpoint.task.lock().map_err(invalid_storage)?.clone();
        embedded.provider_model = prepared.vectors.provider_model;
        let units = prepared.units;
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
            self.repository_handle().persist_ingest(RagPersistIngestCommand { corpus_id: kb.id.clone(), task_id: task.id.clone(), source_code: source.clone(), source_version: "v1".into(), scope: RagIngestScope { scope_type: "SPACE".into(), scope_code: task.workspace_code.clone() }, source_status: "STAGING".into(), title: Some(doc.name.clone()), content_hash: hash(&source), body_hash: None, metadata: json!({"documentId":doc.id,"resourceCode":doc.resource_code,"resourceRevision":doc.resource_revision,"embeddingProviderModel":embedded.provider_model,"embeddingFingerprint":expected,"indexSettings":settings,"graphGenerationId":graph_id,"graphConfigurationHash":task.input_config_hash,"logicalSourceCode":doc.id}), domain_fields: BTreeMap::new(), text_units: units, tree_nodes: vec![], warnings: vec![], workspace_code: task.workspace_code.clone(), user_code: task.user_code.clone(), agent_code: String::new() })?;
            host.repository.save_chunk_contexts(
                &kb.id,
                &doc.id,
                &source,
                crate::adapters::outbound::persistence::sqlite::stored_contexts(
                    &stored.contexts,
                    &stored.chunks,
                ),
            )?;
            if settings.vector || settings.graph {
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
        let mut seen = std::collections::BTreeSet::new();
        Ok(self
            .queue
            .index_tasks_for_user(user)
            .map_err(queue_error)?
            .into_iter()
            .map(|(task, title)| RagTaskView {
                is_latest: seen.insert((task.knowledge_base_id.clone(), task.document_id.clone())),
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
