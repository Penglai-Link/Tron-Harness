use super::*;

pub(super) fn supported_reasoning_efforts_to_storage(
    value: &[ReasoningEffort],
) -> Result<String, ModelError> {
    serde_json::to_string(value).map_err(|_| ModelError::StorageFailure)
}

pub(super) fn supported_reasoning_efforts_from_storage(
    value: Option<&str>,
) -> Result<Vec<ReasoningEffort>, ModelError> {
    let parsed = match value {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str::<Vec<ReasoningEffort>>(raw)
            .map_err(|_| ModelError::StorageFailure)?,
        _ => Vec::new(),
    };
    Ok(ReasoningEffort::ALL
        .into_iter()
        .filter(|effort| parsed.contains(effort))
        .collect())
}

pub(super) fn reasoning_effort_from_storage(
    value: Option<&str>,
) -> Result<Option<ReasoningEffort>, ModelError> {
    value
        .map(|raw| ReasoningEffort::from_storage_value(raw).ok_or(ModelError::StorageFailure))
        .transpose()
}

pub(super) fn model_capabilities_to_storage(
    value: &[ModelCapability],
) -> Result<String, ModelError> {
    serde_json::to_string(value).map_err(|_| ModelError::StorageFailure)
}

pub(super) fn model_capabilities_from_storage(
    value: Option<&str>,
    model_type: ModelType,
) -> Result<Vec<ModelCapability>, ModelError> {
    let parsed = match value {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str::<Vec<ModelCapability>>(raw)
            .map_err(|_| ModelError::StorageFailure)?,
        _ => return Ok(default_model_capabilities_for_type(model_type)),
    };
    let mut normalized = Vec::new();
    for item in parsed {
        if !normalized.contains(&item) {
            normalized.push(item);
        }
    }
    if normalized.is_empty() {
        return Ok(default_model_capabilities_for_type(model_type));
    }
    Ok(normalized)
}

pub(super) fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

pub(super) fn scope_kind_to_storage(value: ModelScopeKind) -> &'static str {
    match value {
        ModelScopeKind::Platform => "platform",
        ModelScopeKind::Workspace => "workspace",
    }
}

pub(super) fn scope_kind_from_storage(value: &str) -> Result<ModelScopeKind, ModelError> {
    match value {
        "platform" => Ok(ModelScopeKind::Platform),
        "workspace" => Ok(ModelScopeKind::Workspace),
        _ => Err(ModelError::StorageFailure),
    }
}

pub(super) fn source_kind_to_storage(value: ModelSourceKind) -> &'static str {
    match value {
        ModelSourceKind::Platform => "platform",
        ModelSourceKind::WorkspaceLocal => "workspace_local",
        ModelSourceKind::WorkspaceOverride => "workspace_override",
        ModelSourceKind::WorkspaceStatusPatch => "workspace_status_patch",
    }
}

pub(super) fn source_kind_from_storage(value: &str) -> Result<ModelSourceKind, ModelError> {
    match value {
        "platform" => Ok(ModelSourceKind::Platform),
        "workspace_local" => Ok(ModelSourceKind::WorkspaceLocal),
        "workspace_override" => Ok(ModelSourceKind::WorkspaceOverride),
        "workspace_status_patch" => Ok(ModelSourceKind::WorkspaceStatusPatch),
        _ => Err(ModelError::StorageFailure),
    }
}

pub(super) fn protocol_to_storage(value: ModelProtocol) -> &'static str {
    match value {
        ModelProtocol::OpenAiCompatible => "OPENAI_COMPATIBLE",
        ModelProtocol::OpenAiCompatibleEmbedding => "OPENAI_COMPATIBLE_EMBEDDING",
        ModelProtocol::Responses => "RESPONSES",
        ModelProtocol::Rerank => "RERANK",
    }
}

pub(super) fn protocol_from_storage(value: &str) -> Result<ModelProtocol, ModelError> {
    match value {
        "OPENAI_COMPATIBLE" => Ok(ModelProtocol::OpenAiCompatible),
        "OPENAI_COMPATIBLE_EMBEDDING" => Ok(ModelProtocol::OpenAiCompatibleEmbedding),
        "RESPONSES" => Ok(ModelProtocol::Responses),
        "RERANK" => Ok(ModelProtocol::Rerank),
        _ => Err(ModelError::StorageFailure),
    }
}

pub(super) fn model_type_to_storage(value: ModelType) -> &'static str {
    match value {
        ModelType::Text => "TEXT",
        ModelType::Embedding => "EMBEDDING",
        ModelType::Rerank => "RERANK",
        ModelType::Image => "IMAGE",
    }
}

pub(super) fn model_type_from_storage(value: &str) -> Result<ModelType, ModelError> {
    match value {
        "TEXT" => Ok(ModelType::Text),
        "EMBEDDING" => Ok(ModelType::Embedding),
        "RERANK" => Ok(ModelType::Rerank),
        "IMAGE" => Ok(ModelType::Image),
        _ => Err(ModelError::StorageFailure),
    }
}

pub(super) fn verify_status_to_storage(value: ModelVerifyStatus) -> &'static str {
    match value {
        ModelVerifyStatus::Succeeded => "SUCCEEDED",
        ModelVerifyStatus::Failed => "FAILED",
    }
}

pub(super) fn verify_status_from_storage(value: &str) -> Result<ModelVerifyStatus, ModelError> {
    match value {
        "SUCCEEDED" => Ok(ModelVerifyStatus::Succeeded),
        "FAILED" => Ok(ModelVerifyStatus::Failed),
        _ => Err(ModelError::StorageFailure),
    }
}

pub(super) fn storage_error(_error: rusqlite::Error) -> ModelError {
    ModelError::StorageFailure
}
