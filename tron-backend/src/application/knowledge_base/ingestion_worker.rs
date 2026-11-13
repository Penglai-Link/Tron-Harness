//! Durable product ingestion. Model calls continue through the existing Framework adapter.
use super::{ingestion::*, *};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;
pub(crate) mod pipeline;
mod recovery;

#[derive(Default)]
pub(super) struct IngestionWorkers {
    started: AtomicBool,
    stop: CancellationToken,
}
pub(crate) fn queue_error(error: IngestionQueueError) -> KnowledgeBaseError {
    match error {
        IngestionQueueError::NotFound => KnowledgeBaseError::NotFound,
        IngestionQueueError::Conflict => KnowledgeBaseError::Conflict,
        IngestionQueueError::Invalid(s) => KnowledgeBaseError::Invalid(s),
        IngestionQueueError::Storage(s) => KnowledgeBaseError::Storage(s),
    }
}
pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
pub(super) fn in_progress(status: &str) -> bool {
    matches!(
        status,
        "QUEUED" | "PARSING" | "CHUNKING" | "EMBEDDING" | "PUBLISHING"
    )
}
pub(crate) fn lease(task: &IngestionTask) -> IngestionLease {
    IngestionLease {
        task_id: task.id.clone(),
        attempt: task.attempt,
        owner: task.lease_owner.clone().unwrap_or_default(),
    }
}
pub(crate) struct RemoteCheckpoint {
    pub(crate) queue: Arc<dyn KnowledgeIngestionQueue>,
    pub(crate) lease: IngestionLease,
    pub(crate) task: Mutex<IngestionTask>,
}
impl parsing::ParseCheckpoint for RemoteCheckpoint {
    fn task(&self) -> Result<IngestionTask, KnowledgeBaseError> {
        let task = self.queue.get(&self.lease.task_id).map_err(queue_error)?;
        if task.state != "RUNNING"
            || task.attempt != self.lease.attempt
            || task.lease_owner.as_deref() != Some(&self.lease.owner)
        {
            return Err(KnowledgeBaseError::Conflict);
        }
        Ok(task)
    }
    fn save(
        &self,
        file: Option<String>,
        job: Option<String>,
        reconciling: bool,
    ) -> Result<(), KnowledgeBaseError> {
        let mut task = self.task.lock().map_err(invalid_storage)?;
        task.remote_file_id = file;
        task.remote_job_id = job;
        task.reconciling = reconciling;
        self.queue
            .checkpoint(&task, &self.lease, now_ms())
            .map_err(queue_error)
    }
    fn chunking(&self) -> Result<(), KnowledgeBaseError> {
        let mut task = self.task.lock().map_err(invalid_storage)?;
        task.stage = IngestionStage::Chunking;
        self.queue
            .checkpoint(&task, &self.lease, now_ms())
            .map_err(queue_error)
    }
}
#[derive(Serialize, Deserialize)]
pub(crate) struct StoredChunks {
    pub(crate) document: KnowledgeBaseDocument,
    pub(crate) chunks: Vec<LocatedChunk>,
    #[serde(default)]
    pub(crate) contexts: Vec<eliteforge_agent_core::ChunkContext>,
    pub(crate) warnings: Vec<String>,
}
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct StoredEmbeddings {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) vectors: Vec<Vec<f32>>,
    #[serde(default)]
    pub(crate) vector_resources: Vec<String>,
    #[serde(default)]
    pub(crate) completed_units: usize,
    pub(crate) provider_model: Option<String>,
}
struct InlineAttempt {
    queue: Arc<dyn KnowledgeIngestionQueue>,
    lease: IngestionLease,
    finished: bool,
}
impl Drop for InlineAttempt {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self
                .queue
                .fail(&self.lease, now_ms(), "索引已中断，请重试", None);
        }
    }
}
impl KnowledgeBaseService {
    pub fn upload_with_request(
        &self,
        user: &str,
        kb: &str,
        name: &str,
        bytes: &[u8],
        key: Option<&str>,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.upload_with_parsing_options(user, kb, name, bytes, key, None, None)
    }
    pub fn upload_with_parsing_options(
        &self,
        user: &str,
        kb: &str,
        name: &str,
        bytes: &[u8],
        key: Option<&str>,
        preference: Option<DocumentParsePreference>,
        confirmed: Option<&DocumentParsePlan>,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let workspace = "PLATFORM";
        let plan = self.preflight_document(user, kb, name, bytes, preference)?;
        if confirmed.is_some_and(|confirmed| confirmed != &plan) {
            return Err(KnowledgeBaseError::Invalid(
                "文件或解析选择已改变，请重新检查后上传".into(),
            ));
        }
        let identity = hash(
            &serde_json::to_string(&(name, format!("{:x}", Sha256::digest(bytes)), &plan))
                .map_err(invalid_storage)?,
        );
        let scoped = key
            .map(|key| {
                if key.is_empty() || key.len() > 128 || key.chars().any(char::is_control) {
                    return Err(KnowledgeBaseError::Invalid("入库请求标识无效".into()));
                }
                Ok(hash(
                    &serde_json::to_string(&(user, workspace, kb, key)).map_err(invalid_storage)?,
                ))
            })
            .transpose()?;
        if let Some(key) = &scoped {
            if let Some((parent, doc)) = self.repository.upload_request(key, &identity)? {
                if self.repository.document(&parent, &doc).is_ok() {
                    if self
                        .queue
                        .for_document(&doc)
                        .map_err(queue_error)?
                        .is_empty()
                    {
                        self.enqueue_index(user, workspace, &parent, &doc)?;
                    }
                    self.start_ingestion_workers()?;
                    return self.document_with_task(user, &parent, &doc);
                }
            }
            let document_id = format!(
                "doc_{}",
                hash(
                    &serde_json::to_string(&(
                        user,
                        workspace,
                        kb,
                        name,
                        format!("{:x}", Sha256::digest(bytes))
                    ))
                    .map_err(invalid_storage)?
                )
            );
            // Reserve the key before Resource creation; a replay repairs a crash
            // between this reservation, original storage and durable enqueue.
            self.repository
                .remember_upload_request(key, &identity, kb, &document_id)?;
        }
        let doc = self.upload_original_with_plan(
            user,
            workspace,
            kb,
            name,
            bytes,
            true,
            Some(plan.preference),
            Some(&plan),
        )?;
        if let Some(key) = &scoped {
            self.repository
                .remember_upload_request(key, &identity, kb, &doc.id)?;
        }
        let tasks = self.queue.for_document(&doc.id).map_err(queue_error)?;
        if tasks.is_empty() {
            self.enqueue_index(user, workspace, kb, &doc.id)?;
        }
        self.start_ingestion_workers()?;
        self.document_with_task(user, kb, &doc.id)
    }
    pub fn with_ingestion_config(
        mut self,
        config: crate::config::KnowledgeIngestionConfig,
    ) -> Self {
        self.ingestion_config = config;
        self
    }
    pub fn stop_ingestion_workers(&self) {
        self.workers.stop.cancel();
    }
    pub fn start_ingestion_workers(&self) -> Result<(), KnowledgeBaseError> {
        self.active_ingestion_config()
            .validate()
            .map_err(KnowledgeBaseError::Invalid)?;
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| KnowledgeBaseError::Invalid("后台索引执行环境不可用".into()))?;
        if self.workers.started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let maintenance = self.clone();
        runtime.spawn(async move {
            loop {
                if maintenance.workers.stop.is_cancelled() { break; }
                if let Err(error) = maintenance.recover_ingestion_intents() {
                    tracing::error!("knowledge ingestion intake recovery failed: {error}");
                }
                tokio::select! {
                    _ = maintenance.workers.stop.cancelled() => break,
                    result = maintenance.refresh_parser_waits() => {
                        if let Err(error) = result { tracing::error!("knowledge parser availability update failed: {error}"); }
                    }
                }
                tokio::select! {
                    _ = maintenance.workers.stop.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_millis(maintenance.active_ingestion_config().poll_ms.max(1000))) => {}
                }
            }
        });
        let cancellations = self.clone();
        runtime.spawn(async move {
            loop {
                if cancellations.workers.stop.is_cancelled() { break; }
                if let Ok(tasks) = cancellations.queue.pending_cancellations() {
                    for task in tasks {
                        tokio::select! {
                            _ = cancellations.workers.stop.cancelled() => return,
                            _ = cancellations.finish_remote_cancellation(&task) => {}
                        }
                    }
                }
                tokio::select! {
                    _ = cancellations.workers.stop.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_millis(cancellations.active_ingestion_config().poll_ms.max(250))) => {}
                }
            }
        });
        for (stage, capacity) in [
            (
                IngestionStage::Parsing,
                self.active_ingestion_config().parse_capacity,
            ),
            (
                IngestionStage::Embedding,
                self.active_ingestion_config().embedding_capacity,
            ),
        ] {
            for _ in 0..capacity {
                let service = self.clone();
                runtime.spawn(async move {
                    let owner = format!("kb-worker-{}", new_id());
                    loop {
                        if service.workers.stop.is_cancelled() { break; }
                        if let Ok(Some(task)) = service.claim_task(stage, &owner, None).await {
                            service.execute_claimed(task, true).await;
                        }
                        tokio::select! {
                            _ = service.workers.stop.cancelled() => break,
                            _ = tokio::time::sleep(Duration::from_millis(service.active_ingestion_config().poll_ms)) => {}
                        }
                    }
                });
            }
        }
        Ok(())
    }
    pub(super) fn document_with_task(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        self.detail(user, kb)?
            .documents
            .into_iter()
            .find(|d| d.id == doc)
            .ok_or(KnowledgeBaseError::NotFound)
    }
    pub fn ingestion_tasks(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
    ) -> Result<Vec<IngestionTask>, KnowledgeBaseError> {
        self.owned(user, kb)?;
        self.repository.document(kb, doc)?;
        self.queue.for_document(doc).map_err(queue_error)
    }
    pub async fn cancel_index(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let tasks = self.ingestion_tasks(user, kb, doc)?;
        let Some(task) = tasks.first() else {
            self.repository.cancel_ingestion_intent(kb, doc)?;
            return self.document_with_task(user, kb, doc);
        };
        let cancelled = self.queue.cancel(&task.id).map_err(queue_error)?;
        // The local fence commits first. A slow/unavailable upstream cannot hold
        // the HTTP response; the durable cancellation worker resumes afterwards.
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            self.finish_remote_cancellation(&cancelled),
        )
        .await;
        self.document_with_task(user, kb, doc)
    }
    async fn finish_remote_cancellation(
        &self,
        task: &IngestionTask,
    ) -> Result<(), KnowledgeBaseError> {
        if !task.cancellation_pending {
            return Ok(());
        }
        let parser = self
            .parser
            .as_ref()
            .ok_or_else(|| KnowledgeBaseError::Storage("原解析器不可用，取消稍后重试".into()))?;
        let job = parser.cancel_task(task).await?;
        self.queue
            .complete_cancellation(task, job)
            .map_err(queue_error)
    }
    pub fn retry_index(
        &self,
        user: &str,
        kb: &str,
        doc: &str,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let tasks = self.ingestion_tasks(user, kb, doc)?;
        let Some(task) = tasks.first() else {
            return self.start_index(user, kb, doc);
        };
        if matches!(task.state.as_str(), "FAILED" | "CANCELLED") {
            self.queue.retry(&task.id, now_ms()).map_err(queue_error)?;
        } else if !matches!(task.state.as_str(), "RUNNING" | "QUEUED") {
            return Err(KnowledgeBaseError::Conflict);
        }
        self.start_ingestion_workers()?;
        self.document_with_task(user, kb, doc)
    }
    pub(super) fn enqueue_index(
        &self,
        user: &str,
        workspace: &str,
        kb_id: &str,
        doc_id: &str,
    ) -> Result<IngestionTask, KnowledgeBaseError> {
        let kb = self.owned(user, kb_id)?;
        let doc = self.repository.document(kb_id, doc_id)?;
        if let Some(active) = self
            .queue
            .for_document(doc_id)
            .map_err(queue_error)?
            .into_iter()
            .find(|t| matches!(t.state.as_str(), "RUNNING" | "QUEUED"))
        {
            return Ok(active);
        }
        let task = self.prepare_index_task(user, workspace, &kb, &doc)?;
        self.queue.enqueue(task).map_err(queue_error)
    }
    pub(super) fn prepare_index_task(
        &self,
        user: &str,
        workspace: &str,
        kb: &KnowledgeBase,
        doc: &KnowledgeBaseDocument,
    ) -> Result<IngestionTask, KnowledgeBaseError> {
        let settings = self.rag.settings_for_corpus(&kb.id)?;
        let model = self.llm.resolve_embedding_index_model_code(
            workspace,
            settings.embedding_model_code.as_deref(),
        );
        let fingerprint = model.as_ref().ok().and_then(|m| {
            self.llm
                .embedding_space_fingerprint(workspace, Some(m), settings.embedding_dimension)
                .ok()
        });
        let reuse_parsed_only = doc.parsing_artifact.is_some();
        let embedding_images = model
            .as_ref()
            .ok()
            .map(|model| self.llm.embedding_accepts_images(workspace, model))
            .transpose()
            .map_err(|error| KnowledgeBaseError::Invalid(error.to_string()))?;
        let parser = if reuse_parsed_only {
            None
        } else {
            self.document_parser(&doc)?
                .as_ref()
                .map(|parser| parser.snapshot().unwrap_or_else(|| parser.clone()))
        };
        let parser_identity = parser
            .as_ref()
            .map(|p| p.profile_identity())
            .unwrap_or(json!({}));
        let (_, original) = self.resources.download_binary_to_file(
            user,
            ResourceBinaryDownloadCommand {
                workspace_code: workspace.into(),
                resource_code: doc.resource_code.clone(),
            },
        )?;
        if original.revision != doc.resource_revision {
            return Err(KnowledgeBaseError::Conflict);
        }
        let mut bytes = Vec::new();
        self.resources
            .download_binary_storage(&original, &mut bytes)?;
        let original_sha = format!("{:x}", Sha256::digest(&bytes));
        let endpoint_ids = if reuse_parsed_only {
            vec!["cached-document-index".into()]
        } else if doc
            .parse_plan
            .as_ref()
            .is_some_and(|plan| plan.features.local_text)
            || doc.file_name.to_ascii_lowercase().ends_with(".md")
        {
            vec!["local-markdown".into()]
        } else {
            parser
                .as_ref()
                .map(|p| p.endpoint_ids())
                .unwrap_or_default()
        };
        let tier = doc
            .parsing_tier
            .or_else(|| parser.as_ref().map(|p| p.default_tier()));
        let mut image_description = self.image_description_config();
        image_description.enabled = true;
        image_description.model_code = settings
            .image_description_model_code
            .clone()
            .unwrap_or_default();
        let chunk_config = doc
            .chunk_config
            .as_ref()
            .unwrap_or(&kb.default_chunk_config);
        let strategy = self
            .chunk_strategies
            .descriptors()
            .into_iter()
            .find(|s| s.id == chunk_config.mode.strategy_id())
            .ok_or_else(|| KnowledgeBaseError::Invalid("切片策略未注册".into()))?;
        let config = json!({"indexPipelineVersion":"source-graph-image-bindings-v2", "reuseParsedOnly":reuse_parsed_only, "embeddingImages":embedding_images, "chunkStrategy":strategy,"ragSettings":settings,"originalSha256":original_sha,"chunkConfig": doc.chunk_config.as_ref().unwrap_or(&kb.default_chunk_config), "parsingTier": tier, "ocrMode": doc.ocr_mode, "parsePlan":doc.parse_plan, "parser": parser_identity, "endpointIds": endpoint_ids, "embeddingModel": model.as_ref().ok(), "embeddingError": model.as_ref().err().map(ToString::to_string), "embeddingFingerprint": fingerprint, "imageDescription": image_description});
        let identity = hash(&format!(
            "{}:{}:{config}",
            doc.resource_code, doc.resource_revision
        ));
        let mut task = IngestionTask::queued(
            format!("ingest_{}", new_id()),
            kb.id.clone(),
            doc.id.clone(),
            user.into(),
            workspace.into(),
            doc.revision,
            doc.resource_revision,
            identity,
            config,
        );
        task.file_size_bytes = Some(original.byte_size);
        Ok(task)
    }
    async fn claim_task(
        &self,
        stage: IngestionStage,
        owner: &str,
        task_id: Option<String>,
    ) -> Result<Option<IngestionTask>, KnowledgeBaseError> {
        if !self.queue.pending(stage, now_ms()).map_err(queue_error)? {
            return Ok(None);
        }
        let endpoints = if stage == IngestionStage::Parsing {
            self.refresh_parser_waits().await?
        } else {
            vec![]
        };
        let config = self.active_ingestion_config();
        self.queue
            .claim(IngestionClaim {
                task_id,
                stage,
                owner: owner.into(),
                now_ms: now_ms(),
                lease_ms: config.lease_ms,
                global_capacity: if stage == IngestionStage::Parsing {
                    config.parse_capacity
                } else {
                    config.embedding_capacity
                },
                embedding_resource_group: config
                    .embedding_resource_group
                    .as_ref()
                    .map(|g| (g.clone(), config.embedding_resource_group_capacity)),
                parse_endpoints: endpoints,
            })
            .map_err(queue_error)
    }
    pub(super) async fn refresh_parser_waits(
        &self,
    ) -> Result<Vec<ParseEndpointCapacity>, KnowledgeBaseError> {
        let mut endpoints = vec![ParseEndpointCapacity {
            supported_tiers: vec![],
            endpoint_id: "local-markdown".into(),
            capacity: self.active_ingestion_config().parse_capacity,
            resource_group: "local-markdown".into(),
            resource_group_capacity: self.active_ingestion_config().parse_capacity,
            weight: 1,
        }];
        endpoints.push(ParseEndpointCapacity {
            supported_tiers: vec![],
            endpoint_id: "cached-document-index".into(),
            capacity: self.active_ingestion_config().parse_capacity,
            resource_group: "cached-document-index".into(),
            resource_group_capacity: self.active_ingestion_config().parse_capacity,
            weight: 1,
        });
        if self
            .queue
            .pending(IngestionStage::Parsing, now_ms())
            .map_err(queue_error)?
        {
            let status = match &self.parser {
                Some(parser) => parser.routing_status().await,
                None => parsing::ParserRoutingStatus {
                    endpoints: vec![],
                    unavailable: vec![(
                        None,
                        IngestionWaitReason::new(WaitReasonCode::ParserMissing),
                    )],
                },
            };
            endpoints.extend(status.endpoints);
            self.queue
                .report_parser_availability(now_ms(), &endpoints, &status.unavailable)
                .map_err(queue_error)?;
        }
        Ok(endpoints)
    }
    pub(super) async fn run_task_inline(&self, id: &str) -> Result<(), KnowledgeBaseError> {
        let owner = format!("kb-inline-{}", new_id());
        let initial = self.queue.get(id).map_err(queue_error)?;
        let mut index_id = id.to_owned();
        for stage in [IngestionStage::Parsing, IngestionStage::Embedding] {
            let selected = if stage == IngestionStage::Parsing {
                id
            } else {
                &index_id
            };
            let Some(task) = self
                .claim_task(stage, &owner, Some(selected.into()))
                .await?
            else {
                continue;
            };
            let mut guard = InlineAttempt {
                queue: self.queue.clone(),
                lease: lease(&task),
                finished: false,
            };
            self.execute_claimed(task, false).await;
            guard.finished = true;
            if stage == IngestionStage::Parsing {
                if let Some(index) = self
                    .queue
                    .for_document(&initial.document_id)
                    .map_err(queue_error)?
                    .into_iter()
                    .find(|candidate| candidate.parsing_task_id.as_deref() == Some(id))
                {
                    index_id = index.id;
                }
            }
        }
        Ok(())
    }
    async fn execute_claimed(&self, task: IngestionTask, automatic_retry: bool) {
        let config = self.active_ingestion_config();
        let owned = lease(&task);
        let work = self.process_task(task.clone());
        tokio::pin!(work);
        let mut check = tokio::time::interval(Duration::from_millis(
            (config.lease_ms / 3).min(100).max(1) as u64,
        ));
        let mut renewed_at = now_ms();
        let mut lease_until = task.lease_until_ms;
        let result = loop {
            tokio::select! {
                _ = self.workers.stop.cancelled() => return,
                result = &mut work => break result,
                _ = check.tick() => {
                    match self.queue.get(&task.id) {
                        Ok(current) if current.state == "RUNNING" && current.attempt == owned.attempt && current.lease_owner.as_deref() == Some(&owned.owner) => {},
                        _ => return,
                    }
                    if now_ms() - renewed_at >= config.lease_ms / 3 || now_ms() >= lease_until - config.lease_ms / 3 {
                        if self.queue.heartbeat(&owned, now_ms(), config.lease_ms).is_err() { return; }
                        renewed_at = now_ms();
                        lease_until = renewed_at.saturating_add(config.lease_ms);
                    }
                }
            }
        };
        if let Err(error) = result {
            // A configuration change during parsing must not strand the document
            // on the old immutable snapshot. Preserve completed parse artifacts,
            // then schedule a fresh supplement after fencing this failed lease.
            let superseded = self
                .rag
                .settings_for_corpus(&task.knowledge_base_id)
                .ok()
                .and_then(|settings| serde_json::to_value(settings).ok())
                .is_some_and(|settings| settings != task.configuration["ragSettings"])
                && self
                    .repository
                    .document(&task.knowledge_base_id, &task.document_id)
                    .is_ok_and(|doc| {
                        doc.revision == task.document_revision && doc.parsing_artifact.is_some()
                    });
            let retry = !superseded
                && automatic_retry
                && matches!(error, KnowledgeBaseError::Storage(_))
                && task.failures.saturating_add(1) < config.max_attempts;
            let after = retry.then(|| {
                now_ms().saturating_add(
                    config
                        .retry_backoff_ms
                        .saturating_mul(1i64 << task.failures.min(10)),
                )
            });
            if self
                .queue
                .fail(&owned, now_ms(), &error.to_string(), after)
                .is_ok()
                && superseded
            {
                let _ = self.enqueue_index(
                    &task.user_code,
                    &task.workspace_code,
                    &task.knowledge_base_id,
                    &task.document_id,
                );
            }
        }
    }
}
