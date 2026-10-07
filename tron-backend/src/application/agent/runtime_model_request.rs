use crate::{application::short_term_memory::PreparedModelContext, domain::agent::AgentRunContext};

pub(super) fn prepared_context_event_payload(
    prepared_context: &PreparedModelContext,
) -> Vec<(String, String)> {
    let mut payload = Vec::new();
    if let Some(breakdown) = prepared_context.budget_breakdown.as_ref()
        && let Ok(value) = serde_json::to_string(breakdown)
    {
        payload.push(("budgetBreakdown".to_string(), value));
    }
    if let Some(reason) = prepared_context.over_budget_reason.as_ref() {
        payload.push(("overBudgetReason".to_string(), reason.clone()));
    }
    payload
}

impl super::ReactAgent {
    pub(super) fn model_context_budget_plan(
        &self,
        context: &AgentRunContext,
    ) -> crate::application::short_term_memory::ModelContextBudgetPlan {
        self.short_term_memory
            .as_ref()
            .map(|service| service.model_context_budget_plan(&context.model))
            .unwrap_or_else(|| {
                crate::application::short_term_memory::ShortTermMemoryConfig::default()
                    .model_context_budget_plan(
                        context.model.context_window_tokens,
                        context.model.max_tokens,
                    )
            })
    }

    pub(super) fn tool_schema_limit(&self, context: &AgentRunContext) -> u32 {
        self.model_context_budget_plan(context)
            .block_budgets
            .tool_schema
            .effective_limit_tokens
    }
}
