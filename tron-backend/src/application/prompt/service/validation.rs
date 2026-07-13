use std::collections::HashMap;

use crate::domain::prompt::{PromptError, PromptVariable, analyze_template};

use super::super::{PromptSpecCommand, PromptVariableSpecCommand};

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
    let variables = normalize_variables(&spec.prompt_body, std::mem::take(&mut spec.variables))?;
    Ok((spec, variables))
}

fn normalize_variables(
    prompt_body: &str,
    variables: Vec<PromptVariableSpecCommand>,
) -> Result<Vec<PromptVariable>, PromptError> {
    let names = analyze_template(prompt_body)?;
    let mut by_name = HashMap::with_capacity(variables.len());
    for variable in variables {
        if by_name.insert(variable.name.clone(), variable).is_some() {
            return Err(variable_contract_error());
        }
    }
    if by_name.len() != names.len() {
        return Err(variable_contract_error());
    }

    let mut normalized = Vec::with_capacity(names.len());
    for name in names {
        let Some(variable) = by_name.remove(&name) else {
            return Err(variable_contract_error());
        };
        let description = variable.description.trim();
        if description.is_empty() {
            return Err(PromptError::InvalidInput(format!(
                "variable description is required: {name}"
            )));
        }
        if description.chars().count() > 200 {
            return Err(PromptError::InvalidInput(format!(
                "variable description must not exceed 200 characters: {name}"
            )));
        }
        normalized.push(PromptVariable {
            name,
            variable_type: variable.variable_type,
            description: description.to_string(),
        });
    }
    if !by_name.is_empty() {
        return Err(variable_contract_error());
    }
    Ok(normalized)
}

fn variable_contract_error() -> PromptError {
    PromptError::InvalidInput("variables must match promptBody placeholders".into())
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
