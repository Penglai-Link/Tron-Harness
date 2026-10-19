//! Structured sources enter the same fixed index preparation and publication flow as documents.
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Mutex;

use super::{
    RagConfigRepository, RagEmbeddingBatchResult, RagEmbeddingRuntime, RagIndexCheckpoint,
    RagIngestRepository, RagIngestRequest, RagIngestResult, RagPersistIngestCommand, RagRecordTask,
    RagService, RagSettings, RagTextUnitWrite, validate_direct_scope,
};
use crate::domain::rag::RagError;

impl<R, E> RagService<R, E>
where
    R: RagIngestRepository + Clone + Send + Sync,
    E: RagEmbeddingRuntime + super::RagGraphRuntime,
{
    pub(super) async fn ingest_record(
        &self,
        request: RagIngestRequest,
    ) -> Result<RagIngestResult, RagError> {
        self.ingest_record_with_checkpoint(request, None).await
    }
    async fn ingest_record_with_checkpoint(
        &self,
        request: RagIngestRequest,
        resume: Option<super::RagRecordCheckpoint>,
    ) -> Result<RagIngestResult, RagError> {
        let task = self.prepare_record_task(request, resume)?;
        self.execute_record_task(task).await
    }
    fn prepare_record_task(
        &self,
        request: RagIngestRequest,
        resume: Option<super::RagRecordCheckpoint>,
    ) -> Result<RagRecordTask, RagError> {
        validate_direct_scope(&request.access_context, &request.target_scope)?;
        if !matches!(request.corpus_id.as_str(), "memory-card" | "skill") {
            return Err(RagError::IngestRequestInvalid);
        }
        let text = request.payload["text"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or(RagError::IngestRequestInvalid)?;
        let corpus = self
            .repository
            .get_corpus(&request.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        let settings = match &resume {
            Some(checkpoint) => checkpoint.settings.clone(),
            None => RagSettings::from_capabilities(&corpus.capabilities)?,
        };
        if resume
            .as_ref()
            .is_some_and(|checkpoint| checkpoint.content_hash != hash(text))
        {
            return Err(RagError::IngestRequestInvalid);
        }
        settings.validate()?;
        let now = chrono::Utc::now().timestamp_millis();
        let task = RagRecordTask {
            id: format!("rag_task_{}", new_id()),
            corpus_id: request.corpus_id.clone(),
            user_code: request.access_context.user_code.clone(),
            workspace_code: request.workspace_code.clone(),
            source_code: request.source.code.clone(),
            source_version: request.source.version.clone(),
            scope_type: request.target_scope.scope_type.clone(),
            scope_code: request.target_scope.scope_code.clone(),
            payload: request.payload.clone(),
            checkpoint: Some(resume.unwrap_or_else(|| super::RagRecordCheckpoint {
                settings: settings.clone(),
                content_hash: hash(text),
                vectors: Default::default(),
            })),
            state: "QUEUED".into(),
            stage: "CHUNKING".into(),
            completed_units: 0,
            total_units: super::split_document(&normalized(text), &Default::default())
                .map_err(|_| RagError::IngestRequestInvalid)?
                .len(),
            error: None,
            created_at: now,
            updated_at: now,
        };
        self.repository.save_record_task(&task)?;
        Ok(task)
    }
    pub async fn execute_record_task(
        &self,
        mut task: RagRecordTask,
    ) -> Result<RagIngestResult, RagError> {
        let current = self
            .repository
            .record_task(&task.user_code, &task.id)?
            .ok_or(RagError::SourceNotFound)?;
        if current.state != "QUEUED" {
            return Err(RagError::IngestRequestInvalid);
        }
        let request = request_for_task(&task);
        validate_direct_scope(&request.access_context, &request.target_scope)?;
        let text = request.payload["text"]
            .as_str()
            .ok_or(RagError::IngestRequestInvalid)?;
        let settings = task
            .checkpoint
            .as_ref()
            .ok_or(RagError::IngestRequestInvalid)?
            .settings
            .clone();
        if !self
            .repository
            .claim_record_task(&task.user_code, &task.id)?
        {
            return Err(RagError::IngestRequestInvalid);
        }
        task.state = "RUNNING".into();
        task.updated_at = chrono::Utc::now().timestamp_millis();
        self.repository.save_record_task(&task)?;
        let checkpoint = RecordCheckpoint {
            repository: self.repository.clone(),
            task: Mutex::new(task),
        };
        let outcome = self
            .index_record(&request, text, &settings, &checkpoint)
            .await;
        let mut task = checkpoint
            .task
            .lock()
            .map_err(|_| RagError::StorageFailure)?
            .clone();
        match &outcome {
            Ok(_) => {
                task.state = "COMPLETED".into();
                task.stage = "PUBLISHING".into();
                task.completed_units = task.total_units;
            }
            Err(error) => {
                task.state = "FAILED".into();
                if task.error.is_none() {
                    task.error = Some(error.to_string());
                }
            }
        }
        task.updated_at = chrono::Utc::now().timestamp_millis();
        self.repository.save_record_task(&task)?;
        outcome
    }
    pub fn prepare_retry_record_task(
        &self,
        user: &str,
        id: &str,
        rebuild: bool,
    ) -> Result<RagRecordTask, RagError> {
        let task = self
            .repository
            .record_task(user, id)?
            .ok_or(RagError::SourceNotFound)?;
        if matches!(task.state.as_str(), "RUNNING" | "QUEUED")
            || !self.repository.record_task_is_latest(&task)?
        {
            return Err(RagError::IngestRequestInvalid);
        }
        self.prepare_record_task(
            request_for_task(&task),
            if rebuild { None } else { task.checkpoint },
        )
    }
    pub async fn retry_record_task(
        &self,
        user: &str,
        id: &str,
    ) -> Result<RagIngestResult, RagError> {
        let task = self.prepare_retry_record_task(user, id, false)?;
        self.execute_record_task(task).await
    }
    pub async fn rebuild_record_task(
        &self,
        user: &str,
        id: &str,
    ) -> Result<RagIngestResult, RagError> {
        let task = self.prepare_retry_record_task(user, id, true)?;
        self.execute_record_task(task).await
    }
    async fn index_record(
        &self,
        request: &RagIngestRequest,
        text: &str,
        settings: &RagSettings,
        checkpoint: &RecordCheckpoint<R>,
    ) -> Result<RagIngestResult, RagError> {
        let dimension = settings.embedding_dimension;
        checkpoint
            .validate()
            .map_err(|_| RagError::IngestRequestInvalid)?;
        let mut metadata = request
            .payload
            .get("metadata")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if !metadata.is_object() {
            return Err(RagError::IngestRequestInvalid);
        }
        let task = checkpoint
            .task
            .lock()
            .map_err(|_| RagError::StorageFailure)?
            .clone();
        metadata["indexTaskId"] = json!(task.id);
        metadata["indexSettings"] = json!(settings);
        metadata["embeddingModelCode"] = json!(settings.embedding_model_code);
        metadata["embeddingDimension"] = json!(dimension);
        metadata["embeddingProviderModel"] = serde_json::Value::Null;

        let domain_fields: BTreeMap<String, serde_json::Value> = request
            .payload
            .get("domainFields")
            .and_then(serde_json::Value::as_object)
            .map(|value| {
                value
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let chunks = super::split_document(&normalized(text), &Default::default())
            .map_err(|_| RagError::IngestRequestInvalid)?;
        let count = chunks.len();
        let units = chunks
            .into_iter()
            .map(|chunk| {
                let text = chunk.text.as_str();
                RagTextUnitWrite {
                    text_unit_code: format!(
                        "{}_{}_{}",
                        request.source.code, task.id, chunk.ordinal
                    ),
                    unit_type: "LEAF".into(),
                    text: text.into(),
                    search_text: text.into(),
                    heading_path: serde_json::to_string(&chunk.heading_path).unwrap_or_default(),
                    ordinal: chunk.ordinal,
                    token_count: text.chars().count().div_ceil(4) as i64,
                    content_hash: hash(text),
                    status: "ACTIVE".into(),
                    fulltext_status: if settings.fulltext {
                        "INDEXED"
                    } else {
                        "NOT_ENABLED"
                    }
                    .into(),
                    embedding_model_code: None,
                    embedding_version: None,
                    embedding_text_hash: None,
                    embedding_dimension: None,
                    embedding: None,
                    embedding_status: if settings.vector {
                        "INDEXED"
                    } else {
                        "NOT_ENABLED"
                    }
                    .into(),
                    metadata: json!({"location":chunk.location,"sourceCode":request.source.code,"sourceSpans":chunk.source_spans,"sourceLocations":chunk.source_locations,"sourceFactsVersion":"normalized-blocks-v3"}),
                }
            })
            .collect();
        let model_workspace = if request.corpus_id == "memory-card" {
            super::MEMORY_MODEL_WORKSPACE
        } else {
            request.workspace_code.as_str()
        };
        {
            let mut task = checkpoint
                .task
                .lock()
                .map_err(|_| RagError::StorageFailure)?;
            if let Some(saved) = task.checkpoint.as_mut() {
                if settings.vector && saved.vectors.vectors.is_empty() {
                    saved.vectors.fingerprint = self
                        .runtime
                        .embedding_fingerprint(
                            model_workspace,
                            settings.embedding_model_code.as_deref(),
                            dimension,
                        )
                        .map_err(RagError::EmbeddingFailure)?;
                }
            }
            self.repository.save_record_task(&task)?;
        }
        let completed = checkpoint
            .task
            .lock()
            .map_err(|_| RagError::StorageFailure)?
            .checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.vectors.clone())
            .unwrap_or_default();
        let graph_context = super::graph::GraphIndexContext {
            partition: crate::domain::rag::graph::GraphPartition {
                corpus_id: request.corpus_id.clone(),
                scope_type: request.target_scope.scope_type.clone(),
                scope_code: request.target_scope.scope_code.clone(),
            },
            logical_source_code: request.source.code.clone(),
            source_code: request.source.code.clone(),
            source_version: request.source.version.clone(),
            task_id: task.id.clone(),
            configuration_hash: hash(&format!(
                "{}:{}",
                hash(text),
                serde_json::to_string(settings).map_err(|_| RagError::IngestRequestInvalid)?
            )),
        };
        let graph_checkpoint = super::graph::GraphComputationCheckpoint::new(
            self.graph.as_ref(),
            checkpoint,
            &graph_context,
            settings,
        )
        .map_err(RagError::EmbeddingFailure)?;
        let prepared = self
            .prepare_index(
                model_workspace,
                settings,
                units,
                completed,
                request.payload.get("relatedGraph").cloned(),
                &graph_checkpoint,
            )
            .await
            .map_err(|error| {
                if let Ok(mut task) = checkpoint.task.lock() {
                    task.error = Some(error.clone());
                }
                RagError::EmbeddingFailure(error)
            })?;
        let graph_id = self
            .stage_graph_generation(
                model_workspace,
                settings,
                &graph_context,
                &prepared,
                &graph_checkpoint,
            )
            .await
            .map_err(RagError::EmbeddingFailure)?;
        graph_checkpoint
            .stage("PUBLISHING")
            .map_err(RagError::EmbeddingFailure)?;
        metadata["graphGenerationId"] = json!(graph_id);
        metadata["graphConfigurationHash"] = json!(graph_context.configuration_hash);
        metadata["logicalSourceCode"] = json!(request.source.code);
        metadata["embeddingModelCode"] = json!(prepared.vectors.model_code);
        metadata["embeddingProviderModel"] = json!(prepared.vectors.provider_model);
        metadata["embeddingFingerprint"] = json!(prepared.vectors.fingerprint);
        self.repository.persist_ingest(RagPersistIngestCommand {
            corpus_id: request.corpus_id.clone(),
            task_id: task.id.clone(),
            source_code: request.source.code.clone(),
            source_version: request.source.version.clone(),
            scope: request.target_scope.clone(),
            source_status: "ACTIVE".into(),
            title: request.payload["title"].as_str().map(str::to_owned),
            content_hash: hash(text),
            body_hash: None,
            metadata,
            domain_fields,
            text_units: prepared.units,
            tree_nodes: vec![],
            warnings: vec![],
            workspace_code: request.workspace_code.clone(),
            user_code: request.access_context.user_code.clone(),
            agent_code: request.access_context.agent_code.clone(),
        })?;
        Ok(RagIngestResult {
            corpus_id: request.corpus_id.clone(),
            source_code: request.source.code.clone(),
            source_version: request.source.version.clone(),
            status: "ACTIVE".into(),
            text_unit_count: count,
            fulltext_status: if settings.fulltext {
                "INDEXED"
            } else {
                "NOT_ENABLED"
            }
            .into(),
            embedding_status: if settings.vector {
                "INDEXED"
            } else {
                "NOT_ENABLED"
            }
            .into(),
            warnings: vec![],
        })
    }
}
struct RecordCheckpoint<R> {
    repository: R,
    task: Mutex<RagRecordTask>,
}
impl<R: RagConfigRepository> RecordCheckpoint<R> {
    fn set_stage(&self, stage: &str) -> Result<(), RagError> {
        let mut task = self.task.lock().map_err(|_| RagError::StorageFailure)?;
        task.stage = stage.into();
        task.updated_at = chrono::Utc::now().timestamp_millis();
        self.repository.save_record_task(&task)
    }
}
impl<R: RagConfigRepository + Send + Sync> RagIndexCheckpoint for RecordCheckpoint<R> {
    fn validate(&self) -> Result<(), String> {
        let task = self.task.lock().map_err(|error| error.to_string())?;
        let current = self
            .repository
            .record_task(&task.user_code, &task.id)
            .map_err(|error| error.to_string())?
            .ok_or("入库任务不存在")?;
        if current.state != "RUNNING"
            || !self
                .repository
                .record_task_is_latest(&current)
                .map_err(|e| e.to_string())?
        {
            return Err("入库任务已取消、被新任务替代或状态已改变".into());
        }
        let corpus = self
            .repository
            .get_corpus(&task.corpus_id)
            .map_err(|e| e.to_string())?
            .ok_or("语料库不存在")?;
        let settings = super::RagSettings::from_capabilities(&corpus.capabilities)
            .map_err(|e| e.to_string())?;
        if current
            .checkpoint
            .as_ref()
            .is_none_or(|saved| saved.settings != settings)
        {
            return Err("索引配置已改变，请重新提交任务".into());
        }
        Ok(())
    }
    fn stage(&self, stage: &str) -> Result<(), String> {
        self.validate()?;
        self.set_stage(stage).map_err(|error| error.to_string())
    }
    fn save_batch(&self, result: &RagEmbeddingBatchResult) -> Result<(), String> {
        let mut task = self.task.lock().map_err(|error| error.to_string())?;
        let mut next = task.clone();
        let checkpoint = next.checkpoint.as_mut().ok_or("任务索引检查点缺失")?;
        checkpoint.vectors.model_code = Some(result.model_code.clone());
        checkpoint.vectors.provider_model = result.provider_model.clone();
        checkpoint
            .vectors
            .vectors
            .extend(result.vectors.iter().map(|vector| vector.values.clone()));
        next.completed_units = checkpoint.vectors.vectors.len();
        next.updated_at = chrono::Utc::now().timestamp_millis();
        self.repository
            .save_record_task(&next)
            .map_err(|error| error.to_string())?;
        *task = next;
        Ok(())
    }
}
fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn new_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn normalized(text: &str) -> crate::application::document::ParsedDocument {
    use crate::application::document::*;
    ParsedDocument {
        format: "STRUCTURED".into(),
        warnings: vec![],
        image_analysis: None,
        sections: vec![LocatedSection {
            text: text.into(),
            heading_level: None,
            location: DocumentLocation {
                char_end: text.chars().count(),
                ..Default::default()
            },
        }],
    }
}

fn request_for_task(task: &RagRecordTask) -> RagIngestRequest {
    RagIngestRequest {
        corpus_id: task.corpus_id.clone(),
        workspace_code: task.workspace_code.clone(),
        access_context: super::RagIngestAccessContext {
            user_code: task.user_code.clone(),
            workspace_code: task.workspace_code.clone(),
            agent_code: if task.scope_type == "AGENT" {
                task.scope_code.clone()
            } else {
                "rag-indexer".into()
            },
        },
        target_scope: super::RagIngestScope {
            scope_type: task.scope_type.clone(),
            scope_code: task.scope_code.clone(),
        },
        source: super::RagIngestSource {
            code: task.source_code.clone(),
            version: task.source_version.clone(),
        },
        payload: task.payload.clone(),
    }
}
