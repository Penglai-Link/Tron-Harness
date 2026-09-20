use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::{
        llm::{LlmRuntimeService, LlmTextGenerationCallCommand},
        prompt::{PromptPageQuery, PromptRepository},
        variable::VariableService,
        workspace::WorkspaceService,
    },
    domain::prompt::{
        Prompt, PromptRuntimeProvider, PromptScopeKind, PromptSourceKind, PromptVariable,
        PromptVariableSourceKind, PromptVariableType, PromptView, analyze_template,
        render_typed_template,
    },
    shared::{
        id::{IdGenerator, SnowflakeId},
        time::current_timestamp,
    },
};

mod catalog;
mod platform;
mod template;
mod validation;
mod workspace;

const RETIRED_BUILTIN_PROMPT_CATALOG_KEY: &str = "agent.steer-continuity";
const RETIRED_BUILTIN_PROMPT_CODE: &str = "SYS_AGENT_STEER_CONTINUITY";

#[derive(Clone)]
pub struct PromptService {
    repository: Arc<Mutex<Box<dyn PromptRepository>>>,
    workspaces: WorkspaceService,
    variables: VariableService,
    ids: Arc<dyn IdGenerator>,
    llm_runtime: Option<LlmRuntimeService>,
}

#[derive(Clone)]
struct ResolvedPrompt {
    view: PromptView,
    definition: Prompt,
    current_owned: Option<Prompt>,
    upstream_enabled: Option<bool>,
}

