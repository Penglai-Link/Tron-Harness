//! Native runtime adaptation of the product's single tool registration table.
use crate::domain::agent::{
    AgentRunContext, ToolError, ToolExecutionControl, ToolExecutionResult, ToolRegistry,
    ToolRequest,
};
use eliteforge_agent_core as fw;
use std::sync::Arc;

pub(crate) trait FrameworkToolRegistryExt {
    fn register_framework_lease(&self, run_code: &str, lease: fw::ToolLease);
    fn take_framework_leases(&self, run_code: &str) -> Vec<fw::ToolLease>;
    fn has_framework_binding(&self, run_code: &str, name: &str) -> bool;
    fn framework_lease_guard(self: &Arc<Self>, run_code: &str) -> FrameworkLeaseGuard;
}
impl FrameworkToolRegistryExt for ToolRegistry {
    fn register_framework_lease(&self, run_code: &str, lease: fw::ToolLease) {
        self.attach_run(run_code, Box::new(lease));
    }
    fn take_framework_leases(&self, run_code: &str) -> Vec<fw::ToolLease> {
        self.take_run_attachments(run_code)
            .into_iter()
            .filter_map(|value| value.downcast::<fw::ToolLease>().ok().map(|value| *value))
            .collect()
    }
    fn has_framework_binding(&self, run_code: &str, name: &str) -> bool {
        self.inspect_run_attachments(run_code, |attachments| {
            attachments
                .iter()
                .filter_map(|value| value.downcast_ref::<fw::ToolLease>())
                .any(|lease| {
                    lease
                        .tools
                        .iter()
                        .any(|binding| binding.descriptor.call_name == name)
                })
        })
    }
    fn framework_lease_guard(self: &Arc<Self>, run_code: &str) -> FrameworkLeaseGuard {
        FrameworkLeaseGuard {
            registry: self.clone(),
            run_code: run_code.into(),
        }
    }
}
pub(crate) struct FrameworkLeaseGuard {
    registry: Arc<ToolRegistry>,
    run_code: String,
}
impl Drop for FrameworkLeaseGuard {
    fn drop(&mut self) {
        drop(self.registry.take_framework_leases(&self.run_code));
    }
}
struct RunControl(fw::RunContext);
impl ToolExecutionControl for RunControl {
    fn check_active(&self) -> Result<(), ToolError> {
        self.0
            .check_cancelled()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        if self.remaining().is_zero() {
            return Err(ToolError::ExecutionFailed("tool deadline exceeded".into()));
        }
        Ok(())
    }
    fn remaining(&self) -> std::time::Duration {
        self.0
            .deadline
            .saturating_duration_since(tokio::time::Instant::now())
    }
    fn cancelled(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(self.0.cancelled())
    }
}
pub(crate) async fn execute(
    registry: &ToolRegistry,
    request: ToolRequest,
    context: AgentRunContext,
    run: &fw::RunContext,
    progress: Arc<dyn Fn(String) + Send + Sync>,
) -> Result<ToolExecutionResult, ToolError> {
    let tool = registry.resolve_allowed(&request, &context)?;
    let request = crate::domain::agent::validate_workspace_request(request, &context)?;
    let control = RunControl(run.clone());
    control.check_active()?;
    if let Some(operation) = tool.exec_async(request.clone(), &context, &control) {
        return operation.await;
    }
    // Legacy sync side effects cannot be physically interrupted. Check admission
    // again on the worker; asynchronous native commands use the branch above.
    tokio::task::spawn_blocking(move || {
        control.check_active()?;
        let mut emit = |line| progress(line);
        tool.exec_with_progress(request, &context, &mut emit)
    })
    .await
    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
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

#[cfg(test)]
#[path = "tool_framework/composition_tests.rs"]
mod composition_tests;
