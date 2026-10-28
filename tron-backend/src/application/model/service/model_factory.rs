use super::*;

pub(super) fn normalize_model_spec(
    command: ModelCommand,
    workspace_local: bool,
    require_api_key: bool,
) -> Result<NormalizedModelSpec, ModelError> {
    let model_capabilities =
        normalize_model_capabilities(command.model_type, command.model_capabilities);
    let api_key_for_validation = if require_api_key || !command.api_key_ref.trim().is_empty() {
        command.api_key_ref.as_str()
    } else {
        MASKED_API_KEY_VALUE
    };
    validate_model_configuration(
        &command.display_name,
        &command.vendor,
        &command.base_url,
        api_key_for_validation,
        &command.model_name,
        command.protocol,
        command.model_type,
        &model_capabilities,
        command.responses_stateful_enabled,
    )?;
    if command.context_window_tokens == Some(0) {
        return Err(ModelError::InvalidContextWindowTokens);
    }
    let (supported_reasoning_efforts, default_reasoning_effort) = normalize_reasoning_config(
        command.thinking_enabled,
        command.model_type,
        command.supported_reasoning_efforts,
        command.default_reasoning_effort,
    )?;

    Ok(NormalizedModelSpec {
        display_name: command.display_name.trim().to_string(),
        vendor: command.vendor.trim().to_string(),
        protocol: command.protocol,
        model_type: command.model_type,
        model_capabilities,
        base_url: command.base_url.trim().trim_end_matches('/').to_string(),
        api_path: trim_optional(command.api_path),
        api_key_ref: command.api_key_ref.trim().to_string(),
        model_name: command.model_name.trim().to_string(),
        enabled: command.enabled,
        description: trim_optional(command.description),
        thinking_enabled: command.thinking_enabled,
        supported_reasoning_efforts,
        default_reasoning_effort,
        structured_output_enabled: command.structured_output_enabled,
        responses_stateful_enabled: command.responses_stateful_enabled,
        temperature: command.temperature,
        max_tokens: command.max_tokens,
        context_window_tokens: command.context_window_tokens,
        top_p: command.top_p,
        max_retries: command.max_retries,
        allow_inheritance: workspace_local.then_some(command.allow_inheritance.unwrap_or(true)),
    })
}

pub(super) fn update_model_from_spec(
    mut model: Model,
    spec: NormalizedModelSpec,
    allow_inheritance: Option<bool>,
) -> Model {
    model.display_name = spec.display_name;
    model.vendor = spec.vendor;
    model.protocol = spec.protocol;
    model.model_type = spec.model_type;
    model.model_capabilities = spec.model_capabilities;
    model.base_url = spec.base_url;
    model.api_path = spec.api_path;
    if !spec.api_key_ref.is_empty() && spec.api_key_ref != MASKED_API_KEY_VALUE {
        model.api_key_ref = spec.api_key_ref;
    }
    model.model_name = spec.model_name;
    model.enabled = spec.enabled;
    model.description = spec.description;
    model.thinking_enabled = spec.thinking_enabled;
    model.supported_reasoning_efforts = spec.supported_reasoning_efforts;
    model.default_reasoning_effort = spec.default_reasoning_effort;
    model.structured_output_enabled = spec.structured_output_enabled;
    model.responses_stateful_enabled = spec.responses_stateful_enabled;
    model.temperature = spec.temperature;
    model.max_tokens = spec.max_tokens;
    model.context_window_tokens = spec.context_window_tokens;
    model.top_p = spec.top_p;
    model.max_retries = spec.max_retries;
    model.allow_inheritance = allow_inheritance;
    model.updated_at = current_timestamp();
    model
}

fn normalize_reasoning_config(
    thinking_enabled: bool,
    model_type: ModelType,
    values: Vec<ReasoningEffort>,
    default_value: Option<ReasoningEffort>,
) -> Result<(Vec<ReasoningEffort>, Option<ReasoningEffort>), ModelError> {
    if !thinking_enabled || model_type != ModelType::Text {
        return Ok((Vec::new(), None));
    }
    let normalized = ReasoningEffort::ALL
        .into_iter()
        .filter(|effort| values.contains(effort))
        .collect::<Vec<_>>();
    if default_value.is_some_and(|value| !normalized.contains(&value)) {
        return Err(ModelError::DefaultReasoningEffortUnsupported);
    }
    Ok((normalized, default_value))
}

