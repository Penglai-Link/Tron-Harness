use std::{future::Future, sync::Arc, thread};

use serde::Deserialize;

use crate::{
    application::{
        memory::{
            MemoryDeleteCommand, MemoryRunContext, MemorySaveCommand, MemorySearchCommand,
            MemoryService,
        },
        rag::{
            RagDiagnosticsRepository, RagEmbeddingRuntime, RagIngestRepository,
            RagSearchRepository, RagSearchRuntime,
        },
    },
    domain::agent::{
        AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation, ToolRequest,
    },
};

pub(super) fn memory_business_tools<R, E>(memory: MemoryService<R, E>) -> Vec<Arc<dyn Tool>>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + Clone
        + Send
        + Sync
        + 'static,
    E: RagEmbeddingRuntime
        + crate::application::rag::RagGraphRuntime
        + RagSearchRuntime
        + Send
        + Sync
        + 'static,
{
    vec![
        Arc::new(MemorySearchTool {
            memory: memory.clone(),
            dude_only: false,
        }),
        Arc::new(MemorySaveTool {
            memory: memory.clone(),
            dude_only: false,
        }),
        Arc::new(MemoryDeleteTool { memory: memory.clone(), dude_only: false }),
        Arc::new(MemorySearchTool { memory: memory.clone(), dude_only: true }),
        Arc::new(MemorySaveTool { memory: memory.clone(), dude_only: true }),
        Arc::new(MemoryDeleteTool { memory, dude_only: true }),
    ]
}

#[derive(Clone)]
struct MemorySearchTool<R, E> {
    memory: MemoryService<R, E>,
    dude_only: bool,
}

#[derive(Clone)]
struct MemorySaveTool<R, E> {
    memory: MemoryService<R, E>,
    dude_only: bool,
}

#[derive(Clone)]
struct MemoryDeleteTool<R, E> {
    memory: MemoryService<R, E>,
    dude_only: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemorySearchInput {
    workspace_code: Option<String>,
    query: Option<String>,
    mode: Option<String>,
    memory_code: Option<String>,
    scope: Option<String>,
    scope_code: Option<String>,
    memory_type: Option<serde_json::Value>,
    memory_status: Option<serde_json::Value>,
    review_status: Option<serde_json::Value>,
    injectable: Option<serde_json::Value>,
    limit: Option<usize>,
    token_budget: Option<usize>,
    trace: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MemorySaveInput {
    workspace_code: Option<String>,
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
    workspace_code: Option<String>,
    memory_code: String,
    reason: Option<String>,
    delete_mode: Option<String>,
    source_refs: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
}

impl<R, E> Tool for MemorySearchTool<R, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + Clone
        + Send
        + Sync
        + 'static,
    E: RagEmbeddingRuntime
        + crate::application::rag::RagGraphRuntime
        + RagSearchRuntime
        + Send
        + Sync
        + 'static,
{
    fn name(&self) -> &str {
        if self.dude_only { "dude_memory_search" } else { "memory_search" }
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: MemorySearchInput = parse_tool_input(request.input)?;
        if self.dude_only && input.scope_code.is_some() { return Err(ToolError::ExecutionFailed("Dude memory tools do not accept scopeCode".into())); }
        let context = memory_run_context(context, self.dude_only, input.workspace_code.as_deref())?;
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
                        injectable: input.injectable,
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

impl<R, E> Tool for MemorySaveTool<R, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + Clone
        + Send
        + Sync
        + 'static,
    E: RagEmbeddingRuntime
        + crate::application::rag::RagGraphRuntime
        + RagSearchRuntime
        + Send
        + Sync
        + 'static,
{
    fn name(&self) -> &str {
        if self.dude_only { "dude_memory_save" } else { "memory_save" }
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: MemorySaveInput = parse_tool_input(request.input)?;
        if self.dude_only && input.scope_code.is_some() { return Err(ToolError::ExecutionFailed("Dude memory tools do not accept scopeCode".into())); }
        let context = memory_run_context(context, self.dude_only, input.workspace_code.as_deref())?;
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

impl<R, E> Tool for MemoryDeleteTool<R, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + Clone
        + Send
        + Sync
        + 'static,
    E: RagEmbeddingRuntime
        + crate::application::rag::RagGraphRuntime
        + RagSearchRuntime
        + Send
        + Sync
        + 'static,
{
    fn name(&self) -> &str {
        if self.dude_only { "dude_memory_delete" } else { "memory_delete" }
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: MemoryDeleteInput = parse_tool_input(request.input)?;
        let context = memory_run_context(context, self.dude_only, input.workspace_code.as_deref())?;
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

fn memory_run_context(context: &AgentRunContext, dude_only: bool, requested_workspace: Option<&str>) -> Result<MemoryRunContext, ToolError> {
    if dude_only && !context.is_owning_dude() {
        return Err(ToolError::ExecutionFailed("Dude memory tools require the owning Dude".into()));
    }
    if !dude_only && context.is_owning_dude() {
        return Err(ToolError::ExecutionFailed("Dude must use dude_memory tools".into()));
    }
    if !dude_only && context.workspace_code.trim().is_empty() {
        return Err(ToolError::ExecutionFailed("workspaceCode is required for memory tools".into()));
    }
    if dude_only && requested_workspace.is_some() {
        return Err(ToolError::ExecutionFailed("Dude memory tools do not accept workspaceCode".into()));
    }
    if !dude_only && requested_workspace.map(str::trim).filter(|s| !s.is_empty()) != Some(context.workspace_code.trim()) {
        return Err(ToolError::ExecutionFailed("workspaceCode is required and must match the current workspace".into()));
    }
    let user_code = context
        .user_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ToolError::ExecutionFailed("userCode is required for memory tools".into())
        })?;
    let workspace_code = if dude_only { "" } else { context.workspace_code.trim() };
    Ok(MemoryRunContext::new(user_code, workspace_code))
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
