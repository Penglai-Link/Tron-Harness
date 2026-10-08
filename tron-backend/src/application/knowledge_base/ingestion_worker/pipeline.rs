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
            return Err(KnowledgeBaseError::Invalid(
                "Embedding模型配置已改变，整份索引未发布；请重建".into(),
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
            return Err(KnowledgeBaseError::Invalid("入库检查点超过上限".into()));
        }
        serde_json::from_slice(&bytes).map_err(invalid_storage)
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
            return Err("索引任务已取消或租约失效".into());
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
        for vector in &result.vectors {
            let code = self
                .service
                .derived_resource(
                    &task.user_code,
                    &task.workspace_code,
                    self.document,
                    "ingestion-vector.json",
                    ResourceKind::File,
                    &serde_json::to_vec(&vector.values).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            next_resources.push(code);
        }
        next.completed_units = next_resources.len();
        next.updated_at = now_ms();
        let manifest = StoredEmbeddings {
            vectors: vec![],
            vector_resources: next_resources.clone(),
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
