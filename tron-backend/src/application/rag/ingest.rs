use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use serde_json::{Value, json};

use crate::{
    application::llm::{LlmEmbeddingCallCommand, LlmRuntimeService},
    domain::rag::{RagError, RagStatus},
    shared::event_bus::{BusOutcome, EventBusHandle},
};

use super::{
    RagConfigRepository, RagStrategyRegistry, ingest_payload::prepare_payload,
    ingest_profile::compile_ingest_profile, publish_rag_event,
};

pub trait RagIngestRepository: RagConfigRepository {
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError>;
}

impl<T> RagIngestRepository for Arc<Mutex<T>>
where
    T: RagIngestRepository,
{
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .persist_ingest(command)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagPersistIngestCommand {
    pub corpus_id: String,
    pub profile_id: String,
    pub source_code: String,
    pub source_version: String,
    pub source_type: String,
    pub scope: RagIngestScope,
    pub source_status: String,
    pub title: Option<String>,
    pub content_hash: String,
    pub metadata: Value,
    pub memory: Option<RagMemorySourceFields>,
    pub project_doc: Option<RagProjectDocSourceFields>,
    pub text_units: Vec<RagTextUnitWrite>,
    pub warnings: Vec<RagIngestWarning>,
    pub workspace_code: String,
    pub user_code: String,
    pub agent_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagMemorySourceFields {
    pub memory_type: Option<String>,
    pub memory_status: Option<String>,
    pub review_status: Option<String>,
    pub version: Option<String>,
    pub importance_score: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagProjectDocSourceFields {
    pub document_code: Option<String>,
    pub document_type: Option<String>,
    pub path: Option<String>,
    pub body_hash: String,
    pub repo_code: Option<String>,
    pub module_code: Option<String>,
    pub module_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagTextUnitWrite {
    pub text_unit_code: String,
    pub document_code: Option<String>,
    pub unit_type: String,
    pub text: String,
    pub search_text: String,
    pub heading_path: String,
    pub ordinal: usize,
    pub token_count: i64,
    pub content_hash: String,
    pub status: String,
    pub fulltext_status: String,
    pub embedding_model_code: Option<String>,
    pub embedding_version: Option<String>,
    pub embedding_text_hash: Option<String>,
    pub embedding_dimension: Option<i64>,
    pub embedding: Option<Vec<u8>>,
    pub embedding_status: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagEmbeddingCall {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub input: String,
}

pub trait RagEmbeddingRuntime: Clone + Send + Sync + 'static {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>>;
}

impl RagEmbeddingRuntime for LlmRuntimeService {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embedding_call(LlmEmbeddingCallCommand {
                workspace_code: call.workspace_code,
                model_code: call.model_code,
                input: call.input,
            })
            .await
            .map(|result| result.embedding)
            .map_err(|error| error.to_string())
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagIngestRequest {
    pub corpus_id: String,
    pub ingest_profile_id: String,
    pub workspace_code: String,
    pub access_context: RagIngestAccessContext,
    pub target_scope: RagIngestScope,
    pub source_code: String,
    pub source_version: String,
    pub source_type: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagIngestAccessContext {
    pub user_code: String,
    pub agent_code: String,
    pub workspace_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagIngestScope {
    pub scope_type: String,
    pub scope_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagIngestResult {
    pub corpus_id: String,
    pub profile_id: String,
    pub source_code: String,
    pub source_version: String,
    pub status: String,
    pub text_unit_count: usize,
    pub fulltext_status: String,
    pub embedding_status: String,
    pub warnings: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagIngestWarning {
    pub stage_id: String,
    pub code: String,
    pub message: String,
}

pub struct RagIngestService<R, S, E> {
    repository: R,
    strategy_registry: S,
    embedding_runtime: E,
    event_bus: EventBusHandle,
}

impl<R, S, E> RagIngestService<R, S, E>
where
    R: RagIngestRepository,
    S: RagStrategyRegistry,
    E: RagEmbeddingRuntime,
{
    pub fn new(repository: R, strategy_registry: S, embedding_runtime: E) -> Self {
        Self {
            repository,
            strategy_registry,
            embedding_runtime,
            event_bus: EventBusHandle::disabled(),
        }
    }

    pub fn with_event_bus(mut self, event_bus: EventBusHandle) -> Self {
        self.event_bus = event_bus;
        self
    }

    pub async fn ingest(&self, request: RagIngestRequest) -> Result<RagIngestResult, RagError> {
        validate_ingest_request(&request)?;
        validate_direct_scope(&request.access_context, &request.target_scope)?;
        publish_ingest_event(
            &self.event_bus,
            "rag.ingest.started",
            BusOutcome::Started,
            &request,
            "started",
        );

        let corpus = self
            .repository
            .get_corpus(&request.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        if corpus.status == RagStatus::Reserved {
            return Err(RagError::CorpusReserved);
        }
        let profile = self
            .repository
            .get_profile(&request.ingest_profile_id)?
            .ok_or(RagError::ProfileNotFound)?;
        let compiled = compile_ingest_profile(&profile, &corpus, &self.strategy_registry)?;

        let prepared = prepare_payload(&request, &compiled)?;
        let mut warnings = Vec::new();
        let mut embedding_failed = false;
        let mut text_units = Vec::with_capacity(prepared.units.len());
        for unit in prepared.units {
            match self
                .embedding_runtime
                .embed(RagEmbeddingCall {
                    workspace_code: request.workspace_code.clone(),
                    model_code: compiled.embedding_model_code.clone(),
                    input: unit.text.clone(),
                })
                .await
            {
                Ok(embedding) => text_units.push(
                    unit.with_embedding(
                        embedding,
                        compiled
                            .embedding_model_code
                            .as_deref()
                            .unwrap_or("fake-embedding"),
                        &compiled.embedding_version,
                    ),
                ),
                Err(_) if request.corpus_id == "project-doc" => {
                    embedding_failed = true;
                    text_units.push(unit.with_failed_embedding());
                }
                Err(_) => {
                    self.repository.persist_ingest(RagPersistIngestCommand {
                        corpus_id: request.corpus_id.clone(),
                        profile_id: compiled.profile_id.clone(),
                        source_code: request.source_code.clone(),
                        source_version: request.source_version.clone(),
                        source_type: request.source_type.clone(),
                        scope: request.target_scope.clone(),
                        source_status: "FAILED".into(),
                        title: prepared.title.clone(),
                        content_hash: prepared.content_hash.clone(),
                        metadata: prepared.metadata.clone(),
                        memory: prepared.memory.clone(),
                        project_doc: prepared.project_doc.clone(),
                        text_units: vec![unit.with_failed_embedding()],
                        warnings: Vec::new(),
                        workspace_code: request.workspace_code.clone(),
                        user_code: request.access_context.user_code.clone(),
                        agent_code: request.access_context.agent_code.clone(),
                    })?;
                    publish_ingest_stage_event(
                        &self.event_bus,
                        "vectorIndex",
                        BusOutcome::Failure,
                        1,
                        &request,
                    );
                    publish_ingest_event(
                        &self.event_bus,
                        "rag.ingest.completed",
                        BusOutcome::Failure,
                        &request,
                        "failed",
                    );
                    return Err(RagError::EmbeddingFailed);
                }
            }
        }

        if embedding_failed {
            warnings.push(RagIngestWarning {
                stage_id: "vectorIndex".into(),
                code: "RAG_VECTOR_INDEX_WARNING".into(),
                message: "Project Doc vector indexing failed; fulltext remains available".into(),
            });
        }

        let embedding_status = if embedding_failed {
            "FAILED"
        } else {
            "INDEXED"
        };
        let text_unit_count = text_units.len();
        self.repository.persist_ingest(RagPersistIngestCommand {
            corpus_id: request.corpus_id.clone(),
            profile_id: compiled.profile_id.clone(),
            source_code: request.source_code.clone(),
            source_version: request.source_version.clone(),
            source_type: request.source_type.clone(),
            scope: request.target_scope.clone(),
            source_status: "ACTIVE".into(),
            title: prepared.title,
            content_hash: prepared.content_hash,
            metadata: prepared.metadata,
            memory: prepared.memory,
            project_doc: prepared.project_doc,
            text_units,
            warnings: warnings.clone(),
            workspace_code: request.workspace_code.clone(),
            user_code: request.access_context.user_code.clone(),
            agent_code: request.access_context.agent_code.clone(),
        })?;
        publish_ingest_stage_event(
            &self.event_bus,
            "persist",
            BusOutcome::Completed,
            text_unit_count,
            &request,
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "fulltextIndex",
            BusOutcome::Completed,
            text_unit_count,
            &request,
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "vectorIndex",
            if embedding_failed {
                BusOutcome::Failure
            } else {
                BusOutcome::Completed
            },
            text_unit_count,
            &request,
        );
        publish_ingest_event(
            &self.event_bus,
            "rag.ingest.completed",
            if embedding_failed {
                BusOutcome::Failure
            } else {
                BusOutcome::Completed
            },
            &request,
            if embedding_failed {
                "partial_warning"
            } else {
                "success"
            },
        );

        Ok(RagIngestResult {
            corpus_id: request.corpus_id,
            profile_id: compiled.profile_id,
            source_code: request.source_code,
            source_version: request.source_version,
            status: "ACTIVE".into(),
            text_unit_count,
            fulltext_status: "INDEXED".into(),
            embedding_status: embedding_status.into(),
            warnings: warnings
                .into_iter()
                .map(|warning| {
                    json!({
                        "stageId": warning.stage_id,
                        "code": warning.code,
                        "message": warning.message
                    })
                })
                .collect(),
        })
    }
}

fn publish_ingest_event(
    event_bus: &EventBusHandle,
    event_name: &'static str,
    outcome: BusOutcome,
    request: &RagIngestRequest,
    status: &str,
) {
    publish_rag_event(
        event_bus,
        event_name,
        outcome,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("corpus_id", request.corpus_id.clone()),
            ("profile_id", request.ingest_profile_id.clone()),
            ("source_code", request.source_code.clone()),
            ("source_version", request.source_version.clone()),
            ("scope_type", request.target_scope.scope_type.clone()),
            ("scope_code", request.target_scope.scope_code.clone()),
            ("status", status.to_string()),
        ],
    );
}

fn publish_ingest_stage_event(
    event_bus: &EventBusHandle,
    stage_id: &str,
    outcome: BusOutcome,
    count: usize,
    request: &RagIngestRequest,
) {
    publish_rag_event(
        event_bus,
        "rag.stage.completed",
        outcome,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("corpus_id", request.corpus_id.clone()),
            ("source_code", request.source_code.clone()),
            ("source_version", request.source_version.clone()),
            ("stage_id", stage_id.to_string()),
            ("count", count.to_string()),
        ],
    );
}

fn validate_ingest_request(request: &RagIngestRequest) -> Result<(), RagError> {
    for value in [
        request.corpus_id.as_str(),
        request.ingest_profile_id.as_str(),
        request.workspace_code.as_str(),
        request.access_context.user_code.as_str(),
        request.access_context.agent_code.as_str(),
        request.access_context.workspace_code.as_str(),
        request.target_scope.scope_type.as_str(),
        request.target_scope.scope_code.as_str(),
        request.source_code.as_str(),
        request.source_version.as_str(),
        request.source_type.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(RagError::IngestRequestInvalid);
        }
    }
    if request.payload.as_object().is_none() {
        return Err(RagError::IngestRequestInvalid);
    }
    Ok(())
}

pub fn validate_direct_scope(
    access_context: &RagIngestAccessContext,
    target_scope: &RagIngestScope,
) -> Result<(), RagError> {
    match target_scope.scope_type.as_str() {
        "USER" if target_scope.scope_code == access_context.user_code => Ok(()),
        "AGENT" if target_scope.scope_code == access_context.agent_code => Ok(()),
        "SPACE" if target_scope.scope_code == access_context.workspace_code => Ok(()),
        "USER" | "AGENT" | "SPACE" => Err(RagError::IngestScopeInvalid),
        _ => Err(RagError::IngestRequestInvalid),
    }
}