impl PromptService {
    pub fn new(
        repository: impl PromptRepository + 'static,
        workspaces: WorkspaceService,
        variables: VariableService,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            variables,
            ids,
            llm_runtime: None,
        }
    }

    pub fn with_llm_runtime(mut self, llm_runtime: LlmRuntimeService) -> Self {
        self.llm_runtime = Some(llm_runtime);
        self
    }

    pub fn bootstrap_builtins(&self) -> Result<(), crate::domain::prompt::PromptError> {
        let existing = self.all_prompts()?;
        for definition in crate::domain::prompt::builtin_prompts() {
            if let Some(current) = existing.iter().find(|prompt| {
                prompt.catalog_key.as_deref() == Some(definition.catalog_key.as_str())
                    || prompt.prompt_code == definition.prompt_code
            }) {
                if current.built_in && current.version < definition.version {
                    let mut prompt = current.clone();
                    let translation = definition
                        .translations
                        .get("en-US")
                        .or_else(|| definition.translations.values().next())
                        .ok_or(crate::domain::prompt::PromptError::StorageFailure)?;
                    let now = current_timestamp();
                    prompt.display_name = translation.display_name.clone();
                    prompt.prompt_body = translation.prompt_body.clone();
                    prompt.description = Some(translation.description.clone());
                    prompt.version = definition.version;
                    prompt.variables =
                        crate::domain::prompt::builtin_prompt_variables(&definition.catalog_key)
                            .expect(
                                "builtin prompt definition must produce its variable contracts",
                            );
                    prompt.translations = definition
                        .translations
                        .iter()
                        .map(|(locale, value)| crate::domain::prompt::PromptTranslation {
                            locale: locale.clone(),
                            display_name: value.display_name.clone(),
                            prompt_body: value.prompt_body.clone(),
                            description: Some(value.description.clone()),
                            status: crate::domain::prompt::PromptTranslationStatus::Published,
                            source_version: definition.version,
                            updated_at: now.clone(),
                        })
                        .collect();
                    prompt.updated_at = now;
                    self.repository
                        .lock()
                        .expect("prompt repository mutex poisoned")
                        .update_prompt(&prompt, current.version)?;
                }
                continue;
            }
            let translation = definition
                .translations
                .get("en-US")
                .or_else(|| definition.translations.values().next())
                .ok_or(crate::domain::prompt::PromptError::StorageFailure)?;
            let now = current_timestamp();
            let prompt = Prompt {
                id: SnowflakeId::new(
                    self.ids
                        .next_id()
                        .map_err(|_| crate::domain::prompt::PromptError::StorageFailure)?
                        .value(),
                ),
                scope_kind: PromptScopeKind::Platform,
                workspace_code: None,
                prompt_code: definition.prompt_code.clone(),
                display_name: translation.display_name.clone(),
                category: definition.category,
                prompt_body: translation.prompt_body.clone(),
                description: Some(translation.description.clone()),
                enabled: true,
                source_kind: PromptSourceKind::Platform,
                source_prompt_code: None,
                allow_inheritance: None,
                version: definition.version,
                variables: crate::domain::prompt::builtin_prompt_variables(&definition.catalog_key)
                    .expect("builtin prompt definition must produce its variable contracts"),
                translations: definition
                    .translations
                    .iter()
                    .map(|(locale, value)| crate::domain::prompt::PromptTranslation {
                        locale: locale.clone(),
                        display_name: value.display_name.clone(),
                        prompt_body: value.prompt_body.clone(),
                        description: Some(value.description.clone()),
                        status: crate::domain::prompt::PromptTranslationStatus::Published,
                        source_version: definition.version,
                        updated_at: now.clone(),
                    })
                    .collect(),
                built_in: true,
                runtime_required: definition.runtime_required,
                catalog_key: Some(definition.catalog_key.clone()),
                created_at: now.clone(),
                updated_at: now,
            };
            self.repository
                .lock()
                .expect("prompt repository mutex poisoned")
                .create_prompt(&prompt)?;
        }
        Ok(())
    }

    pub fn retire_removed_builtins(&self) -> Result<(), crate::domain::prompt::PromptError> {
        let retired_prompt_codes = self
            .all_prompts()?
            .into_iter()
            .filter(|prompt| {
                prompt.catalog_key.as_deref() == Some(RETIRED_BUILTIN_PROMPT_CATALOG_KEY)
                    || prompt.prompt_code == RETIRED_BUILTIN_PROMPT_CODE
                    || prompt.source_prompt_code.as_deref() == Some(RETIRED_BUILTIN_PROMPT_CODE)
            })
            .map(|prompt| prompt.prompt_code)
            .collect::<Vec<_>>();
        let mut repository = self
            .repository
            .lock()
            .expect("prompt repository mutex poisoned");
        for prompt_code in retired_prompt_codes {
            repository.delete_prompt(&prompt_code)?;
        }
        Ok(())
    }

    pub async fn translate_prompt_draft(
        &self,
        prompt_code: Option<&str>,
        workspace_code: Option<&str>,
        source_locale: &str,
        target_locale: &str,
        display_name: &str,
        prompt_body: &str,
        description: Option<&str>,
    ) -> Result<crate::domain::prompt::PromptTranslation, crate::domain::prompt::PromptError> {
        if let Some(prompt_code) = prompt_code.filter(|value| !value.trim().is_empty()) {
            self.all_prompts()?
                .into_iter()
                .find(|prompt| prompt.prompt_code == prompt_code.trim())
                .ok_or(crate::domain::prompt::PromptError::NotFound)?;
        }
        if !matches!(source_locale, "en-US" | "zh-CN")
            || !matches!(target_locale, "en-US" | "zh-CN")
            || source_locale == target_locale
        {
            return Err(crate::domain::prompt::PromptError::InvalidInput(
                "sourceLocale and targetLocale must be different supported locales".into(),
            ));
        }
        let workspace_code = workspace_code
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(crate::domain::prompt::PromptError::WorkspaceRequired)?;
        let llm_runtime = self.llm_runtime.as_ref().ok_or_else(|| {
            crate::domain::prompt::PromptError::InvalidInput("LLM runtime is unavailable".into())
        })?;
        let generated = llm_runtime
            .text_generation_call(LlmTextGenerationCallCommand {
                workspace_code: workspace_code.to_string(),
                model_code: None,
                prompt: translation_prompt(
                    source_locale,
                    target_locale,
                    display_name,
                    prompt_body,
                    description,
                ),
            })
            .await
            .map_err(|error| crate::domain::prompt::PromptError::InvalidInput(error.to_string()))?;
        let translation = parse_translation_output(&generated.output, target_locale)?;
        let source_variables = analyze_template(prompt_body)?;
        let translated_variables = analyze_template(&translation.prompt_body)?;
        if source_variables != translated_variables {
            return Err(crate::domain::prompt::PromptError::InvalidInput(
                "translation placeholders must match promptBody".into(),
            ));
        }
        let now = current_timestamp();
        Ok(crate::domain::prompt::PromptTranslation {
            locale: translation.locale,
            display_name: translation.display_name,
            prompt_body: translation.prompt_body,
            description: translation.description,
            status: crate::domain::prompt::PromptTranslationStatus::Draft,
            source_version: 1,
            updated_at: now,
        })
    }

    pub fn runtime_prompt_text(
        &self,
        workspace_code: Option<&str>,
        catalog_key: &str,
        locale: &str,
        runtime_values: &HashMap<PromptRuntimeProvider, Value>,
    ) -> Result<Option<String>, crate::domain::prompt::PromptError> {
        let prompt = if let Some(workspace_code) = workspace_code {
            self.workspace_detail(workspace_code, &builtin_prompt_code(catalog_key)?)
        } else {
            self.platform_page(PromptPageQuery {
                current: 1,
                size: 200,
                search: None,
                category: None,
                source_kind: None,
                enabled: None,
            })
            .and_then(|page| {
                page.records
                    .into_iter()
                    .find(|record| record.catalog_key.as_deref() == Some(catalog_key))
                    .ok_or(crate::domain::prompt::PromptError::NotFound)
            })
        };
        let (prompt_body, variables) = match prompt {
            Ok(prompt) => {
                if !prompt.enabled {
                    return Ok(None);
                }
                let prompt_body = prompt
                    .translations
                    .iter()
                    .find(|translation| translation.locale == locale)
                    .or_else(|| {
                        prompt
                            .translations
                            .iter()
                            .find(|translation| translation.locale == "en-US")
                    })
                    .map(|value| value.prompt_body.clone())
                    .unwrap_or(prompt.prompt_body.clone());
                (prompt_body, prompt.variables)
            }
            Err(crate::domain::prompt::PromptError::NotFound) => {
                let definition = crate::domain::prompt::builtin_prompt(catalog_key)
                    .ok_or(crate::domain::prompt::PromptError::NotFound)?;
                let prompt_body = definition
                    .translations
                    .get(locale)
                    .or_else(|| definition.translations.get("en-US"))
                    .map(|translation| translation.prompt_body.clone())
                    .ok_or(crate::domain::prompt::PromptError::NotFound)?;
                let variables = crate::domain::prompt::builtin_prompt_variables(catalog_key)
                    .ok_or(crate::domain::prompt::PromptError::NotFound)?;
                (prompt_body, variables)
            }
            Err(error) => return Err(error),
        };
        Ok(Some(self.render_prompt_body(
            workspace_code,
            &variables,
            &prompt_body,
            runtime_values,
        )?))
    }

    fn render_prompt_body(
        &self,
        workspace_code: Option<&str>,
        variables: &[PromptVariable],
        prompt_body: &str,
        runtime_values: &HashMap<PromptRuntimeProvider, Value>,
    ) -> Result<String, crate::domain::prompt::PromptError> {
        let keys = managed_variable_keys(variables);
        let managed_values = if keys.is_empty() {
            BTreeMap::new()
        } else {
            match workspace_code {
                Some(workspace_code) => self
                    .variables
                    .resolve_workspace_prompt_variable_values(workspace_code, &keys),
                None => self.variables.resolve_platform_prompt_variable_values(&keys),
            }
            .map_err(managed_variable_validation_error)?
        };
        let mut typed_values = HashMap::with_capacity(variables.len());
        for variable in variables {
            let value = match variable.source_kind {
                PromptVariableSourceKind::Managed => {
                    let key = variable
                        .managed_variable_key
                        .as_deref()
                        .expect("managed variables are normalized before rendering");
                    let value = managed_values.get(key).ok_or_else(|| {
                        crate::domain::prompt::PromptError::InvalidInput(format!(
                            "managed variable must exist, be non-sensitive, and have a value: {key}"
                        ))
                    })?;
                    managed_variable_value(&variable.name, variable.variable_type, value)?
                }
                PromptVariableSourceKind::Runtime => {
                    let provider = variable
                        .runtime_provider
                        .expect("runtime variables are normalized before rendering");
                    runtime_values.get(&provider).cloned().ok_or_else(|| {
                        crate::domain::prompt::PromptError::InvalidInput(format!(
                            "runtime variable is unavailable: {}",
                            variable.name
                        ))
                    })?
                }
            };
            typed_values.insert(variable.name.clone(), value);
        }
        render_typed_template(prompt_body, variables, &typed_values).map_err(Into::into)
    }

    fn validate_workspace_managed_variables(
        &self,
        workspace_code: &str,
        variables: &[PromptVariable],
    ) -> Result<(), crate::domain::prompt::PromptError> {
        let options = self
            .variables
            .list_workspace_prompt_variable_options(
                workspace_code,
                crate::domain::variable::VariableCatalogQuery { search: None },
            )
            .map_err(managed_variable_validation_error)?;
        let values = self
            .variables
            .resolve_workspace_prompt_variable_values(
                workspace_code,
                &managed_variable_keys(variables),
            )
            .map_err(managed_variable_validation_error)?;
        validate_managed_variables(variables, options.iter().map(|option| option.key.as_str()), &values)
    }

    fn validate_platform_managed_variables(
        &self,
        variables: &[PromptVariable],
    ) -> Result<(), crate::domain::prompt::PromptError> {
        let options = self
            .variables
            .list_platform_prompt_variable_options(
                crate::domain::variable::VariableCatalogQuery { search: None },
            )
            .map_err(managed_variable_validation_error)?;
        let values = self
            .variables
            .resolve_platform_prompt_variable_values(&managed_variable_keys(variables))
            .map_err(managed_variable_validation_error)?;
        validate_managed_variables(variables, options.iter().map(|option| option.key.as_str()), &values)
    }
}

