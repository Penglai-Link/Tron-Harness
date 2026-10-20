fn category_to_storage(value: PromptCategory) -> &'static str {
    match value {
        PromptCategory::System => "SYSTEM",
        PromptCategory::Developer => "DEVELOPER",
        PromptCategory::Persona => "PERSONA",
        PromptCategory::AgentRole => "AGENT_ROLE",
        PromptCategory::Tool => "TOOL",
        PromptCategory::Template => "TEMPLATE",
        PromptCategory::Action => "ACTION",
    }
}

fn category_from_storage(value: &str) -> rusqlite::Result<PromptCategory> {
    match value {
        "SYSTEM" => Ok(PromptCategory::System),
        "DEVELOPER" => Ok(PromptCategory::Developer),
        "PERSONA" => Ok(PromptCategory::Persona),
        "AGENT_ROLE" => Ok(PromptCategory::AgentRole),
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

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}
