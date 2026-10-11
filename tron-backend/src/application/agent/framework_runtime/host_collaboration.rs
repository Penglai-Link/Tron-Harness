//! Application-owned named Agent resolution over official collaboration tools.
use super::*;
use crate::application::agent::{AgentInvocationCommand, AgentRunContextBuilder};
use crate::domain::agent::EffectiveTool;
use eliteforge_agent_core::{AgentCollaborationRuntime, AgentCollaborationService};
use eliteforge_agent_official_plugin_tools_agents as official;
use std::sync::Weak;

#[path = "host_collaboration_threads.rs"]
mod threads;
pub(super) use threads::Threads;
pub use threads::{DudeThreadCatalogItem, ThreadFeedback, ThreadFeedbackSink};

#[derive(Clone, Copy)]
enum ExecutionLimits {
    AgentDefinition,
    Request,
}

#[derive(Clone)]
pub(in crate::application::agent::framework_runtime) struct Runtime {
    host: Weak<Host>,
    pub builder: AgentRunContextBuilder,
    pub context: AgentRunContext,
}
fn failure(error: impl std::fmt::Display) -> fw::AgentError {
    fw::AgentError::InvalidConfiguration(error.to_string())
}

fn enabled(host: &FrameworkHost) -> bool {
    let (packs, extensions) = host.capability_state();
    packs
        .get(eliteforge_agent_capability_tools::AGENT_COLLABORATION_PACK)
        .copied()
        .unwrap_or(true)
        && extensions.get("tool").copied().unwrap_or(true)
}

