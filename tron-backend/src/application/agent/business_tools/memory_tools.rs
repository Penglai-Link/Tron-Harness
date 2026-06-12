use std::{future::Future, sync::Arc, thread};

use serde::Deserialize;

use crate::{
    application::{
        memory::{
            MemoryDeleteCommand, MemoryRunContext, MemorySaveCommand, MemorySearchCommand,
            MemoryService,
        },
        rag::{
            RagDiagnosticsRepository, RagDocumentSourceRuntime, RagEmbeddingRuntime,
            RagIngestRepository, RagRetryIndexRepository, RagSearchRepository, RagSearchRuntime,
            RagStrategyRegistry,
        },
    },
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation, ToolRequest,
    },
};

pub(super) fn memory_business_tools<R, S, E>(memory: MemoryService<R, S, E>) -> Vec<Arc<dyn Tool>>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + RagRetryIndexRepository
        + Clone
        + Send
        + Sync
        + 'static,
    S: RagStrategyRegistry + Send + Sync + 'static,
    E: RagEmbeddingRuntime + RagSearchRuntime + RagDocumentSourceRuntime + Send + Sync + 'static,
{
    vec![
        Arc::new(MemorySearchTool {
            memory: memory.clone(),
        }),
        Arc::new(MemorySaveTool {
            memory: memory.clone(),
        }),
        Arc::new(MemoryDeleteTool { memory }),
    ]
}

#[derive(Clone)]
struct MemorySearchTool<R, S, E> {
    memory: MemoryService<R, S, E>,
}

#[derive(Clone)]
struct MemorySaveTool<R, S, E> {
    memory: MemoryService<R, S, E>,
}

#[derive(Clone)]
struct MemoryDeleteTool<R, S, E> {
    memory: MemoryService<R, S, E>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemorySearchInput {
    query: Option<String>,
    mode: Option<String>,
    memory_code: Option<String>,
    scope: Option<String>,
    scope_code: Option<String>,
    memory_type: Option<serde_json::Value>,
    memory_status: Option<serde_json::Value>,
    review_status: Option<serde_json::Value>,
    limit: Option<usize>,
    token_budget: Option<usize>,
    trace: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MemorySaveInput {
    memory_type: Option<String>,
    text: Option<String>,
    memory_code: Option<String>,
    scope: Option<String>,
    scope_code: Option<String>,
    title: Option<String>,
    semantic_key: Option<String>,
    memory_status: Option<String>,
    review_status: Option<String>,
    injectable: Option<bool>,
    importance_score: Option<serde_json::Value>,
    confidence_score: Option<serde_json::Value>,
    quality_score: Option<serde_json::Value>,
    expires_at: Option<i64>,
    derived_source_type: Option<String>,
    derived_source_code: Option<String>,
    source_refs: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryDeleteInput {
    memory_code: String,
    reason: Option<String>,
    delete_mode: Option<String>,
    source_refs: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
}

impl<R, S, E> Tool for MemorySearchTool<R, S, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + RagRetryIndexRepository
        + Clone
        + Send
        + Sync
        + 'static,
    S: RagStrategyRegistry + Send + Sync + 'static,
    E: RagEmbeddingRuntime + RagSearchRuntime + RagDocumentSourceRuntime + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        "memory.search"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: MemorySearchInput = parse_tool_input(request.input)?;
        let context = memory_run_context(context)?;
        let memory = self.memory.clone();
        let output = run_memory_future(async move {
            memory
                .search(
                    context,
                    MemorySearchCommand {
                        query: input.query,
                        mode: input.mode,
                        memory_code: input.memory_code,
                        scope: input.scope,
                        scope_code: input.scope_code,
                        memory_type: input.memory_type,
                        memory_status: input.memory_status,
                        review_status: input.review_status,
                        limit: input.limit,
                        token_budget: input.token_budget,
                        trace: input.trace,
                    },
                )
                .await
        })?;
        json_observation(self.name(), &output)
    }
}

impl<R, S, E> Tool for MemorySaveTool<R, S, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + RagRetryIndexRepository
        + Clone
        + Send
        + Sync
        + 'static,
    S: RagStrategyRegistry + Send + Sync + 'static,
    E: RagEmbeddingRuntime + RagSearchRuntime + RagDocumentSourceRuntime + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        "memory.save"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: MemorySaveInput = parse_tool_input(request.input)?;
        let context = memory_run_context(context)?;
        let memory = self.memory.clone();
        let output = run_memory_future(async move {
            memory
                .save(
                    context,
                    MemorySaveCommand {
                        memory_type: input.memory_type,
                        text: input.text,
                        memory_code: input.memory_code,
                        scope: input.scope,
                        scope_code: input.scope_code,
                        title: input.title,
                        semantic_key: input.semantic_key,
                        memory_status: input.memory_status,
                        review_status: input.review_status,
                        injectable: input.injectable,
                        importance_score: input.importance_score,
                        confidence_score: input.confidence_score,
                        quality_score: input.quality_score,
                        expires_at: input.expires_at,
                        derived_source_type: input.derived_source_type,
                        derived_source_code: input.derived_source_code,
                        source_refs: input.source_refs,
                        metadata: input.metadata,
                    },
                )
                .await
        })?;
        json_observation(self.name(), &output)
    }
}

impl<R, S, E> Tool for MemoryDeleteTool<R, S, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + RagRetryIndexRepository
        + Clone
        + Send
        + Sync
        + 'static,
    S: RagStrategyRegistry + Send + Sync + 'static,
    E: RagEmbeddingRuntime + RagSearchRuntime + RagDocumentSourceRuntime + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        "memory.delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: MemoryDeleteInput = parse_tool_input(request.input)?;
        let context = memory_run_context(context)?;
        let memory = self.memory.clone();
        let output = run_memory_future(async move {
            memory
                .delete(
                    context,
                    MemoryDeleteCommand {
                        memory_code: input.memory_code,
                        reason: input
                            .reason
                            .unwrap_or_else(|| "agent requested delete".into()),
                        delete_mode: input.delete_mode,
                        source_refs: input.source_refs,
                        metadata: input.metadata,
                    },
                )
                .await
        })?;
        json_observation(self.name(), &output)
    }
}

fn memory_run_context(context: &AgentRunContext) -> Result<MemoryRunContext, ToolError> {
    let user_code = context
        .user_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ToolError::ExecutionFailed("userCode is required for memory tools".into())
        })?;
    let workspace_code = context.workspace_code.trim();
    if workspace_code.is_empty() {
        return Err(ToolError::ExecutionFailed(
            "workspaceCode is required for memory tools".into(),
        ));
    }
    Ok(MemoryRunContext {
        user_code: user_code.to_string(),
        workspace_code: workspace_code.to_string(),
    })
}

fn parse_tool_input<T: for<'de> Deserialize<'de>>(
    input: serde_json::Value,
) -> Result<T, ToolError> {
    serde_json::from_value(input)
        .map_err(|error| ToolError::ExecutionFailed(format!("tool input is invalid: {error}")))
}

fn json_observation<T: serde::Serialize>(
    tool_name: &str,
    value: &T,
) -> Result<ToolExecutionResult, ToolError> {
    let output = serde_json::to_string(value)
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
    Ok(ToolExecutionResult::observation(ToolObservation {
        tool_name: tool_name.to_string(),
        output,
    }))
}

fn run_memory_future<F, T>(future: F) -> Result<T, ToolError>
where
    F: Future<Output = Result<T, crate::application::memory::MemoryError>> + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?
            .block_on(future)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    })
    .join()
    .map_err(|_| ToolError::ExecutionFailed("memory tool worker panicked".into()))?
}
