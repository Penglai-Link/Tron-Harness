//! External execution admission and host-owned session persistence.
use super::FrameworkHost;
use crate::{
    application::agent::ModelStreamSink,
    domain::agent::{AgentRunContext, AgentRunResult},
};
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::AgentBuilder;
use eliteforge_agent_official_plugin_storage_local::SqliteConversationStore;
use std::{fs, sync::Arc, time::Duration};
impl FrameworkHost {
    pub(crate) async fn run_external(
        &self,
        transport: Arc<dyn crate::application::agent::AgentRuntime>,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
        resume: bool,
    ) -> Result<AgentRunResult, crate::application::agent::AgentInvocationError> {
        use crate::application::agent::AgentInvocationError;
        let map = |error: fw::AgentError| AgentInvocationError::RuntimeFailed(error.to_string());
        let admission = self.0.config_lock.lock().await;
        self.ensure_open()
            .map_err(AgentInvocationError::RuntimeFailed)?;
        let (packs, extensions) = self.capability_state();
        if !packs
            .get("framework.external-agent")
            .copied()
            .unwrap_or(true)
            || !extensions.get("runtime").copied().unwrap_or(true)
        {
            return Err(AgentInvocationError::RuntimeFailed(
                "external runtime is disabled".into(),
            ));
        }
        let agent = self
            .0
            .external_agent
            .get_or_try_init(|| async {
                AgentBuilder::new()
                    .default_models(false)
                    .capability_pack(eliteforge_agent_capability_external::external_agent_pack(
                        super::super::external::HostExternalAgent,
                    ))
                    .build()
                    .await
            })
            .await
            .map_err(map)?;
        if let Some(parent) = self.0.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AgentInvocationError::RuntimeFailed(e.to_string()))?;
        }
        let store = self
            .0
            .native_sessions
            .get_or_try_init(|| async {
                SqliteConversationStore::open(&self.0.path, Default::default()).await
            })
            .await
            .map_err(|e| AgentInvocationError::RuntimeFailed(e.to_string()))?;
        let session = context
            .conversation_code
            .clone()
            .unwrap_or_else(|| context.run_code.clone());
        let input = context
            .messages
            .iter()
            .rfind(|message| message.role == "user")
            .ok_or(AgentInvocationError::PromptRequired)?;
        let input = super::super::messages::encode(input)
            .map_err(AgentInvocationError::RuntimeFailed)?
            .content;
        let mut request = fw::RunRequest::new(&session, input);
        request.limits.max_steps = 1;
        request.limits.max_model_retries = 0;
        request.limits.max_context_tokens =
            context.model.context_window_tokens.unwrap_or(128_000) as usize;
        request.limits.max_model_output_tokens = context
            .model
            .max_tokens
            .filter(|value| *value > 0)
            .unwrap_or(4096);
        request.limits.model_timeout = Duration::from_secs(3600);
        request.limits.run_timeout = Duration::from_secs(3600);
        request
            .extensions
            .insert(super::super::external::ExternalExecution {
                transport,
                context: context.clone(),
                sink,
                result: std::sync::Mutex::new(None),
            });
        tron_agent_harness::TronRunScope {
            user_code: context.user_code.unwrap_or_else(|| "system".into()),
            workspace_code: context.workspace_code,
            session_code: session.clone(),
            run_code: context.run_code,
            app_code: context.execution_scope.app_code,
            task_code: context.execution_scope.task_code,
            agent_code: context
                .short_term_memory
                .and_then(|memory| memory.agent_code),
        }
        .attach(&mut request)
        .map_err(map)?;
        let execution = request
            .extensions
            .get::<super::super::external::ExternalExecution>()
            .expect("registered external execution");
        let binding = super::super::native_sessions::Binding::open(
            store,
            &session,
            &execution,
            resume,
            &self.0.native_bindings,
        )
        .await?;
        if let Some(native) = binding.session.clone() {
            request.extensions.insert(native);
        }
        let handle = agent.start(request).map_err(map)?;
        drop(admission);
        let result = agent.wait(handle).await;
        let native = execution
            .result
            .lock()
            .map_err(|_| {
                AgentInvocationError::RuntimeFailed("external result lock unavailable".into())
            })?
            .take();
        if let Some(Err(error)) = native {
            binding.failed().await?;
            return Err(error);
        }
        if let Err(error) = result {
            binding.failed().await?;
            return Err(map(error));
        }
        let native = native.ok_or_else(|| {
            AgentInvocationError::RuntimeFailed(
                "external runtime completed without a result".into(),
            )
        })??;
        binding.save(&native).await?;
        Ok(native)
    }
}
