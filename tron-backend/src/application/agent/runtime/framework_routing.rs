use super::*;
impl ReactAgent {
    pub(crate) fn framework_model_error(
        &self,
        context: &AgentRunContext,
        message: &str,
    ) -> eliteforge_agent_core::ModelError {
        if runtime_helpers::is_context_overflow_error(message)
            && self.short_term_memory.is_some()
            && context.short_term_memory.is_some()
        {
            return eliteforge_agent_core::ModelError::ContextOverflow(message.into());
        }
        if ((self.auto_model_router.is_some() && context.auto_model_route.is_some())
            || context.model.max_retries.unwrap_or(0) > 0)
            && is_retryable_auto_route_error(message, &context.model)
        {
            return eliteforge_agent_core::ModelError::Transient(message.into());
        }
        eliteforge_agent_core::ModelError::Fatal(message.into())
    }
    pub(crate) fn framework_overflow_limit(&self) -> usize {
        self.short_term_memory
            .as_ref()
            .map(|memory| memory.overflow_retry_limit() as usize)
            .unwrap_or(0)
    }
    pub(crate) fn framework_fallback(
        &self,
        context: &mut AgentRunContext,
        turn: &mut TurnStateAdapter,
        message: &str,
    ) -> Result<(), AgentError> {
        let unavailable = is_person_ai_auth_or_connection_error(message, &context.model);
        let (Some(router), Some(route)) = (
            self.auto_model_router.as_ref(),
            context.auto_model_route.as_mut(),
        ) else {
            // Fixed-model retry is admitted and counted by Framework.
            return Ok(());
        };
        let model = router.next_fallback(
            &context.workspace_code,
            route,
            &context.model,
            context.model.reasoning_effort,
            unavailable,
        );
        let mut model = match model {
            Ok(Some(model)) => model,
            Ok(None) => return Err(AgentError::ModelFailure(message.into())),
            Err(error) => {
                self.publish(
                    context,
                    "agent.model.fallback.failed",
                    BusOutcome::Failure,
                    Some(&error),
                )?;
                return Err(AgentError::ModelFailure(message.into()));
            }
        };
        if let Some(session) = context
            .auto_model_route
            .as_ref()
            .map(|route| route.session_code.as_str())
        {
            model = crate::adapters::outbound::model::with_person_ai_conversation(
                model,
                session,
                Some(&context.run_code),
            );
        }
        self.publish_with_payload(
            context,
            "agent.model.fallback",
            BusOutcome::Started,
            Some("retryable_model_failure"),
            vec![
                ("failed_model_code".into(), context.model.model_code.clone()),
                ("model_code".into(), model.model_code.clone()),
                ("model_name".into(), model.model_name.clone()),
                ("reason".into(), message.into()),
            ],
        )?;
        context.model = model;
        *turn = TurnStateAdapter::new(
            context.model.protocol,
            context.model.responses_stateful_enabled,
        );
        Ok(())
    }
}
