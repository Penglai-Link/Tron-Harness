use std::{
    collections::{BTreeSet, HashSet},
    sync::{Arc, Mutex},
};

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        task::{
            PagedTaskRecords, TASK_STATUSES, Task, TaskCodeLink, TaskDetail, TaskError,
            TaskKanbanColumn, TaskLifecycleResult, TaskListItem, TaskListQuery, TaskOptions,
            TaskPriority, TaskSourceType, TaskStatus, TaskSummary, TaskType, TaskUserAssignment,
            infer_task_role, task_default_status_for_role, task_options,
            task_status_allowed_for_role,
        },
        workspace::{WorkspaceAccessListQuery, WorkspaceError, WorkspaceStatus},
    },
    shared::{
        event_bus::{BusEventDraft, EventBusHandle},
        id::SnowflakeId,
    },
};

const DEFAULT_TIMESTAMP: &str = "2026-05-16T00:00:00Z";

#[derive(Debug, Clone)]
pub struct CreateTaskCommand {
    pub workspace_code: String,
    pub parent_task_code: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    pub source_type: TaskSourceType,
    pub source_url: Option<String>,
    pub assignee_user_codes: Vec<String>,
    pub agent_codes: Vec<String>,
    pub app_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateTaskCommand {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
    pub source_type: Option<TaskSourceType>,
    pub source_url: Option<Option<String>>,
    pub assignee_user_codes: Option<Vec<String>>,
    pub agent_codes: Option<Vec<String>>,
    pub app_codes: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct UpdateTaskStatusCommand {
    pub status: TaskStatus,
    pub progress: Option<u8>,
}

pub trait TaskRepository: Send {
    fn list_tasks(&mut self) -> Result<Vec<Task>, TaskError>;
    fn next_id(&mut self) -> Result<i64, TaskError>;
    fn create_task(&mut self, task: &Task) -> Result<(), TaskError>;
    fn update_task(&mut self, task: &Task) -> Result<(), TaskError>;
    fn list_user_assignments(
        &mut self,
        task_code: &str,
    ) -> Result<Vec<TaskUserAssignment>, TaskError>;
    fn replace_user_assignments(
        &mut self,
        task_code: &str,
        entries: &[TaskUserAssignment],
    ) -> Result<Vec<TaskUserAssignment>, TaskError>;
    fn list_agent_links(&mut self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError>;
    fn replace_agent_links(
        &mut self,
        task_code: &str,
        entries: &[TaskCodeLink],
    ) -> Result<Vec<TaskCodeLink>, TaskError>;
    fn list_app_links(&mut self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError>;
    fn replace_app_links(
        &mut self,
        task_code: &str,
        entries: &[TaskCodeLink],
    ) -> Result<Vec<TaskCodeLink>, TaskError>;
}

#[derive(Clone)]
pub struct TaskService {
    repository: Arc<Mutex<Box<dyn TaskRepository>>>,
    workspaces: WorkspaceService,
    event_bus: EventBusHandle,
}

impl TaskService {
    pub fn new(repository: impl TaskRepository + 'static, workspaces: WorkspaceService) -> Self {
        Self::with_event_bus(repository, workspaces, EventBusHandle::disabled())
    }

    pub fn with_event_bus(
        repository: impl TaskRepository + 'static,
        workspaces: WorkspaceService,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            event_bus,
        }
    }
    pub fn options(&self) -> TaskOptions {
        task_options()
    }

    pub fn list_tasks(
        &self,
        query: TaskListQuery,
    ) -> Result<PagedTaskRecords<TaskListItem>, TaskError> {
        let current = query.current.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let records = self.filtered_task_items(&query)?;
        let total = records.len();
        let start = (current - 1) * size;
        Ok(PagedTaskRecords {
            current,
            size,
            total,
            records: records.into_iter().skip(start).take(size).collect(),
        })
    }

    pub fn kanban(&self, query: TaskListQuery) -> Result<Vec<TaskKanbanColumn>, TaskError> {
        let include_archived = query.include_archived.unwrap_or(false);
        let items = self.filtered_task_items(&query)?;
        Ok(TASK_STATUSES
            .into_iter()
            .filter(|status| include_archived || *status != TaskStatus::Archived)
            .map(|status| TaskKanbanColumn {
                status,
                title: status_title(status).into(),
                tasks: items
                    .iter()
                    .filter(|item| item.status == status)
                    .cloned()
                    .collect(),
            })
            .collect())
    }

    pub fn create_task(&self, command: CreateTaskCommand) -> Result<TaskDetail, TaskError> {
        let workspace_code =
            normalized_required(command.workspace_code).ok_or(TaskError::WorkspaceRequired)?;
        self.ensure_active_workspace(&workspace_code)?;
        let title = normalized_required(command.title).ok_or(TaskError::InvalidRequest)?;
        let parent_task_code = optional_trimmed(command.parent_task_code);
        if let Some(parent_code) = parent_task_code.as_deref() {
            self.ensure_valid_parent(parent_code, &workspace_code)?;
        }
        let task_role = infer_task_role(command.task_type, parent_task_code.as_deref());
        let assignees = self.validated_assignees(&workspace_code, command.assignee_user_codes)?;
        let agent_links = normalized_code_links(command.agent_codes);
        let app_links = normalized_code_links(command.app_codes);
        let id = self
            .repository
            .lock()
            .expect("task repository mutex poisoned")
            .next_id()?;
        let task_code = format!("TSK-{id}");
        let task = Task {
            id: SnowflakeId::new(id),
            task_code: task_code.clone(),
            workspace_code,
            parent_task_code,
            title,
            description: optional_trimmed(command.description),
            status: task_default_status_for_role(task_role),
            previous_status: None,
            priority: command.priority,
            task_type: command.task_type,
            source_type: command.source_type,
            source_url: optional_source_url(command.source_url),
            progress: 0,
            created_at: DEFAULT_TIMESTAMP.into(),
            updated_at: DEFAULT_TIMESTAMP.into(),
            archived_at: None,
        };
        {
            let mut repository = self
                .repository
                .lock()
                .expect("task repository mutex poisoned");
            repository.create_task(&task)?;
            repository.replace_user_assignments(&task_code, &assignees)?;
            repository.replace_agent_links(&task_code, &agent_links)?;
            repository.replace_app_links(&task_code, &app_links)?;
        }
        let detail = self.get_task(&task_code)?;
        self.publish_task_detail_event("task.created", "Created task", &detail);
        Ok(detail)
    }

    pub fn get_task(&self, task_code: &str) -> Result<TaskDetail, TaskError> {
        self.detail_from_task(self.get_task_record(task_code)?)
    }

    pub fn update_task(
        &self,
        task_code: &str,
        command: UpdateTaskCommand,
    ) -> Result<TaskDetail, TaskError> {
        let mut task = self.get_task_record(task_code)?;
        if task.status == TaskStatus::Archived {
            return Err(TaskError::ArchivedMutationDenied);
        }
        if let Some(title) = command.title {
            task.title = normalized_required(title).ok_or(TaskError::InvalidRequest)?;
        }
        if let Some(description) = command.description {
            task.description = description.and_then(|value| optional_trimmed(Some(value)));
        }
        if let Some(priority) = command.priority {
            task.priority = priority;
        }
        if let Some(task_type) = command.task_type {
            let next_role = infer_task_role(task_type, task.parent_task_code.as_deref());
            if !task_status_allowed_for_role(next_role, task.status) {
                return Err(TaskError::StatusRoleInvalid);
            }
            task.task_type = task_type;
        }
        if let Some(source_type) = command.source_type {
            task.source_type = source_type;
        }
        if let Some(source_url) = command.source_url {
            task.source_url = optional_source_url(source_url);
        }
        task.updated_at = DEFAULT_TIMESTAMP.into();
        let assignees = command
            .assignee_user_codes
            .map(|codes| self.validated_assignees(&task.workspace_code, codes))
            .transpose()?;
        let agent_links = command.agent_codes.map(normalized_code_links);
        let app_links = command.app_codes.map(normalized_code_links);
        {
            let mut repository = self
                .repository
                .lock()
                .expect("task repository mutex poisoned");
            repository.update_task(&task)?;
            if let Some(entries) = assignees {
                repository.replace_user_assignments(task_code, &entries)?;
            }
            if let Some(entries) = agent_links {
                repository.replace_agent_links(task_code, &entries)?;
            }
            if let Some(entries) = app_links {
                repository.replace_app_links(task_code, &entries)?;
            }
        }
        let detail = self.get_task(task_code)?;
        self.publish_task_detail_event("task.updated", "Updated task", &detail);
        Ok(detail)
    }

    pub fn update_status(
        &self,
        task_code: &str,
        command: UpdateTaskStatusCommand,
    ) -> Result<TaskDetail, TaskError> {
        if command.status == TaskStatus::Archived {
            return Err(TaskError::InvalidRequest);
        }
        let mut task = self.get_task_record(task_code)?;
        if task.status == TaskStatus::Archived {
            return Err(TaskError::ArchivedMutationDenied);
        }
        let task_role = infer_task_role(task.task_type, task.parent_task_code.as_deref());
        if !task_status_allowed_for_role(task_role, command.status) {
            return Err(TaskError::StatusRoleInvalid);
        }
        task.status = command.status;
        task.progress = if command.status == TaskStatus::Completed {
            100
        } else {
            command.progress.unwrap_or(task.progress).min(100)
        };
        task.updated_at = DEFAULT_TIMESTAMP.into();
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .update_task(&task)?;
        let detail = self.get_task(task_code)?;
        self.publish_task_detail_event("task.status_updated", "Updated task status", &detail);
        Ok(detail)
    }

    pub fn archive_task(&self, task_code: &str) -> Result<TaskLifecycleResult, TaskError> {
        let mut task = self.get_task_record(task_code)?;
        if task.status != TaskStatus::Archived {
            let has_active_subtasks = self.all_tasks()?.iter().any(|candidate| {
                candidate.parent_task_code.as_deref() == Some(&task.task_code)
                    && candidate.status != TaskStatus::Archived
            });
            if has_active_subtasks {
                return Err(TaskError::ActiveSubtaskExists);
            }
            task.previous_status = Some(task.status);
            task.status = TaskStatus::Archived;
            task.archived_at = Some(DEFAULT_TIMESTAMP.into());
            task.updated_at = DEFAULT_TIMESTAMP.into();
            self.repository
                .lock()
                .expect("task repository mutex poisoned")
                .update_task(&task)?;
        }
        let result = self.lifecycle_result(task_code)?;
        self.publish_task_lifecycle_event("task.archived", "Archived task", &result);
        Ok(result)
    }

    pub fn restore_task(&self, task_code: &str) -> Result<TaskLifecycleResult, TaskError> {
        let mut task = self.get_task_record(task_code)?;
        if task.status == TaskStatus::Archived {
            let task_role = infer_task_role(task.task_type, task.parent_task_code.as_deref());
            task.status = task
                .previous_status
                .filter(|status| task_status_allowed_for_role(task_role, *status))
                .unwrap_or_else(|| task_default_status_for_role(task_role));
            task.previous_status = None;
            task.archived_at = None;
            task.updated_at = DEFAULT_TIMESTAMP.into();
            self.repository
                .lock()
                .expect("task repository mutex poisoned")
                .update_task(&task)?;
        }
        let result = self.lifecycle_result(task_code)?;
        self.publish_task_lifecycle_event("task.restored", "Restored task", &result);
        Ok(result)
    }

    fn lifecycle_result(&self, task_code: &str) -> Result<TaskLifecycleResult, TaskError> {
        let task = self.get_task_record(task_code)?;
        Ok(TaskLifecycleResult {
            task_code: task.task_code,
            status: task.status,
            previous_status: task.previous_status,
            archived_at: task.archived_at,
        })
    }

    fn filtered_task_items(&self, query: &TaskListQuery) -> Result<Vec<TaskListItem>, TaskError> {
        let workspace_code = query
            .workspace_code
            .as_deref()
            .and_then(|value| normalized_required(value.to_owned()))
            .ok_or(TaskError::WorkspaceRequired)?;
        self.ensure_active_workspace(&workspace_code)?;
        let include_archived = query.include_archived.unwrap_or(false);
        let all_tasks = self.all_tasks()?;
        let mut records = all_tasks
            .iter()
            .filter(|task| task.workspace_code == workspace_code)
            .filter(|task| include_archived || task.status != TaskStatus::Archived)
            .filter(|task| query.status.map_or(true, |status| task.status == status))
            .filter(|task| {
                query
                    .priority
                    .map_or(true, |priority| task.priority == priority)
            })
            .filter(|task| {
                query
                    .task_type
                    .map_or(true, |task_type| task.task_type == task_type)
            })
            .filter(|task| {
                query
                    .source_type
                    .map_or(true, |source_type| task.source_type == source_type)
            })
            .filter(|task| parent_filter_matches(task, query.parent_task_code.as_deref()))
            .filter(|task| matches_search(task, query.search.as_deref()))
            .map(|task| self.item_from_task(task, &all_tasks))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(assignee_user_code) = query
            .assignee_user_code
            .as_deref()
            .and_then(|value| normalized_required(value.to_owned()))
        {
            records.retain(|record| record.assignee_user_codes.contains(&assignee_user_code));
        }
        records.sort_by(|left, right| left.task_code.cmp(&right.task_code));
        Ok(records)
    }

    fn detail_from_task(&self, task: Task) -> Result<TaskDetail, TaskError> {
        let all_tasks = self.all_tasks()?;
        let parent = task
            .parent_task_code
            .as_deref()
            .and_then(|parent_code| {
                all_tasks
                    .iter()
                    .find(|candidate| candidate.task_code == parent_code)
            })
            .map(|parent| summary_from_task(parent, &all_tasks));
        let mut subtasks = all_tasks
            .iter()
            .filter(|candidate| candidate.parent_task_code.as_deref() == Some(&task.task_code))
            .filter(|candidate| candidate.status != TaskStatus::Archived)
            .map(|candidate| summary_from_task(candidate, &all_tasks))
            .collect::<Vec<_>>();
        subtasks.sort_by(|left, right| left.task_code.cmp(&right.task_code));
        let assignee_user_codes = self
            .raw_user_assignments(&task.task_code)?
            .into_iter()
            .map(|entry| entry.user_code)
            .collect();
        let agent_codes = self
            .raw_agent_links(&task.task_code)?
            .into_iter()
            .map(|entry| entry.code)
            .collect();
        let app_codes = self
            .raw_app_links(&task.task_code)?
            .into_iter()
            .map(|entry| entry.code)
            .collect();
        let progress = effective_progress(&task, &all_tasks);
        Ok(TaskDetail {
            id: task.id.as_decimal_string(),
            task_code: task.task_code,
            workspace_code: task.workspace_code,
            parent_task_code: task.parent_task_code,
            title: task.title,
            description: task.description,
            status: task.status,
            previous_status: task.previous_status,
            priority: task.priority,
            task_type: task.task_type,
            source_type: task.source_type,
            source_url: task.source_url,
            progress,
            assignee_user_codes,
            agent_codes,
            app_codes,
            parent,
            subtasks,
            created_at: task.created_at,
            updated_at: task.updated_at,
            archived_at: task.archived_at,
        })
    }

    fn item_from_task(&self, task: &Task, all_tasks: &[Task]) -> Result<TaskListItem, TaskError> {
        Ok(TaskListItem {
            task_code: task.task_code.clone(),
            workspace_code: task.workspace_code.clone(),
            parent_task_code: task.parent_task_code.clone(),
            title: task.title.clone(),
            status: task.status,
            priority: task.priority,
            progress: effective_progress(task, all_tasks),
            task_type: task.task_type,
            source_type: task.source_type,
            source_url: task.source_url.clone(),
            assignee_user_codes: self
                .raw_user_assignments(&task.task_code)?
                .into_iter()
                .map(|entry| entry.user_code)
                .collect(),
            agent_codes: self
                .raw_agent_links(&task.task_code)?
                .into_iter()
                .map(|entry| entry.code)
                .collect(),
            app_codes: self
                .raw_app_links(&task.task_code)?
                .into_iter()
                .map(|entry| entry.code)
                .collect(),
            subtask_count: all_tasks
                .iter()
                .filter(|candidate| candidate.parent_task_code.as_deref() == Some(&task.task_code))
                .filter(|candidate| candidate.status != TaskStatus::Archived)
                .count(),
            updated_at: task.updated_at.clone(),
            archived_at: task.archived_at.clone(),
        })
    }

    fn get_task_record(&self, task_code: &str) -> Result<Task, TaskError> {
        let normalized = task_code.trim();
        if normalized.is_empty() {
            return Err(TaskError::NotFound);
        }
        self.all_tasks()?
            .into_iter()
            .find(|task| task.task_code == normalized)
            .ok_or(TaskError::NotFound)
    }
    fn all_tasks(&self) -> Result<Vec<Task>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_tasks()
    }
    fn raw_user_assignments(&self, task_code: &str) -> Result<Vec<TaskUserAssignment>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_user_assignments(task_code)
    }
    fn raw_agent_links(&self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_agent_links(task_code)
    }
    fn raw_app_links(&self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError> {
        self.repository
            .lock()
            .expect("task repository mutex poisoned")
            .list_app_links(task_code)
    }
    fn ensure_active_workspace(&self, workspace_code: &str) -> Result<(), TaskError> {
        let detail = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(map_workspace_error)?;
        if detail.status != WorkspaceStatus::Active {
            return Err(TaskError::InvalidWorkspace);
        }
        Ok(())
    }
    fn ensure_valid_parent(
        &self,
        parent_task_code: &str,
        workspace_code: &str,
    ) -> Result<(), TaskError> {
        let parent = self.get_task_record(parent_task_code)?;
        if parent.workspace_code != workspace_code || parent.status == TaskStatus::Archived {
            return Err(TaskError::InvalidParent);
        }
        Ok(())
    }

    fn publish_task_detail_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        detail: &TaskDetail,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "task_service")
                .subject("task", detail.task_code.clone())
                .summary(summary)
                .attr("workspaceCode", detail.workspace_code.clone())
                .attr("status", format!("{:?}", detail.status))
                .attr("progress", detail.progress.to_string())
                .attr("assigneeCount", detail.assignee_user_codes.len().to_string())
                .attr("agentCount", detail.agent_codes.len().to_string())
                .attr("appCount", detail.app_codes.len().to_string()),
        );
    }

    fn publish_task_lifecycle_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        result: &TaskLifecycleResult,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "task_service")
                .subject("task", result.task_code.clone())
                .summary(summary)
                .attr("status", format!("{:?}", result.status))
                .attr(
                    "previousStatus",
                    result
                        .previous_status
                        .map(|status| format!("{status:?}"))
                        .unwrap_or_else(|| "None".into()),
                ),
        );
    }

    fn validated_assignees(
        &self,
        workspace_code: &str,
        user_codes: Vec<String>,
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        let effective_users = self
            .workspaces
            .list_effective_users(
                workspace_code,
                WorkspaceAccessListQuery {
                    current: Some(1),
                    size: Some(500),
                    search: None,
                },
            )
            .map_err(map_workspace_error)?;
        let allowed_user_codes = effective_users
            .records
            .into_iter()
            .map(|user| user.user_code)
            .collect::<BTreeSet<_>>();
        let mut unique = BTreeSet::new();
        user_codes
            .into_iter()
            .filter_map(normalized_required)
            .filter(|user_code| unique.insert(user_code.clone()))
            .map(|user_code| {
                if !allowed_user_codes.contains(&user_code) {
                    return Err(TaskError::AssigneeNotInWorkspace);
                }
                Ok(TaskUserAssignment {
                    user_code,
                    assigned_at: DEFAULT_TIMESTAMP.into(),
                })
            })
            .collect()
    }
}

