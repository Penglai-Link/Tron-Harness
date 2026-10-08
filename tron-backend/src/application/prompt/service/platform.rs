use crate::{
    application::prompt::{
        CreatePromptCommand, PromptDeleteResult, PromptPage, PromptPageQuery, PromptSpecCommand,
        PromptTranslationSpecCommand, PromptVariableSpecCommand, UpdatePromptCommand,
    },
    domain::prompt::{
        Prompt, PromptCategory, PromptError, PromptScopeKind, PromptSourceKind, PromptView,
        PromptViewSourceKind,
    },
    shared::{id::SnowflakeId, time::current_timestamp},
};

use super::{
    PromptService,
    validation::{
        ensure_version, normalize_spec, synchronize_translation_statuses, translation_models,
    },
};

impl PromptService {
    pub fn platform_page(&self, query: PromptPageQuery) -> Result<PromptPage, PromptError> {
        let mut records = self
            .platform_prompts()?
            .into_iter()
            .map(platform_view)
            .collect::<Vec<_>>();
        filter_platform_records(&mut records, &query);
        paginate_platform(records, query)
    }

    pub fn platform_detail(&self, prompt_code: &str) -> Result<PromptView, PromptError> {
        self.platform_prompts()?
            .into_iter()
            .find(|prompt| prompt.prompt_code == prompt_code.trim())
            .map(platform_view)
            .ok_or(PromptError::NotFound)
    }

    pub fn create_platform_prompt(
        &self,
        command: CreatePromptCommand,
    ) -> Result<PromptView, PromptError> {
        let (spec, variables) = normalize_spec(command.spec)?;
        self.validate_platform_managed_variables(&variables)?;
        ensure_platform_display_name_unique(&self.platform_prompts()?, &spec.display_name, None)?;
        let prompt =
            new_platform_prompt(self.next_platform_id()?, command.category, spec, variables);
        self.repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .create_prompt(&prompt)?;
        self.platform_detail(&prompt.prompt_code)
    }

    pub fn update_platform_prompt(
        &self,
        command: UpdatePromptCommand,
    ) -> Result<PromptView, PromptError> {
        let mut prompt = self.platform_prompt(&command.prompt_code)?;
        ensure_version(prompt.version, Some(command.expected_version))?;
        let (mut spec, variables) = normalize_spec(command.spec)?;
        self.validate_platform_managed_variables(&variables)?;
        ensure_platform_display_name_unique(
            &self.platform_prompts()?,
            &spec.display_name,
            Some(&prompt.prompt_code),
        )?;
        synchronize_translation_statuses(&prompt.translations, &mut spec);
        let expected = prompt.version;
        apply_platform_spec(&mut prompt, spec, variables);
        prompt.version += 1;
        self.repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .update_prompt(&prompt, expected)?;
        self.platform_detail(&prompt.prompt_code)
    }

    pub fn duplicate_platform_prompt(
        &self,
        prompt_code: &str,
        target_category: PromptCategory,
    ) -> Result<PromptView, PromptError> {
        let source = self.platform_detail(prompt_code)?;
        let existing = self.platform_prompts()?;
        let display_name = copy_platform_display_name(
            &existing
                .iter()
                .map(|prompt| prompt.display_name.as_str())
                .collect::<Vec<_>>(),
            &source.display_name,
        );
        self.create_platform_prompt(CreatePromptCommand {
            category: target_category,
            spec: PromptSpecCommand {
                display_name,
                prompt_body: source.prompt_body,
                description: source.description,
                enabled: source.enabled,
                translations: source
                    .translations
                    .into_iter()
                    .map(|translation| PromptTranslationSpecCommand {
                        locale: translation.locale,
                        display_name: translation.display_name,
                        prompt_body: translation.prompt_body,
                        description: translation.description,
                        status: translation.status,
                    })
                    .collect(),
                variables: source
                    .variables
                    .into_iter()
                    .map(|variable| PromptVariableSpecCommand {
                        name: variable.name,
                        variable_type: variable.variable_type,
                        description: variable.description,
                        source_kind: variable.source_kind,
                        managed_variable_key: variable.managed_variable_key,
                        runtime_provider: variable.runtime_provider,
                    })
                    .collect(),
            },
            allow_inheritance: None,
        })
    }

    pub fn delete_platform_prompt(
        &self,
        prompt_code: &str,
    ) -> Result<PromptDeleteResult, PromptError> {
        let prompt = self.platform_prompt(prompt_code)?;
        if prompt.built_in {
            return Err(PromptError::BuiltinMutationDenied);
        }
        if !self
            .repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .delete_prompt(&prompt.prompt_code)?
        {
            return Err(PromptError::NotFound);
        }
        Ok(PromptDeleteResult {
            workspace_code: None,
            prompt_code: prompt.prompt_code,
        })
    }

    pub fn set_platform_enabled(
        &self,
        prompt_code: &str,
        enabled: bool,
        expected_version: u64,
    ) -> Result<PromptView, PromptError> {
        let mut prompt = self.platform_prompt(prompt_code)?;
        if prompt.runtime_required && !enabled {
            return Err(PromptError::RuntimeRequired);
        }
        ensure_version(prompt.version, Some(expected_version))?;
        let expected = prompt.version;
        prompt.enabled = enabled;
        prompt.version += 1;
        prompt.updated_at = current_timestamp();
        self.repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .update_prompt(&prompt, expected)?;
        self.platform_detail(&prompt.prompt_code)
    }

