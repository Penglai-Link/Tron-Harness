use crate::{
    application::prompt::{
        CreatePromptCommand, PromptDeleteResult, PromptExportResult, PromptPage, PromptPageQuery,
        PromptSpecCommand, PromptTranslationSpecCommand, PromptVariableSpecCommand,
        UpdatePromptCommand,
    },
    domain::prompt::{
        Prompt, PromptError, PromptScopeKind, PromptSourceKind, PromptView, PromptViewSourceKind,
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
    pub fn workspace_page(
        &self,
        workspace_code: &str,
        query: PromptPageQuery,
    ) -> Result<PromptPage, PromptError> {
        let workspace = self.active_workspace(workspace_code)?;
        let mut records = self
            .resolve_workspace(&workspace)?
            .into_iter()
            .map(|record| record.view)
            .collect::<Vec<_>>();
        filter_records(&mut records, &query);
        paginate(records, query)
    }

    pub fn workspace_detail(
        &self,
        workspace_code: &str,
        prompt_code: &str,
    ) -> Result<PromptView, PromptError> {
        self.resolve_workspace_prompt(workspace_code, prompt_code)
            .map(|(_, record)| record.view)
    }

    pub fn workspace_export(
        &self,
        workspace_code: &str,
        query: PromptPageQuery,
    ) -> Result<PromptExportResult, PromptError> {
        validate_page_query(&query)?;
        let workspace = self.active_workspace(workspace_code)?;
        let mut records = self
            .resolve_workspace(&workspace)?
            .into_iter()
            .map(|record| record.view)
            .collect::<Vec<_>>();
        filter_records(&mut records, &query);
        let content =
            serde_json::to_string_pretty(&records).map_err(|_| PromptError::StorageFailure)?;
        Ok(PromptExportResult {
            file_name: format!("{}-prompts.json", workspace_code.trim().to_lowercase()),
            content,
        })
    }

    pub fn create_workspace_prompt(
        &self,
        workspace_code: &str,
        command: CreatePromptCommand,
    ) -> Result<PromptView, PromptError> {
        let workspace = self.active_workspace(workspace_code)?;
        let (spec, variables) = normalize_spec(command.spec)?;
        self.validate_workspace_managed_variables(&workspace.workspace_code, &variables)?;
        ensure_display_name_unique(
            &self.resolve_workspace(&workspace)?,
            &spec.display_name,
            None,
        )?;
        let prompt = new_prompt(
            self.next_id()?,
            PromptScopeKind::Workspace,
            Some(workspace.workspace_code.clone()),
            PromptSourceKind::WorkspaceLocal,
            None,
            command.category,
            spec,
            variables,
            Some(command.allow_inheritance.unwrap_or(true)),
        );
        self.repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .create_prompt(&prompt)?;
        self.workspace_detail(&workspace.workspace_code, &prompt.prompt_code)
    }

    pub fn update_workspace_prompt(
        &self,
        workspace_code: &str,
        command: UpdatePromptCommand,
    ) -> Result<PromptView, PromptError> {
        let (workspace, resolved) =
            self.resolve_workspace_prompt(workspace_code, &command.prompt_code)?;
        ensure_version(resolved.view.version, Some(command.expected_version))?;
        let requested_allow_inheritance = command.allow_inheritance;
        let (mut spec, variables) = normalize_spec(command.spec)?;
        self.validate_workspace_managed_variables(&workspace.workspace_code, &variables)?;
        if requested_allow_inheritance.is_some()
            && resolved.view.source_kind != PromptViewSourceKind::Local
        {
            return Err(PromptError::AllowInheritanceUnsupported);
        }
        let catalog = self.resolve_workspace(&workspace)?;
        ensure_display_name_unique(
            &catalog,
            &spec.display_name,
            Some(&resolved.view.prompt_code),
        )?;
        if let Some(current_owned) = resolved.current_owned.as_ref() {
            synchronize_translation_statuses(&current_owned.translations, &mut spec);
        }
        match resolved.view.source_kind {
            PromptViewSourceKind::Local | PromptViewSourceKind::Override => {
                let mut owned = resolved.current_owned.ok_or(PromptError::StorageFailure)?;
                let expected = owned.version;
                apply_spec(&mut owned, spec, variables);
                if let Some(allow_inheritance) = requested_allow_inheritance {
                    owned.allow_inheritance = Some(allow_inheritance);
                }
                owned.version += 1;
                self.repository
                    .lock()
                    .expect("prompt repository mutex poisoned")
                    .update_prompt(&owned, expected)?;
            }
            PromptViewSourceKind::Inherited => {
                if let Some(mut owned) = resolved.current_owned {
                    let expected = owned.version;
                    apply_spec(&mut owned, spec, variables);
                    owned.source_kind = PromptSourceKind::WorkspaceOverride;
                    owned.version += 1;
                    self.repository
                        .lock()
                        .expect("prompt repository mutex poisoned")
                        .update_prompt(&owned, expected)?;
                } else {
                    let prompt = new_prompt(
                        self.next_id()?,
                        PromptScopeKind::Workspace,
                        Some(workspace.workspace_code.clone()),
                        PromptSourceKind::WorkspaceOverride,
                        Some(resolved.view.prompt_code.clone()),
                        resolved.view.category,
                        spec,
                        variables,
                        None,
                    );
                    self.repository
                        .lock()
                        .expect("prompt repository mutex poisoned")
                        .create_prompt(&prompt)?;
                }
            }
            PromptViewSourceKind::Platform => return Err(PromptError::StorageFailure),
        }
        self.workspace_detail(&workspace.workspace_code, &command.prompt_code)
    }

    pub fn duplicate_workspace_prompt(
        &self,
        workspace_code: &str,
        prompt_code: &str,
        target_category: crate::domain::prompt::PromptCategory,
    ) -> Result<PromptView, PromptError> {
        let (workspace, resolved) = self.resolve_workspace_prompt(workspace_code, prompt_code)?;
        let catalog = self.resolve_workspace(&workspace)?;
        let display_name = copy_display_name(
            &catalog
                .iter()
                .map(|item| item.view.display_name.as_str())
                .collect::<Vec<_>>(),
            &resolved.view.display_name,
        );
        self.create_workspace_prompt(
            &workspace.workspace_code,
            CreatePromptCommand {
                category: target_category,
                spec: PromptSpecCommand {
                    display_name,
                    prompt_body: resolved.view.prompt_body,
                    description: resolved.view.description,
                    enabled: resolved.view.enabled,
                    translations: resolved
                        .view
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
                    variables: resolved
                        .view
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
                allow_inheritance: Some(true),
            },
        )
    }

    pub fn delete_workspace_prompt(
        &self,
        workspace_code: &str,
        prompt_code: &str,
    ) -> Result<PromptDeleteResult, PromptError> {
        let (workspace, resolved) = self.resolve_workspace_prompt(workspace_code, prompt_code)?;
        if resolved.view.source_kind != PromptViewSourceKind::Local {
            return Err(PromptError::InheritedMutationDenied);
        }
        let owned = resolved.current_owned.ok_or(PromptError::StorageFailure)?;
        if !self
            .repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .delete_prompt(&owned.prompt_code)?
        {
            return Err(PromptError::NotFound);
        }
        Ok(PromptDeleteResult {
            workspace_code: Some(workspace.workspace_code),
            prompt_code: prompt_code.into(),
        })
    }

    pub fn revert_workspace_prompt(
        &self,
        workspace_code: &str,
        prompt_code: &str,
    ) -> Result<PromptView, PromptError> {
        let (workspace, resolved) = self.resolve_workspace_prompt(workspace_code, prompt_code)?;
        let owned = resolved
            .current_owned
            .filter(|prompt| {
                matches!(
                    prompt.source_kind,
                    PromptSourceKind::WorkspaceOverride | PromptSourceKind::WorkspaceStatusPatch
                )
            })
            .ok_or(PromptError::RevertUnsupported)?;
        if !self
            .repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .delete_prompt(&owned.prompt_code)?
        {
            return Err(PromptError::NotFound);
        }
        self.workspace_detail(&workspace.workspace_code, prompt_code)
    }

    pub fn set_workspace_allow_inheritance(
        &self,
        workspace_code: &str,
        prompt_code: &str,
        allow: bool,
        expected_version: u64,
    ) -> Result<PromptView, PromptError> {
        let (workspace, resolved) = self.resolve_workspace_prompt(workspace_code, prompt_code)?;
        ensure_version(resolved.view.version, Some(expected_version))?;
        if resolved.view.source_kind != PromptViewSourceKind::Local {
            return Err(PromptError::AllowInheritanceUnsupported);
        }
        let mut owned = resolved.current_owned.ok_or(PromptError::StorageFailure)?;
        let expected = owned.version;
        owned.allow_inheritance = Some(allow);
        owned.version += 1;
        owned.updated_at = current_timestamp();
        self.repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .update_prompt(&owned, expected)?;
        self.workspace_detail(&workspace.workspace_code, prompt_code)
    }

    pub fn set_workspace_enabled(
        &self,
        workspace_code: &str,
        prompt_code: &str,
        enabled: bool,
        expected_version: u64,
    ) -> Result<PromptView, PromptError> {
        let (workspace, resolved) = self.resolve_workspace_prompt(workspace_code, prompt_code)?;
        if resolved.view.runtime_required && !enabled {
            return Err(PromptError::RuntimeRequired);
        }
        ensure_version(resolved.view.version, Some(expected_version))?;
        match resolved.view.source_kind {
            PromptViewSourceKind::Local | PromptViewSourceKind::Override => {
                let mut owned = resolved.current_owned.ok_or(PromptError::StorageFailure)?;
                let expected = owned.version;
                owned.enabled = enabled;
                owned.version += 1;
                owned.updated_at = current_timestamp();
                self.repository
                    .lock()
                    .expect("prompt repository mutex poisoned")
                    .update_prompt(&owned, expected)?;
            }
            PromptViewSourceKind::Inherited => {
                let upstream = resolved.upstream_enabled.unwrap_or(resolved.view.enabled);
                match resolved.current_owned {
                    Some(owned) if enabled == upstream => {
                        self.repository
                            .lock()
                            .expect("prompt repository mutex poisoned")
                            .delete_prompt(&owned.prompt_code)?;
                    }
                    Some(mut owned) => {
                        let expected = owned.version;
                        owned.enabled = enabled;
                        owned.version += 1;
                        owned.updated_at = current_timestamp();
                        self.repository
                            .lock()
                            .expect("prompt repository mutex poisoned")
                            .update_prompt(&owned, expected)?;
                    }
                    None if enabled != upstream => {
                        let mut patch = resolved.definition;
                        let id = self.next_id()?;
                        patch.id = SnowflakeId::new(id);
                        patch.prompt_code = format!("PROMPT-{id}");
                        patch.scope_kind = PromptScopeKind::Workspace;
                        patch.workspace_code = Some(workspace.workspace_code.clone());
                        patch.source_kind = PromptSourceKind::WorkspaceStatusPatch;
                        patch.source_prompt_code = Some(prompt_code.into());
                        patch.allow_inheritance = None;
                        patch.enabled = enabled;
                        patch.version = 1;
                        patch.created_at = current_timestamp();
                        patch.updated_at = patch.created_at.clone();
                        self.repository
                            .lock()
                            .expect("prompt repository mutex poisoned")
                            .create_prompt(&patch)?;
                    }
                    None => {}
                }
            }
            PromptViewSourceKind::Platform => return Err(PromptError::StorageFailure),
        }
        self.workspace_detail(&workspace.workspace_code, prompt_code)
    }

    fn next_id(&self) -> Result<i64, PromptError> {
        self.ids
            .next_id()
            .map(|id| id.value())
            .map_err(|_| PromptError::StorageFailure)
    }
}

fn new_prompt(
    id: i64,
    scope_kind: PromptScopeKind,
    workspace_code: Option<String>,
    source_kind: PromptSourceKind,
    source_prompt_code: Option<String>,
    category: crate::domain::prompt::PromptCategory,
    spec: PromptSpecCommand,
    variables: Vec<crate::domain::prompt::PromptVariable>,
    allow_inheritance: Option<bool>,
) -> Prompt {
    let now = current_timestamp();
    let translations = translation_models(&spec, &now, 1);
    Prompt {
        id: SnowflakeId::new(id),
        scope_kind,
        workspace_code,
        prompt_code: format!("PROMPT-{id}"),
        display_name: spec.display_name,
        category,
        prompt_body: spec.prompt_body,
        description: spec.description,
        enabled: spec.enabled,
        source_kind,
        source_prompt_code,
        allow_inheritance,
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

fn apply_spec(
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

fn ensure_display_name_unique(
    records: &[super::ResolvedPrompt],
    display_name: &str,
    excluded_code: Option<&str>,
) -> Result<(), PromptError> {
    if records.iter().any(|record| {
        record.view.prompt_code != excluded_code.unwrap_or_default()
            && record.view.display_name.eq_ignore_ascii_case(display_name)
    }) {
        Err(PromptError::DuplicateDisplayName)
    } else {
        Ok(())
    }
}

fn filter_records(records: &mut Vec<PromptView>, query: &PromptPageQuery) {
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);
    records.retain(|record| {
        query.category.is_none_or(|value| value == record.category)
            && query
                .source_kind
                .is_none_or(|value| value == record.source_kind)
            && query.enabled.is_none_or(|value| value == record.enabled)
            && search.as_ref().is_none_or(|value| {
                record.display_name.to_lowercase().contains(value)
                    || record.prompt_code.to_lowercase().contains(value)
            })
    });
}

fn paginate(records: Vec<PromptView>, query: PromptPageQuery) -> Result<PromptPage, PromptError> {
    validate_page_query(&query)?;
    let total = records.len();
    let start = (query.current - 1).saturating_mul(query.size);
    let records = records.into_iter().skip(start).take(query.size).collect();
    Ok(PromptPage {
        current: query.current,
        size: query.size,
        total,
        records,
    })
}

fn validate_page_query(query: &PromptPageQuery) -> Result<(), PromptError> {
    if query.current == 0 || query.size == 0 || query.size > 200 {
        return Err(PromptError::InvalidInput(
            "current must be at least 1 and size must be between 1 and 200".into(),
        ));
    }
    Ok(())
}

fn copy_display_name(existing: &[&str], base: &str) -> String {
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
