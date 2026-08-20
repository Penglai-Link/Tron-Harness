use super::*;

const MIN_VERIFY_OUTPUT_TOKENS: u32 = 16;

pub(super) fn default_verify_path(protocol: ModelProtocol, model_type: ModelType) -> &'static str {
    match (protocol, model_type) {
        (ModelProtocol::OpenAiCompatible, ModelType::Text) => "/chat/completions",
        (ModelProtocol::OpenAiCompatible, ModelType::Embedding) => "/embeddings",
        (ModelProtocol::Responses, ModelType::Text) => "/responses",
        (ModelProtocol::Responses, ModelType::Image) => "/responses",
        (ModelProtocol::Rerank, ModelType::Rerank) => "/rerank",
        _ => "/verify",
    }
}

pub(super) fn verify_request_payload(
    protocol: ModelProtocol,
    model_type: ModelType,
    model_name: &str,
) -> Value {
    match (protocol, model_type) {
        (ModelProtocol::OpenAiCompatible, ModelType::Text) => json!({
            "model": model_name,
            "messages": [{ "role": "user", "content": "ping" }],
            "max_tokens": MIN_VERIFY_OUTPUT_TOKENS
        }),
        (ModelProtocol::OpenAiCompatible, ModelType::Embedding) => json!({
            "model": model_name,
            "input": "ping"
        }),
        (ModelProtocol::Responses, ModelType::Text) => json!({
            "model": model_name,
            "input": "ping",
            "max_output_tokens": MIN_VERIFY_OUTPUT_TOKENS
        }),
        (ModelProtocol::Responses, ModelType::Image) => json!({
            "model": model_name,
            "input": [
                {
                    "role": "user",
                    "content": [
                        { "type": "input_text", "text": "Generate a verification image." }
                    ]
                }
            ],
            "max_output_tokens": MIN_VERIFY_OUTPUT_TOKENS
        }),
        (ModelProtocol::Rerank, ModelType::Rerank) => json!({
            "model": model_name,
            "query": "ping",
            "documents": ["ping"]
        }),
        _ => json!({}),
    }
}

pub(super) fn join_verify_url(base_url: &str, path: &str) -> Result<Url, ModelError> {
    let mut base = Url::parse(base_url).map_err(|_| ModelError::InvalidBaseUrl)?;
    let trimmed_path = path.trim().trim_start_matches('/');
    if trimmed_path.is_empty() {
        return Ok(base);
    }

    let base_path = base.path().trim_end_matches('/');
    let joined_path = if base_path.is_empty() {
        format!("/{trimmed_path}")
    } else {
        format!("{base_path}/{trimmed_path}")
    };
    base.set_path(&joined_path);
    Ok(base)
}

pub(super) fn truncate_message(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= 180 {
        return trimmed.into();
    }
    let truncated: String = trimmed.chars().take(177).collect();
    format!("{truncated}...")
}
