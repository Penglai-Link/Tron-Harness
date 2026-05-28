use super::*;

pub(super) fn normalized_name(value: String) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::NameRequired);
    }
    Ok(trimmed.to_string())
}

pub(super) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn required_local_directory(value: String) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::LocalDirectoryRequired);
    }
    Ok(trimmed.to_string())
}

pub(super) fn normalized_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen.insert(tag.to_ascii_lowercase()))
        .collect()
}

pub(super) fn tag_rows(app_code: &str, tags: &[String]) -> Vec<AppTag> {
    tags.iter()
        .map(|tag| AppTag {
            app_code: app_code.to_string(),
            tag: tag.clone(),
            linked_at: current_timestamp(),
        })
        .collect()
}

pub(super) fn normalized_search(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
}

pub(super) fn matches_search(
    app: &App,
    tags_by_code: &BTreeMap<String, Vec<String>>,
    search: Option<&str>,
) -> bool {
    let Some(search) = search else {
        return true;
    };
    let search_blob = [
        app.app_code.as_str(),
        app.name.as_str(),
        app.description.as_deref().unwrap_or_default(),
        app.local_directory.as_str(),
        match app.runtime_role {
            AppRuntimeRole::Frontend => "frontend",
            AppRuntimeRole::Backend => "backend",
        },
        app.repository_url.as_deref().unwrap_or_default(),
        app.default_branch.as_deref().unwrap_or_default(),
        match app.status {
            AppStatus::Active => "active",
            AppStatus::Archived => "archived",
        },
        &tags_by_code
            .get(&app.app_code)
            .cloned()
            .unwrap_or_default()
            .join(" "),
    ]
    .join(" ")
    .to_lowercase();

    search_blob.contains(search)
}

pub(super) fn map_workspace_error(error: WorkspaceError) -> AppError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::ArchivedMutationDenied => {
            AppError::InvalidWorkspace
        }
        _ => AppError::StorageFailure,
    }
}