fn managed_variable_keys(variables: &[PromptVariable]) -> BTreeSet<String> {
    variables
        .iter()
        .filter(|variable| variable.source_kind == PromptVariableSourceKind::Managed)
        .filter_map(|variable| variable.managed_variable_key.clone())
        .collect()
}

fn validate_managed_variables<'a>(
    variables: &[PromptVariable],
    available_keys: impl Iterator<Item = &'a str>,
    values: &BTreeMap<String, String>,
) -> Result<(), crate::domain::prompt::PromptError> {
    let available_keys = available_keys.collect::<BTreeSet<_>>();
    for variable in variables
        .iter()
        .filter(|variable| variable.source_kind == PromptVariableSourceKind::Managed)
    {
        let key = variable
            .managed_variable_key
            .as_deref()
            .expect("managed variables are normalized before validation");
        if !available_keys.contains(key) {
            return Err(crate::domain::prompt::PromptError::InvalidInput(format!(
                "managed variable must exist, be non-sensitive, and have a value: {key}"
            )));
        }
        let value = values.get(key).ok_or_else(|| {
            crate::domain::prompt::PromptError::InvalidInput(format!(
                "managed variable must exist, be non-sensitive, and have a value: {key}"
            ))
        })?;
        validate_managed_variable_type(&variable.name, variable.variable_type, value)?;
    }
    Ok(())
}

