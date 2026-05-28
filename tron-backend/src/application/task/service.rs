use std::{
    collections::{BTreeSet, HashSet},
    sync::{Arc, Mutex},
};

use crate::{
    application::{app::AppService, workspace::WorkspaceService},
    domain::{
        app::AppError,
        task::{
            PagedTaskRecords, TASK_STATUSES, Task, TaskCodeLink, TaskDetail, TaskError,
            TaskKanbanColumn, TaskLifecycleResult, TaskListItem, TaskListQuery, TaskOptions,
            TaskPriority, TaskSourceType, TaskStatus, TaskSummary, TaskType, TaskUserAssignment,
            infer_task_role, task_default_status_for_role, task_options,
            task_status_allowed_for_role, task_statuses_for_role,
        },
        workspace::{WorkspaceAccessListQuery, WorkspaceError, WorkspaceStatus},
    },
    shared::{
        event_bus::{BusEventDraft, EventBusHandle},
        id::SnowflakeId,
        time::current_timestamp,
    },
};

mod projection;
mod use_cases;

use self::projection::*;

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
    apps: AppService,
    event_bus: EventBusHandle,
}

impl TaskService {
    pub fn new(
        repository: impl TaskRepository + 'static,
        workspaces: WorkspaceService,
        apps: AppService,
    ) -> Self {
        Self::with_event_bus(repository, workspaces, apps, EventBusHandle::disabled())
    }

    pub fn with_event_bus(
        repository: impl TaskRepository + 'static,
        workspaces: WorkspaceService,
        apps: AppService,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            apps,
            event_bus,
        }
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
                .attr(
                    "assigneeCount",
                    detail.assignee_user_codes.len().to_string(),
                )
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
                    assigned_at: current_timestamp(),
                })
            })
            .collect()
    }

    fn validated_app_links(
        &self,
        workspace_code: &str,
        app_links: Vec<TaskCodeLink>,
    ) -> Result<Vec<TaskCodeLink>, TaskError> {
        let allowed_codes = self
            .apps
            .assignable_app_codes(workspace_code)
            .map_err(map_app_error)?;
        for app_link in &app_links {
            if !allowed_codes.contains(&app_link.code) {
                return Err(TaskError::InvalidApp);
            }
        }
        Ok(app_links)
    }
}
