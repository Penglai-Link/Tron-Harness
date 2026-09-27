use std::collections::HashMap;

use crate::domain::prompt::{
    PromptError, PromptTranslation, PromptTranslationStatus, PromptVariable,
    PromptVariableSourceKind, analyze_template, registered_runtime_prompt_variable,
};

use super::super::{PromptSpecCommand, PromptTranslationSpecCommand, PromptVariableSpecCommand};

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
    normalize_translations(&mut spec)?;
    Ok((spec, variables))
}

fn normalize_translations(spec: &mut PromptSpecCommand) -> Result<(), PromptError> {
    let base_description = spec.description.clone();
    if spec.translations.is_empty() {
        spec.translations = vec![
            super::super::PromptTranslationSpecCommand {
                locale: "en-US".into(),
                display_name: spec.display_name.clone(),
                prompt_body: spec.prompt_body.clone(),
                description: base_description.clone(),
                status: PromptTranslationStatus::Published,
            },
            super::super::PromptTranslationSpecCommand {
                locale: "zh-CN".into(),
                display_name: spec.display_name.clone(),
                prompt_body: spec.prompt_body.clone(),
                description: base_description,
                status: PromptTranslationStatus::Published,
            },
        ];
    }
    let mut seen = std::collections::BTreeSet::new();
    for translation in &mut spec.translations {
        translation.locale = translation.locale.trim().to_string();
        if !matches!(translation.locale.as_str(), "en-US" | "zh-CN")
            || !seen.insert(translation.locale.clone())
        {
            return Err(PromptError::InvalidInput(
                "translations must contain unique en-US and zh-CN entries".into(),
            ));
        }
        translation.display_name =
            required(translation.display_name.clone(), "translation.displayName")?;
        translation.prompt_body =
            required(translation.prompt_body.clone(), "translation.promptBody")?;
        let names = analyze_template(&translation.prompt_body)?;
        let base_names = analyze_template(&spec.prompt_body)?;
        if names != base_names {
            return Err(PromptError::InvalidInput(
                "translation placeholders must match promptBody".into(),
            ));
        }
        translation.description = optional(translation.description.take());
    }
    for locale in ["en-US", "zh-CN"] {
        if !seen.contains(locale) {
            spec.translations
                .push(super::super::PromptTranslationSpecCommand {
                    locale: locale.into(),
                    display_name: spec.display_name.clone(),
                    prompt_body: spec.prompt_body.clone(),
                    description: spec.description.clone(),
                    status: PromptTranslationStatus::Draft,
                });
        }
    }
    Ok(())
}

pub(super) fn translation_models(
    spec: &PromptSpecCommand,
    updated_at: &str,
    source_version: u64,
) -> Vec<PromptTranslation> {
    spec.translations
        .iter()
        .map(|translation| PromptTranslation {
            locale: translation.locale.clone(),
            display_name: translation.display_name.clone(),
            prompt_body: translation.prompt_body.clone(),
            description: translation.description.clone(),
            status: translation.status,
            source_version,
            updated_at: updated_at.to_string(),
        })
        .collect()
}

pub(super) fn synchronize_translation_statuses(
    previous: &[PromptTranslation],
    spec: &mut PromptSpecCommand,
) {
    let base_locale = spec
        .translations
        .iter()
        .find(|translation| {
            translation.display_name == spec.display_name
                && translation.prompt_body == spec.prompt_body
                && translation.description == spec.description
        })
        .map(|translation| translation.locale.clone());
    let base_changed = base_locale.as_deref().is_some_and(|locale| {
        let current = spec
            .translations
            .iter()
            .find(|translation| translation.locale == locale);
        let previous = previous
            .iter()
            .find(|translation| translation.locale == locale);
        match (current, previous) {
            (Some(current), Some(previous)) => translation_content_changed(current, previous),
            (Some(_), None) => true,
            _ => false,
        }
    });

    for translation in &mut spec.translations {
        let changed = previous
            .iter()
            .find(|item| item.locale == translation.locale)
            .is_none_or(|previous| translation_content_changed(translation, previous));
        if changed {
            translation.status = PromptTranslationStatus::Draft;
        } else if base_changed && base_locale.as_deref() != Some(translation.locale.as_str()) {
            translation.status = PromptTranslationStatus::UpdateAvailable;
        }
    }
}

fn translation_content_changed(
    current: &PromptTranslationSpecCommand,
    previous: &PromptTranslation,
) -> bool {
    current.display_name != previous.display_name
        || current.prompt_body != previous.prompt_body
        || current.description != previous.description
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
        let (managed_variable_key, runtime_provider) = match variable.source_kind {
            PromptVariableSourceKind::Managed => {
                let managed_variable_key = required_optional(
                    variable.managed_variable_key,
                    "variable.managedVariableKey",
                )?;
                if variable.runtime_provider.is_some() {
                    return Err(PromptError::InvalidInput(format!(
                        "runtimeProvider is not allowed for managed variable: {name}"
                    )));
                }
                (Some(managed_variable_key), None)
            }
            PromptVariableSourceKind::Runtime => {
                let runtime_provider = variable.runtime_provider.ok_or_else(|| {
                    PromptError::InvalidInput("variable.runtimeProvider is required".into())
                })?;
                if optional(variable.managed_variable_key).is_some() {
                    return Err(PromptError::InvalidInput(format!(
                        "managedVariableKey is not allowed for runtime variable: {name}"
                    )));
                }
                let Some(registered) = registered_runtime_prompt_variable(&name, runtime_provider)
                else {
                    return Err(PromptError::InvalidInput(format!(
                        "unsupported runtimeProvider for variable: {name}"
                    )));
                };
                if registered.variable_type != variable.variable_type {
                    return Err(PromptError::InvalidInput(format!(
                        "runtime variable type does not match provider: {name}"
                    )));
                }
                (None, Some(runtime_provider))
            }
        };
        normalized.push(PromptVariable {
            name,
            variable_type: variable.variable_type,
            description: description.to_string(),
            source_kind: variable.source_kind,
            managed_variable_key,
            runtime_provider,
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

fn required_optional(value: Option<String>, field: &str) -> Result<String, PromptError> {
    value
        .and_then(|value| {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        })
        .ok_or_else(|| PromptError::InvalidInput(format!("{field} is required")))
}
