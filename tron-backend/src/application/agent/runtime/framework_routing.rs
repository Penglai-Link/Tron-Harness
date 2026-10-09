use super::*;
impl ReactAgent {
    pub(crate) fn framework_model_scope(
        &self,
        context: &AgentRunContext,
    ) -> Result<crate::application::agent::framework_runtime::NativeModelScope, AgentError> {
        crate::application::agent::framework_runtime::NativeModelScope::new(self, context)
    }
    pub(crate) fn framework_authorized_models(
        &self,
        context: &AgentRunContext,
    ) -> Result<Vec<AgentModelConfig>, AgentError> {
        self.framework_model_candidates(context, true)
    }
    pub(crate) fn framework_refreshed_models(
        &self,
        context: &AgentRunContext,
    ) -> Result<Vec<AgentModelConfig>, AgentError> {
        self.framework_model_candidates(context, false)
    }
    fn framework_model_candidates(
        &self,
        context: &AgentRunContext,
        initial: bool,
    ) -> Result<Vec<AgentModelConfig>, AgentError> {
        if context.auto_model_route.is_some()
            && let Some(router) = &self.auto_model_router
        {
            let mut models = match context.auto_model_route.as_ref() {
                Some(route) if initial && !route.authorized_candidates.is_empty() => router
                    .snapshot_models(route, &context.model)
                    .map_err(AgentError::InvalidConfiguration)?,
                _ => router
                    .authorized_models(&context.workspace_code, &context.model)
                    .map_err(AgentError::InvalidConfiguration)?,
            };
            if let Some(route) = &context.auto_model_route {
                for model in &mut models {
                    *model = crate::adapters::outbound::model::with_person_ai_conversation(
                        model.clone(),
                        &route.session_code,
                        Some(&context.run_code),
                    );
                }
            }
            if initial
                && !models
                    .iter()
                    .any(|model| model.identity() == context.model.identity())
            {
                return Err(AgentError::InvalidConfiguration(format!(
                    "initial Auto model is no longer authorized: selected {}; authorized candidates [{}]",
                    context.model.model_code,
                    models
                        .iter()
                        .map(|model| model.model_code.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            Ok(models)
        } else {
            Ok(vec![context.model.clone()])
        }
    }
    pub(crate) fn framework_bound_model_client(
        &self,
        config: &AgentModelConfig,
    ) -> Result<Arc<dyn ModelClient>, AgentError> {
        Ok(self
            .model_client
            .bind_for_run(config)
            .map_err(AgentError::InvalidConfiguration)?
            .unwrap_or_else(|| self.model_client.clone()))
    }
    pub(crate) fn framework_model_authorization_scope(
        &self,
        config: &AgentModelConfig,
    ) -> Result<Option<String>, String> {
        self.model_client.authorization_scope(config)
    }
    pub(crate) fn framework_with_model_client(&self, client: Arc<dyn ModelClient>) -> Self {
        let mut harness = self.clone();
        harness.model_client = client;
        harness
    }

    pub(crate) fn framework_model_error(
        &self,
        context: &AgentRunContext,
        message: &str,
    ) -> eliteforge_agent_core::ModelError {
        if let Some(error) =
            eliteforge_agent_official_plugin_model_http::decode_model_error(message)
        {
            return error;
        }
        if runtime_helpers::is_context_overflow_error(message)
            && self.short_term_memory.is_some()
            && context.short_term_memory.is_some()
        {
            return eliteforge_agent_core::ModelError::ContextOverflow(message.into());
        }
        eliteforge_agent_core::ModelError::Fatal(message.into())
    }
    pub(crate) fn framework_overflow_limit(&self) -> usize {
        self.short_term_memory
            .as_ref()
            .map(|memory| memory.overflow_retry_limit() as usize)
            .unwrap_or(0)
    }
}
