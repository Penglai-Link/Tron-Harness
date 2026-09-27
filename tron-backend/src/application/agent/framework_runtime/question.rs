//! Product answer DTO -> official human-tool answer schema.
use crate::domain::agent::AgentError;
use serde_json::{Value, json};
pub(super) fn framework_answer(payload: &Value) -> Result<Value, AgentError> {
    let answers = payload
        .get("answers")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AgentError::InvalidConfiguration("structured question answers are missing".into())
        })?;
    let mut result = serde_json::Map::new();
    for answer in answers {
        let question = answer["questionCode"].as_str().ok_or_else(|| {
            AgentError::InvalidConfiguration("question identity is missing".into())
        })?;
        let value = match answer["answerKind"].as_str() {
            Some("Option") => json!({"option_id": answer["optionCode"]}),
            Some("Other") => json!({"text": answer["otherText"]}),
            Some("Skipped") => json!({"skip": true}),
            _ => {
                return Err(AgentError::InvalidConfiguration(
                    "unknown question answer kind".into(),
                ));
            }
        };
        if result.insert(question.into(), value).is_some() {
            return Err(AgentError::InvalidConfiguration(
                "duplicate question answer".into(),
            ));
        }
    }
    Ok(json!({"answers": result}))
}