pub(super) fn authorize_tools(host: &FrameworkHost, context: &mut AgentRunContext) {
    if host
        .0
        .collaboration_builder
        .get()
        .and_then(Weak::upgrade)
        .is_none()
        || !enabled(host)
    {
        return;
    }
    if context.effective_tools.is_empty() {
        context.effective_tools = context.tools.clone();
    }
    for descriptor in [
        official::AgentSpawnTool::descriptor(),
        official::ThreadCreateTool::descriptor(),
        official::ThreadSendTool::descriptor(),
        official::ThreadAnswerTool::descriptor(),
        official::ThreadWaitTool::descriptor(),
        official::ThreadReadTool::descriptor(),
        official::ThreadListTool::descriptor(),
        official::ThreadCloseTool::descriptor(),
    ] {
        let tool = EffectiveTool {
            tool_name: descriptor.call_name,
            domain_code: None,
            description: descriptor.description,
            input_schema: descriptor.input_schema,
            skill_name: None,
            required_variable_keys: vec![],
        };
        for tools in [&mut context.tools, &mut context.effective_tools] {
            if !tools.iter().any(|entry| entry.tool_name == tool.tool_name) {
                tools.push(tool.clone());
            }
        }
    }
}
pub(super) fn attach(
    host: &FrameworkHost,
    context: &AgentRunContext,
    request: &mut fw::RunRequest,
) {
    if !enabled(host) {
        return;
    }
    if let Some(builder) = host.0.collaboration_builder.get().and_then(Weak::upgrade) {
        let runtime = Runtime {
            host: Arc::downgrade(&host.0),
            builder: (*builder).clone(),
            context: context.clone(),
        };
        request.extensions.insert(runtime.clone());
        request
            .extensions
            .insert(AgentCollaborationService(Arc::new(runtime)));
    }
}
impl Runtime {
    fn host(&self) -> Result<FrameworkHost, fw::AgentError> {
        let host = FrameworkHost(
            self.host
                .upgrade()
                .ok_or_else(|| failure("Agent host is closed"))?,
        );
        host.ensure_open().map_err(failure)?;
        Ok(host)
    }
    pub fn directory(
        &self,
    ) -> Result<Vec<crate::domain::agent::InternalAgentToolPolicy>, fw::AgentError> {
        let management = self.builder.agent_management();
        if self.context.workspace_code.is_empty() {
            management
                .list_platform_internal_agents()
                .map_err(|error| failure(format!("Agent directory load failed: {error:?}")))?
                .into_iter()
                .map(|agent| {
                    management
                        .platform_internal_agent_policy(&agent.agent_code)
                        .map_err(|error| failure(format!("Agent policy load failed: {error:?}")))
                })
                .collect()
        } else {
            management
                .list_workspace_internal_agents(&self.context.workspace_code)
                .map_err(|error| failure(format!("Agent directory load failed: {error:?}")))
        }
    }
    fn authorize(&self, code: &str) -> Result<String, fw::AgentError> {
        self.directory()?
            .into_iter()
            .find(|entry| entry.agent_code == code)
            .map(|entry| entry.agent_code)
            .ok_or_else(|| failure(format!("Agent `{code}` is unavailable in this workspace")))
    }
    async fn prepare(
        &self,
        request: fw::RunRequest,
        session: String,
        bound: Option<&AgentRunContext>,
        parent: &fw::RunContext,
        limits: ExecutionLimits,
    ) -> Result<(Agent, fw::RunRequest, AgentRunContext), fw::AgentError> {
        self.host()?;
        let caller = HostExecution::from_extensions(&parent.extensions)?;
        let caller_context = caller.state.lock().await.context.clone();
        if caller_context.workspace_code != self.context.workspace_code
            || caller_context.user_code != self.context.user_code
            || caller_context
                .collaboration_owner
                .as_ref()
                .map(|owner| (&owner.kind, &owner.code))
                != self
                    .context
                    .collaboration_owner
                    .as_ref()
                    .map(|owner| (&owner.kind, &owner.code))
        {
            return Err(failure("collaboration caller scope changed"));
        }
        self.prepare_owned(
            request,
            session,
            bound.unwrap_or(&caller_context),
            &caller_context,
            caller.harness.clone(),
            Some(parent),
            limits,
            None,
        )
        .await
    }
    async fn prepare_owned(
        &self,
        mut request: fw::RunRequest,
        session: String,
        base: &AgentRunContext,
        caller_context: &AgentRunContext,
        harness: ReactAgent,
        parent: Option<&fw::RunContext>,
        limits: ExecutionLimits,
        sender: Option<fw::MessageSender>,
    ) -> Result<(Agent, fw::RunRequest, AgentRunContext), fw::AgentError> {
        let host = self.host()?;
        let code = request
            .agent_code
            .take()
            .or_else(|| base.agent_code.clone())
            .ok_or_else(|| failure("Agent code is required"))?;
        let code = self.authorize(&code)?;
        let text = request.input.text();
        let preparation_prompt = if text.trim().is_empty() && request.resume.is_some() {
            "Continue after the interaction response.".to_owned()
        } else {
            text
        };
        let router = self
            .builder
            .model_router()
            .ok_or_else(|| failure("Agent Auto model router is unavailable"))?;
        let (model, route) = router
            .resolve_initial(&base.workspace_code, &session, &code, None)
            .map_err(failure)?;
        let harness = harness.framework_with_model_router(router);
        let mut command: AgentInvocationCommand = serde_json::from_value(serde_json::json!({
            "prompt": preparation_prompt, "workspaceCode": base.workspace_code, "conversationCode": session,
            "agentCode": code, "model": model,
        }))
        .map_err(failure)?;
        command.supports_user_input_requests = caller_context
            .collaboration_owner
            .as_ref()
            .is_some_and(|owner| owner.kind == "DUDE");
        command.run_code = request
            .resume
            .as_ref()
            .map(|resume| resume.run_id.to_string());
        command.execution_scope_override = Some(base.execution_scope.clone());
        // Collaboration uses native sessions; no product chat or checkpoint is fabricated.
        let mut prepared = self
            .builder
            .clone()
            .with_execution_identity(
                base.workspace_code.is_empty(),
                caller_context.collaboration_owner.clone(),
            )
            .prepare(command, base.user_code.clone(), Some(base.locale.clone()))
            .await
            .map_err(|error| failure(error.message()))?;
        let context = &mut prepared.context;
        context.collaboration_owner = caller_context.collaboration_owner.clone();
        context.conversation_code = Some(session.clone());
        context.stream_key = Some(session.clone());
        context.model = crate::adapters::outbound::model::with_person_ai_conversation(
            context.model.clone(),
            &session,
            Some(&context.run_code),
        );
        context.auto_model_route = Some(route);
        let current_input =
            super::super::messages::decode(&fw::Message::user(request.input.clone()))
                .map_err(failure)?;
        if let Some(memory) = &mut context.short_term_memory {
            memory.current_input_message =
                (!request.input.text().is_empty()).then_some(current_input);
        } else {
            context.messages.pop();
            context.messages.push(current_input);
        }
        authorize_tools(&host, context);
        let agent = host.internal_agent(&context.workspace_code, &code).await?;
        request.session_id = session.clone();
        request.run_id = Some(fw::RunId::parse_str(&context.run_code).map_err(failure)?);
        request.model = None;
        request.extensions = fw::Extensions::default();
        // Capture the active caller tool span before SDK-owned tasks are started.
        let trace_parent = opentelemetry::Context::current();
        request.extensions.insert(eliteforge_agent_official_plugin_otel::TraceParent(trace_parent));
        // Ownership is derived from the trusted caller, never tool arguments.
        let mut host_context = parent
            .and_then(fw::RunContext::host_context)
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(metadata) = host_context.as_object_mut() {
            metadata.insert(
                "collaborationOwner".into(),
                serde_json::to_value(&context.collaboration_owner).map_err(failure)?,
            );
        } else {
            return Err(failure("host collaboration context must be an object"));
        }
        request.host_context = Some(host_context);
        request.message_provenance = Some(fw::MessageProvenance {
            sender: sender.unwrap_or_else(|| fw::MessageSender {
                kind: if caller_context
                    .collaboration_owner
                    .as_ref()
                    .is_some_and(|owner| {
                        owner.kind == "DUDE"
                            && caller_context.agent_code.as_deref() == Some(owner.code.as_str())
                    }) {
                    "DUDE"
                } else {
                    "AGENT"
                }
                .into(),
                id: caller_context.agent_code.clone(),
                display_name: caller_context.agent_code.clone(),
                avatar_url: None,
            }),
            source: Some("THREAD".into()),
            correlation: Some(
                serde_json::json!({"originSessionCode": parent.map(|parent| &parent.session_id), "initiatorAgentCode": caller_context.agent_code}),
            ),
        });
        request.assistant_provenance = Some(fw::MessageProvenance {
            sender: fw::MessageSender {
                kind: "AGENT".into(),
                id: Some(code.clone()),
                display_name: Some(code.clone()),
                avatar_url: None,
            },
            source: Some("THREAD".into()),
            correlation: None,
        });
        match limits {
            ExecutionLimits::AgentDefinition => {
                request.limits.max_steps = context.limits.max_steps as usize;
                request.limits.max_tool_calls = context.limits.max_tool_calls as usize;
            }
            ExecutionLimits::Request => {
                request.limits.max_steps = request
                    .limits
                    .max_steps
                    .min(context.limits.max_steps as usize)
                    .min(parent.map_or(usize::MAX, |parent| parent.limits.max_steps));
                request.limits.max_tool_calls = request
                    .limits
                    .max_tool_calls
                    .min(context.limits.max_tool_calls as usize);
            }
        }
        request.limits.max_parallel_tools = context.limits.max_parallel_tool_calls.max(1) as usize;
        request.limits.max_total_tokens = request
            .limits
            .max_total_tokens
            .min(parent.map_or(u64::MAX, |parent| parent.token_ceiling));
        let invocation = host.invocation_config();
        request.limits.model_timeout = Duration::from_secs(invocation.agent_model_timeout_seconds);
        request.limits.run_timeout = Duration::from_secs(
            invocation
                .agent_step_timeout_seconds
                .saturating_mul(request.limits.max_steps as u64),
        );
        request.limits.max_model_retries = if context.auto_model_route.is_some() {
            2
        } else {
            context.model.max_retries.unwrap_or(0).min(3) as usize
        };
        request.limits.max_model_calls_per_step =
            Some((request.limits.max_model_retries + 1).max(3));
        request.limits.max_overflow_recoveries = harness.framework_overflow_limit();
        request.limits.allow_unexposed_tools = true;
        request.limits.tool_batch_budget_policy = fw::ToolBatchBudgetPolicy::ExecuteAvailable;
        request.limits.max_context_tokens =
            context.model.context_window_tokens.unwrap_or(128_000) as usize;
        request.limits.max_model_output_tokens = context
            .model
            .max_tokens
            .unwrap_or(host.invocation_config().default_output_tokens);
        request.generation.temperature = context.model.temperature;
        request.generation.top_p = context.model.top_p;
        request.generation.max_output_tokens = context.model.max_tokens;
        request
            .extensions
            .insert(fw::ToolResultProcessorService(Arc::new(ToolAudit)));
        request.extensions.insert(HostExecution {
            harness: harness.clone(),
            state: Arc::new(Mutex::new(HostRunState::new(context.clone()))),
            summaries: host.summary_store().await?,
            sink: None,
        });
        request.extensions.insert(
            eliteforge_agent_official_plugin_tools_local::CommandProgressSinkService(Arc::new(
                super::super::tools::CommandProgress {
                    harness: harness.clone(),
                    context: context.clone(),
                },
            )),
        );
        tron_harness_capability::TronRunScope {
            platform_scope: context.is_platform_dude(),
            user_code: context.user_code.clone().unwrap_or_else(|| "system".into()),
            workspace_code: context.workspace_code.clone(),
            session_code: session,
            run_code: context.run_code.clone(),
            app_code: context.execution_scope.app_code.clone(),
            task_code: context.execution_scope.task_code.clone(),
            agent_code: context.agent_code.clone(),
        }
        .attach(&mut request)?;
        if let Some(resources) = &host.0.resources {
            request
                .extensions
                .insert(FileReferenceReaderService(Arc::new(AttachmentReader(
                    resources.clone(),
                ))));
        }
        attach(&host, context, &mut request);
        harness
            .framework_model_scope(context)
            .map_err(failure)?
            .attach(&mut request);
        let context = context.clone();
        // The native run owns MCP leases even if the caller drops its child waiter.
        request.extensions.insert(prepared.leases);
        Ok((agent, request, context))
    }
}
