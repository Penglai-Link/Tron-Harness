use crate::{
    application::{
        agent::{
            AgentInvocationService, AgentManagementService, build_system_runtime_model_config,
        },
        automation::{AutomationExecutionRunner, AutomationService},
        chat::{ChatService, CreateChatSessionCommand, SendChatMessageCommand},
        model::ModelService,
    },
    domain::{
        automation::{Automation, AutomationExecutionRecord, AutomationExecutionStatus},
        chat::ChatSessionSourceType,
    },
};

const AUTOMATION_AGENT_CODE: &str = "main-agent";
const AUTOMATION_LOCALE: &str = "en-US";

pub(super) struct MainAgentAutomationRunner {
    automations: AutomationService,
    chats: ChatService,
    agent_management: AgentManagementService,
    models: ModelService,
    agent: AgentInvocationService,
}

impl MainAgentAutomationRunner {
    pub(super) fn new(
        automations: AutomationService,
        chats: ChatService,
        agent_management: AgentManagementService,
        models: ModelService,
        agent: AgentInvocationService,
    ) -> Self {
        Self {
            automations,
            chats,
            agent_management,
            models,
            agent,
        }
    }
}

impl AutomationExecutionRunner for MainAgentAutomationRunner {
    fn dispatch(&self, automation: Automation, execution: AutomationExecutionRecord) {
        let automations = self.automations.clone();
        let chats = self.chats.clone();
        let agent_management = self.agent_management.clone();
        let models = self.models.clone();
        let agent = self.agent.clone();
        tokio::spawn(async move {
            let result = run_automation_with_main_agent(
                automation.clone(),
                execution.clone(),
                chats,
                agent_management,
                models,
                agent,
            )
            .await;
            let (status, summary, session_code) = match result {
                Ok(result) => (
                    AutomationExecutionStatus::Success,
                    result.summary,
                    Some(result.session_code),
                ),
                Err(summary) => (AutomationExecutionStatus::Failed, summary, None),
            };
            let _ = automations.complete_execution(
                &automation.automation_code,
                &execution.execution_code,
                status,
                Some(summary),
                session_code,
            );
        });
    }
}

struct AutomationRunResult {
    session_code: String,
    summary: String,
}

async fn run_automation_with_main_agent(
    automation: Automation,
    execution: AutomationExecutionRecord,
    chats: ChatService,
    agent_management: AgentManagementService,
    models: ModelService,
    agent: AgentInvocationService,
) -> Result<AutomationRunResult, String> {
    let agent_code = automation.agent_code.trim();
    let effective_agent_code = if agent_code.is_empty() {
        AUTOMATION_AGENT_CODE
    } else {
        agent_code
    };
    let resolved_runtime = agent_management
        .resolve_workspace_agent_runtime(&automation.workspace_code, Some(effective_agent_code))
        .map_err(|error| {
            format!(
                "Automation agent `{effective_agent_code}` is unavailable for workspace {}: {error:?}",
                automation.workspace_code
            )
        })?;
    let uses_main_agent = resolved_runtime
        .agent_code
        .eq_ignore_ascii_case(AUTOMATION_AGENT_CODE);
    let (model, model_code, system_model_id) = if uses_main_agent {
        let model = models
            .text_agent_model_config(&automation.workspace_code, automation.model_code.as_deref())
            .map_err(|error| format!("Text model resolution failed: {error}"))?
            .ok_or_else(|| {
                "No matching text model is configured for automation execution.".to_string()
            })?;
        let model_code = model.model_code.clone();
        (model, Some(model_code), None)
    } else {
        let system_model_id = automation
            .model_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "Automation agent `{}` requires a system model selection.",
                    resolved_runtime.agent_code
                )
            })?
            .to_string();
        let system_agents = agent_management
            .list_workspace_system_agents(&automation.workspace_code)
            .map_err(|error| {
                format!(
                    "Automation agent `{}` model validation failed for workspace {}: {error:?}",
                    resolved_runtime.agent_code, automation.workspace_code
                )
            })?;
        let system_agent = system_agents
            .into_iter()
            .find(|agent| {
                agent
                    .agent_code
                    .eq_ignore_ascii_case(&resolved_runtime.agent_code)
            })
            .ok_or_else(|| {
                format!(
                    "Automation agent `{}` is not registered for workspace {}.",
                    resolved_runtime.agent_code, automation.workspace_code
                )
            })?;
        if !system_agent.enabled || !system_agent.available {
            return Err(format!(
                "Automation agent `{}` is disabled or unavailable for workspace {}.",
                resolved_runtime.agent_code, automation.workspace_code
            ));
        }
        let context_window_tokens = if system_agent.models.is_empty() {
            None
        } else {
            Some(
                system_agent
                    .models
                    .iter()
                    .find(|model| model.model_id == system_model_id)
                    .ok_or_else(|| {
                        format!(
                            "Automation model `{system_model_id}` is not available for agent `{}`.",
                            resolved_runtime.agent_code
                        )
                    })?
                    .context_window_tokens,
            )
            .flatten()
        };
        (
            build_system_runtime_model_config(&system_model_id, context_window_tokens),
            None,
            Some(system_model_id),
        )
    };
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some(automation.workspace_code.clone()),
                title: Some(format!("Automation: {}", automation.name)),
                app_code: None,
                task_code: None,
                source_type: Some(ChatSessionSourceType::Automation),
                source_code: Some(automation.automation_code.clone()),
                agent_code: Some(resolved_runtime.agent_code.clone()),
            },
            None,
        )
        .map_err(|error| format!("Automation chat session creation failed: {error}"))?;
    let result = chats
        .send_message(
            &session.session_code,
            SendChatMessageCommand {
                run_code: None,
                content: Some(automation.prompt_text.clone()),
                workspace_code: Some(automation.workspace_code.clone()),
                app_code: None,
                task_code: None,
                agent_code: Some(resolved_runtime.agent_code.clone()),
                model_code,
                system_model_id,
                reasoning_effort: None,
                limits: None,
                tools: Vec::new(),
                attachments: Vec::new(),
                client_capabilities: None,
                resume: Vec::new(),
            },
            model,
            None,
            Some(AUTOMATION_LOCALE.to_string()),
            None,
            agent,
        )
        .await
        .map_err(|error| format!("Main agent automation run failed: {error:?}"))?;
    let session_code = session.session_code.clone();
    let final_reply = result.run.final_output.trim();
    Ok(AutomationRunResult {
        session_code: session_code.clone(),
        summary: if final_reply.is_empty() {
            format!(
                "Agent {} completed automation execution {} in session {} with run {}.",
                resolved_runtime.agent_code,
                execution.execution_code,
                session_code,
                result.run.run_code
            )
        } else {
            final_reply.to_string()
        },
    })
}
