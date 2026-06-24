use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::task::{
        CreateTaskCommand, TaskService, UpdateTaskCommand, UpdateTaskStatusCommand,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        task::{
            TaskError, TaskListQuery, TaskPriority, TaskSourceType, TaskStatus, TaskType,
            infer_task_role, task_status_allowed_for_role,
        },
    },
};

use super::common::{
    ensure_workspace_match, enum_tool_error, json_observation, normalize_optional_token,
    optional_trimmed, parse_tool_input, resolve_workspace_code, service_tool_error,
};

pub(super) fn tools(tasks: TaskService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(TaskPageTool {
            tasks: tasks.clone(),
        }),
        Arc::new(TaskCreateTool {
            tasks: tasks.clone(),
        }),
        Arc::new(TaskUpdateTool {
            tasks: tasks.clone(),
        }),
        Arc::new(TaskDeleteTool {
            tasks: tasks.clone(),
        }),
        Arc::new(TaskRestoreTool { tasks }),
    ]
}

#[derive(Clone)]
struct TaskPageTool {
    tasks: TaskService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskPageInput {
    workspace_code: Option<String>,
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    task_type: Option<String>,
    source_type: Option<String>,
    assignee_user_code: Option<String>,
    parent_task_code: Option<String>,
    include_archived: Option<bool>,
}

impl Tool for TaskPageTool {
    fn name(&self) -> &str {
        "task.page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskPageInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .tasks
            .list_tasks(TaskListQuery {
                workspace_code: Some(workspace_code),
                current: input.current,
                size: input.size,
                search: input.search,
                status: parse_task_status(input.status.as_deref())?,
                priority: parse_task_priority(input.priority.as_deref())?,
                task_type: parse_task_type(input.task_type.as_deref())?,
                source_type: parse_task_source_type(input.source_type.as_deref())?,
                assignee_user_code: input
                    .assignee_user_code
                    .and_then(|value| optional_trimmed(Some(value))),
                parent_task_code: input
                    .parent_task_code
                    .and_then(|value| optional_trimmed(Some(value))),
                include_archived: Some(input.include_archived.unwrap_or(false)),
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct TaskCreateTool {
    tasks: TaskService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskCreateInput {
    workspace_code: Option<String>,
    parent_task_code: Option<String>,
    title: String,
    description: Option<String>,
    priority: Option<String>,
    task_type: Option<String>,
    source_type: Option<String>,
    source_url: Option<String>,
    #[serde(default)]
    assignee_user_codes: Vec<String>,
    #[serde(default)]
    agent_codes: Vec<String>,
    #[serde(default)]
    app_codes: Vec<String>,
    app_code: Option<String>,
}

impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "task.create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskCreateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let mut app_codes = input.app_codes;
        if let Some(app_code) = input
            .app_code
            .and_then(|value| optional_trimmed(Some(value)))
        {
            app_codes.push(app_code);
        }
        let output = self
            .tasks
            .create_task(CreateTaskCommand {
                workspace_code,
                parent_task_code: input.parent_task_code,
                title: input.title.trim().to_string(),
                description: input.description.map(|value| value.trim().to_string()),
                priority: parse_task_priority(input.priority.as_deref())?
                    .unwrap_or(TaskPriority::Medium),
                task_type: parse_task_type(input.task_type.as_deref())?
                    .unwrap_or(TaskType::Feature),
                source_type: parse_task_source_type(input.source_type.as_deref())?
                    .unwrap_or(TaskSourceType::Internal),
                source_url: input.source_url,
                agent_code: None,
                model_code: None,
                assignee_user_codes: input.assignee_user_codes,
                agent_codes: input.agent_codes,
                app_codes,
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct TaskUpdateTool {
    tasks: TaskService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskUpdateInput {
    workspace_code: Option<String>,
    task_code: String,
    title: Option<String>,
    description: Option<String>,
    priority: Option<String>,
    task_type: Option<String>,
    source_type: Option<String>,
    source_url: Option<String>,
    assignee_user_codes: Option<Vec<String>>,
    agent_codes: Option<Vec<String>>,
    app_codes: Option<Vec<String>>,
    status: Option<String>,
    progress: Option<u8>,
}

impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "task.update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskUpdateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let task_code = input.task_code.trim().to_string();
        let current = self
            .tasks
            .get_task(&task_code)
            .map_err(service_tool_error)?;
        ensure_workspace_match(&workspace_code, &current.workspace_code, "task")?;
        let priority = parse_task_priority(input.priority.as_deref())?;
        let task_type = parse_task_type(input.task_type.as_deref())?;
        let source_type = parse_task_source_type(input.source_type.as_deref())?;
        let status = parse_task_status(input.status.as_deref())?;
        if let Some(status) = status {
            let effective_task_type = task_type.unwrap_or(current.task_type);
            let task_role =
                infer_task_role(effective_task_type, current.parent_task_code.as_deref());
            if status == TaskStatus::Archived || !task_status_allowed_for_role(task_role, status) {
                return Err(service_tool_error(TaskError::StatusRoleInvalid));
            }
        }
        let mut output = self
            .tasks
            .update_task(
                &task_code,
                UpdateTaskCommand {
                    title: input.title,
                    description: input.description.map(|value| optional_trimmed(Some(value))),
                    priority,
                    task_type,
                    source_type,
                    source_url: input.source_url.map(|value| optional_trimmed(Some(value))),
                    agent_code: None,
                    model_code: None,
                    assignee_user_codes: input.assignee_user_codes,
                    agent_codes: input.agent_codes,
                    app_codes: input.app_codes,
                },
            )
            .map_err(service_tool_error)?;
        if let Some(status) = status {
            output = self
                .tasks
                .update_status(
                    &task_code,
                    UpdateTaskStatusCommand {
                        status,
                        progress: input.progress,
                    },
                )
                .map_err(service_tool_error)?;
        }
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct TaskDeleteTool {
    tasks: TaskService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskCodeInput {
    workspace_code: Option<String>,
    task_code: String,
}

impl Tool for TaskDeleteTool {
    fn name(&self) -> &str {
        "task.delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let current = self
            .tasks
            .get_task(input.task_code.trim())
            .map_err(service_tool_error)?;
        ensure_workspace_match(&workspace_code, &current.workspace_code, "task")?;
        let output = self
            .tasks
            .archive_task(input.task_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct TaskRestoreTool {
    tasks: TaskService,
}

impl Tool for TaskRestoreTool {
    fn name(&self) -> &str {
        "task.restore"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let current = self
            .tasks
            .get_task(input.task_code.trim())
            .map_err(service_tool_error)?;
        ensure_workspace_match(&workspace_code, &current.workspace_code, "task")?;
        let output = self
            .tasks
            .restore_task(input.task_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

fn parse_task_status(value: Option<&str>) -> Result<Option<TaskStatus>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("open") => Ok(Some(TaskStatus::Open)),
        Some("designing") => Ok(Some(TaskStatus::Designing)),
        Some("red") => Ok(Some(TaskStatus::Red)),
        Some("green") => Ok(Some(TaskStatus::Green)),
        Some("regressioning") => Ok(Some(TaskStatus::Regressioning)),
        Some("review") => Ok(Some(TaskStatus::Review)),
        Some("in_progress") => Ok(Some(TaskStatus::InProgress)),
        Some("reopen") => Ok(Some(TaskStatus::Reopen)),
        Some("completed") => Ok(Some(TaskStatus::Completed)),
        Some("archived") => Ok(Some(TaskStatus::Archived)),
        Some(other) => Err(enum_tool_error("status", other)),
    }
}

fn parse_task_priority(value: Option<&str>) -> Result<Option<TaskPriority>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("low") => Ok(Some(TaskPriority::Low)),
        Some("medium") => Ok(Some(TaskPriority::Medium)),
        Some("high") => Ok(Some(TaskPriority::High)),
        Some(other) => Err(enum_tool_error("priority", other)),
    }
}

fn parse_task_type(value: Option<&str>) -> Result<Option<TaskType>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("feature") => Ok(Some(TaskType::Feature)),
        Some("bug") => Ok(Some(TaskType::Bug)),
        Some(other) => Err(enum_tool_error("taskType", other)),
    }
}

fn parse_task_source_type(value: Option<&str>) -> Result<Option<TaskSourceType>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("internal") => Ok(Some(TaskSourceType::Internal)),
        Some("github") => Ok(Some(TaskSourceType::GitHub)),
        Some("gitlab") => Ok(Some(TaskSourceType::GitLab)),
        Some(other) => Err(enum_tool_error("sourceType", other)),
    }
}
