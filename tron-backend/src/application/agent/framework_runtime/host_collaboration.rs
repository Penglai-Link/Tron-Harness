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
        self.builder
            .agent_management()
            .list_workspace_internal_agents(&self.context.workspace_code)
            .map_err(|error| failure(format!("Agent directory load failed: {error:?}")))
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
        mut request: fw::RunRequest,
        session: String,
        bound: Option<&AgentRunContext>,
        parent: &fw::RunContext,
        limits: ExecutionLimits,
    ) -> Result<(Agent, fw::RunRequest, AgentRunContext), fw::AgentError> {
        let host = self.host()?;
        let caller = HostExecution::from_extensions(&parent.extensions)?;
        let caller_context = caller.state.lock().await.context.clone();
        if caller_context.workspace_code != self.context.workspace_code
            || caller_context.user_code != self.context.user_code
        {
            return Err(failure("collaboration caller scope changed"));
        }
        let base = bound.unwrap_or(&caller_context);
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
        let mut command: AgentInvocationCommand = serde_json::from_value(serde_json::json!({
            "prompt": preparation_prompt, "workspaceCode": base.workspace_code,
            "agentCode": code, "model": base.model,
        }))
        .map_err(failure)?;
        command.run_code = request
            .resume
            .as_ref()
            .map(|resume| resume.run_id.to_string());
        command.execution_scope_override = Some(base.execution_scope.clone());
        // Collaboration uses native sessions; no product chat or checkpoint is fabricated.
        let mut prepared = self
            .builder
            .prepare(command, base.user_code.clone(), Some(base.locale.clone()))
            .await
            .map_err(|error| failure(error.message()))?;
        let context = &mut prepared.context;
        context.conversation_code = Some(session.clone());
        context.stream_key = Some(session.clone());
        context.model = crate::adapters::outbound::model::with_person_ai_conversation(
            context.model.clone(),
            &session,
            Some(&context.run_code),
        );
        context.auto_model_route = base.auto_model_route.clone().map(|mut route| {
            route.session_code = session.clone();
            route.agent_code = code.clone();
            route
        });
        context.messages.pop();
        context.messages.push(
            super::super::messages::decode(&fw::Message::user(request.input.clone()))
                .map_err(failure)?,
        );
        authorize_tools(&host, context);
        let agent = host.internal_agent(&context.workspace_code, &code).await?;
        request.session_id = session.clone();
        request.run_id = Some(fw::RunId::parse_str(&context.run_code).map_err(failure)?);
        request.model = None;
        request.extensions = fw::Extensions::default();
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
                    .min(parent.limits.max_steps);
                request.limits.max_tool_calls = request
                    .limits
                    .max_tool_calls
                    .min(context.limits.max_tool_calls as usize);
            }
        }
        request.limits.max_parallel_tools = context.limits.max_parallel_tool_calls.max(1) as usize;
        request.limits.max_total_tokens = request.limits.max_total_tokens.min(parent.token_ceiling);
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
        request.limits.max_overflow_recoveries = caller.harness.framework_overflow_limit();
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
            harness: caller.harness.clone(),
            state: Arc::new(Mutex::new(HostRunState::new(context.clone()))),
            summaries: host.summary_store().await?,
            sink: None,
        });
        request.extensions.insert(
            eliteforge_agent_official_plugin_tools_local::CommandProgressSinkService(Arc::new(
                super::super::tools::CommandProgress {
                    harness: caller.harness.clone(),
                    context: context.clone(),
                },
            )),
        );
        tron_agent_harness::TronRunScope {
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
        caller
            .harness
            .framework_model_scope(context)
            .map_err(failure)?
            .attach(&mut request);
        let context = context.clone();
        // The native run owns MCP leases even if the caller drops its child waiter.
        request.extensions.insert(prepared.leases);
        Ok((agent, request, context))
    }
}
