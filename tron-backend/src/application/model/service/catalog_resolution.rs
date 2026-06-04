use super::*;

pub(super) fn platform_models(models: &[Model]) -> Vec<Model> {
    let mut records = models
        .iter()
        .filter(|model| model.scope_kind == ModelScopeKind::Platform)
        .cloned()
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.model_code.cmp(&right.model_code))
    });
    records
}

pub(super) fn build_platform_view(
    model: Model,
    defaults_by_type: &HashMap<ModelType, String>,
) -> ModelView {
    let model_code = model.model_code.clone();
    let model_type = model.model_type;
    let is_default = defaults_by_type
        .get(&model_type)
        .is_some_and(|value| value == &model_code);
    ModelView {
        scope_kind: ModelScopeKind::Platform,
        workspace_code: None,
        model_code,
        display_name: model.display_name,
        vendor: model.vendor,
        protocol: model.protocol,
        model_type,
        model_capabilities: model.model_capabilities,
        base_url: model.base_url,
        api_path: model.api_path,
        api_key_ref: masked_api_key_ref(&model.api_key_ref),
        model_name: model.model_name,
        enabled: model.enabled,
        description: model.description,
        thinking_enabled: model.thinking_enabled,
        structured_output_enabled: model.structured_output_enabled,
        temperature: model.temperature,
        max_tokens: model.max_tokens,
        context_window_tokens: model.context_window_tokens,
        top_p: model.top_p,
        max_retries: model.max_retries,
        last_verified_at: model.last_verified_at,
        last_verify_status: model.last_verify_status,
        last_verify_message: model.last_verify_message,
        source_kind: ModelViewSourceKind::Platform,
        source_model_code: None,
        source_label: "Platform".into(),
        source_workspace_code: None,
        source_workspace_name: None,
        source_workspace_path: Vec::new(),
        allow_inheritance: None,
        is_default,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(super) fn build_workspace_source_label(source_kind: ModelViewSourceKind) -> &'static str {
    match source_kind {
        ModelViewSourceKind::Platform => "Platform",
        ModelViewSourceKind::Local => "Local",
        ModelViewSourceKind::Inherited => "Inherited",
        ModelViewSourceKind::Override => "Override",
    }
}

pub(super) fn agent_model_config_from_view(
    view: &ModelView,
    api_key: &str,
) -> Option<AgentModelConfig> {
    let protocol = match (view.protocol, view.model_type) {
        (ModelProtocol::OpenAiCompatible, ModelType::Text) => AgentModelProtocol::ChatCompletions,
        (ModelProtocol::Responses, ModelType::Text) => AgentModelProtocol::Responses,
        _ => return None,
    };
    Some(AgentModelConfig {
        model_code: view.model_code.clone(),
        protocol,
        base_url: view.base_url.clone(),
        api_path: view.api_path.clone(),
        api_key: api_key.to_string(),
        model_name: view.model_name.clone(),
        thinking_enabled: view.thinking_enabled,
        temperature: view.temperature,
        max_tokens: view.max_tokens,
        context_window_tokens: view.context_window_tokens,
        top_p: view.top_p,
        max_retries: view.max_retries,
    })
}

pub(super) fn runtime_model_config_from_view(
    view: &ModelView,
    api_key: &str,
) -> Option<RuntimeModelConfig> {
    match (view.protocol, view.model_type) {
        (ModelProtocol::OpenAiCompatible, ModelType::Text)
        | (ModelProtocol::OpenAiCompatible, ModelType::Embedding)
        | (ModelProtocol::Responses, ModelType::Text)
        | (ModelProtocol::Responses, ModelType::Image)
        | (ModelProtocol::Rerank, ModelType::Rerank) => Some(RuntimeModelConfig {
            model_code: view.model_code.clone(),
            protocol: view.protocol,
            model_type: view.model_type,
            base_url: view.base_url.clone(),
            api_path: view.api_path.clone(),
            api_key: api_key.to_string(),
            model_name: view.model_name.clone(),
            max_retries: view.max_retries,
        }),
        _ => None,
    }
}

pub(super) fn build_workspace_effective_view(
    workspace: &WorkspaceDetail,
    visible_model_code: &str,
    config_model: &Model,
    source_kind: ModelViewSourceKind,
    inherited_from: Option<&EffectiveProvider>,
    enabled: bool,
    allow_inheritance: Option<bool>,
    timestamps_model: &Model,
) -> ModelView {
    let (source_workspace_code, source_workspace_name) = if let Some(workspace_provider) =
        inherited_from.and_then(|provider| provider.workspace.as_ref())
    {
        (
            Some(workspace_provider.code.clone()),
            Some(workspace_provider.name.clone()),
        )
    } else {
        (None, None)
    };
    let source_workspace_path = inherited_from
        .map(|provider| {
            provider
                .workspace_path
                .iter()
                .map(|entry| ModelWorkspacePathItem {
                    workspace_code: entry.code.clone(),
                    name: entry.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    ModelView {
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace.workspace_code.clone()),
        model_code: visible_model_code.to_string(),
        display_name: config_model.display_name.clone(),
        vendor: config_model.vendor.clone(),
        protocol: config_model.protocol,
        model_type: config_model.model_type,
        model_capabilities: config_model.model_capabilities.clone(),
        base_url: config_model.base_url.clone(),
        api_path: config_model.api_path.clone(),
        api_key_ref: masked_api_key_ref(&config_model.api_key_ref),
        model_name: config_model.model_name.clone(),
        enabled,
        description: config_model.description.clone(),
        thinking_enabled: config_model.thinking_enabled,
        structured_output_enabled: config_model.structured_output_enabled,
        temperature: config_model.temperature,
        max_tokens: config_model.max_tokens,
        context_window_tokens: config_model.context_window_tokens,
        top_p: config_model.top_p,
        max_retries: config_model.max_retries,
        last_verified_at: config_model.last_verified_at.clone(),
        last_verify_status: config_model.last_verify_status,
        last_verify_message: config_model.last_verify_message.clone(),
        source_kind,
        source_model_code: match source_kind {
            ModelViewSourceKind::Local => None,
            _ => Some(visible_model_code.to_string()),
        },
        source_label: build_workspace_source_label(source_kind).into(),
        source_workspace_code,
        source_workspace_name,
        source_workspace_path,
        allow_inheritance,
        is_default: false,
        created_at: timestamps_model.created_at.clone(),
        updated_at: timestamps_model.updated_at.clone(),
    }
}

pub(super) fn build_workspace_preview_view(
    current_view: &ModelView,
    preview_model: &Model,
    enabled: bool,
    allow_inheritance: Option<bool>,
) -> ModelView {
    let mut view = current_view.clone();
    view.display_name = preview_model.display_name.clone();
    view.vendor = preview_model.vendor.clone();
    view.protocol = preview_model.protocol;
    view.model_type = preview_model.model_type;
    view.model_capabilities = preview_model.model_capabilities.clone();
    view.base_url = preview_model.base_url.clone();
    view.api_path = preview_model.api_path.clone();
    view.api_key_ref = masked_api_key_ref(&preview_model.api_key_ref);
    view.model_name = preview_model.model_name.clone();
    view.enabled = enabled;
    view.description = preview_model.description.clone();
    view.thinking_enabled = preview_model.thinking_enabled;
    view.structured_output_enabled = preview_model.structured_output_enabled;
    view.temperature = preview_model.temperature;
    view.max_tokens = preview_model.max_tokens;
    view.context_window_tokens = preview_model.context_window_tokens;
    view.top_p = preview_model.top_p;
    view.max_retries = preview_model.max_retries;
    view.last_verified_at = preview_model.last_verified_at.clone();
    view.last_verify_status = preview_model.last_verify_status;
    view.last_verify_message = preview_model.last_verify_message.clone();
    view.allow_inheritance = allow_inheritance;
    view.updated_at = preview_model.updated_at.clone();
    view
}

pub(super) fn workspace_provider(
    model: &Model,
    workspace: &LineageEntry,
    lineage: &[LineageEntry],
) -> EffectiveProvider {
    EffectiveProvider {
        model: model.clone(),
        workspace: Some(workspace.clone()),
        workspace_path: workspace_path_to_workspace(lineage, workspace.code.as_str()),
    }
}

pub(super) fn platform_provider(model: &Model) -> EffectiveProvider {
    EffectiveProvider {
        model: model.clone(),
        workspace: None,
        workspace_path: Vec::new(),
    }
}

pub(super) fn workspace_path_to_workspace(
    lineage: &[LineageEntry],
    workspace_code: &str,
) -> Vec<LineageEntry> {
    let Some(index) = lineage
        .iter()
        .position(|entry| entry.code == workspace_code)
    else {
        return Vec::new();
    };
    lineage.iter().take(index + 1).cloned().collect()
}

pub(super) fn find_workspace_slot_record(
    models_by_slot: &HashMap<(String, String), Model>,
    workspace_code: &str,
    slot_model_code: &str,
) -> Option<Model> {
    models_by_slot
        .get(&(workspace_code.to_string(), slot_model_code.to_string()))
        .cloned()
}

pub(super) fn build_resolved_workspace_record(
    root: &Model,
    workspace: &WorkspaceDetail,
    lineage: &[LineageEntry],
    lineage_map: &HashMap<String, LineageEntry>,
    workspace_slot_records: &HashMap<(String, String), Model>,
) -> Result<ResolvedWorkspaceRecord, ModelError> {
    let current_workspace_code = workspace.workspace_code.as_str();
    let visible_model_code = root.model_code.clone();

    if root.scope_kind == ModelScopeKind::Workspace
        && root.workspace_code.as_deref() == Some(current_workspace_code)
    {
        let view = build_workspace_effective_view(
            workspace,
            &visible_model_code,
            root,
            ModelViewSourceKind::Local,
            None,
            root.enabled,
            root.allow_inheritance,
            root,
        );
        return Ok(ResolvedWorkspaceRecord {
            view,
            definition_model: root.clone(),
            current_owned_record: Some(root.clone()),
            upstream_enabled: None,
        });
    }

    let mut definition_provider = if root.scope_kind == ModelScopeKind::Platform {
        platform_provider(root)
    } else {
        let owner_workspace = root
            .workspace_code
            .as_deref()
            .and_then(|code| lineage_map.get(code))
            .ok_or(ModelError::StorageFailure)?;
        workspace_provider(root, owner_workspace, lineage)
    };
    let mut inherited_provider = definition_provider.clone();

    let ancestor_entries: Vec<&LineageEntry> = if root.scope_kind == ModelScopeKind::Platform {
        lineage
            .iter()
            .filter(|entry| entry.code != current_workspace_code)
            .collect()
    } else {
        let owner_workspace_code = root
            .workspace_code
            .as_deref()
            .ok_or(ModelError::StorageFailure)?;
        let start_index = lineage
            .iter()
            .position(|entry| entry.code == owner_workspace_code)
            .ok_or(ModelError::StorageFailure)?;
        lineage
            .iter()
            .skip(start_index + 1)
            .filter(|entry| entry.code != current_workspace_code)
            .collect()
    };

    for ancestor in ancestor_entries {
        let Some(candidate) =
            find_workspace_slot_record(workspace_slot_records, &ancestor.code, &visible_model_code)
        else {
            continue;
        };
        match candidate.source_kind {
            ModelSourceKind::WorkspaceOverride => {
                definition_provider = workspace_provider(&candidate, ancestor, lineage);
                inherited_provider = definition_provider.clone();
            }
            ModelSourceKind::WorkspaceStatusPatch => {
                inherited_provider = workspace_provider(&candidate, ancestor, lineage);
            }
            _ => {}
        }
    }

    let current_owned_record = find_workspace_slot_record(
        workspace_slot_records,
        current_workspace_code,
        &visible_model_code,
    );

    let record = match current_owned_record
        .as_ref()
        .map(|record| record.source_kind)
    {
        Some(ModelSourceKind::WorkspaceOverride) => {
            let current_override = current_owned_record
                .as_ref()
                .expect("override record should exist");
            let view = build_workspace_effective_view(
                workspace,
                &visible_model_code,
                current_override,
                ModelViewSourceKind::Override,
                Some(&inherited_provider),
                current_override.enabled,
                None,
                current_override,
            );
            ResolvedWorkspaceRecord {
                view,
                definition_model: current_override.clone(),
                current_owned_record,
                upstream_enabled: Some(inherited_provider.model.enabled),
            }
        }
        Some(ModelSourceKind::WorkspaceStatusPatch) => {
            let current_patch = current_owned_record
                .as_ref()
                .expect("status patch record should exist");
            let view = build_workspace_effective_view(
                workspace,
                &visible_model_code,
                &definition_provider.model,
                ModelViewSourceKind::Inherited,
                Some(&inherited_provider),
                current_patch.enabled,
                None,
                current_patch,
            );
            ResolvedWorkspaceRecord {
                view,
                definition_model: definition_provider.model.clone(),
                current_owned_record,
                upstream_enabled: Some(inherited_provider.model.enabled),
            }
        }
        Some(_) => return Err(ModelError::StorageFailure),
        None => {
            let view = build_workspace_effective_view(
                workspace,
                &visible_model_code,
                &definition_provider.model,
                ModelViewSourceKind::Inherited,
                Some(&inherited_provider),
                inherited_provider.model.enabled,
                None,
                &inherited_provider.model,
            );
            ResolvedWorkspaceRecord {
                view,
                definition_model: definition_provider.model.clone(),
                current_owned_record: None,
                upstream_enabled: Some(inherited_provider.model.enabled),
            }
        }
    };

    Ok(record)
}

pub(super) fn sort_resolved_workspace_records(records: &mut [ResolvedWorkspaceRecord]) {
    records.sort_by(|left, right| {
        left.view
            .model_type
            .cmp(&right.view.model_type)
            .then_with(|| {
                left.view
                    .display_name
                    .to_lowercase()
                    .cmp(&right.view.display_name.to_lowercase())
            })
            .then_with(|| {
                left.view
                    .source_label
                    .to_lowercase()
                    .cmp(&right.view.source_label.to_lowercase())
            })
            .then_with(|| left.view.model_code.cmp(&right.view.model_code))
    });
}

pub(super) fn resolve_effective_workspace_defaults(
    records: &[ResolvedWorkspaceRecord],
    workspace_assignments: &[WorkspaceDefaultModelAssignment],
    platform_assignments: &[PlatformDefaultModelAssignment],
    workspace: &WorkspaceDetail,
    models: &[Model],
) -> HashMap<ModelType, String> {
    let record_map: HashMap<_, _> = records
        .iter()
        .map(|record| (record.view.model_code.as_str(), &record.view))
        .collect();
    let mut workspace_resolution_order = vec![workspace.workspace_code.clone()];
    workspace_resolution_order.extend(
        workspace
            .path
            .iter()
            .rev()
            .filter(|entry| entry.workspace_code != workspace.workspace_code)
            .map(|entry| entry.workspace_code.clone()),
    );
    let platform_defaults = resolve_platform_defaults(models, platform_assignments);

    model_type_sequence()
        .into_iter()
        .filter_map(|model_type| {
            for workspace_code in &workspace_resolution_order {
                let Some(assignment) = workspace_assignments.iter().find(|assignment| {
                    assignment.workspace_code == *workspace_code
                        && assignment.model_type == model_type
                }) else {
                    continue;
                };
                let Some(record) = record_map.get(assignment.model_code.as_str()) else {
                    continue;
                };
                if record.enabled && record.model_type == model_type {
                    return Some((model_type, record.model_code.clone()));
                }
            }

            platform_defaults.get(&model_type).and_then(|model_code| {
                let record = record_map.get(model_code.as_str())?;
                if !record.enabled || record.model_type != model_type {
                    return None;
                }
                Some((model_type, record.model_code.clone()))
            })
        })
        .collect()
}

pub(super) fn resolve_platform_defaults(
    models: &[Model],
    assignments: &[PlatformDefaultModelAssignment],
) -> HashMap<ModelType, String> {
    let record_map: HashMap<_, _> = models
        .iter()
        .filter(|model| model.scope_kind == ModelScopeKind::Platform)
        .map(|model| (model.model_code.as_str(), model))
        .collect();
    let explicit_defaults = assignments
        .iter()
        .filter_map(|assignment| {
            let model = record_map.get(assignment.model_code.as_str())?;
            if !model.enabled || model.model_type != assignment.model_type {
                return None;
            }
            Some((assignment.model_type, model.model_code.clone()))
        })
        .collect::<HashMap<_, _>>();

    let fallback_defaults = model_type_sequence()
        .into_iter()
        .filter_map(|model_type| {
            first_enabled_platform_model_for_type(models, model_type)
                .map(|model| (model_type, model.model_code.clone()))
        })
        .collect::<HashMap<_, _>>();

    model_type_sequence()
        .into_iter()
        .filter_map(|model_type| {
            explicit_defaults
                .get(&model_type)
                .or_else(|| fallback_defaults.get(&model_type))
                .map(|model_code| (model_type, model_code.clone()))
        })
        .collect()
}

pub(super) fn first_enabled_platform_model_for_type(
    models: &[Model],
    model_type: ModelType,
) -> Option<&Model> {
    models
        .iter()
        .filter(|model| {
            model.scope_kind == ModelScopeKind::Platform
                && model.enabled
                && model.model_type == model_type
        })
        .min_by_key(|model| model.id.value())
}

pub(super) fn model_type_sequence() -> [ModelType; 4] {
    [
        ModelType::Text,
        ModelType::Embedding,
        ModelType::Rerank,
        ModelType::Image,
    ]
}
