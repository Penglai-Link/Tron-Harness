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
    pub fn with_auto_router(
        mut self,
        router: std::sync::Arc<dyn eliteforge_agent_core::ChatAutoRouter>,
    ) -> Self {
        self.inner = self.inner.with_router(router);
        self
    }
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

    /// Business workspace context must never select a Dude's model catalog.
    fn model_workspace<'a>(&self, workspace: &'a str, agent: &str) -> Result<&'a str, String> {
        if workspace.is_empty() {
            return Ok("");
        }
        let is_dude = self
            .management
            .as_ref()
            .map(|management| {
                management
                    .is_dude_definition(agent)
                    .map_err(|error| format!("Agent kind lookup failed: {error:?}"))
            })
            .transpose()?
            .unwrap_or(false);
        Ok(if is_dude { "" } else { workspace })
    }

    fn route_policy<'a>(
        &self,
        workspace: &'a str,
        agent: &str,
    ) -> Result<(&'a str, Vec<String>), String> {
        let model_workspace = self.model_workspace(workspace, agent)?;
        let Some(management) = self.management.as_ref() else {
            return Ok((model_workspace, Vec::new()));
        };
        let policy = if model_workspace.is_empty() {
            management.platform_internal_agent_policy(agent)
        } else {
            management.workspace_internal_agent_policy(model_workspace, agent)
        }
        .map_err(|error| format!("Agent model candidates lookup failed: {error:?}"))?;
        Ok((model_workspace, policy.candidate_model_codes))
    }

    pub fn authorized_models_for_route(
        &self,
        workspace: &str,
        route: &crate::domain::agent::AutoModelRouteContext,
        current: &crate::domain::agent::AgentModelConfig,
    ) -> Result<Vec<crate::domain::agent::AgentModelConfig>, String> {
        // A refresh may revoke candidates, but must never escape this run's configured pool.
        let (workspace, current_codes) = self.route_policy(workspace, &route.agent_code)?;
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
        let (workspace_code, codes) = self.route_policy(workspace_code, agent_code)?;
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
        let workspace_code = self.model_workspace(workspace_code, &route.agent_code)?;
        self.inner.mark_success(workspace_code, route, model)
    }
}

impl super::router::ModelRouter for AgentModelRouter {}
