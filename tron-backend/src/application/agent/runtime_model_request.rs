use crate::{
    application::short_term_memory::{ModelRequestTranscriptFacts, PreparedModelContext},
    domain::agent::{AgentModelProtocol, AgentRunContext, ModelRequest},
};

use super::runtime_helpers::hash_text;

pub(super) fn model_request_facts(
    request: &ModelRequest,
    context: &AgentRunContext,
    prepared_context: &PreparedModelContext,
    usage_estimate: crate::domain::agent::TokenEstimateSnapshot,
) -> ModelRequestTranscriptFacts {
    ModelRequestTranscriptFacts {
        estimated_tokens: prepared_context.estimated_tokens,
        message_hashes: request
            .messages
            .iter()
            .map(|message| hash_text(&message.text_content()))
            .collect(),
        tool_schema_token_estimate: prepared_context.tool_schema_token_estimate,
        protocol: protocol_code(context.model.protocol).into(),
        previous_response_id_present: request.previous_response_id.is_some(),
        context_window_estimate: context.model.context_window_tokens,
        checkpoint_code: prepared_context.checkpoint_code.clone(),
        budget_breakdown: prepared_context.budget_breakdown.clone(),
        over_budget_reason: prepared_context.over_budget_reason.clone(),
        usage_estimate: Some(usage_estimate),
    }
}

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

fn protocol_code(protocol: AgentModelProtocol) -> &'static str {
    match protocol {
        AgentModelProtocol::ChatCompletions => "chatCompletions",
        AgentModelProtocol::Responses => "responses",
        AgentModelProtocol::Embeddings => "embeddings",
    }
}
