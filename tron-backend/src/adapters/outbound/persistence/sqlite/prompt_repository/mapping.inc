fn scope_kind_to_storage(value: PromptScopeKind) -> &'static str {
    match value {
        PromptScopeKind::Platform => "PLATFORM",
        PromptScopeKind::Workspace => "WORKSPACE",
    }
}

fn scope_kind_from_storage(value: &str) -> rusqlite::Result<PromptScopeKind> {
    match value {
        "PLATFORM" => Ok(PromptScopeKind::Platform),
        "WORKSPACE" => Ok(PromptScopeKind::Workspace),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn category_to_storage(value: PromptCategory) -> &'static str {
    match value {
        PromptCategory::System => "SYSTEM",
        PromptCategory::Persona => "PERSONA",
        PromptCategory::Tool => "TOOL",
        PromptCategory::Template => "TEMPLATE",
        PromptCategory::Action => "ACTION",
    }
}

fn category_from_storage(value: &str) -> rusqlite::Result<PromptCategory> {
    match value {
        "SYSTEM" => Ok(PromptCategory::System),
        "PERSONA" => Ok(PromptCategory::Persona),
        "TOOL" => Ok(PromptCategory::Tool),
        "TEMPLATE" => Ok(PromptCategory::Template),
        "ACTION" => Ok(PromptCategory::Action),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn variable_type_from_storage(value: &str) -> rusqlite::Result<PromptVariableType> {
    match value {
        "STRING" => Ok(PromptVariableType::String),
        "NUMBER" => Ok(PromptVariableType::Number),
        "BOOLEAN" => Ok(PromptVariableType::Boolean),
        "JSON" => Ok(PromptVariableType::Json),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn variable_source_kind_from_storage(
    value: &str,
) -> rusqlite::Result<PromptVariableSourceKind> {
    match value {
        "MANAGED" => Ok(PromptVariableSourceKind::Managed),
        "RUNTIME" => Ok(PromptVariableSourceKind::Runtime),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn runtime_provider_from_storage(value: &str) -> rusqlite::Result<PromptRuntimeProvider> {
    PromptRuntimeProvider::from_code(value).ok_or(rusqlite::Error::InvalidQuery)
}

fn translation_status_to_storage(value: PromptTranslationStatus) -> &'static str {
    match value {
        PromptTranslationStatus::Published => "PUBLISHED",
        PromptTranslationStatus::Draft => "DRAFT",
        PromptTranslationStatus::UpdateAvailable => "UPDATE_AVAILABLE",
    }
}

fn translation_status_from_storage(value: &str) -> rusqlite::Result<PromptTranslationStatus> {
    match value {
        "PUBLISHED" => Ok(PromptTranslationStatus::Published),
        "DRAFT" => Ok(PromptTranslationStatus::Draft),
        "UPDATE_AVAILABLE" => Ok(PromptTranslationStatus::UpdateAvailable),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn source_kind_to_storage(value: PromptSourceKind) -> &'static str {
    match value {
        PromptSourceKind::Platform => "PLATFORM",
        PromptSourceKind::WorkspaceLocal => "WORKSPACE_LOCAL",
        PromptSourceKind::WorkspaceOverride => "WORKSPACE_OVERRIDE",
        PromptSourceKind::WorkspaceStatusPatch => "WORKSPACE_STATUS_PATCH",
    }
}

fn source_kind_from_storage(value: &str) -> rusqlite::Result<PromptSourceKind> {
    match value {
        "PLATFORM" => Ok(PromptSourceKind::Platform),
        "WORKSPACE_LOCAL" => Ok(PromptSourceKind::WorkspaceLocal),
        "WORKSPACE_OVERRIDE" => Ok(PromptSourceKind::WorkspaceOverride),
        "WORKSPACE_STATUS_PATCH" => Ok(PromptSourceKind::WorkspaceStatusPatch),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}
