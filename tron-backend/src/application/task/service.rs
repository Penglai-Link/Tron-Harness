use std::{
    collections::{BTreeSet, HashSet},
    sync::{Arc, Mutex},
};

use crate::{
    application::{app::AppService, workspace::WorkspaceService},
    domain::{
        app::AppError,
        task::{
            PagedTaskRecords, Task, TaskCodeLink, TaskDetail, TaskError, TaskFlow, TaskFlowDraft,
            TaskKanbanColumn, TaskLifecycleResult, TaskListItem, TaskListQuery, TaskOptions,
            TaskPriority, TaskSourceType, TaskSummary, TaskType, TaskUserAssignment,
            WorkspaceTaskFlow,
        },
        workspace::{WorkspaceError, WorkspaceStatus},
    },
    shared::{
        event_bus::{BusEventDraft, EventBusHandle},
        id::SnowflakeId,
        time::current_timestamp,
    },
};

mod events;
mod flows;
mod projection;
mod repository_access;
mod task_projection;
mod use_cases;
mod validation;

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
    pub agent_code: Option<String>,
    pub model_code: Option<String>,
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
    pub agent_code: Option<String>,
    pub model_code: Option<Option<String>>,
    pub assignee_user_codes: Option<Vec<String>>,
    pub agent_codes: Option<Vec<String>>,
    pub app_codes: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct UpdateTaskStatusCommand {
    pub status: String,
    pub progress: Option<u8>,
}

pub trait TaskRepository: Send {
    fn list_tasks(&mut self) -> Result<Vec<Task>, TaskError>;
    fn list_task_flows(&mut self) -> Result<Vec<TaskFlow>, TaskError>;
    fn get_task_flow(&mut self, flow_code: &str) -> Result<Option<TaskFlow>, TaskError>;
    fn create_task_flow(&mut self, flow: &TaskFlow) -> Result<(), TaskError>;
    fn update_task_flow(&mut self, flow: &TaskFlow) -> Result<(), TaskError>;
    fn delete_task_flow(&mut self, flow_code: &str) -> Result<(), TaskError>;
    fn workspace_task_flow_code(
        &mut self,
        workspace_code: &str,
    ) -> Result<Option<String>, TaskError>;
    fn assign_workspace_task_flow(
        &mut self,
        workspace_code: &str,
        flow_code: &str,
    ) -> Result<(), TaskError>;
    fn update_workspace_task_flow(
        &mut self,
        workspace_code: &str,
        flow_code: &str,
    ) -> Result<(), TaskError>;
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
}