fn validate_managed_variable_type(
    placeholder_name: &str,
    variable_type: PromptVariableType,
    value: &str,
) -> Result<(), crate::domain::prompt::PromptError> {
    managed_variable_value(placeholder_name, variable_type, value).map(|_| ())
}

fn managed_variable_value(
    placeholder_name: &str,
    variable_type: PromptVariableType,
    value: &str,
) -> Result<Value, crate::domain::prompt::PromptError> {
    if variable_type == PromptVariableType::String {
        return Ok(Value::String(value.to_string()));
    }
    let parsed = serde_json::from_str::<Value>(value).map_err(|_| {
        crate::domain::prompt::PromptError::InvalidInput(format!(
            "managed variable value does not match {}: {placeholder_name}",
            variable_type.code()
        ))
    })?;
    let matches_type = match variable_type {
        PromptVariableType::String => true,
        PromptVariableType::Number => parsed.is_number(),
        PromptVariableType::Boolean => parsed.is_boolean(),
        PromptVariableType::Json => parsed.is_object() || parsed.is_array(),
    };
    matches_type.then_some(parsed).ok_or_else(|| {
        crate::domain::prompt::PromptError::InvalidInput(format!(
            "managed variable value does not match {}: {placeholder_name}",
            variable_type.code()
        ))
    })
}

