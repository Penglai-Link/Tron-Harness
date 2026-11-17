use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::AgentRunContext;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub tool_name: String,
    pub input: Value,
    /// Runtime-generated identity for retry-safe side effects. Never supplied by the model.
    pub invocation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolObservation {
    pub tool_name: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionResult {
    pub observation: ToolObservation,
}

impl ToolExecutionResult {
    pub fn observation(observation: ToolObservation) -> Self {
        Self { observation }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    MissingTool(String),
    UnauthorizedTool(String),
    ExecutionFailed(String),
}

impl Display for ToolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTool(tool) => write!(f, "tool `{tool}` is not registered"),
            Self::UnauthorizedTool(tool) => write!(f, "tool `{tool}` is not allowed"),
            Self::ExecutionFailed(message) => f.write_str(message),
        }
    }
}

impl Error for ToolError {}

/// Cooperative control supplied by the application runtime, without a vendor
/// execution or process implementation in the domain contract.
pub trait ToolExecutionControl: Send + Sync {
    fn check_active(&self) -> Result<(), ToolError>;
    fn remaining(&self) -> std::time::Duration;
    fn cancelled(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>>;
}
pub type ToolExecutionFuture<'a> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<ToolExecutionResult, ToolError>> + Send + 'a>,
>;

pub trait Tool: Send + Sync {
    /// Optional cancellable application execution. The selected registration
    /// remains authoritative; synchronous tools keep the compatibility adapter.
    fn exec_async<'a>(
        &'a self,
        _request: ToolRequest,
        _context: &'a AgentRunContext,
        _control: &'a dyn ToolExecutionControl,
    ) -> Option<ToolExecutionFuture<'a>> {
        None
    }

    fn name(&self) -> &str;
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError>;
    fn exec_with_progress(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
        progress: &mut dyn FnMut(String),
    ) -> Result<ToolExecutionResult, ToolError> {
        let _ = progress;
        self.exec(request, context)
    }
}

pub struct InProcessTool<F>
where
    F: Fn(ToolRequest) -> Result<ToolObservation, ToolError> + Send + Sync + 'static,
{
    name: String,
    handler: F,
}

impl<F> InProcessTool<F>
where
    F: Fn(ToolRequest) -> Result<ToolObservation, ToolError> + Send + Sync + 'static,
{
    pub fn new(name: impl Into<String>, handler: F) -> Self {
        Self {
            name: name.into(),
            handler,
        }
    }
}

impl<F> Tool for InProcessTool<F>
where
    F: Fn(ToolRequest) -> Result<ToolObservation, ToolError> + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        (self.handler)(request).map(ToolExecutionResult::observation)
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: Mutex<HashMap<String, Arc<dyn Tool>>>,
    // Opaque preparation attachments; lifecycle and protocol belong to the app.
    run_attachments: Mutex<HashMap<String, Vec<Box<dyn std::any::Any + Send>>>>,
}

impl ToolRegistry {
    pub(crate) fn attach_run(&self, run: &str, value: Box<dyn std::any::Any + Send>) {
        self.run_attachments
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(run.into())
            .or_default()
            .push(value);
    }
    pub(crate) fn inspect_run_attachments<R>(
        &self,
        run: &str,
        inspect: impl FnOnce(&[Box<dyn std::any::Any + Send>]) -> R,
    ) -> R {
        let attachments = self
            .run_attachments
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        inspect(attachments.get(run).map_or(&[], Vec::as_slice))
    }
    pub(crate) fn take_run_attachments(&self, run: &str) -> Vec<Box<dyn std::any::Any + Send>> {
        self.run_attachments
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(run)
            .unwrap_or_default()
    }
    pub(crate) fn resolve_allowed(
        &self,
        request: &ToolRequest,
        context: &AgentRunContext,
    ) -> Result<Arc<dyn Tool>, ToolError> {
        if !context.is_tool_authorized(&request.tool_name) {
            return Err(ToolError::UnauthorizedTool(request.tool_name.clone()));
        }
        self.tools
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&request.tool_name)
            .cloned()
            .ok_or_else(|| ToolError::MissingTool(request.tool_name.clone()))
    }
    pub fn register(&self, tool: Arc<dyn Tool>) -> Result<(), ToolError> {
        let name = tool.name().trim();
        if name.is_empty() {
            return Err(ToolError::ExecutionFailed("tool name is required".into()));
        }
        self.tools
            .lock()
            .expect("tool registry mutex poisoned")
            .insert(name.to_string(), tool);
        Ok(())
    }

    pub fn execute_allowed(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let mut progress = |_line: String| {};
        self.execute_allowed_with_progress(request, context, &mut progress)
    }

    pub fn execute_allowed_with_progress(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
        progress: &mut dyn FnMut(String),
    ) -> Result<ToolExecutionResult, ToolError> {
        let tool = self.resolve_allowed(&request, context)?;
        let request = validate_workspace_request(request, context)?;
        tool.exec_with_progress(request, context, progress)
    }
}

pub(crate) fn validate_workspace_request(
    mut request: ToolRequest,
    context: &AgentRunContext,
) -> Result<ToolRequest, ToolError> {
    if let Some(tool) = context
        .authorized_tools()
        .iter()
        .find(|tool| tool.tool_name == request.tool_name)
    {
        if tool
            .input_schema
            .pointer("/properties/workspaceCode")
            .is_some()
        {
            let workspace = request
                .input
                .get("workspaceCode")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if context.is_owning_assistant()
                || workspace.is_none()
                || workspace != Some(context.execution_workspace_code().trim())
            {
                return Err(ToolError::ExecutionFailed(
                    "workspaceCode is required and must match the current authorized workspace"
                        .into(),
                ));
            }
            if tool.domain_code.as_deref() == Some("harness_rag") {
                if let Some(input) = request.input.as_object_mut() {
                    input.remove("workspaceCode");
                }
            }
        }
    }
    Ok(request)
}
