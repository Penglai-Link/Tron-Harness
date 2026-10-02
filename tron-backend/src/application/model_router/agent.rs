use crate::application::{
    chat_auto_model_router::ChatAutoModelRouter, model::ModelService,
    model_provider::ModelProviderService,
};

/// Agent conversation router. The implementation keeps the existing
/// session/agent-aware Auto routing semantics behind the common router name.
#[derive(Clone)]
pub struct AgentModelRouter {
    inner: ChatAutoModelRouter,
}

impl AgentModelRouter {
    pub fn new(models: ModelService, providers: ModelProviderService) -> Self {
        Self {
            inner: ChatAutoModelRouter::new(models, providers),
        }
    }

    pub fn resolve_initial(
        &self,
        workspace_code: &str,
        session_code: &str,
        agent_code: &str,
        reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    ) -> Result<
        (
            crate::domain::agent::AgentModelConfig,
            crate::domain::agent::AutoModelRouteContext,
        ),
        String,
    > {
        self.inner
            .resolve_initial(workspace_code, session_code, agent_code, reasoning_effort)
    }

    pub fn authorized_models(
        &self,
        workspace: &str,
        current: &crate::domain::agent::AgentModelConfig,
    ) -> Result<Vec<crate::domain::agent::AgentModelConfig>, String> {
        self.inner.authorized_models(workspace, current)
    }
    pub fn mark_success(
        &self,
        workspace_code: &str,
        route: &mut crate::domain::agent::AutoModelRouteContext,
        model: &crate::domain::agent::AgentModelConfig,
    ) -> Result<(), String> {
        self.inner.mark_success(workspace_code, route, model)
    }
}

impl super::router::ModelRouter for AgentModelRouter {}
