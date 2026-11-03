use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    application::llm::{
        CharLengthChunkStrategy, LlmChunkedEmbeddingCallCommand, LlmEmbeddingCallCommand,
        LlmRuntimeError, LlmRuntimeService,
    },
    domain::graph::{
        GraphEdgeWrite, GraphError, GraphNodeWrite, GraphSearchQuery, ResolvedGraphScope,
    },
};

const SEMANTIC_CHUNK_CHAR_LIMIT: usize = 512;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphSemanticProjection {
    values: Arc<[f32]>,
}

impl GraphSemanticProjection {
    pub fn from_values(values: impl Into<Arc<[f32]>>) -> Self {
        Self {
            values: values.into(),
        }
    }

    pub fn as_slice(&self) -> &[f32] {
        self.values.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphSemanticSlice {
    pub chunk_index: usize,
    pub content: String,
    pub projection: GraphSemanticProjection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedGraphNodeWrite {
    pub command: GraphNodeWrite,
    pub semantic_model_code: Option<String>,
    pub semantic_projection: Option<GraphSemanticProjection>,
    pub semantic_slices: Vec<GraphSemanticSlice>,
}

impl PreparedGraphNodeWrite {
    pub fn plain(command: GraphNodeWrite) -> Self {
        Self {
            command,
            semantic_model_code: None,
            semantic_projection: None,
            semantic_slices: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedGraphEdgeWrite {
    pub command: GraphEdgeWrite,
    pub semantic_model_code: Option<String>,
    pub semantic_projection: Option<GraphSemanticProjection>,
    pub semantic_slices: Vec<GraphSemanticSlice>,
}

impl PreparedGraphEdgeWrite {
    pub fn plain(command: GraphEdgeWrite) -> Self {
        Self {
            command,
            semantic_model_code: None,
            semantic_projection: None,
            semantic_slices: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedGraphSearchQuery {
    pub query: GraphSearchQuery,
    pub semantic_model_code: Option<String>,
    pub semantic_projection: Option<GraphSemanticProjection>,
}

impl PreparedGraphSearchQuery {
    pub fn plain(query: GraphSearchQuery) -> Self {
        Self {
            query,
            semantic_model_code: None,
            semantic_projection: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphEmbeddingCallResult {
    pub model_code: String,
    pub projection: GraphSemanticProjection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphChunkedEmbeddingCallResult {
    pub model_code: String,
    pub slices: Vec<GraphSemanticSlice>,
}

pub trait GraphEmbeddingRuntime: Send + Sync {
    fn embedding_call<'a>(
        &'a self,
        workspace_code: &'a str,
        model_code: Option<&'a str>,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GraphEmbeddingCallResult, GraphError>> + Send + 'a>>;

    fn embedding_call_with_chunking<'a>(
        &'a self,
        workspace_code: &'a str,
        model_code: Option<&'a str>,
        input: &'a str,
        max_chars_per_chunk: usize,
    ) -> Pin<
        Box<dyn Future<Output = Result<GraphChunkedEmbeddingCallResult, GraphError>> + Send + 'a>,
    >;
}

impl GraphEmbeddingRuntime for LlmRuntimeService {
    fn embedding_call<'a>(
        &'a self,
        workspace_code: &'a str,
        model_code: Option<&'a str>,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GraphEmbeddingCallResult, GraphError>> + Send + 'a>>
    {
        Box::pin(async move {
            let result = self
                .embedding_call(LlmEmbeddingCallCommand {
                    provider_id: None,
                    required_modalities: vec![],
                    workspace_code: workspace_code.to_owned(),
                    model_code: model_code.map(str::to_owned),
                    input: input.to_owned(),
                })
                .await
                .map_err(map_embedding_error)?;
            Ok(GraphEmbeddingCallResult {
                model_code: result.model_code,
                projection: GraphSemanticProjection::from_values(result.embedding),
            })
        })
    }

    fn embedding_call_with_chunking<'a>(
        &'a self,
        workspace_code: &'a str,
        model_code: Option<&'a str>,
        input: &'a str,
        max_chars_per_chunk: usize,
    ) -> Pin<
        Box<dyn Future<Output = Result<GraphChunkedEmbeddingCallResult, GraphError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let result = self
                .embedding_call_with_chunking(
                    LlmChunkedEmbeddingCallCommand {
                        provider_id: None,
                        required_modalities: vec![],
                        workspace_code: workspace_code.to_owned(),
                        model_code: model_code.map(str::to_owned),
                        input: input.to_owned(),
                    },
                    &CharLengthChunkStrategy {
                        max_chars_per_chunk,
                    },
                )
                .await
                .map_err(map_embedding_error)?;
            Ok(GraphChunkedEmbeddingCallResult {
                model_code: result.model_code,
                slices: result
                    .chunks
                    .into_iter()
                    .map(|chunk| GraphSemanticSlice {
                        chunk_index: chunk.chunk_index,
                        content: chunk.content,
                        projection: GraphSemanticProjection::from_values(chunk.embedding),
                    })
                    .collect(),
            })
        })
    }
}

pub async fn prepare_node_write(
    runtime: Option<&dyn GraphEmbeddingRuntime>,
    scope: &ResolvedGraphScope,
    command: GraphNodeWrite,
) -> Result<PreparedGraphNodeWrite, GraphError> {
    let Some(embedding_text) = command.embedding_text.as_deref() else {
        return Ok(PreparedGraphNodeWrite::plain(command));
    };
    let runtime = require_embedding_runtime(runtime)?;
    let model_code = command.embedding_model_code.as_deref();
    if requires_chunking(embedding_text) {
        let result = runtime
            .embedding_call_with_chunking(
                scope.current_workspace_code.as_str(),
                model_code,
                embedding_text,
                SEMANTIC_CHUNK_CHAR_LIMIT,
            )
            .await?;
        return Ok(PreparedGraphNodeWrite {
            command,
            semantic_model_code: Some(result.model_code),
            semantic_projection: None,
            semantic_slices: result.slices,
        });
    }

    let result = runtime
        .embedding_call(
            scope.current_workspace_code.as_str(),
            model_code,
            embedding_text,
        )
        .await?;
    Ok(PreparedGraphNodeWrite {
        command,
        semantic_model_code: Some(result.model_code),
        semantic_projection: Some(result.projection),
        semantic_slices: Vec::new(),
    })
}

pub async fn prepare_edge_write(
    runtime: Option<&dyn GraphEmbeddingRuntime>,
    scope: &ResolvedGraphScope,
    command: GraphEdgeWrite,
) -> Result<PreparedGraphEdgeWrite, GraphError> {
    let Some(embedding_text) = command.embedding_text.as_deref() else {
        return Ok(PreparedGraphEdgeWrite::plain(command));
    };
    let runtime = require_embedding_runtime(runtime)?;
    let model_code = command.embedding_model_code.as_deref();
    if requires_chunking(embedding_text) {
        let result = runtime
            .embedding_call_with_chunking(
                scope.current_workspace_code.as_str(),
                model_code,
                embedding_text,
                SEMANTIC_CHUNK_CHAR_LIMIT,
            )
            .await?;
        return Ok(PreparedGraphEdgeWrite {
            command,
            semantic_model_code: Some(result.model_code),
            semantic_projection: None,
            semantic_slices: result.slices,
        });
    }

    let result = runtime
        .embedding_call(
            scope.current_workspace_code.as_str(),
            model_code,
            embedding_text,
        )
        .await?;
    Ok(PreparedGraphEdgeWrite {
        command,
        semantic_model_code: Some(result.model_code),
        semantic_projection: Some(result.projection),
        semantic_slices: Vec::new(),
    })
}

pub async fn prepare_search_query(
    runtime: Option<&dyn GraphEmbeddingRuntime>,
    scope: &ResolvedGraphScope,
    query: GraphSearchQuery,
) -> Result<PreparedGraphSearchQuery, GraphError> {
    let Some(vector_text) = query.vector_text.as_deref() else {
        return Ok(PreparedGraphSearchQuery::plain(query));
    };
    let runtime = require_embedding_runtime(runtime)?;
    let result = runtime
        .embedding_call(
            scope.current_workspace_code.as_str(),
            query.embedding_model_code.as_deref(),
            vector_text,
        )
        .await?;
    Ok(PreparedGraphSearchQuery {
        query,
        semantic_model_code: Some(result.model_code),
        semantic_projection: Some(result.projection),
    })
}

fn requires_chunking(value: &str) -> bool {
    value.chars().count() > SEMANTIC_CHUNK_CHAR_LIMIT
}

fn require_embedding_runtime(
    runtime: Option<&dyn GraphEmbeddingRuntime>,
) -> Result<&dyn GraphEmbeddingRuntime, GraphError> {
    runtime.ok_or(GraphError::EmbeddingUnavailable)
}

fn map_embedding_error(error: LlmRuntimeError) -> GraphError {
    match error {
        LlmRuntimeError::EmbeddingModelUnavailable => GraphError::EmbeddingUnavailable,
        LlmRuntimeError::WorkspaceRequired
        | LlmRuntimeError::InputRequired
        | LlmRuntimeError::QueryRequired
        | LlmRuntimeError::DocumentsRequired
        | LlmRuntimeError::ModelResolutionFailed(_)
        | LlmRuntimeError::ProviderFailed(_)
        | LlmRuntimeError::InvalidProviderResponse(_)
        | LlmRuntimeError::InvalidChunkStrategy(_)
        | LlmRuntimeError::TextModelUnavailable
        | LlmRuntimeError::RerankModelUnavailable => GraphError::EmbeddingProviderFailure,
    }
}
