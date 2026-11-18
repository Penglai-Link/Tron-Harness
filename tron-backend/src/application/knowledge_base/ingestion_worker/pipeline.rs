//! Parse/chunk checkpoints and whole-generation vector publication.
use super::*;

impl KnowledgeBaseService {
    pub(super) async fn process_task(&self, task: IngestionTask) -> Result<(), KnowledgeBaseError> {
        self.rag.execute_document_task(self, task).await
    }
    pub(crate) fn verify_embedding_space(
        &self,
        workspace: &str,
        model: &str,
        expected: &str,
        dimension: usize,
    ) -> Result<(), KnowledgeBaseError> {
        let actual = self
            .llm
            .embedding_space_fingerprint(workspace, Some(model), dimension)
            .map_err(invalid_storage)?;
        if actual != expected {
            return Err(KnowledgeBaseError::invalid(
                "knowledge.base.embedding_config_changed_unpublished",
            ));
        }
        Ok(())
    }
    pub(crate) fn read_checkpoint<T: DeserializeOwned>(
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
            return Err(KnowledgeBaseError::invalid(
                "knowledge.base.checkpoint_limit_exceeded",
            ));
        }
        serde_json::from_slice(&bytes).map_err(invalid_storage)
    }

    pub(crate) async fn restore_embeddings(
        &self,
        task: &IngestionTask,
        max_units: usize,
    ) -> Result<StoredEmbeddings, KnowledgeBaseError> {
        // Large resumptions can read thousands of local checkpoint files. Keep
        // catalog resolution batched and CPU/file work outside the async worker
        // so execute_claimed can continue renewing the lease while restoring.
        let service = self.clone();
        let task = task.clone();
        tokio::task::spawn_blocking(move || {
            let mut saved: StoredEmbeddings = match &task.embedding_resource_code {
                Some(code) => service.read_checkpoint(&task, code)?,
                None => return Ok(StoredEmbeddings::default()),
            };
            if saved.vector_resources.is_empty() {
                return Ok(saved);
            }
            if !saved.vectors.is_empty() || saved.completed_units > max_units {
                return Err(KnowledgeBaseError::invalid(
                    "knowledge.base.vector_checkpoint_mismatch",
                ));
            }
            let records = service.resources.download_binary_records(
                &task.user_code,
                &task.workspace_code,
                &saved.vector_resources,
            )?;
            for record in records {
                let mut bytes = Vec::new();
                service
                    .resources
                    .download_binary_storage(&record, &mut bytes)?;
                if bytes.len() > local_documents::MAX_EXPANDED_BYTES {
                    return Err(KnowledgeBaseError::invalid(
                        "knowledge.base.checkpoint_limit_exceeded",
                    ));
                }
                let batch: Vec<Vec<f32>> =
                    serde_json::from_slice(&bytes).map_err(invalid_storage)?;
                if batch.len() > saved.completed_units.saturating_sub(saved.vectors.len()) {
                    return Err(KnowledgeBaseError::invalid(
                        "knowledge.base.vector_checkpoint_count_mismatch",
                    ));
                }
                saved.vectors.extend(batch);
            }
            if saved.vectors.len() != saved.completed_units {
                return Err(KnowledgeBaseError::invalid(
                    "knowledge.base.vector_checkpoint_count_mismatch",
                ));
            }
            Ok(saved)
        })
        .await
        .map_err(invalid_storage)?
    }
}

pub(crate) struct VectorCheckpoint<'a> {
    pub(crate) dimension: usize,
    pub(crate) service: &'a KnowledgeBaseService,
    pub(crate) task: Mutex<IngestionTask>,
    pub(crate) lease: &'a IngestionLease,
    pub(crate) model: &'a str,
    pub(crate) fingerprint: &'a str,
    pub(crate) resources: Mutex<Vec<String>>,
    pub(crate) provider_model: Mutex<Option<String>>,
    pub(crate) document: &'a KnowledgeBaseDocument,
}
impl RagIndexCheckpoint for VectorCheckpoint<'_> {
    fn validate(&self) -> Result<(), String> {
        let task = self.task.lock().map_err(|error| error.to_string())?;
        let current = self
            .service
            .queue
            .get(&task.id)
            .map_err(|error| queue_error(error).to_string())?;
        if current.state != "RUNNING"
            || current.attempt != self.lease.attempt
            || current.lease_owner.as_deref() != Some(&self.lease.owner)
        {
            return Err("knowledge.base.index_cancelled_lease_lost".into());
        }
        let settings = self
            .service
            .rag
            .settings_for_corpus(&task.knowledge_base_id)
            .map_err(|error| error.to_string())?;
        if serde_json::to_value(settings).map_err(|error| error.to_string())?
            != task.configuration["ragSettings"]
        {
            return Err("knowledge.base.index_config_changed_rebuild".into());
        }
        if self.model.is_empty() {
            return Ok(());
        }
        self.service
            .verify_embedding_space(
                &task.workspace_code,
                self.model,
                self.fingerprint,
                self.dimension,
            )
            .map_err(|error| error.to_string())
    }
    fn stage(&self, stage: &str) -> Result<(), String> {
        self.validate()?;
        let mut task = self.task.lock().map_err(|error| error.to_string())?;
        task.index_stage = Some(stage.into());
        task.updated_at = now_ms();
        self.service
            .queue
            .checkpoint(&task, self.lease, now_ms())
            .map_err(|error| queue_error(error).to_string())
    }
    fn save_batch(&self, result: &RagEmbeddingBatchResult) -> Result<(), String> {
        let mut task = self.task.lock().map_err(|error| error.to_string())?;
        let mut resources = self.resources.lock().map_err(|error| error.to_string())?;
        let mut provider = self
            .provider_model
            .lock()
            .map_err(|error| error.to_string())?;
        let mut next = task.clone();
        let mut next_resources = resources.clone();
        let batch = result
            .vectors
            .iter()
            .map(|vector| &vector.values)
            .collect::<Vec<_>>();
        let code = self
            .service
            .derived_resource(
                &task.user_code,
                &task.workspace_code,
                self.document,
                "ingestion-vector-batch.json",
                ResourceKind::File,
                &serde_json::to_vec(&batch).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        next_resources.push(code);
        next.completed_units += result.vectors.len();
        next.updated_at = now_ms();
        let manifest = StoredEmbeddings {
            vectors: vec![],
            vector_resources: next_resources.clone(),
            completed_units: next.completed_units,
            provider_model: result.provider_model.clone(),
        };
        next.embedding_resource_code = Some(
            self.service
                .derived_resource(
                    &task.user_code,
                    &task.workspace_code,
                    self.document,
                    "ingestion-embeddings.json",
                    ResourceKind::File,
                    &serde_json::to_vec(&manifest).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?,
        );
        // The lease-fenced queue update commits the entire manifest in one transaction.
        self.service
            .queue
            .checkpoint(&next, self.lease, now_ms())
            .map_err(|error| queue_error(error).to_string())?;
        *task = next;
        *resources = next_resources;
        *provider = result.provider_model.clone();
        Ok(())
    }
}
