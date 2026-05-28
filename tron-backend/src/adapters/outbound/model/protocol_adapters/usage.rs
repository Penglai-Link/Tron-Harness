use serde_json::Value;

use crate::domain::agent::ModelTokenUsage;

pub(super) fn parse_openai_compatible_usage(body: &Value) -> Option<ModelTokenUsage> {
    let usage = body.get("usage")?;
    let input_tokens = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(token_count)?;
    let output_tokens = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(token_count)?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(token_count)
        .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));
    Some(ModelTokenUsage {
        input_tokens,
        output_tokens,
        total_tokens,
    })
}

pub(super) fn parse_responses_usage(body: &Value) -> Option<ModelTokenUsage> {
    let usage = body
        .get("usage")
        .or_else(|| body.pointer("/response/usage"))?;
    let input_tokens = usage
        .get("input_tokens")
        .or_else(|| usage.get("prompt_tokens"))
        .and_then(token_count)?;
    let output_tokens = usage
        .get("output_tokens")
        .or_else(|| usage.get("completion_tokens"))
        .and_then(token_count)?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(token_count)
        .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));
    Some(ModelTokenUsage {
        input_tokens,
        output_tokens,
        total_tokens,
    })
}

pub(super) fn parse_input_only_usage(body: &Value) -> Option<ModelTokenUsage> {
    let usage = body.get("usage")?;
    let input_tokens = usage
        .get("input_tokens")
        .or_else(|| usage.get("prompt_tokens"))
        .or_else(|| usage.get("total_tokens"))
        .and_then(token_count)?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(token_count)
        .unwrap_or(input_tokens);
    Some(ModelTokenUsage {
        input_tokens,
        output_tokens: 0,
        total_tokens,
    })
}

fn token_count(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}
