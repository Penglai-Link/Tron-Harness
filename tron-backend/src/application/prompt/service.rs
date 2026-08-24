use std::sync::{Arc, Mutex};

use crate::{
    application::{prompt::{PromptPageQuery, PromptRepository}, workspace::WorkspaceService},
    domain::prompt::{Prompt, PromptScopeKind, PromptSourceKind, PromptView, PromptVariable},
    shared::{id::{IdGenerator, SnowflakeId}, time::current_timestamp},
};

mod catalog;
mod platform;
mod template;
mod validation;
mod workspace;

#[derive(Clone)]
pub struct PromptService {
    repository: Arc<Mutex<Box<dyn PromptRepository>>>,
    workspaces: WorkspaceService,
    ids: Arc<dyn IdGenerator>,
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
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            ids,
        }
    }

    pub fn bootstrap_builtins(&self) -> Result<(), crate::domain::prompt::PromptError> {
        let existing = self.all_prompts()?;
        for definition in crate::domain::prompt::builtin_prompts() {
            if existing.iter().any(|prompt| {
                prompt.catalog_key.as_deref() == Some(definition.catalog_key.as_str())
                    || prompt.prompt_code == definition.prompt_code
            }) {
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
                version: 1,
                variables: definition
                    .parameters
                    .iter()
                    .map(|parameter| PromptVariable {
                        name: parameter.name.clone(),
                        variable_type: parameter.variable_type,
                        description: parameter.description.clone(),
                    })
                    .collect(),
                translations: definition
                    .translations
                    .iter()
                    .map(|(locale, value)| crate::domain::prompt::PromptTranslation {
                        locale: locale.clone(),
                        display_name: value.display_name.clone(),
                        prompt_body: value.prompt_body.clone(),
                        description: Some(value.description.clone()),
                        status: crate::domain::prompt::PromptTranslationStatus::Published,
                        source_version: 1,
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

    pub fn translate_prompt_draft(
        &self,
        prompt_code: Option<&str>,
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
        let now = current_timestamp();
        Ok(crate::domain::prompt::PromptTranslation {
            locale: target_locale.into(),
            display_name: display_name.trim().into(),
            prompt_body: prompt_body.into(),
            description: description
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
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
    ) -> Result<Option<String>, crate::domain::prompt::PromptError> {
        let prompt = if let Some(workspace_code) = workspace_code {
            self.workspace_detail(workspace_code, &builtin_prompt_code(catalog_key)?)?
        } else {
            self.platform_page(PromptPageQuery {
                current: 1,
                size: 200,
                search: None,
                category: None,
                source_kind: None,
                enabled: None,
            })?
            .records
            .into_iter()
            .find(|record| record.catalog_key.as_deref() == Some(catalog_key))
            .ok_or(crate::domain::prompt::PromptError::NotFound)?
        };
        if !prompt.enabled {
            return Ok(None);
        }
        let translation = prompt
            .translations
            .iter()
            .find(|translation| translation.locale == locale)
            .or_else(|| prompt.translations.iter().find(|translation| translation.locale == "en-US"));
        Ok(Some(
            translation
                .map(|value| value.prompt_body.clone())
                .unwrap_or(prompt.prompt_body),
        ))
    }
}

fn builtin_prompt_code(catalog_key: &str) -> Result<String, crate::domain::prompt::PromptError> {
    crate::domain::prompt::builtin_prompt(catalog_key)
        .map(|prompt| prompt.prompt_code.clone())
        .ok_or(crate::domain::prompt::PromptError::NotFound)
}
