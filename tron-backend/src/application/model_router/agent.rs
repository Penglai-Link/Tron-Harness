use crate::application::{
    chat_auto_model_router::ChatAutoModelRouter, model::ModelService,
    model_provider::ModelProviderService,
};

/// Agent conversation router. The implementation keeps the existing
/// session/agent-aware Auto routing semantics behind the common router name.
#[derive(Clone)]
pub struct AgentModelRouter {
    inner: ChatAutoModelRouter,
    management: Option<crate::application::agent::AgentManagementService>,
}

impl AgentModelRouter {
    pub fn new(models: ModelService, providers: ModelProviderService) -> Self {
        Self {
            inner: ChatAutoModelRouter::new(models, providers),
            management: None,
        }
    }

    pub fn with_agent_management(
        mut self,
        management: crate::application::agent::AgentManagementService,
    ) -> Self {
        self.management = Some(management);
        self
    }

    fn candidate_codes(&self, workspace: &str, agent: &str) -> Result<Vec<String>, String> {
        self.management
            .as_ref()
            .map(|management| {
                if workspace.is_empty()
                    || management
                        .is_dude_definition(agent)
                        .map_err(|error| format!("Agent kind lookup failed: {error:?}"))?
                {
                    management.platform_internal_agent_policy(agent)
                } else {
                    management.workspace_internal_agent_policy(workspace, agent)
                }
                .map(|policy| policy.candidate_model_codes)
                .map_err(|error| format!("Agent model candidates lookup failed: {error:?}"))
            })
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    pub fn authorized_models_for_route(
        &self,
        workspace: &str,
        route: &crate::domain::agent::AutoModelRouteContext,
        current: &crate::domain::agent::AgentModelConfig,
    ) -> Result<Vec<crate::domain::agent::AgentModelConfig>, String> {
        // A refresh may revoke candidates, but must never escape this run's configured pool.
        let current_codes = self.candidate_codes(workspace, &route.agent_code)?;
        let mut models = self.inner.authorized_models_with_candidates(
            workspace,
            current,
            &route.candidate_model_codes,
        )?;
        if !current_codes.is_empty() {
            let currently_allowed =
                self.inner
                    .authorized_models_with_candidates(workspace, current, &current_codes)?;
            models.retain(|model| {
                currently_allowed
                    .iter()
                    .any(|allowed| allowed.identity() == model.identity())
            });
        }
        Ok(models)
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
        let codes = self.candidate_codes(workspace_code, agent_code)?;
        self.inner.resolve_initial_with_candidates(
            workspace_code,
            session_code,
            agent_code,
            reasoning_effort,
            &codes,
        )
    }

    pub fn authorized_models(
        &self,
        workspace: &str,
        current: &crate::domain::agent::AgentModelConfig,
    ) -> Result<Vec<crate::domain::agent::AgentModelConfig>, String> {
        self.inner.authorized_models(workspace, current)
    }
    pub fn snapshot_models(
        &self,
        route: &crate::domain::agent::AutoModelRouteContext,
        current: &crate::domain::agent::AgentModelConfig,
    ) -> Result<Vec<crate::domain::agent::AgentModelConfig>, String> {
        self.inner.snapshot_models(route, current)
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
