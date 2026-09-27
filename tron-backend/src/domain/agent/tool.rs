use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AgentRunContext, EffectiveTool};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolContextDelta {
    #[serde(default)]
    pub add_tools: Vec<EffectiveTool>,
    #[serde(default)]
    pub expanded_domain_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionResult {
    pub observation: ToolObservation,
    #[serde(default)]
    pub context_delta: ToolContextDelta,
}

impl ToolExecutionResult {
    pub fn observation(observation: ToolObservation) -> Self {
        Self {
            observation,
            context_delta: ToolContextDelta::default(),
        }
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

pub trait Tool: Send + Sync {
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
    // Public plugin leases are transferred to Framework's run lifecycle. This
    // registry only transports bindings; it implements no external tool protocol.
    framework_leases: Mutex<HashMap<String, Vec<eliteforge_agent_core::ToolLease>>>,
}

/// Releases run-scoped plugin connections if preparation or execution is
/// cancelled before the Framework takes ownership of their leases.
pub(crate) struct FrameworkLeaseGuard {
    registry: Arc<ToolRegistry>,
    run_code: String,
}
impl Drop for FrameworkLeaseGuard {
    fn drop(&mut self) {
        drop(self.registry.take_framework_leases(&self.run_code));
    }
}

impl ToolRegistry {
    pub(crate) fn framework_lease_guard(self: &Arc<Self>, run_code: &str) -> FrameworkLeaseGuard {
        FrameworkLeaseGuard {
            registry: self.clone(),
            run_code: run_code.into(),
        }
    }
    pub fn register_framework_lease(
        &self,
        run_code: &str,
        lease: eliteforge_agent_core::ToolLease,
    ) {
        self.framework_leases
            .lock()
            .expect("framework leases poisoned")
            .entry(run_code.into())
            .or_default()
            .push(lease);
    }
    pub fn take_framework_leases(&self, run_code: &str) -> Vec<eliteforge_agent_core::ToolLease> {
        self.framework_leases
            .lock()
            .expect("framework leases poisoned")
            .remove(run_code)
            .unwrap_or_default()
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
        if !context.is_tool_authorized(&request.tool_name) {
            return Err(ToolError::UnauthorizedTool(request.tool_name));
        }
        let tool = self
            .tools
            .lock()
            .expect("tool registry mutex poisoned")
            .get(&request.tool_name)
            .cloned()
            .ok_or_else(|| ToolError::MissingTool(request.tool_name.clone()))?;
        tool.exec_with_progress(request, context, progress)
    }
}

#[cfg(test)]
mod framework_lease_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn preparation_guard_releases_only_its_runs_pending_plugins() {
        let registry = Arc::new(ToolRegistry::default());
        let released = Arc::new(AtomicBool::new(false));
        let other = Arc::new(AtomicBool::new(false));
        let guard = registry.framework_lease_guard("failed");
        for (run, flag) in [("failed", released.clone()), ("other", other.clone())] {
            registry.register_framework_lease(
                run,
                eliteforge_agent_core::ToolLease::new(vec![]).with_cleanup(move || {
                    flag.store(true, Ordering::SeqCst);
                }),
            );
        }
        drop(guard);
        assert!(released.load(Ordering::SeqCst));
        assert!(!other.load(Ordering::SeqCst));
        let transferred = registry.take_framework_leases("other");
        drop(registry.framework_lease_guard("other"));
        assert!(!other.load(Ordering::SeqCst));
        drop(transferred);
        assert!(other.load(Ordering::SeqCst));
    }
}
