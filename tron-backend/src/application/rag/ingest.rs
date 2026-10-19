use super::RagConfigRepository;
use crate::{
    application::llm::{LlmEmbeddingBatchCallCommand, LlmEmbeddingCallCommand, LlmRuntimeService},
    domain::rag::RagError,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

pub trait RagIngestRepository: RagConfigRepository {
    fn cached_vectors(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: usize,
    ) -> Result<BTreeMap<String, Vec<f32>>, RagError> {
        Ok(BTreeMap::new())
    }
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError>;
}

impl<T> RagIngestRepository for Arc<Mutex<T>>
where
    T: RagIngestRepository,
{
    fn cached_vectors(
        &self,
        corpus: &str,
        source: &str,
        workspace: &str,
        fingerprint: &str,
        dimension: usize,
    ) -> Result<BTreeMap<String, Vec<f32>>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .cached_vectors(corpus, source, workspace, fingerprint, dimension)
    }
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .persist_ingest(command)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagPersistIngestCommand {
    pub corpus_id: String,
    pub task_id: String,
    pub source_code: String,
    pub source_version: String,
    pub scope: RagIngestScope,
    pub source_status: String,
    pub title: Option<String>,
    pub content_hash: String,
    pub body_hash: Option<String>,
    pub metadata: Value,
    pub domain_fields: BTreeMap<String, Value>,
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
    pub provider_model: Option<String>,
    pub model_code: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct RagEmbeddingBatchCall {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub inputs: Vec<String>,
    pub dimensions: usize,
}

#[derive(Debug, Clone)]
pub struct RagEmbeddingContentBatchCall {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub inputs: Vec<eliteforge_agent_framework::Content>,
    pub dimensions: usize,
}

pub use crate::application::llm::LlmEmbeddingBatchCallResult as RagEmbeddingBatchResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagGraphCall {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagGraphResult {
    pub model_code: String,
    pub graph_json: String,
}

pub trait RagEmbeddingRuntime: Clone + Send + Sync + 'static {
    fn embedding_batch_input_limit(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<Option<usize>, String> {
        Ok(None)
    }
    fn embed_content_batch<'a>(
        &'a self,
        call: RagEmbeddingContentBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        Box::pin(async move {
            if call.inputs.iter().any(|input| input.has_media()) {
                return Err("image embedding is unavailable".into());
            }
            self.embed_batch(RagEmbeddingBatchCall {
                workspace_code: call.workspace_code,
                model_code: call.model_code,
                inputs: call
                    .inputs
                    .iter()
                    .map(eliteforge_agent_framework::Content::text)
                    .collect(),
                dimensions: call.dimensions,
            })
            .await
        })
    }
    fn resolve_index_model(
        &self,
        _workspace: &str,
        model: Option<&str>,
    ) -> Result<Option<String>, String> {
        Ok(model.map(str::to_owned))
    }
    fn embedding_fingerprint(
        &self,
        _workspace: &str,
        _model: Option<&str>,
        _dimension: usize,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>>;
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>>;
}

pub trait RagGraphRuntime: Send + Sync + 'static {
    fn extract_graph<'a>(
        &'a self,
        call: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>>;
}

impl RagEmbeddingRuntime for LlmRuntimeService {
    fn embedding_batch_input_limit(
        &self,
        workspace: &str,
        model: Option<&str>,
    ) -> Result<Option<usize>, String> {
        model
            .map(|model| {
                LlmRuntimeService::embedding_batch_input_limit(self, workspace, model)
                    .map_err(|error| error.to_string())
            })
            .transpose()
            .map(Option::flatten)
    }
    fn embed_content_batch<'a>(
        &'a self,
        call: RagEmbeddingContentBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embedding_content_batch_call(
                crate::application::llm::LlmEmbeddingContentBatchCallCommand {
                    workspace_code: call.workspace_code,
                    model_code: call.model_code,
                    inputs: call.inputs,
                    dimensions: call.dimensions,
                },
            )
            .await
            .map_err(|error| error.to_string())
        })
    }
    fn resolve_index_model(
        &self,
        workspace: &str,
        model: Option<&str>,
    ) -> Result<Option<String>, String> {
        self.resolve_embedding_index_model_code(workspace, model)
            .map(Some)
            .map_err(|error| error.to_string())
    }
    fn embedding_fingerprint(
        &self,
        workspace: &str,
        model: Option<&str>,
        dimension: usize,
    ) -> Result<Option<String>, String> {
        self.embedding_space_fingerprint(workspace, model, dimension)
            .map(Some)
            .map_err(|error| error.to_string())
    }
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.embedding_batch_call(LlmEmbeddingBatchCallCommand {
                workspace_code: call.workspace_code,
                model_code: call.model_code,
                inputs: call.inputs,
                dimensions: call.dimensions,
            })
            .await
            .map_err(|error| error.to_string())
        })
    }
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
                provider_model: result.provider_model,
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
