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
        let registration = self
            .reserve_native(&context.run_code)
            .map_err(|error| match error {
                crate::domain::agent::AgentError::SessionBusy(session) => {
                    AgentInvocationError::SessionBusy(session)
                }
                _ => AgentInvocationError::RuntimeFailed(error.to_string()),
            })?;
        let mut caller = super::host_run::CancelOnDrop {
            host: self.clone(),
            key: Some(context.run_code.clone()),
        };
        let host = self.clone();
        let parent = eliteforge_agent_framework::current_continuation_parent();
        let result = tokio::spawn(async move {
            let execution = host.run_external_inner(transport, context, sink, resume, registration);
            if let Some(parent) = parent {
                eliteforge_agent_framework::with_continuation_parent(&parent, execution).await
            } else {
                execution.await
            }
        })
        .await
        .map_err(|error| {
            AgentInvocationError::RuntimeFailed(format!("external native owner failed: {error}"))
        })?;
        caller.key.take();
        result
    }
    async fn run_external_inner(
        &self,
        transport: Arc<dyn crate::application::agent::AgentRuntime>,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
        resume: bool,
        registration: super::host_control::NativeRegistration,
    ) -> Result<AgentRunResult, crate::application::agent::AgentInvocationError> {
        use crate::application::agent::AgentInvocationError;
        let map = |error: fw::AgentError| match error.primary() {
            fw::AgentError::SessionBusy(session) => {
                AgentInvocationError::SessionBusy(session.clone())
            }
            fw::AgentError::Cancelled => AgentInvocationError::RunStopped,
            _ => AgentInvocationError::RuntimeFailed(error.to_string()),
        };
        let admission = self.0.config_lock.lock().await;
        registration
            .ensure_active()
            .map_err(|_| AgentInvocationError::RunStopped)?;
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
        if let Some(parent) = self.0.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?;
        }
        self.prepare_artifacts()
            .map_err(AgentInvocationError::RuntimeFailed)?;
        let agent = self
            .0
            .external_agent
            .get_or_try_init(|| async {
                AgentBuilder::new()
                    .default_models(false)
                    .plugin(eliteforge_agent_official_plugin_agui::AgUiPlugin::default())
                    .plugin(
                        eliteforge_agent_official_plugin_storage_local::LocalStoragePlugin::new(
                            &self.0.path,
                            self.0.path.with_extension("artifacts"),
                        ),
                    )
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
        let answers = super::host_interactions::answers_from_parts(&input.parts)
            .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?;
        let input = super::super::messages::encode(input)
            .map_err(AgentInvocationError::RuntimeFailed)?
            .content;
        let mut request = fw::RunRequest::new(&session, input);
        super::host_interactions::apply_answers(agent, &session, &answers, &mut request)
            .await
            .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?;
        request.limits.max_steps = context.limits.max_steps.max(1) as usize;
        request.limits.max_model_calls_per_step = Some(1);
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
            user_code: context.user_code.clone().unwrap_or_else(|| "system".into()),
            workspace_code: context.workspace_code.clone(),
            session_code: session.clone(),
            run_code: context.run_code.clone(),
            app_code: context.execution_scope.app_code.clone(),
            task_code: context.execution_scope.task_code.clone(),
            agent_code: context
                .short_term_memory
                .as_ref()
                .and_then(|memory| memory.agent_code.clone()),
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
        self.ensure_open()
            .map_err(AgentInvocationError::RuntimeFailed)?;
        let parent = eliteforge_agent_framework::current_continuation_parent();
        let result = if let Some(parent) = parent {
            let started = agent
                .start_child_with_output_owned::<eliteforge_agent_official_plugin_agui::AgUiStream>(
                    request, &parent,
                )
                .await;
            drop(admission);
            match started {
                Ok((handle, stream)) => {
                    self.wait_native_child(
                        agent.clone(),
                        handle,
                        stream,
                        &context,
                        registration.clone(),
                    )
                    .await
                }
                Err(error) => Err(error),
            }
        } else {
            let started = agent
                .start_with_output::<eliteforge_agent_official_plugin_agui::AgUiStream>(request);
            drop(admission);
            match started {
                Ok((handle, stream)) => {
                    self.wait_native(
                        agent.clone(),
                        handle,
                        stream,
                        &context,
                        registration.clone(),
                    )
                    .await
                }
                Err(error) => Err(error),
            }
        };
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
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                binding.failed().await?;
                return Err(map(error));
            }
        };
        let mut native = native.ok_or_else(|| {
            AgentInvocationError::RuntimeFailed(
                "external runtime completed without a result".into(),
            )
        })??;
        native.native = Some(
            super::host_control::native_outcome(registration.run_id(), &result)
                .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?,
        );
        let mut snapshot = registration.snapshot(&result).map_err(map)?;
        snapshot.opaque = true;
        native.sdk_execution = Some(
            super::host_snapshot::product_snapshot(snapshot)
                .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?,
        );
        binding.save(&native).await?;
        Ok(native)
    }
}
