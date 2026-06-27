mod agui_warning;
mod attachments;
mod helpers;
mod run_context_builder;
mod short_term_memory;

use std::sync::Arc;

use rand::RngCore;

use crate::{
    application::{
        app::AppService, mcp::McpService, short_term_memory::ShortTermMemoryService,
        skill::SkillRegistryService, variable::VariableService, workspace::WorkspaceService,
    },
    domain::agent::{
        AgentModelConfig, AgentModelProtocol, AgentRunHistoryRecord, AgentRunStatus, ModelRequest,
        ModelResponse, ToolRegistry,
    },
    shared::{event_bus::EventBusHandle, time::current_timestamp},
};

use super::{
    AgentInvocationCommand, AgentInvocationError, AgentInvocationResult, AgentManagementService,
    AgentOrchestrationRun, AgentOrchestrator, AgentRunExecutionMetadata, AgentRunExecutor,
    AgentRunHistoryService, AgentRuntimeRegistry, ModelClient, ModelStreamSink,
};
use helpers::safe_preview;
pub(crate) use run_context_builder::{AgentRunContextBuilder, PreparedAgentRun};

#[derive(Clone)]
pub struct AgentInvocationService {
    workspaces: WorkspaceService,
    apps: AppService,
    variables: VariableService,
    skills: SkillRegistryService,
    management: AgentManagementService,
    history: AgentRunHistoryService,
    mcp: McpService,
    event_bus: EventBusHandle,
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    runtime_http_client: reqwest::Client,
    short_term_memory: Option<ShortTermMemoryService>,
}

impl AgentInvocationService {
    pub fn new(
        workspaces: WorkspaceService,
        apps: AppService,
        variables: VariableService,
        skills: SkillRegistryService,
        management: AgentManagementService,
        history: AgentRunHistoryService,
        mcp: McpService,
        event_bus: EventBusHandle,
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        runtime_http_client: reqwest::Client,
    ) -> Self {
        Self {
            workspaces,
            apps,
            variables,
            skills,
            management,
            history,
            mcp,
            event_bus,
            model_client,
            tool_registry,
            runtime_http_client,
            short_term_memory: None,
        }
    }

    pub fn with_short_term_memory(mut self, service: ShortTermMemoryService) -> Self {
        self.short_term_memory = Some(service);
        self
    }

    pub fn short_term_memory(&self) -> Option<ShortTermMemoryService> {
        self.short_term_memory.clone()
    }

    pub(crate) fn run_context_builder(&self) -> AgentRunContextBuilder {
        AgentRunContextBuilder::new(
            self.workspaces.clone(),
            self.apps.clone(),
            self.variables.clone(),
            self.skills.clone(),
            self.management.clone(),
            self.mcp.clone(),
            self.event_bus.clone(),
            self.tool_registry.clone(),
            self.short_term_memory.clone(),
        )
    }

    pub async fn generate_stateless_model(
        &self,
        model: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<ModelResponse, AgentInvocationError> {
        self.model_client
            .generate(model, request)
            .await
            .map_err(AgentInvocationError::ModelFailed)
    }

    pub async fn invoke(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        self.invoke_with_stream(command, user_code, locale, None)
            .await
    }

    pub async fn invoke_stream(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
        sink: ModelStreamSink,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        self.invoke_with_stream(command, user_code, locale, Some(sink))
            .await
    }

    async fn invoke_with_stream(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
        sink: Option<ModelStreamSink>,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        let runtime_registry = AgentRuntimeRegistry::new(
            self.model_client.clone(),
            self.tool_registry.clone(),
            self.event_bus.clone(),
            self.runtime_http_client.clone(),
        )
        .with_short_term_memory(self.short_term_memory.clone());
        let executor = AgentRunExecutor::new(runtime_registry);
        let orchestrator = AgentOrchestrator::new(self.run_context_builder(), executor);
        let AgentOrchestrationRun {
            metadata,
            execution,
        } = orchestrator
            .execute_single_agent(command, user_code, locale, sink)
            .await?;
        let AgentRunExecutionMetadata {
            run_code,
            conversation_code,
            workspace_code,
            user_code,
            app_code,
            task_code,
            model_code,
            started_at,
        } = metadata;

        match execution {
            Ok(orchestrated) => {
                let result = orchestrated.run;
                let finished_at = current_timestamp();
                let token_usage = result.token_usage;
                self.history
                    .create_run(AgentRunHistoryRecord {
                        id: 0,
                        run_code: result.run_code.clone(),
                        conversation_code: result.conversation_code.clone(),
                        workspace_code: result.workspace_code.clone(),
                        user_code,
                        app_code,
                        task_code,
                        model_code,
                        status: AgentRunStatus::Completed,
                        outcome: "completed".into(),
                        step_count: result.step_count,
                        observation_count: result.observations.len() as u32,
                        input_tokens: token_usage.map(|usage| usage.input_tokens),
                        output_tokens: token_usage.map(|usage| usage.output_tokens),
                        total_tokens: token_usage.map(|usage| usage.total_tokens),
                        output_preview: Some(safe_preview("output", &result.final_output)),
                        error_code: None,
                        error_message: None,
                        started_at,
                        finished_at,
                        created_at: current_timestamp(),
                    })
                    .map_err(|_| AgentInvocationError::HistoryStorageFailed)?;
                Ok(AgentInvocationResult {
                    run_code: result.run_code,
                    conversation_code: result.conversation_code,
                    workspace_code: result.workspace_code,
                    runtime_adapter_key: orchestrated.runtime_adapter_key,
                    status: "COMPLETED",
                    final_output: result.final_output,
                    step_count: result.step_count,
                    token_usage,
                    latest_request_estimate: result.latest_request_estimate,
                    latest_usage_audit: result.latest_usage_audit,
                    system_agent_session_id: result.system_agent_session_id,
                })
            }
            Err(invocation_error) => {
                let finished_at = current_timestamp();
                self.history
                    .create_run(AgentRunHistoryRecord {
                        id: 0,
                        run_code,
                        conversation_code,
                        workspace_code,
                        user_code,
                        app_code,
                        task_code,
                        model_code,
                        status: AgentRunStatus::Failed,
                        outcome: "failed".into(),
                        step_count: 0,
                        observation_count: 0,
                        input_tokens: None,
                        output_tokens: None,
                        total_tokens: None,
                        output_preview: None,
                        error_code: Some(invocation_error.code().into()),
                        error_message: Some(safe_preview(
                            "diagnostic",
                            &invocation_error.message(),
                        )),
                        started_at,
                        finished_at,
                        created_at: current_timestamp(),
                    })
                    .map_err(|_| AgentInvocationError::HistoryStorageFailed)?;
                Err(invocation_error)
            }
        }
    }
}

pub fn generate_agent_run_code() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let mut rng = rand::rng();
    format!("RUN-{timestamp}-{:016X}", rng.next_u64())
}

pub(crate) fn build_system_runtime_model_config(
    model_name: &str,
    context_window_tokens: Option<u32>,
) -> AgentModelConfig {
    AgentModelConfig {
        model_code: "SYSTEM_AGENT_MODEL".into(),
        protocol: AgentModelProtocol::ChatCompletions,
        base_url: "http://system-agent.internal".into(),
        api_path: None,
        api_key: "sk-system-agent".into(),
        model_name: model_name.to_string(),
        thinking_enabled: true,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens,
        top_p: None,
        max_retries: Some(0),
    }
}
