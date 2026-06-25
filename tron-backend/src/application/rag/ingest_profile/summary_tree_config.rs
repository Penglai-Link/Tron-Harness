use serde_json::Value;

use crate::domain::rag::RagError;

use super::{optional_stage_strategy_id, stage_config};

pub(super) fn validate_summary_tree_config(
    stages: &serde_json::Map<String, Value>,
) -> Result<(), RagError> {
    if optional_stage_strategy_id(stages, "summarizeTree")?.as_deref()
        != Some("RAG_TREE_LLM_SUMMARY_V1")
    {
        return Ok(());
    }
    let Some(config_value) = stage_config(stages, "summarizeTree") else {
        return Ok(());
    };
    let config = config_value
        .as_object()
        .ok_or(RagError::IngestProfileInvalid)?;
    for (key, value) in config {
        match key.as_str() {
            "modelCode" => validate_optional_string_config(value)?,
            "timeoutSeconds" | "maxPromptChars" => {
                validate_optional_positive_integer_config(value)?
            }
            _ => return Err(RagError::IngestProfileInvalid),
        }
    }
    Ok(())
}

fn validate_optional_string_config(value: &Value) -> Result<(), RagError> {
    match value {
        Value::Null | Value::String(_) => Ok(()),
        _ => Err(RagError::IngestProfileInvalid),
    }
}

fn validate_optional_positive_integer_config(value: &Value) -> Result<(), RagError> {
    match value {
        Value::Null => Ok(()),
        Value::Number(number) if number.as_u64().is_some_and(|value| value > 0) => Ok(()),
        _ => Err(RagError::IngestProfileInvalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_summary_tree_config_allows_default_llm_model() {
        for config in [
            None,
            Some(json!({})),
            Some(json!({ "modelCode": null })),
            Some(json!({ "modelCode": "" })),
            Some(json!({ "modelCode": "   " })),
        ] {
            assert!(validate_summary_tree_config(&summarize_tree_stages(config)).is_ok());
        }
    }

    #[test]
    fn validate_summary_tree_config_rejects_invalid_model_code_shape() {
        for config in [
            json!({ "modelCode": 42 }),
            json!({ "timeoutSeconds": 0 }),
            json!({ "maxPromptChars": -1 }),
            json!({ "summaryMaxTokens": 128 }),
            json!({ "thinkingEnabled": false }),
            json!({ "modelCode": "MODEL-1", "unexpected": true }),
        ] {
            assert!(matches!(
                validate_summary_tree_config(&summarize_tree_stages(Some(config))),
                Err(RagError::IngestProfileInvalid)
            ));
        }
    }

    #[test]
    fn validate_summary_tree_config_allows_llm_runtime_overrides() {
        assert!(
            validate_summary_tree_config(&summarize_tree_stages(Some(json!({
                "modelCode": "MODEL-1",
                "timeoutSeconds": 15,
                "maxPromptChars": 2048
            }))))
            .is_ok()
        );
    }

    fn summarize_tree_stages(config: Option<Value>) -> serde_json::Map<String, Value> {
        let mut stage = serde_json::Map::from_iter([(
            "strategyId".into(),
            Value::String("RAG_TREE_LLM_SUMMARY_V1".into()),
        )]);
        if let Some(config) = config {
            stage.insert("config".into(), config);
        }
        serde_json::Map::from_iter([("summarizeTree".into(), Value::Object(stage))])
    }
}