    fn platform_prompts(&self) -> Result<Vec<Prompt>, PromptError> {
        self.all_prompts().map(|prompts| {
            prompts
                .into_iter()
                .filter(|prompt| prompt.scope_kind == PromptScopeKind::Platform)
                .collect()
        })
    }

    fn platform_prompt(&self, prompt_code: &str) -> Result<Prompt, PromptError> {
        self.platform_prompts()?
            .into_iter()
            .find(|prompt| prompt.prompt_code == prompt_code.trim())
            .ok_or(PromptError::NotFound)
    }

    fn next_platform_id(&self) -> Result<i64, PromptError> {
        self.ids
            .next_id()
            .map(|id| id.value())
            .map_err(|_| PromptError::StorageFailure)
    }
}

fn new_platform_prompt(
    id: i64,
    category: PromptCategory,
    spec: PromptSpecCommand,
    variables: Vec<crate::domain::prompt::PromptVariable>,
) -> Prompt {
    let now = current_timestamp();
    let translations = translation_models(&spec, &now, 1);
    Prompt {
        id: SnowflakeId::new(id),
        scope_kind: PromptScopeKind::Platform,
        workspace_code: None,
        prompt_code: format!("PROMPT-{id}"),
        display_name: spec.display_name,
        category,
        prompt_body: spec.prompt_body,
        description: spec.description,
        enabled: spec.enabled,
        source_kind: PromptSourceKind::Platform,
        source_prompt_code: None,
        allow_inheritance: None,
        version: 1,
        variables,
        translations,
        built_in: false,
        runtime_required: false,
        catalog_key: None,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn platform_view(prompt: Prompt) -> PromptView {
    PromptView {
        scope_kind: PromptScopeKind::Platform,
        workspace_code: None,
        prompt_code: prompt.prompt_code,
        display_name: prompt.display_name,
        category: prompt.category,
        prompt_body: prompt.prompt_body,
        description: prompt.description,
        enabled: prompt.enabled,
        source_kind: PromptViewSourceKind::Platform,
        source_label: "Platform".into(),
        source_prompt_code: None,
        source_workspace_code: None,
        source_workspace_name: None,
        source_workspace_path: Vec::new(),
        allow_inheritance: None,
        version: prompt.version,
        variables: prompt.variables,
        translations: prompt.translations,
        built_in: prompt.built_in,
        runtime_required: prompt.runtime_required,
        catalog_key: prompt.catalog_key,
        created_at: prompt.created_at,
        updated_at: prompt.updated_at,
    }
}

fn apply_platform_spec(
    prompt: &mut Prompt,
    spec: PromptSpecCommand,
    variables: Vec<crate::domain::prompt::PromptVariable>,
) {
    let updated_at = current_timestamp();
    let translations = translation_models(&spec, &updated_at, prompt.version + 1);
    prompt.display_name = spec.display_name;
    prompt.prompt_body = spec.prompt_body;
    prompt.description = spec.description;
    prompt.enabled = spec.enabled;
    prompt.variables = variables;
    prompt.translations = translations;
    prompt.updated_at = updated_at;
}

fn ensure_platform_display_name_unique(
    records: &[Prompt],
    display_name: &str,
    excluded_code: Option<&str>,
) -> Result<(), PromptError> {
    if records.iter().any(|record| {
        record.prompt_code != excluded_code.unwrap_or_default()
            && record.display_name.eq_ignore_ascii_case(display_name)
    }) {
        Err(PromptError::DuplicateDisplayName)
    } else {
        Ok(())
    }
}

fn filter_platform_records(records: &mut Vec<PromptView>, query: &PromptPageQuery) {
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);
    records.retain(|record| {
        query.category.is_none_or(|value| value == record.category)
            && query.enabled.is_none_or(|value| value == record.enabled)
            && search.as_ref().is_none_or(|value| {
                record.display_name.to_lowercase().contains(value)
                    || record.prompt_code.to_lowercase().contains(value)
                    || record
                        .description
                        .as_ref()
                        .is_some_and(|text| text.to_lowercase().contains(value))
                    || record.translations.iter().any(|translation| {
                        translation.display_name.to_lowercase().contains(value)
                            || translation
                                .description
                                .as_ref()
                                .is_some_and(|text| text.to_lowercase().contains(value))
                    })
            })
    });
}

fn paginate_platform(
    records: Vec<PromptView>,
    query: PromptPageQuery,
) -> Result<PromptPage, PromptError> {
    if query.current == 0 || query.size == 0 || query.size > 200 {
        return Err(PromptError::InvalidInput(
            "current must be at least 1 and size must be between 1 and 200".into(),
        ));
    }
    let total = records.len();
    let start = (query.current - 1).saturating_mul(query.size);
    Ok(PromptPage {
        current: query.current,
        size: query.size,
        total,
        records: records.into_iter().skip(start).take(query.size).collect(),
    })
}

fn copy_platform_display_name(existing: &[&str], base: &str) -> String {
    let first = format!("{base} Copy");
    if !existing
        .iter()
        .any(|name| name.eq_ignore_ascii_case(&first))
    {
        return first;
    }
    (2..)
        .map(|index| format!("{base} Copy {index}"))
        .find(|candidate| {
            !existing
                .iter()
                .any(|name| name.eq_ignore_ascii_case(candidate))
        })
        .expect("copy name sequence is unbounded")
}
