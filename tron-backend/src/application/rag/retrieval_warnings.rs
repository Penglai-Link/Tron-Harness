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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::rag::RagRetrievalStage;
    #[test]
    fn input_and_provider_failures_are_classified_and_not_duplicated() {
        let provider = r#"SDK_MODEL_ERROR:{"Provider":{"provider":"test","category":"transient","http_status":502,"business_code":null,"error_type":"gateway_error","model":null,"message":"Bad Gateway"}}"#;
        let mut result = RagEvidenceResult::default();
        for error in ["query is required", provider] {
            result
                .warnings
                .push(format!("重排失败，已保留融合排序: {error}"));
            result.stages.push(RagRetrievalStage {
                corpus_id: String::new(),
                channel: "rerank".into(),
                status: "FAILED".into(),
                hit_count: 1,
                duration_ms: 1,
                error: Some(error.into()),
            });
        }
        let warnings = result.warning_details();
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0]["category"], "INVALID_REQUEST");
        assert_eq!(warnings[1]["category"], "UPSTREAM");
        assert_eq!(warnings[1]["httpStatus"], 502);
        assert!(
            warnings
                .iter()
                .all(|w| w["stage"] == "rerank" && w["fallback"] == "FUSED_ORDER_RETAINED")
        );
    }
}