pub(super) fn normalize_model_capabilities(
    model_type: ModelType,
    values: Vec<ModelCapability>,
) -> Vec<ModelCapability> {
    let seeded = if values.is_empty() {
        default_model_capabilities_for_type(model_type)
    } else {
        values
    };
    let mut normalized = Vec::new();
    for value in seeded {
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    if normalized.is_empty() {
        return default_model_capabilities_for_type(model_type);
    }
    normalized
}

pub(super) fn ensure_platform_display_name_unique(
    models: &[Model],
    display_name: &str,
    exclude_model_code: Option<&str>,
) -> Result<(), ModelError> {
    if models.iter().any(|model| {
        model.scope_kind == ModelScopeKind::Platform
            && model.display_name == display_name
            && Some(model.model_code.as_str()) != exclude_model_code
    }) {
        return Err(ModelError::DuplicatePlatformDisplayName);
    }
    Ok(())
}

pub(super) fn next_platform_copy_display_name(models: &[Model], display_name: &str) -> String {
    let base_name = format!("{display_name}-Copy");
    if !models.iter().any(|model| {
        model.scope_kind == ModelScopeKind::Platform && model.display_name == base_name
    }) {
        return base_name;
    }

    let mut copy_index = 2;
    loop {
        let candidate = format!("{display_name}-Copy-{copy_index}");
        if !models.iter().any(|model| {
            model.scope_kind == ModelScopeKind::Platform && model.display_name == candidate
        }) {
            return candidate;
        }
        copy_index += 1;
    }
}

pub(super) fn next_workspace_copy_display_name(
    records: &[ModelView],
    display_name: &str,
) -> String {
    let base_name = format!("{display_name}-Copy");
    if !records
        .iter()
        .any(|record| record.display_name == base_name)
    {
        return base_name;
    }

    let mut copy_index = 2;
    loop {
        let candidate = format!("{display_name}-Copy-{copy_index}");
        if !records
            .iter()
            .any(|record| record.display_name == candidate)
        {
            return candidate;
        }
        copy_index += 1;
    }
}

pub(super) fn ensure_workspace_catalog_display_name_unique(
    records: &[ResolvedWorkspaceRecord],
    workspace_code: &str,
    display_name: &str,
    exclude_model_code: Option<&str>,
) -> Result<(), ModelError> {
    if records.iter().any(|record| {
        record.view.display_name == display_name
            && Some(record.view.model_code.as_str()) != exclude_model_code
    }) {
        return Err(ModelError::DuplicateWorkspaceDisplayName {
            workspace_code: workspace_code.into(),
        });
    }
    Ok(())
}

pub(super) fn find_platform_model<'a>(
    models: &'a [Model],
    model_code: &str,
) -> Result<&'a Model, ModelError> {
    models
        .iter()
        .find(|model| {
            model.scope_kind == ModelScopeKind::Platform && model.model_code == model_code.trim()
        })
        .ok_or(ModelError::NotFound)
}

pub(super) fn build_model_code(id: i64) -> String {
    format!("MODEL-{id}")
}