fn normalized_required(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
fn optional_source_url(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string())
}
fn normalized_code_links(codes: Vec<String>) -> Vec<TaskCodeLink> {
    let mut unique = HashSet::new();
    codes
        .into_iter()
        .filter_map(normalized_required)
        .filter(|code| unique.insert(code.clone()))
        .map(|code| TaskCodeLink {
            code,
            linked_at: DEFAULT_TIMESTAMP.into(),
        })
        .collect()
}
fn parent_filter_matches(task: &Task, parent_task_code: Option<&str>) -> bool {
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
fn summary_from_task(task: &Task, all_tasks: &[Task]) -> TaskSummary {
    TaskSummary {
        task_code: task.task_code.clone(),
        title: task.title.clone(),
        status: task.status,
        priority: task.priority,
        progress: effective_progress(task, all_tasks),
        task_type: task.task_type,
    }
}
fn effective_progress(task: &Task, all_tasks: &[Task]) -> u8 {
    let active_subtasks = all_tasks
        .iter()
        .filter(|candidate| candidate.parent_task_code.as_deref() == Some(&task.task_code))
        .filter(|candidate| candidate.status != TaskStatus::Archived)
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
fn matches_search(task: &Task, search: Option<&str>) -> bool {
    let Some(search) = search.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    let search = search.to_lowercase();
    [
        task.task_code.as_str(),
        task.workspace_code.as_str(),
        task.title.as_str(),
        task.description.as_deref().unwrap_or_default(),
        task.source_url.as_deref().unwrap_or_default(),
    ]
    .join(" ")
    .to_lowercase()
    .contains(&search)
}
fn status_title(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Open => "Open",
        TaskStatus::Designing => "Designing",
        TaskStatus::Red => "Red",
        TaskStatus::Green => "Green",
        TaskStatus::Regressioning => "Regressioning",
        TaskStatus::Review => "Review",
        TaskStatus::InProgress => "In Progress",
        TaskStatus::Reopen => "Reopen",
        TaskStatus::Completed => "Completed",
        TaskStatus::Archived => "Archived",
    }
}
fn map_workspace_error(error: WorkspaceError) -> TaskError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::ArchivedMutationDenied => {
            TaskError::InvalidWorkspace
        }
        _ => TaskError::StorageFailure,
    }
}
