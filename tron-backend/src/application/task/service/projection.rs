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

/// One adjacency index and one bottom-up progress calculation per projection.
/// A large task tree must not rescan every task for every parent and child.
pub(super) struct TaskHierarchy<'a> {
    tasks: std::collections::BTreeMap<&'a str, &'a Task>,
    children: std::collections::BTreeMap<&'a str, Vec<&'a Task>>,
    progress: std::collections::BTreeMap<&'a str, u8>,
}
impl<'a> TaskHierarchy<'a> {
    pub(super) fn new(tasks: &'a [Task]) -> Self {
        let mut hierarchy = Self {
            tasks: tasks
                .iter()
                .map(|task| (task.task_code.as_str(), task))
                .collect(),
            children: Default::default(),
            progress: Default::default(),
        };
        for task in tasks.iter().filter(|task| task.archived_at.is_none()) {
            if let Some(parent) = task.parent_task_code.as_deref() {
                hierarchy.children.entry(parent).or_default().push(task);
            }
        }
        for task in tasks {
            if hierarchy.progress.contains_key(task.task_code.as_str()) {
                continue;
            }
            let mut stack = vec![(task, false)];
            let mut visiting = HashSet::new();
            while let Some((task, expanded)) = stack.pop() {
                let code = task.task_code.as_str();
                if hierarchy.progress.contains_key(code) {
                    continue;
                }
                let children = hierarchy
                    .children
                    .get(code)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                if expanded {
                    let total = children
                        .iter()
                        .map(|child| {
                            usize::from(
                                hierarchy
                                    .progress
                                    .get(child.task_code.as_str())
                                    .copied()
                                    .unwrap_or(child.progress),
                            )
                        })
                        .sum::<usize>();
                    let value = if children.is_empty() {
                        task.progress
                    } else {
                        ((total + children.len() / 2) / children.len()) as u8
                    };
                    hierarchy.progress.insert(code, value);
                    visiting.remove(code);
                } else if visiting.insert(code) {
                    stack.push((task, true));
                    stack.extend(children.iter().map(|child| (*child, false)));
                } else {
                    // Persisted corruption must not recurse forever.
                    hierarchy.progress.insert(code, task.progress);
                }
            }
        }
        hierarchy
    }
    pub(super) fn task(&self, code: &str) -> Option<&Task> {
        self.tasks.get(code).copied()
    }
    pub(super) fn children(&self, code: &str) -> &[&Task] {
        self.children
            .get(code)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
    pub(super) fn progress(&self, task: &Task) -> u8 {
        self.progress
            .get(task.task_code.as_str())
            .copied()
            .unwrap_or(task.progress)
    }
}
pub(super) fn summary_from_task(task: &Task, hierarchy: &TaskHierarchy<'_>) -> TaskSummary {
    TaskSummary {
        task_code: task.task_code.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        progress: hierarchy.progress(task),
        task_type: task.task_type,
    }
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
