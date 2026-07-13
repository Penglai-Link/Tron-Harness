use std::collections::HashMap;

use crate::domain::{
    prompt::{
        Prompt, PromptError, PromptScopeKind, PromptSourceKind, PromptView, PromptViewSourceKind,
        PromptWorkspacePathItem,
    },
    workspace::{WorkspaceDetail, WorkspaceStatus},
};

use super::{PromptService, ResolvedPrompt};

impl PromptService {
    pub(super) fn active_workspace(
        &self,
        workspace_code: &str,
    ) -> Result<WorkspaceDetail, PromptError> {
        let workspace_code = workspace_code.trim();
        if workspace_code.is_empty() {
            return Err(PromptError::WorkspaceRequired);
        }
        let workspace = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(|_| PromptError::InvalidWorkspace)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(PromptError::InvalidWorkspace);
        }
        Ok(workspace)
    }

    pub(super) fn all_prompts(&self) -> Result<Vec<Prompt>, PromptError> {
        self.repository
            .lock()
            .expect("prompt repository mutex poisoned")
            .list_prompts()
    }

    pub(super) fn resolve_workspace(
        &self,
        workspace: &WorkspaceDetail,
    ) -> Result<Vec<ResolvedPrompt>, PromptError> {
        let prompts = self.all_prompts()?;
        let lineage = workspace
            .path
            .iter()
            .map(|item| item.workspace_code.clone())
            .collect::<Vec<_>>();
        let lineage_names = workspace
            .path
            .iter()
            .map(|item| (item.workspace_code.clone(), item.name.clone()))
            .collect::<HashMap<_, _>>();
        let current = workspace.workspace_code.as_str();
        let slots = prompts
            .iter()
            .filter_map(|prompt| {
                Some((
                    (
                        prompt.workspace_code.clone()?,
                        prompt.source_prompt_code.clone()?,
                    ),
                    prompt.clone(),
                ))
            })
            .collect::<HashMap<_, _>>();
        let mut roots = prompts
            .iter()
            .filter(|prompt| match prompt.source_kind {
                PromptSourceKind::Platform => prompt.scope_kind == PromptScopeKind::Platform,
                PromptSourceKind::WorkspaceLocal => {
                    prompt.workspace_code.as_deref().is_some_and(|owner| {
                        owner == current
                            || (lineage.iter().any(|code| code == owner)
                                && prompt.allow_inheritance.unwrap_or(true))
                    })
                }
                _ => false,
            })
            .cloned()
            .collect::<Vec<_>>();
        roots.sort_by(|left, right| {
            left.display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase())
                .then_with(|| left.prompt_code.cmp(&right.prompt_code))
        });
        roots
            .into_iter()
            .map(|root| resolve_root(workspace, &lineage, &lineage_names, &slots, root))
            .collect()
    }

    pub(super) fn resolve_workspace_prompt(
        &self,
        workspace_code: &str,
        prompt_code: &str,
    ) -> Result<(WorkspaceDetail, ResolvedPrompt), PromptError> {
        let workspace = self.active_workspace(workspace_code)?;
        let record = self
            .resolve_workspace(&workspace)?
            .into_iter()
            .find(|record| record.view.prompt_code == prompt_code.trim())
            .ok_or(PromptError::NotFound)?;
        Ok((workspace, record))
    }
}

