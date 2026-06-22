use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

use crate::{
    domain::rag::RagError,
    shared::event_bus::{BusOutcome, EventBusHandle},
};

use super::{RagEmbeddingCall, RagEmbeddingRuntime, publish_rag_event};

pub trait RagRetryIndexRepository {
    fn retry_text_units(
        &self,
        request: &RagRetryIndexRequest,
    ) -> Result<Vec<RagRetryTextUnit>, RagError>;
    fn rebuild_fulltext_index(&self, request: &RagRetryIndexRequest) -> Result<usize, RagError>;
    fn update_vector_index(&self, update: RagRetryVectorUpdate) -> Result<(), RagError>;
    fn mark_vector_retry_failed(&self, request: &RagRetryIndexRequest) -> Result<(), RagError>;
    fn persist_retry_warning(
        &self,
        request: &RagRetryIndexRequest,
        stage_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), RagError>;
}

impl<T> RagRetryIndexRepository for Arc<Mutex<T>>
where
    T: RagRetryIndexRepository,
{
    fn retry_text_units(
        &self,
        request: &RagRetryIndexRequest,
    ) -> Result<Vec<RagRetryTextUnit>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .retry_text_units(request)
    }

    fn rebuild_fulltext_index(&self, request: &RagRetryIndexRequest) -> Result<usize, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .rebuild_fulltext_index(request)
    }

    fn update_vector_index(&self, update: RagRetryVectorUpdate) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .update_vector_index(update)
    }

    fn mark_vector_retry_failed(&self, request: &RagRetryIndexRequest) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .mark_vector_retry_failed(request)
    }

    fn persist_retry_warning(
        &self,
        request: &RagRetryIndexRequest,
        stage_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .persist_retry_warning(request, stage_id, code, message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagRetryIndexRequest {
    pub workspace_code: String,
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub scope_type: String,
    pub scope_code: String,
    pub stages: Vec<String>,
    pub embedding_model_code: Option<String>,
    pub user_code: String,
    pub agent_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagRetryIndexResult {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub status: String,
    pub stage_results: Vec<RagRetryStageResult>,
    pub warnings: Vec<Value>,
    pub trace_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagRetryStageResult {
    pub stage_id: String,
    pub status: String,
    pub updated_count: usize,
    pub warnings: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagRetryTextUnit {
    pub text_unit_code: String,
    pub text: String,
    pub index_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagRetryVectorUpdate {
    pub corpus_id: String,
    pub text_unit_code: String,
    pub embedding_model_code: String,
    pub embedding_version: String,
    pub embedding_text_hash: String,
    pub embedding_dimension: i64,
    pub embedding: Vec<u8>,
}

pub struct RagRetryIndexService<R, E> {
    repository: R,
    embedding_runtime: E,
    event_bus: EventBusHandle,
}

impl<R, E> RagRetryIndexService<R, E>
where
    R: RagRetryIndexRepository,
    E: RagEmbeddingRuntime,
{
    pub fn new(repository: R, embedding_runtime: E) -> Self {
        Self {
            repository,
            embedding_runtime,
            event_bus: EventBusHandle::disabled(),
        }
    }

    pub fn with_event_bus(mut self, event_bus: EventBusHandle) -> Self {
        self.event_bus = event_bus;
        self
    }

    pub async fn retry_index(
        &self,
        request: RagRetryIndexRequest,
    ) -> Result<RagRetryIndexResult, RagError> {
        validate_retry_request(&request)?;
        publish_retry_event(
            &self.event_bus,
            "rag.retry.started",
            BusOutcome::Started,
            &request,
            "started",
        );
        let result = self.retry_index_after_started(&request).await;
        if let Err(error) = &result {
            publish_retry_failed_event(&self.event_bus, &request, error);
        }
        result
    }

    async fn retry_index_after_started(
        &self,
        request: &RagRetryIndexRequest,
    ) -> Result<RagRetryIndexResult, RagError> {
        let units = self.repository.retry_text_units(request)?;
        if units.is_empty() {
            return Err(RagError::RetryTextUnitsMissing);
        }
        let mut stage_results = Vec::new();
        let mut warnings = Vec::new();
        for stage in &request.stages {
            match stage.as_str() {
                "fulltextIndex" => {
                    let updated = self.repository.rebuild_fulltext_index(request)?;
                    stage_results.push(RagRetryStageResult {
                        stage_id: stage.clone(),
                        status: "success".into(),
                        updated_count: updated,
                        warnings: Vec::new(),
                    });
                    publish_stage_event(
                        &self.event_bus,
                        "rag.retry.stage.completed",
                        BusOutcome::Completed,
                        stage,
                        "completed",
                        updated,
                        request,
                    );
                }
                "vectorIndex" => {
                    let result = self.retry_vector(request, &units).await?;
                    warnings.extend(result.warnings.clone());
                    stage_results.push(result);
                }
                _ => return Err(RagError::RetryStageUnsupported),
            }
        }
        let status = if warnings.is_empty() {
            "success"
        } else {
            "partial_warning"
        };
        publish_retry_event(
            &self.event_bus,
            "rag.retry.completed",
            BusOutcome::Completed,
            request,
            status,
        );
        Ok(RagRetryIndexResult {
            corpus_id: request.corpus_id.clone(),
            source_code: request.source_code.clone(),
            source_version: request.source_version.clone(),
            status: status.into(),
            stage_results,
            warnings,
            trace_code: format!("rag_retry_{}", now_millis()),
        })
    }

    async fn retry_vector(
        &self,
        request: &RagRetryIndexRequest,
        units: &[RagRetryTextUnit],
    ) -> Result<RagRetryStageResult, RagError> {
        let model = request
            .embedding_model_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let mut updated = 0usize;
        for unit in units {
            match self
                .embedding_runtime
                .embed(RagEmbeddingCall {
                    workspace_code: request.workspace_code.clone(),
                    model_code: model.clone(),
                    input: unit.index_text.clone(),
                })
                .await
            {
                Ok(result) => {
                    self.repository.update_vector_index(RagRetryVectorUpdate {
                        corpus_id: request.corpus_id.clone(),
                        text_unit_code: unit.text_unit_code.clone(),
                        embedding_model_code: result.model_code,
                        embedding_version: "v1".into(),
                        embedding_text_hash: format!("retry:{}", unit.text_unit_code),
                        embedding_dimension: result.embedding.len() as i64,
                        embedding: encode_embedding(&result.embedding),
                    })?;
                    updated += 1;
                }
                Err(_) => {
                    self.repository.mark_vector_retry_failed(request)?;
                    self.repository.persist_retry_warning(
                        request,
                        "vectorIndex",
                        "RAG_VECTOR_INDEX_WARNING",
                        "vector index degraded",
                    )?;
                    publish_stage_event(
                        &self.event_bus,
                        "rag.retry.stage.warning",
                        BusOutcome::Failure,
                        "vectorIndex",
                        "warning",
                        updated,
                        request,
                    );
                    let warning = json!({
                        "code": "RAG_VECTOR_INDEX_WARNING",
                        "stageId": "vectorIndex",
                        "message": "vector index degraded"
                    });
                    return Ok(RagRetryStageResult {
                        stage_id: "vectorIndex".into(),
                        status: "warning".into(),
                        updated_count: updated,
                        warnings: vec![warning],
                    });
                }
            }
        }
        publish_stage_event(
            &self.event_bus,
            "rag.retry.stage.completed",
            BusOutcome::Completed,
            "vectorIndex",
            "completed",
            updated,
            request,
        );
        Ok(RagRetryStageResult {
            stage_id: "vectorIndex".into(),
            status: "success".into(),
            updated_count: updated,
            warnings: Vec::new(),
        })
    }
}

fn validate_retry_request(request: &RagRetryIndexRequest) -> Result<(), RagError> {
    for value in [
        request.workspace_code.as_str(),
        request.corpus_id.as_str(),
        request.source_code.as_str(),
        request.source_version.as_str(),
        request.scope_type.as_str(),
        request.scope_code.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(RagError::RetryRequestInvalid);
        }
    }
    if request.stages.is_empty() {
        return Err(RagError::RetryStageUnsupported);
    }
    if request
        .stages
        .iter()
        .any(|stage| !matches!(stage.as_str(), "fulltextIndex" | "vectorIndex"))
    {
        return Err(RagError::RetryStageUnsupported);
    }
    Ok(())
}

fn publish_retry_event(
    event_bus: &EventBusHandle,
    event_name: &'static str,
    outcome: BusOutcome,
    request: &RagRetryIndexRequest,
    status: &str,
) {
    publish_rag_event(
        event_bus,
        event_name,
        outcome,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("corpus_id", request.corpus_id.clone()),
            ("source_code", request.source_code.clone()),
            ("source_version", request.source_version.clone()),
            ("scope_type", request.scope_type.clone()),
            ("scope_code", request.scope_code.clone()),
            ("status", status.to_string()),
        ],
    );
}

fn publish_retry_failed_event(
    event_bus: &EventBusHandle,
    request: &RagRetryIndexRequest,
    error: &RagError,
) {
    publish_rag_event(
        event_bus,
        "rag.retry.failed",
        BusOutcome::Failure,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("corpus_id", request.corpus_id.clone()),
            ("source_code", request.source_code.clone()),
            ("source_version", request.source_version.clone()),
            ("status", "failed".to_string()),
            ("error_code", format!("{error:?}")),
        ],
    );
}

fn publish_stage_event(
    event_bus: &EventBusHandle,
    event_name: &'static str,
    outcome: BusOutcome,
    stage_id: &str,
    status: &str,
    updated: usize,
    request: &RagRetryIndexRequest,
) {
    publish_rag_event(
        event_bus,
        event_name,
        outcome,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("corpus_id", request.corpus_id.clone()),
            ("source_code", request.source_code.clone()),
            ("source_version", request.source_version.clone()),
            ("stage_id", stage_id.to_string()),
            ("status", status.to_string()),
            ("updated_count", updated.to_string()),
        ],
    );
}

fn encode_embedding(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