pub(super) fn duplicate_workspace_source_model(
    source: &Model,
    workspace: &WorkspaceDetail,
    id: i64,
    display_name: String,
) -> Model {
    Model {
        provider_id: None,
        request_headers: Default::default(),
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace.workspace_code.clone()),
        model_code: build_model_code(id),
        display_name,
        vendor: source.vendor.clone(),
        protocol: source.protocol,
        model_type: source.model_type,
        model_capabilities: source.model_capabilities.clone(),
        base_url: source.base_url.clone(),
        api_path: source.api_path.clone(),
        api_key_ref: source.api_key_ref.clone(),
        model_name: source.model_name.clone(),
        enabled: source.enabled,
        description: source.description.clone(),
        thinking_enabled: source.thinking_enabled,
        supported_reasoning_efforts: source.supported_reasoning_efforts.clone(),
        default_reasoning_effort: source.default_reasoning_effort,
        structured_output_enabled: source.structured_output_enabled,
        responses_stateful_enabled: source.responses_stateful_enabled,
        temperature: source.temperature,
        max_tokens: source.max_tokens,
        context_window_tokens: source.context_window_tokens,
        top_p: source.top_p,
        max_retries: source.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::WorkspaceLocal,
        source_model_code: None,
        allow_inheritance: Some(source.allow_inheritance.unwrap_or(true)),
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

pub(super) fn status_patch_display_name(slot_model_code: &str) -> String {
    format!("__STATUS_PATCH__{slot_model_code}")
}

pub(super) fn workspace_status_patch_model(
    workspace: &WorkspaceDetail,
    source_slot_code: &str,
    source: &Model,
    id: i64,
    enabled: bool,
) -> Model {
    Model {
        provider_id: None,
        request_headers: Default::default(),
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace.workspace_code.clone()),
        model_code: build_model_code(id),
        display_name: status_patch_display_name(source_slot_code),
        vendor: source.vendor.clone(),
        protocol: source.protocol,
        model_type: source.model_type,
        model_capabilities: source.model_capabilities.clone(),
        base_url: source.base_url.clone(),
        api_path: source.api_path.clone(),
        api_key_ref: source.api_key_ref.clone(),
        model_name: source.model_name.clone(),
        enabled,
        description: source.description.clone(),
        thinking_enabled: source.thinking_enabled,
        supported_reasoning_efforts: source.supported_reasoning_efforts.clone(),
        default_reasoning_effort: source.default_reasoning_effort,
        structured_output_enabled: source.structured_output_enabled,
        responses_stateful_enabled: source.responses_stateful_enabled,
        temperature: source.temperature,
        max_tokens: source.max_tokens,
        context_window_tokens: source.context_window_tokens,
        top_p: source.top_p,
        max_retries: source.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::WorkspaceStatusPatch,
        source_model_code: Some(source_slot_code.to_string()),
        allow_inheritance: None,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

pub(super) fn update_workspace_status_patch_model(
    mut patch: Model,
    source_slot_code: &str,
    source: &Model,
    enabled: bool,
) -> Model {
    patch.display_name = status_patch_display_name(source_slot_code);
    patch.vendor = source.vendor.clone();
    patch.protocol = source.protocol;
    patch.model_type = source.model_type;
    patch.model_capabilities = source.model_capabilities.clone();
    patch.base_url = source.base_url.clone();
    patch.api_path = source.api_path.clone();
    patch.api_key_ref = source.api_key_ref.clone();
    patch.model_name = source.model_name.clone();
    patch.enabled = enabled;
    patch.description = source.description.clone();
    patch.thinking_enabled = source.thinking_enabled;
    patch.supported_reasoning_efforts = source.supported_reasoning_efforts.clone();
    patch.default_reasoning_effort = source.default_reasoning_effort;
    patch.structured_output_enabled = source.structured_output_enabled;
    patch.responses_stateful_enabled = source.responses_stateful_enabled;
    patch.temperature = source.temperature;
    patch.max_tokens = source.max_tokens;
    patch.context_window_tokens = source.context_window_tokens;
    patch.top_p = source.top_p;
    patch.max_retries = source.max_retries;
    patch.last_verified_at = None;
    patch.last_verify_status = None;
    patch.last_verify_message = None;
    patch.source_kind = ModelSourceKind::WorkspaceStatusPatch;
    patch.source_model_code = Some(source_slot_code.to_string());
    patch.allow_inheritance = None;
    patch.updated_at = current_timestamp();
    patch
}

pub(super) fn workspace_override_model(
    workspace: &WorkspaceDetail,
    source_slot_code: &str,
    id: i64,
    spec: NormalizedModelSpec,
    fallback_api_key_ref: &str,
) -> Model {
    Model {
        provider_id: None,
        request_headers: Default::default(),
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace.workspace_code.clone()),
        model_code: build_model_code(id),
        display_name: spec.display_name,
        vendor: spec.vendor,
        protocol: spec.protocol,
        model_type: spec.model_type,
        model_capabilities: spec.model_capabilities,
        base_url: spec.base_url,
        api_path: spec.api_path,
        api_key_ref: if !spec.api_key_ref.is_empty() && spec.api_key_ref != MASKED_API_KEY_VALUE {
            spec.api_key_ref
        } else {
            fallback_api_key_ref.to_string()
        },
        model_name: spec.model_name,
        enabled: spec.enabled,
        description: spec.description,
        thinking_enabled: spec.thinking_enabled,
        supported_reasoning_efforts: spec.supported_reasoning_efforts,
        default_reasoning_effort: spec.default_reasoning_effort,
        structured_output_enabled: spec.structured_output_enabled,
        responses_stateful_enabled: spec.responses_stateful_enabled,
        temperature: spec.temperature,
        max_tokens: spec.max_tokens,
        context_window_tokens: spec.context_window_tokens,
        top_p: spec.top_p,
        max_retries: spec.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::WorkspaceOverride,
        source_model_code: Some(source_slot_code.to_string()),
        allow_inheritance: None,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

pub(super) fn update_workspace_override_model(
    mut existing: Model,
    source_slot_code: &str,
    spec: NormalizedModelSpec,
    fallback_api_key_ref: &str,
) -> Model {
    existing.display_name = spec.display_name;
    existing.vendor = spec.vendor;
    existing.protocol = spec.protocol;
    existing.model_type = spec.model_type;
    existing.model_capabilities = spec.model_capabilities;
    existing.base_url = spec.base_url;
    existing.api_path = spec.api_path;
    existing.api_key_ref =
        if !spec.api_key_ref.is_empty() && spec.api_key_ref != MASKED_API_KEY_VALUE {
            spec.api_key_ref
        } else {
            fallback_api_key_ref.to_string()
        };
    existing.model_name = spec.model_name;
    existing.enabled = spec.enabled;
    existing.description = spec.description;
    existing.thinking_enabled = spec.thinking_enabled;
    existing.supported_reasoning_efforts = spec.supported_reasoning_efforts;
    existing.default_reasoning_effort = spec.default_reasoning_effort;
    existing.structured_output_enabled = spec.structured_output_enabled;
    existing.responses_stateful_enabled = spec.responses_stateful_enabled;
    existing.temperature = spec.temperature;
    existing.max_tokens = spec.max_tokens;
    existing.context_window_tokens = spec.context_window_tokens;
    existing.top_p = spec.top_p;
    existing.max_retries = spec.max_retries;
    existing.last_verified_at = None;
    existing.last_verify_status = None;
    existing.last_verify_message = None;
    existing.source_kind = ModelSourceKind::WorkspaceOverride;
    existing.source_model_code = Some(source_slot_code.to_string());
    existing.allow_inheritance = None;
    existing.updated_at = current_timestamp();
    existing
}

pub(super) fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn masked_api_key_ref(value: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        MASKED_API_KEY_VALUE.to_string()
    }
}

pub(super) fn apply_verify_outcome_to_model(mut model: Model, outcome: VerifyOutcome) -> Model {
    model.last_verified_at = Some(current_timestamp());
    model.last_verify_status = Some(outcome.status);
    model.last_verify_message = Some(outcome.message);
    model.updated_at = current_timestamp();
    model
}

pub(super) fn build_platform_model_from_spec(
    spec: NormalizedModelSpec,
    id: i64,
    model_code: String,
) -> Model {
    Model {
        provider_id: None,
        request_headers: Default::default(),
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Platform,
        workspace_code: None,
        model_code,
        display_name: spec.display_name,
        vendor: spec.vendor,
        protocol: spec.protocol,
        model_type: spec.model_type,
        model_capabilities: spec.model_capabilities,
        base_url: spec.base_url,
        api_path: spec.api_path,
        api_key_ref: spec.api_key_ref,
        model_name: spec.model_name,
        enabled: spec.enabled,
        description: spec.description,
        thinking_enabled: spec.thinking_enabled,
        supported_reasoning_efforts: spec.supported_reasoning_efforts,
        default_reasoning_effort: spec.default_reasoning_effort,
        structured_output_enabled: spec.structured_output_enabled,
        responses_stateful_enabled: spec.responses_stateful_enabled,
        temperature: spec.temperature,
        max_tokens: spec.max_tokens,
        context_window_tokens: spec.context_window_tokens,
        top_p: spec.top_p,
        max_retries: spec.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::Platform,
        source_model_code: None,
        allow_inheritance: None,
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}

pub(super) fn build_workspace_local_model_from_spec(
    workspace_code: &str,
    spec: NormalizedModelSpec,
    id: i64,
    model_code: String,
) -> Model {
    Model {
        provider_id: None,
        request_headers: Default::default(),
        id: SnowflakeId::new(id),
        scope_kind: ModelScopeKind::Workspace,
        workspace_code: Some(workspace_code.to_string()),
        model_code,
        display_name: spec.display_name,
        vendor: spec.vendor,
        protocol: spec.protocol,
        model_type: spec.model_type,
        model_capabilities: spec.model_capabilities,
        base_url: spec.base_url,
        api_path: spec.api_path,
        api_key_ref: spec.api_key_ref,
        model_name: spec.model_name,
        enabled: spec.enabled,
        description: spec.description,
        thinking_enabled: spec.thinking_enabled,
        supported_reasoning_efforts: spec.supported_reasoning_efforts,
        default_reasoning_effort: spec.default_reasoning_effort,
        structured_output_enabled: spec.structured_output_enabled,
        responses_stateful_enabled: spec.responses_stateful_enabled,
        temperature: spec.temperature,
        max_tokens: spec.max_tokens,
        context_window_tokens: spec.context_window_tokens,
        top_p: spec.top_p,
        max_retries: spec.max_retries,
        last_verified_at: None,
        last_verify_status: None,
        last_verify_message: None,
        source_kind: ModelSourceKind::WorkspaceLocal,
        source_model_code: None,
        allow_inheritance: Some(spec.allow_inheritance.unwrap_or(true)),
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
    }
}