fn managed_variable_validation_error(
    _: crate::domain::variable::VariableError,
) -> crate::domain::prompt::PromptError {
    crate::domain::prompt::PromptError::InvalidInput(
        "managed variable must exist, be non-sensitive, and have a value".into(),
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedTranslation {
    display_name: String,
    prompt_body: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug)]
struct ParsedTranslation {
    locale: String,
    display_name: String,
    prompt_body: String,
    description: Option<String>,
}

fn translation_prompt(
    source_locale: &str,
    target_locale: &str,
    display_name: &str,
    prompt_body: &str,
    description: Option<&str>,
) -> String {
    format!(
        "Translate the following prompt from {} to {}. Return only one valid JSON object with exactly these keys: displayName, promptBody, description. Keep every {{{{variable}}}} placeholder in promptBody unchanged, including its name and order. Do not add markdown fences or commentary. The description may be null when the source description is empty.\n\n<display_name>{display_name}</display_name>\n<prompt_body>{prompt_body}</prompt_body>\n<description>{}</description>",
        locale_name(source_locale),
        locale_name(target_locale),
        description.unwrap_or_default(),
    )
}

fn locale_name(locale: &str) -> String {
    match locale {
        "en-US" => "English (United States)".into(),
        "zh-CN" => "Simplified Chinese (China)".into(),
        _ => locale.to_string(),
    }
}

fn parse_translation_output(
    output: &str,
    target_locale: &str,
) -> Result<ParsedTranslation, crate::domain::prompt::PromptError> {
    let output = output.trim();
    let output = output
        .strip_prefix("```json")
        .or_else(|| output.strip_prefix("```JSON"))
        .or_else(|| output.strip_prefix("```"))
        .unwrap_or(output)
        .trim();
    let output = output.strip_suffix("```").unwrap_or(output).trim();
    let json = output
        .find('{')
        .and_then(|start| output.rfind('}').map(|end| &output[start..=end]))
        .ok_or_else(|| {
            crate::domain::prompt::PromptError::InvalidInput(
                "translation response must be a JSON object".into(),
            )
        })?;
    let generated: GeneratedTranslation = serde_json::from_str(json).map_err(|error| {
        crate::domain::prompt::PromptError::InvalidInput(format!(
            "translation response is invalid JSON: {error}"
        ))
    })?;
    let display_name = generated.display_name.trim().to_string();
    let prompt_body = generated.prompt_body.trim().to_string();
    if display_name.is_empty() || prompt_body.is_empty() {
        return Err(crate::domain::prompt::PromptError::InvalidInput(
            "translation response must include displayName and promptBody".into(),
        ));
    }
    let description = generated
        .description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    Ok(ParsedTranslation {
        locale: target_locale.to_string(),
        display_name,
        prompt_body,
        description,
    })
}

fn builtin_prompt_code(catalog_key: &str) -> Result<String, crate::domain::prompt::PromptError> {
    crate::domain::prompt::builtin_prompt(catalog_key)
        .map(|prompt| prompt.prompt_code.clone())
        .ok_or(crate::domain::prompt::PromptError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_output_accepts_json_code_fence_and_normalizes_fields() {
        let translation = parse_translation_output(
            "```json\n{\"displayName\":\" English summary \",\"promptBody\":\" Please summarize {{topic}}. \",\"description\":\" A short description \"}\n```",
            "en-US",
        )
        .expect("translation JSON should parse");

        assert_eq!(translation.locale, "en-US");
        assert_eq!(translation.display_name, "English summary");
        assert_eq!(translation.prompt_body, "Please summarize {{topic}}.");
        assert_eq!(
            translation.description.as_deref(),
            Some("A short description")
        );
    }

    #[test]
    fn translation_output_rejects_missing_prompt_fields() {
        let error = parse_translation_output(
            r#"{"displayName":"English summary","promptBody":""}"#,
            "en-US",
        )
        .expect_err("empty prompt body should be rejected");

        assert!(error.to_string().contains("displayName and promptBody"));
    }
}
