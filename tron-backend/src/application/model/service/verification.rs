use super::*;
use eliteforge_agent_official_plugin_model_http::{MIN_PROBE_OUTPUT_TOKENS, ModelProbeKind};
pub(super) const MIN_VERIFY_OUTPUT_TOKENS: u32 = MIN_PROBE_OUTPUT_TOKENS;
fn probe_kind(protocol: ModelProtocol, model_type: ModelType) -> Option<ModelProbeKind> {
    match (protocol, model_type) {
        (ModelProtocol::OpenAiCompatible, ModelType::Text) => {
            Some(ModelProbeKind::ChatCompletionsText)
        }
        (ModelProtocol::OpenAiCompatibleEmbedding, ModelType::Embedding) => {
            Some(ModelProbeKind::Embedding)
        }
        (ModelProtocol::Responses, ModelType::Text) => Some(ModelProbeKind::ResponsesText),
        (ModelProtocol::Responses, ModelType::Image) => Some(ModelProbeKind::ResponsesImage),
        (ModelProtocol::Rerank, ModelType::Rerank) => Some(ModelProbeKind::Rerank),
        _ => None,
    }
}
pub(super) fn default_verify_path(protocol: ModelProtocol, model_type: ModelType) -> &'static str {
    probe_kind(protocol, model_type)
        .map(ModelProbeKind::default_path)
        .unwrap_or("/verify")
}
pub(super) fn verify_request_payload(
    protocol: ModelProtocol,
    model_type: ModelType,
    model_name: &str,
) -> Value {
    probe_kind(protocol, model_type)
        .map(|kind| kind.payload(model_name))
        .unwrap_or_else(|| json!({}))
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
