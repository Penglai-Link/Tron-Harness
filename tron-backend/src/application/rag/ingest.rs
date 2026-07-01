use std::{
    any::Any,
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use serde_json::{Value, json};

mod executor;

use crate::{
    application::llm::{LlmEmbeddingCallCommand, LlmRuntimeService},
    domain::rag::{RagError, RagStatus},
    shared::event_bus::{BusOutcome, EventBusHandle},
};

use super::{
    RagConfigRepository, RagDocumentSourceRuntime, RagStageExecutionResult, RagStrategyRegistry,
    ingest_artifact::ArtifactBundle, ingest_profile::compile_ingest_profile, publish_rag_event,
    runtime::DefaultRagRuntime,
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
    pub scope: RagIngestScope,
    pub source_status: String,
    pub title: Option<String>,
    pub content_hash: String,
    pub body_hash: Option<String>,
    pub metadata: Value,
    pub domain_fields: BTreeMap<String, Value>,
    pub related_graph: Option<Value>,
    pub graph_write_policy: Value,
    pub text_units: Vec<RagTextUnitWrite>,
    pub tree_nodes: Vec<RagTreeNodeWrite>,
    pub warnings: Vec<RagIngestWarning>,
    pub workspace_code: String,
    pub user_code: String,
    pub agent_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagTextUnitWrite {
    pub text_unit_code: String,
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

#[derive(Debug, Clone, PartialEq)]
pub struct RagTreeNodeWrite {
    pub tree_node_code: String,
    pub parent_node_code: String,
    pub source_code: String,
    pub source_version: String,
    pub scope_type: String,
    pub scope_code: String,
    pub path_key: String,
    pub level: i64,
    pub ordinal: i64,
    pub title: String,
    pub summary_text: String,
    pub summary_token_count: i64,
    pub summary_content_hash: String,
    pub summary_fulltext_status: String,
    pub summary_embedding_status: String,
    pub summary_embedding_model_code: Option<String>,
    pub summary_embedding_version: Option<String>,
    pub summary_embedding_text_hash: Option<String>,
    pub summary_embedding_dimension: Option<i64>,
    pub summary_embedding: Option<Vec<u8>>,
    pub covered_text_unit_codes: Vec<String>,
    pub covered_text_unit_count: i64,
    pub child_start_ordinal: i64,
    pub child_end_ordinal: i64,
    pub metadata: Value,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagEmbeddingCall {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagEmbeddingResult {
    pub model_code: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagTreeSummaryCall {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagTreeSummaryResult {
    pub model_code: String,
    pub summary_text: String,
}

pub trait RagEmbeddingRuntime: Clone + Send + Sync + 'static {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>>;
}

pub trait RagTreeSummaryRuntime: Send + Sync + 'static {
    fn summarize_tree<'a>(
        &'a self,
        call: RagTreeSummaryCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagTreeSummaryResult, String>> + Send + 'a>>;
}

impl RagEmbeddingRuntime for LlmRuntimeService {
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embedding_call(LlmEmbeddingCallCommand {
                workspace_code: call.workspace_code,
                model_code: call.model_code,
                input: call.input,
            })
            .await
            .map(|result| RagEmbeddingResult {
                model_code: result.model_code,
                embedding: result.embedding,
            })
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
    pub source: RagIngestSource,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagIngestSource {
    pub code: String,
    pub version: String,
    /// Legacy inbound compatibility only; RAG no longer consumes source type.
    pub source_type: String,
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
    pub stage_results: Vec<RagStageExecutionResult>,
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
    tree_summary_runtime: Option<Arc<dyn RagTreeSummaryRuntime>>,
    event_bus: EventBusHandle,
}

impl<R, S, E> RagIngestService<R, S, E>
where
    R: RagIngestRepository,
    S: RagStrategyRegistry,
    E: RagEmbeddingRuntime + RagDocumentSourceRuntime + Any,
{
    pub fn new(repository: R, strategy_registry: S, embedding_runtime: E) -> Self {
        let tree_summary_runtime = default_tree_summary_runtime(&embedding_runtime);
        Self {
            repository,
            strategy_registry,
            embedding_runtime,
            tree_summary_runtime,
            event_bus: EventBusHandle::disabled(),
        }
    }

    pub fn with_tree_summary_runtime<T>(mut self, runtime: T) -> Self
    where
        T: RagTreeSummaryRuntime,
    {
        self.tree_summary_runtime = Some(Arc::new(runtime));
        self
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
        let result = self.ingest_after_started(&request).await;
        if let Err(error) = &result {
            publish_ingest_failed_event(&self.event_bus, &request, error);
        }
        result
    }

    async fn ingest_after_started(
        &self,
        request: &RagIngestRequest,
    ) -> Result<RagIngestResult, RagError> {
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

        let mut bundle = ArtifactBundle::from_request(request);
        let (persisted, stage_results) = self
            .execute_ingest_stages(request, &compiled, &mut bundle)
            .await?;
        if let Some(error) = persisted.fatal_error.clone() {
            return Err(error);
        }
        publish_ingest_event(
            &self.event_bus,
            "rag.ingest.completed",
            BusOutcome::Completed,
            request,
            if !persisted.warnings.is_empty() {
                "partial_warning"
            } else {
                "success"
            },
        );

        Ok(RagIngestResult {
            corpus_id: request.corpus_id.clone(),
            profile_id: compiled.profile_id,
            source_code: request.source.code.clone(),
            source_version: request.source.version.clone(),
            status: persisted.source_status,
            text_unit_count: persisted.text_unit_count,
            fulltext_status: persisted.fulltext_status,
            embedding_status: persisted.embedding_status,
            warnings: persisted
                .warnings
                .into_iter()
                .map(|warning| {
                    json!({
                        "stageId": warning.stage_id,
                        "code": warning.code,
                        "message": warning.message
                    })
                })
                .collect(),
            stage_results,
        })
    }
}

fn default_tree_summary_runtime<E>(runtime: &E) -> Option<Arc<dyn RagTreeSummaryRuntime>>
where
    E: Any,
{
    (runtime as &dyn Any)
        .downcast_ref::<DefaultRagRuntime>()
        .map(|runtime| Arc::new(runtime.clone()) as Arc<dyn RagTreeSummaryRuntime>)
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
            ("source_code", request.source.code.clone()),
            ("source_version", request.source.version.clone()),
            ("scope_type", request.target_scope.scope_type.clone()),
            ("scope_code", request.target_scope.scope_code.clone()),
            ("status", status.to_string()),
        ],
    );
}

fn publish_ingest_failed_event(
    event_bus: &EventBusHandle,
    request: &RagIngestRequest,
    error: &RagError,
) {
    publish_rag_event(
        event_bus,
        "rag.ingest.failed",
        BusOutcome::Failure,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("corpus_id", request.corpus_id.clone()),
            ("profile_id", request.ingest_profile_id.clone()),
            ("source_code", request.source.code.clone()),
            ("source_version", request.source.version.clone()),
            ("status", "failed".to_string()),
            ("error_code", format!("{error:?}")),
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
    if !matches!(
        stage_id,
        "fulltextIndex" | "vectorIndex" | "persistTree" | "persistSource"
    ) {
        return;
    }
    let (event_name, status) = if outcome == BusOutcome::Failure {
        ("rag.ingest.stage.failed", "failed")
    } else {
        ("rag.ingest.stage.completed", "completed")
    };
    publish_rag_event(
        event_bus,
        event_name,
        outcome,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("corpus_id", request.corpus_id.clone()),
            ("source_code", request.source.code.clone()),
            ("source_version", request.source.version.clone()),
            ("stage_id", stage_id.to_string()),
            ("status", status.to_string()),
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
        request.source.code.as_str(),
        request.source.version.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(RagError::IngestRequestInvalid);
        }
    }
    if request.payload.as_object().is_none() {
        return Err(RagError::IngestRequestInvalid);
    }
    validate_payload_source(request)?;
    Ok(())
}

fn validate_payload_source(request: &RagIngestRequest) -> Result<(), RagError> {
    let source = request
        .payload
        .get("source")
        .and_then(Value::as_object)
        .ok_or(RagError::IngestRequestInvalid)?;
    let source_code = source_string(source, "code")?;
    let source_version = source_string(source, "version")?;
    if source_code != request.source.code || source_version != request.source.version {
        return Err(RagError::IngestRequestInvalid);
    }
    Ok(())
}

fn source_string<'a>(
    source: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Result<&'a str, RagError> {
    source
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(RagError::IngestRequestInvalid)
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
