use crate::domain::prompt::{PromptError, PromptVariable, analyze_template};

use super::super::PromptSpecCommand;

pub(super) fn normalize_spec(
    mut spec: PromptSpecCommand,
) -> Result<(PromptSpecCommand, Vec<PromptVariable>), PromptError> {
    spec.display_name = required(spec.display_name, "displayName")?;
    if spec.display_name.chars().count() > 120 {
        return Err(PromptError::InvalidInput(
            "displayName must not exceed 120 characters".into(),
        ));
    }
    spec.prompt_body = required(spec.prompt_body, "promptBody")?;
    spec.description = optional(spec.description);
    spec.model_code = optional(spec.model_code);
    if spec
        .temperature
        .is_some_and(|value| !value.is_finite() || !(0.0..=2.0).contains(&value))
    {
        return Err(PromptError::InvalidInput(
            "temperature must be between 0 and 2".into(),
        ));
    }
    if spec.max_output_tokens == Some(0) {
        return Err(PromptError::InvalidInput(
            "maxOutputTokens must be greater than zero".into(),
        ));
    }
    let variables = analyze_template(&spec.prompt_body)?
        .into_iter()
        .map(PromptVariable::string)
        .collect();
    Ok((spec, variables))
}

pub(super) fn ensure_version(actual: u64, expected: Option<u64>) -> Result<(), PromptError> {
    if expected.is_some_and(|version| version != actual) {
        Err(PromptError::VersionConflict)
    } else {
        Ok(())
    }
}

fn required(value: String, field: &str) -> Result<String, PromptError> {
    let value = value.trim();
    if value.is_empty() {
        Err(PromptError::InvalidInput(format!("{field} is required")))
    } else {
        Ok(value.into())
    }
}

fn optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}
