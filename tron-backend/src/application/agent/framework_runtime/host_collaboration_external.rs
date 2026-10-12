//! Named external Agents reuse the Framework external driver and native session binding.
use super::*;
use crate::application::agent::runtime_adapter::{SystemAgentCliKind, SystemCliRuntime};
use crate::application::agent::{
    AgentManagementService, RuntimeAdapterKey, RuntimeAdapterSelection, RuntimeCapabilities,
};
use crate::domain::agent::AgentRuntimeType;

impl FrameworkHost {
    pub(super) async fn collaboration_agent(
        &self,
        workspace: &str,
        code: &str,
        management: &AgentManagementService,
    ) -> Result<Agent, fw::AgentError> {
        let runtime = if workspace.is_empty() {
            management.resolve_platform_agent_runtime(Some(code))
        } else {
            management.resolve_workspace_agent_runtime(workspace, Some(code))
        }
        .map_err(|error| failure(format!("Agent unavailable: {error:?}")))?;
        if runtime.runtime_type == AgentRuntimeType::React {
            return self.internal_agent(workspace, code).await;
        }
        let (packs, extensions) = self.capability_state();
        if !packs
            .get("framework.external-agent")
            .copied()
            .unwrap_or(true)
            || !extensions.get("runtime").copied().unwrap_or(true)
        {
            return Err(failure("external runtime is disabled"));
        }
        self.0
            .external_agent
            .get_or_try_init(|| async { self.build_driver(true).await })
            .await
    }

    pub(super) async fn prepare_collaboration_external(
        &self,
        selection: RuntimeAdapterSelection,
        context: &AgentRunContext,
        request: &mut fw::RunRequest,
    ) -> Result<(), fw::AgentError> {
        let transport: Arc<dyn crate::application::agent::AgentRuntime> = match selection {
            RuntimeAdapterSelection::CodexExec {
                command_name,
                working_directory,
            } => Arc::new(SystemCliRuntime::new(
                RuntimeAdapterKey::CodexExec,
                RuntimeCapabilities::system_cli(),
                SystemAgentCliKind::Codex,
                command_name,
                working_directory,
            )),
            RuntimeAdapterSelection::OpenCodeRun {
                command_name,
                working_directory,
            } => Arc::new(SystemCliRuntime::new(
                RuntimeAdapterKey::OpenCodeRun,
                RuntimeCapabilities::system_cli(),
                SystemAgentCliKind::OpenCode,
                command_name,
                working_directory,
            )),
            _ => return Err(failure("unsupported external collaboration runtime")),
        };
        let execution = super::super::super::external::ExternalExecution {
            transport,
            context: context.clone(),
            sink: None,
            result: std::sync::Mutex::new(None),
        };
        let framework = self.framework().await?;
        let store = framework
            .services()?
            .get::<fw::ConversationStoreService>()
            .ok_or_else(|| failure("Framework conversation storage is missing"))?
            .0
            .clone();
        let binding = super::super::super::native_sessions::Binding::open(
            store.as_ref(),
            &request.session_id,
            &execution,
            false,
            &self.0.native_bindings,
        )
        .await
        .map_err(|error| failure(error.message()))?;
        if let Some(session) = binding.session.clone() {
            request.extensions.insert(session);
        }
        request
            .extensions
            .insert(super::super::super::native_sessions::CollaborationBinding(
                Arc::new(binding),
            ));
        request.extensions.insert(execution);
        let config = self.invocation_config().external_agent;
        request.limits.model_timeout = Duration::from_secs(config.call_timeout_seconds);
        request.limits.run_timeout = Duration::from_secs(config.operation_timeout_seconds);
        request.limits.max_model_retries = 0;
        request.limits.max_model_calls_per_step = Some(1);
        Ok(())
    }
}
