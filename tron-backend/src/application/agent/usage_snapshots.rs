use crate::{
    application::short_term_memory::PreparedModelContext,
    domain::agent::{
        AgentModelProtocol, AgentRunContext, ModelResponse, ModelTokenUsage, TokenEstimateSnapshot,
        TokenUsageAuditSnapshot,
    },
};

pub(crate) fn request_estimate_snapshot(
    context: &AgentRunContext,
    prepared_context: &PreparedModelContext,
) -> TokenEstimateSnapshot {
    TokenEstimateSnapshot {
        estimator: super::framework_runtime::token_estimation::TronTokenEstimator::source(
            &context.model,
        ),
        estimated_input_tokens: prepared_context.estimated_tokens,
        message_token_estimate: prepared_context.message_token_estimate,
        tool_schema_token_estimate: prepared_context.tool_schema_token_estimate,
        context_window_limit: prepared_context
            .context_window_limit
            .or(context.model.context_window_tokens),
        effective_input_budget: prepared_context.effective_input_budget,
        output_token_reserve: prepared_context.output_token_reserve,
        rebased: prepared_context.rebased,
        over_budget: prepared_context.over_budget,
        compaction_trigger_reached: prepared_context.compaction_trigger_reached,
        checkpoint_code: prepared_context.checkpoint_code.clone(),
        model_code: context.model.model_code.clone(),
        model_name: context.model.model_name.clone(),
        model_protocol: protocol_code(context.model.protocol).into(),
        usage_source: "local_estimate".into(),
        usage_confidence: "estimated".into(),
    }
}

pub(crate) fn usage_audit_snapshot(
    request_estimate: TokenEstimateSnapshot,
    response: &ModelResponse,
    cumulative_run_usage: Option<ModelTokenUsage>,
) -> TokenUsageAuditSnapshot {
    let provider_usage = response.token_usage();
    let input_token_delta = provider_usage.map(|usage| {
        i64::from(usage.input_tokens) - i64::from(request_estimate.estimated_input_tokens)
    });
    let input_token_ratio = provider_usage
        .and_then(|usage| token_ratio(usage.input_tokens, request_estimate.estimated_input_tokens));
    let usage_source = if provider_usage.is_some() {
        "provider"
    } else {
        "unavailable"
    };
    let usage_confidence = if provider_usage.is_some() {
        "actual"
    } else {
        "unavailable"
    };

    TokenUsageAuditSnapshot {
        request_estimate,
        provider_usage,
        cumulative_run_usage,
        input_token_delta,
        input_token_ratio,
        response_id: response.response_id().map(str::to_string),
        usage_source: usage_source.into(),
        usage_confidence: usage_confidence.into(),
    }
}

pub(crate) fn usage_event_payload<T: serde::Serialize>(
    key: &str,
    value: &T,
) -> Vec<(String, String)> {
    serde_json::to_string(value)
        .ok()
        .map(|json| vec![(key.to_string(), json)])
        .unwrap_or_default()
}

fn token_ratio(actual: u32, estimated: u32) -> Option<f64> {
    if estimated == 0 {
        return None;
    }
    Some(f64::from(actual) / f64::from(estimated))
}

fn protocol_code(protocol: AgentModelProtocol) -> &'static str {
    match protocol {
        AgentModelProtocol::ChatCompletions => "chatCompletions",
        AgentModelProtocol::Responses => "responses",
        AgentModelProtocol::Embeddings => "embeddings",
        AgentModelProtocol::Anthropic => "anthropic",
        AgentModelProtocol::Gemini => "gemini",
    }
}
