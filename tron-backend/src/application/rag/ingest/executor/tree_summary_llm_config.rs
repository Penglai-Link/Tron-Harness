const DEFAULT_TREE_SUMMARY_MAX_PROMPT_CHARS: usize = 12_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LlmSummaryConfig {
    model_code: Option<String>,
    timeout_seconds: Option<u64>,
    max_prompt_chars: usize,
}

impl Default for LlmSummaryConfig {
    fn default() -> Self {
        Self {
            model_code: None,
            timeout_seconds: None,
            max_prompt_chars: DEFAULT_TREE_SUMMARY_MAX_PROMPT_CHARS,
        }
    }
}

fn llm_summary_config(node: &CompiledRagPipelineNode) -> Result<LlmSummaryConfig, RagError> {
    let Some(config) = node.config.as_ref() else {
        return Ok(LlmSummaryConfig::default());
    };
    let config = config.as_object().ok_or(RagError::IngestProfileInvalid)?;
    if config.keys().any(|key| !is_llm_summary_config_key(key)) {
        return Err(RagError::IngestProfileInvalid);
    }

    let defaults = LlmSummaryConfig::default();
    Ok(LlmSummaryConfig {
        model_code: optional_summary_model_code(config.get("modelCode"))?,
        timeout_seconds: optional_positive_u64(config.get("timeoutSeconds"))?,
        max_prompt_chars: optional_positive_usize(
            config.get("maxPromptChars"),
            defaults.max_prompt_chars,
        )?,
    })
}

fn is_llm_summary_config_key(key: &str) -> bool {
    matches!(key, "modelCode" | "timeoutSeconds" | "maxPromptChars")
}

fn optional_summary_model_code(value: Option<&Value>) -> Result<Option<String>, RagError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(model_code)) => {
            let model_code = model_code.trim();
            Ok((!model_code.is_empty()).then(|| model_code.to_string()))
        }
        Some(_) => Err(RagError::IngestProfileInvalid),
    }
}

fn optional_positive_u64(value: Option<&Value>) -> Result<Option<u64>, RagError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_u64()
            .filter(|value| *value > 0)
            .map(Some)
            .ok_or(RagError::IngestProfileInvalid),
        Some(_) => Err(RagError::IngestProfileInvalid),
    }
}

fn optional_positive_usize(value: Option<&Value>, default: usize) -> Result<usize, RagError> {
    let parsed = optional_positive_u64(value)?.unwrap_or(default as u64);
    usize::try_from(parsed).map_err(|_| RagError::IngestProfileInvalid)
}

fn truncate_tree_summary_prompt(prompt: String, max_chars: usize) -> String {
    if prompt.chars().count() <= max_chars {
        return prompt;
    }
    prompt.chars().take(max_chars).collect()
}

#[cfg(test)]
mod llm_summary_config_tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn llm_summary_config_defaults_when_config_is_blank() {
        for config in [
            None,
            Some(json!({})),
            Some(json!({ "modelCode": null })),
            Some(json!({ "modelCode": "" })),
            Some(json!({ "modelCode": "   " })),
        ] {
            assert_eq!(
                llm_summary_config(&summary_node(config)).unwrap(),
                LlmSummaryConfig::default()
            );
        }
    }

    #[test]
    fn llm_summary_config_trims_explicit_model_code_and_reads_overrides() {
        let config = llm_summary_config(&summary_node(Some(json!({
            "modelCode": " MODEL-1 ",
            "timeoutSeconds": 15,
            "maxPromptChars": 2048
        }))))
        .unwrap();

        assert_eq!(config.model_code, Some("MODEL-1".into()));
        assert_eq!(config.timeout_seconds, Some(15));
        assert_eq!(config.max_prompt_chars, 2048);
    }

    #[test]
    fn llm_summary_config_rejects_invalid_shapes() {
        for config in [
            json!({ "modelCode": 42 }),
            json!({ "timeoutSeconds": 0 }),
            json!({ "timeoutSeconds": "30" }),
            json!({ "maxPromptChars": -1 }),
            json!({ "summaryMaxTokens": 128 }),
            json!({ "thinkingEnabled": false }),
            json!({ "unexpected": true }),
        ] {
            assert!(matches!(
                llm_summary_config(&summary_node(Some(config))),
                Err(RagError::IngestProfileInvalid)
            ));
        }
    }

    #[test]
    fn truncate_tree_summary_prompt_limits_by_chars() {
        assert_eq!(
            truncate_tree_summary_prompt("abcdef".into(), 3),
            "abc".to_string()
        );
    }

    fn summary_node(config: Option<Value>) -> CompiledRagPipelineNode {
        CompiledRagPipelineNode {
            node_id: "summarize-tree".into(),
            stage_type: "SUMMARIZE_TREE".into(),
            strategy_id: "RAG_TREE_LLM_SUMMARY_V1".into(),
            execution_stage_id: "summarizeTree".into(),
            join_policy: "ALL_SUCCESS".into(),
            failure_policy: "FAIL_PIPELINE".into(),
            input_artifact_keys: Vec::new(),
            output_artifact_keys: Vec::new(),
            config,
        }
    }
}