fn resolve_root(
    workspace: &WorkspaceDetail,
    lineage: &[String],
    names: &HashMap<String, String>,
    slots: &HashMap<(String, String), Prompt>,
    root: Prompt,
) -> Result<ResolvedPrompt, PromptError> {
    let current = workspace.workspace_code.as_str();
    if root.workspace_code.as_deref() == Some(current) {
        return Ok(resolved_view(
            workspace,
            &root.prompt_code,
            &root,
            PromptViewSourceKind::Local,
            None,
            root.enabled,
            root.allow_inheritance,
            &root,
        )
        .with_state(root.clone(), Some(root), None));
    }
    let visible_code = root.prompt_code.clone();
    let mut definition = root.clone();
    let mut provider = root.workspace_code.clone();
    let mut upstream_enabled = root.enabled;
    let start = root
        .workspace_code
        .as_ref()
        .and_then(|owner| {
            lineage
                .iter()
                .position(|code| code == owner)
                .map(|index| index + 1)
        })
        .unwrap_or(0);
    for ancestor in lineage
        .iter()
        .skip(start)
        .filter(|code| code.as_str() != current)
    {
        let Some(candidate) = slots.get(&(ancestor.clone(), visible_code.clone())) else {
            continue;
        };
        match candidate.source_kind {
            PromptSourceKind::WorkspaceOverride => {
                definition = candidate.clone();
                provider = Some(ancestor.clone());
                upstream_enabled = candidate.enabled;
            }
            PromptSourceKind::WorkspaceStatusPatch => {
                provider = Some(ancestor.clone());
                upstream_enabled = candidate.enabled;
            }
            _ => return Err(PromptError::StorageFailure),
        }
    }
    let current_owned = slots
        .get(&(current.to_string(), visible_code.clone()))
        .cloned();
    let source = provider
        .as_ref()
        .map(|code| source_info(code, lineage, names));
    match current_owned.as_ref().map(|prompt| prompt.source_kind) {
        Some(PromptSourceKind::WorkspaceOverride) => {
            let owned = current_owned.clone().expect("current override exists");
            Ok(resolved_view(
                workspace,
                &visible_code,
                &owned,
                PromptViewSourceKind::Override,
                source,
                owned.enabled,
                None,
                &owned,
            )
            .with_state(owned, current_owned, Some(upstream_enabled)))
        }
        Some(PromptSourceKind::WorkspaceStatusPatch) => {
            let patch = current_owned.clone().expect("current status patch exists");
            Ok(resolved_view(
                workspace,
                &visible_code,
                &definition,
                PromptViewSourceKind::Inherited,
                source,
                patch.enabled,
                None,
                &patch,
            )
            .with_state(definition, current_owned, Some(upstream_enabled)))
        }
        Some(_) => Err(PromptError::StorageFailure),
        None => Ok(resolved_view(
            workspace,
            &visible_code,
            &definition,
            PromptViewSourceKind::Inherited,
            source,
            upstream_enabled,
            None,
            &definition,
        )
        .with_state(definition, None, Some(upstream_enabled))),
    }
}

fn resolved_view(
    workspace: &WorkspaceDetail,
    visible_code: &str,
    definition: &Prompt,
    source_kind: PromptViewSourceKind,
    source: Option<(String, String, Vec<PromptWorkspacePathItem>)>,
    enabled: bool,
    allow_inheritance: Option<bool>,
    timestamps: &Prompt,
) -> ResolvedPrompt {
    let (source_workspace_code, source_workspace_name, source_workspace_path) = source
        .map(|value| (Some(value.0), Some(value.1), value.2))
        .unwrap_or((None, None, Vec::new()));
    ResolvedPrompt {
        view: PromptView {
            scope_kind: PromptScopeKind::Workspace,
            workspace_code: Some(workspace.workspace_code.clone()),
            prompt_code: visible_code.into(),
            display_name: definition.display_name.clone(),
            category: definition.category,
            prompt_body: definition.prompt_body.clone(),
            description: definition.description.clone(),
            model_code: definition.model_code.clone(),
            temperature: definition.temperature,
            max_output_tokens: definition.max_output_tokens,
            enabled,
            source_kind,
            source_label: source_label(source_kind, source_workspace_name.as_deref()),
            source_prompt_code: (source_kind != PromptViewSourceKind::Local)
                .then(|| visible_code.into()),
            source_workspace_code,
            source_workspace_name,
            source_workspace_path,
            allow_inheritance,
            version: timestamps.version,
            variables: definition.variables.clone(),
            created_at: timestamps.created_at.clone(),
            updated_at: timestamps.updated_at.clone(),
        },
        definition: definition.clone(),
        current_owned: None,
        upstream_enabled: None,
    }
}

fn source_label(source_kind: PromptViewSourceKind, source_workspace_name: Option<&str>) -> String {
    match source_kind {
        PromptViewSourceKind::Platform => "Platform".into(),
        PromptViewSourceKind::Local => "Local".into(),
        PromptViewSourceKind::Inherited => source_workspace_name
            .map(|name| format!("Inherited from {name}"))
            .unwrap_or_else(|| "Platform".into()),
        PromptViewSourceKind::Override => "Local override".into(),
    }
}

impl ResolvedPrompt {
    fn with_state(
        mut self,
        definition: Prompt,
        current_owned: Option<Prompt>,
        upstream_enabled: Option<bool>,
    ) -> Self {
        self.definition = definition;
        self.current_owned = current_owned;
        self.upstream_enabled = upstream_enabled;
        self
    }
}

fn source_info(
    code: &str,
    lineage: &[String],
    names: &HashMap<String, String>,
) -> (String, String, Vec<PromptWorkspacePathItem>) {
    let path = lineage
        .iter()
        .take(
            lineage
                .iter()
                .position(|item| item == code)
                .map(|index| index + 1)
                .unwrap_or(0),
        )
        .filter_map(|item| {
            names.get(item).map(|name| PromptWorkspacePathItem {
                workspace_code: item.clone(),
                name: name.clone(),
            })
        })
        .collect();
    (
        code.into(),
        names.get(code).cloned().unwrap_or_else(|| code.into()),
        path,
    )
}
