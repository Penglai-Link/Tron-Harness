use super::*;

pub(super) fn find_local_variable(
    variables: &[Variable],
    workspace_code: &str,
    key: &str,
) -> Option<Variable> {
    variables
        .iter()
        .find(|variable| variable.workspace_code == workspace_code && variable.key == key)
        .cloned()
}

pub(super) fn map_workspace_error(error: WorkspaceError) -> VariableError {
    match error {
        WorkspaceError::StorageFailure => VariableError::StorageFailure,
        WorkspaceError::NotFound => VariableError::InvalidWorkspace,
        _ => VariableError::InvalidWorkspace,
    }
}

pub(super) fn matches_search(record: &EffectiveVariable, search: &str) -> bool {
    [
        record.key.as_str(),
        record.value.as_str(),
        record.description.as_deref().unwrap_or(""),
        record.source_workspace_code.as_str(),
        record.source_workspace_name.as_str(),
        record.source_label.as_str(),
    ]
    .join(" ")
    .to_lowercase()
    .contains(search)
}

pub(super) fn normalized_key(value: String) -> Result<String, VariableError> {
    validate_variable_key(&value)?;
    Ok(value.trim().to_string())
}

pub(super) fn normalized_search(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
}

pub(super) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn workspace_lineage(workspace: &WorkspaceDetail) -> Vec<LineageEntry> {
    let mut labels = Vec::new();
    workspace
        .path
        .iter()
        .map(|item| {
            labels.push(item.name.clone());
            LineageEntry {
                code: item.workspace_code.clone(),
                label: labels.join(" / "),
                name: item.name.clone(),
            }
        })
        .collect()
}
