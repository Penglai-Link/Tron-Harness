use super::RagEvidenceResult;
use eliteforge_agent_core::ModelError;
use serde_json::{Value, json};

fn error_details(message: &str) -> Value {
    let parsed = message
        .split_once("SDK_MODEL_ERROR:")
        .and_then(|(_, payload)| serde_json::from_str::<ModelError>(payload).ok());
    match parsed {
        Some(ModelError::Provider(error)) => {
            json!({"category":"UPSTREAM","provider":error.provider,"providerCategory":error.category,"httpStatus":error.http_status})
        }
        Some(ModelError::Transient(_) | ModelError::Unavailable(_)) => {
            json!({"category":"UPSTREAM"})
        }
        _ if message.to_lowercase().contains("query is required") => {
            json!({"category":"INVALID_REQUEST"})
        }
        _ => json!({"category":"EXECUTION"}),
    }
}
impl RagEvidenceResult {
    /// A stage failure and its fallback are separate from informational warnings.
    /// Preserve the full message while exposing structured provider categories.
    pub fn warning_details(&self) -> Vec<Value> {
        let failures = self
            .stages
            .iter()
            .filter(|stage| stage.error.is_some())
            .collect::<Vec<_>>();
        let mut details = failures
            .iter()
            .map(|stage| {
                let message = stage.error.as_deref().unwrap();
                let mut detail = error_details(message);
                detail["code"] = json!(format!("RAG_{}_FAILED", stage.channel.to_uppercase()));
                detail["stage"] = json!(stage.channel);
                detail["corpusId"] = json!(stage.corpus_id);
                detail["message"] = json!(message);
                detail["fallback"] = json!(if stage.channel == "rerank" {
                    "FUSED_ORDER_RETAINED"
                } else {
                    "OTHER_CHANNELS_RETAINED"
                });
                detail
            })
            .collect::<Vec<_>>();
        details.extend(self.warnings.iter().filter(|warning| !failures.iter().any(|stage| warning.contains(stage.error.as_deref().unwrap())))
            .map(|warning| json!({"code":"RAG_RETRIEVAL_PARTIAL","stage":"retrieval","category":"NOTICE","message":warning})));
        details
    }
}
