use super::*;

pub(super) fn normalized_required(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
pub(super) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
pub(super) fn optional_source_url(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string())
}
pub(super) fn normalized_code_links(codes: Vec<String>) -> Vec<TaskCodeLink> {
    let mut unique = HashSet::new();
    codes
        .into_iter()
        .filter_map(normalized_required)
        .filter(|code| unique.insert(code.clone()))
        .map(|code| TaskCodeLink {
            code,
            linked_at: current_timestamp(),
        })
        .collect()
}
pub(super) fn parent_filter_matches(task: &Task, parent_task_code: Option<&str>) -> bool {
    let Some(parent_filter) = parent_task_code.map(str::trim) else {
        return true;
    };
    if parent_filter.is_empty() {
        return true;
    }
    if parent_filter.eq_ignore_ascii_case("__ROOT__") {
        return task.parent_task_code.is_none();
    }
    task.parent_task_code.as_deref() == Some(parent_filter)
}
pub(super) fn summary_from_task(task: &Task, all_tasks: &[Task]) -> TaskSummary {
    TaskSummary {
        task_code: task.task_code.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        progress: effective_progress(task, all_tasks),
        task_type: task.task_type,
    }
}
pub(super) fn effective_progress(task: &Task, all_tasks: &[Task]) -> u8 {
    let active_subtasks = all_tasks
        .iter()
        .filter(|candidate| candidate.parent_task_code.as_deref() == Some(&task.task_code))
        .filter(|candidate| candidate.archived_at.is_none())
        .collect::<Vec<_>>();
    if active_subtasks.is_empty() {
        return task.progress;
    }
    let total = active_subtasks
        .iter()
        .map(|candidate| usize::from(effective_progress(candidate, all_tasks)))
        .sum::<usize>();
    ((total + (active_subtasks.len() / 2)) / active_subtasks.len()) as u8
}
pub(super) fn matches_search(task: &Task, search: Option<&str>) -> bool {
    let Some(search) = search.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    let search = search.to_lowercase();
    [
        task.task_code.as_str(),
        task.workspace_code.as_str(),
        task.title.as_str(),
        task.description.as_deref().unwrap_or_default(),
        task.execution_result.as_deref().unwrap_or_default(),
        task.source_url.as_deref().unwrap_or_default(),
    ]
    .join(" ")
    .to_lowercase()
    .contains(&search)
}
pub(super) fn map_workspace_error(error: WorkspaceError) -> TaskError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::ArchivedMutationDenied => {
            TaskError::InvalidWorkspace
        }
        _ => TaskError::StorageFailure,
    }
}

pub(super) fn map_app_error(error: AppError) -> TaskError {
    match error {
        AppError::StorageFailure => TaskError::StorageFailure,
        _ => TaskError::InvalidApp,
    }
}
