//! Resolve business selection once; all execution/checkpointing belongs to Framework.
use super::model::HostExecution;
use crate::{
    application::agent::{AgentManagementService, management::loop_designs},
    domain::agent::loop_design::*,
};
use eliteforge_agent_core as fw;
use fw::{AgentLoop, WorkflowNode};

pub(super) struct ConfiguredLoop(pub Option<AgentManagementService>);
#[async_trait::async_trait]
impl AgentLoop for ConfiguredLoop {
    fn execution_kind(&self) -> &str {
        "TRON_CONFIGURED"
    }
    fn supports_resume(&self) -> bool {
        true
    }
    fn requires_interactions(&self) -> bool {
        true
    }
    fn supports_tool_policy(&self) -> bool {
        true
    }
    fn requires_model(&self) -> bool {
        true
    }
    fn requires_state(&self) -> bool {
        true
    }
    fn cooperative_cancellation(&self) -> bool {
        true
    }
    async fn run(
        &self,
        request: &fw::RunRequest,
        context: &fw::RunContext,
        services: &fw::AgentServices,
    ) -> Result<fw::RunResult, fw::AgentError> {
        // Resume the exact admitted definition, even after edits/deletion/rebinding.
        if let Some(resume) = &request.resume {
            let checkpoint = services
                .state
                .as_ref()
                .ok_or(fw::AgentError::MissingService("StateStore"))?
                .load(resume.run_id)
                .await?;
            if checkpoint.loop_id == "framework.react" {
                return eliteforge_agent_framework::ReactAgentLoop
                    .run(request, context, services)
                    .await;
            }
            if checkpoint.state.get("schema_version").is_some() {
                let state: fw::DagWorkflowState =
                    serde_json::from_value(checkpoint.state).map_err(invalid)?;
                return eliteforge_agent_framework::DagWorkflowLoop::new(state.definition)?
                    .run(request, context, services)
                    .await;
            }
            let state: fw::WorkflowState =
                serde_json::from_value(checkpoint.state).map_err(invalid)?;
            return eliteforge_agent_framework::WorkflowLoop::new(state.definition)?
                .run(request, context, services)
                .await;
        }
        let design = if let Some(design) = request.extensions.get::<LoopDesign>() {
            (*design).clone()
        } else if let Some(management) = &self.0 {
            let host = HostExecution::from_extensions(&request.extensions)?;
            let state = host.state.lock().await;
            let policy = management
                .workspace_internal_agent_policy(
                    &state.context.workspace_code,
                    state.context.agent_code.as_deref().unwrap_or("main-agent"),
                )
                .map_err(|e| invalid(format!("{e:?}")))?;
            management
                .loop_design(&policy.loop_code)
                .map_err(|e| invalid(format!("{e:?}")))?
        } else {
            LoopDesign::react()
        };
        match design.kind {
            LoopKind::React => {
                eliteforge_agent_framework::ReactAgentLoop
                    .run(request, context, services)
                    .await
            }
            LoopKind::Workflow => {
                eliteforge_agent_framework::WorkflowLoop::new(loop_designs::workflow(&design))?
                    .run(request, context, services)
                    .await
            }
            LoopKind::Dag => {
                eliteforge_agent_framework::DagWorkflowLoop::new(
                    loop_designs::dag(&design).map_err(|e| invalid(format!("{e:?}")))?,
                )?
                .run(request, context, services)
                .await
            }
        }
    }
}
fn invalid(e: impl ToString) -> fw::AgentError {
    fw::AgentError::InvalidConfiguration(e.to_string())
}

/// Pure conditional routing node; never invokes a model or tool.
pub(super) struct BranchNode;
#[async_trait::async_trait]
impl WorkflowNode for BranchNode {
    fn kind(&self) -> &str {
        "tron.branch"
    }
    fn supports_tool_policy(&self) -> bool {
        true
    }
    fn supports_resume(&self) -> bool {
        true
    }
    fn validate(&self, config: &serde_json::Value) -> Result<(), fw::AgentError> {
        serde_json::from_value::<BranchCondition>(config.clone())
            .map(|_| ())
            .map_err(invalid)
    }
    async fn execute(
        &self,
        input: fw::WorkflowNodeInput,
        _: &fw::WorkflowNodeContext,
        _: &fw::AgentServices,
    ) -> Result<fw::WorkflowNodeResult, fw::AgentError> {
        let condition: BranchCondition = serde_json::from_value(input.config).map_err(invalid)?;
        let source = input
            .predecessors
            .get(&condition.source_node)
            .ok_or_else(|| invalid("branch source was not selected"))?;
        // Model nodes return text. Decode JSON text when the condition selects a field.
        let decoded = if condition.pointer.is_empty() {
            None
        } else {
            source
                .as_str()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        };
        let source = decoded.as_ref().unwrap_or(source);
        let matched = source.pointer(&condition.pointer) == Some(&condition.equals);
        let target = if matched {
            condition.match_target
        } else {
            condition.else_target
        };
        let mut result = fw::WorkflowNodeResult::completed(serde_json::json!({"matched":matched}));
        result.routes = Some([target].into_iter().collect());
        Ok(result)
    }
}

/// Give parallel nodes independent provider continuation/context state. Framework
/// still owns node execution, tools, budgets, cancellation and durable recovery.
pub(super) struct ModelStageNode;
#[async_trait::async_trait]
impl WorkflowNode for ModelStageNode {
    fn kind(&self) -> &str {
        "tron.model-stage"
    }
    fn requires_model(&self) -> bool {
        true
    }
    fn supports_tool_policy(&self) -> bool {
        true
    }
    fn supports_resume(&self) -> bool {
        true
    }
    fn configuration_schema(&self) -> serde_json::Value {
        eliteforge_agent_framework::ReactWorkflowNode.configuration_schema()
    }
    fn validate(&self, config: &serde_json::Value) -> Result<(), fw::AgentError> {
        eliteforge_agent_framework::ReactWorkflowNode.validate(config)
    }
    async fn execute(
        &self,
        input: fw::WorkflowNodeInput,
        context: &fw::WorkflowNodeContext,
        services: &fw::AgentServices,
    ) -> Result<fw::WorkflowNodeResult, fw::AgentError> {
        let Some(root) = context.run.extensions.get::<HostExecution>() else {
            return eliteforge_agent_framework::ReactWorkflowNode
                .execute(input, context, services)
                .await;
        };
        let state = {
            let parent = root.state.lock().await;
            let mut state = super::model::HostRunState::new(parent.context.clone());
            state.trusted_prefix = parent.trusted_prefix.clone();
            state.context.observations.clear();
            state
        };
        let state = std::sync::Arc::new(tokio::sync::Mutex::new(state));
        let mut context = context.clone();
        context.run.extensions.insert(HostExecution {
            harness: root.harness.clone(),
            state: state.clone(),
            sink: root.sink.clone(),
            summaries: root.summaries.clone(),
        });
        let result = eliteforge_agent_framework::ReactWorkflowNode
            .execute(input, &context, services)
            .await;
        let node = state.lock().await;
        let mut parent = root.state.lock().await;
        parent.usage = match (parent.usage, node.usage) {
            (Some(a), Some(b)) => Some(a.add(b)),
            (a, b) => a.or(b),
        };
        parent
            .context
            .observations
            .extend(node.context.observations.clone());
        parent.estimate = node.estimate.clone();
        parent.audit = node.audit.clone();
        if node.error.is_some() {
            parent.error = node.error.clone();
        }
        result
    }
}
