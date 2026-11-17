use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::task::{
        CreateTaskCommand, TaskService, UpdateTaskCommand, UpdateTaskStatusCommand,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        task::{TaskDetail, TaskListQuery, TaskPriority, TaskSourceType, TaskType},
    },
};

use super::common::{
    enum_tool_error, json_observation, normalize_optional_token, optional_trimmed,
    parse_tool_input, service_tool_error,
};

pub(super) fn tools(tasks: TaskService) -> Vec<Arc<dyn Tool>> {
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(TaskGetTool {
            tasks: tasks.clone(),
        }),
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
    ];
    let mut result: Vec<Arc<dyn Tool>> = Vec::new();
    for tool in tools {
        let assistant_name = match tool.name() {
            "task_get" => "assistant_task_get",
            "task_page" => "assistant_task_page",
            "task_create" => "assistant_task_create",
            "task_update" => "assistant_task_update",
            "task_delete" => "assistant_task_delete",
            "task_restore" => "assistant_task_restore",
            _ => unreachable!(),
        };
        result.push(Arc::new(TaskScopeTool {
            name: tool.name().to_string(),
            inner: tool.clone(),
            assistant: false,
        }));
        result.push(Arc::new(TaskScopeTool {
            name: assistant_name.into(),
            inner: tool,
            assistant: true,
        }));
    }
    result
}

struct TaskScopeTool {
    name: String,
    inner: Arc<dyn Tool>,
    assistant: bool,
}
impl Tool for TaskScopeTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn exec(
        &self,
        mut request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        if self.assistant {
            if !context.is_owning_assistant() || !context.workspace_code.trim().is_empty() {
                return Err(ToolError::ExecutionFailed(
                    "PersonAI task tools require the owning platform PersonAI".into(),
                ));
            }
            if request.input.get("workspaceCode").is_some() {
                return Err(ToolError::ExecutionFailed(
                    "PersonAI task tools do not accept workspaceCode".into(),
                ));
            }
        } else {
            if context.is_owning_assistant() {
                return Err(ToolError::ExecutionFailed(
                    "PersonAI must use assistant_task tools".into(),
                ));
            }
            if request
                .input
                .get("workspaceCode")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|s| s.trim().is_empty())
            {
                return Err(ToolError::ExecutionFailed(
                    "workspaceCode is required".into(),
                ));
            }
        }
        request.tool_name = self.inner.name().into();
        let mut result = self.inner.exec(request, context)?;
        result.observation.tool_name = self.name.clone();
        Ok(result)
    }
}

#[derive(Clone)]
struct TaskPageTool {
    tasks: TaskService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
        "task_page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskPageInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_task_workspace(&input.workspace_code, context)?;
        let output = self
            .tasks
            .list_tasks(TaskListQuery {
                workspace_code,
                owner_assistant_code: trusted_assistant_owner(context)?,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskCreateInput {
    workspace_code: Option<String>,
    parent_task_code: Option<String>,
    title: String,
    description: Option<String>,
    due_at: Option<i64>,
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
        "task_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskCreateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_task_workspace(&input.workspace_code, context)?;
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
                owner_assistant_code: trusted_assistant_owner(context)?,
                due_at: input.due_at,
                workspace_code,
                parent_task_code: input.parent_task_code,
                title: input.title.trim().to_string(),
                description: input.description.map(|value| value.trim().to_string()),
                priority: parse_task_priority(input.priority.as_deref())?
                    .unwrap_or(TaskPriority::Medium),
                task_type: parse_task_type(input.task_type.as_deref())?.unwrap_or(
                    if context.workspace_code.trim().is_empty()
                        || trusted_assistant_owner(context)?.is_some()
                    {
                        TaskType::General
                    } else {
                        TaskType::Feature
                    },
                ),
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskUpdateInput {
    workspace_code: Option<String>,
    task_code: String,
    #[serde(default, deserialize_with = "deserialize_optional_deadline")]
    due_at: Option<Option<i64>>,
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
        "task_update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskUpdateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_task_workspace(&input.workspace_code, context)?;
        let task_code = input.task_code.trim().to_string();
        let current = self
            .tasks
            .get_task(&task_code)
            .map_err(service_tool_error)?;
        ensure_task_access(context, workspace_code.as_deref(), &current)?;
        let priority = parse_task_priority(input.priority.as_deref())?;
        let task_type = parse_task_type(input.task_type.as_deref())?;
        let source_type = parse_task_source_type(input.source_type.as_deref())?;
        let status = parse_task_status(input.status.as_deref())?;
        if let Some(status) = &status {
            let flow = self
                .tasks
                .get_task_flow(&current.flow_code)
                .map_err(service_tool_error)?;
            if !flow.statuses.iter().any(|item| item.status_code == *status) {
                return Err(ToolError::ExecutionFailed(format!(
                    "Status {status:?} is not in task flow {}. Valid statuses: {}. Read {}task_get for the current flow; CLOSED closes the task.",
                    flow.flow_code,
                    flow.statuses
                        .iter()
                        .map(|item| item.status_code.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    if context.is_owning_assistant() {
                        "assistant_"
                    } else {
                        ""
                    }
                )));
            }
        }
        if status.as_deref() == Some(crate::domain::task::CLOSED_STATUS_CODE)
            && current
                .owner_assistant_code
                .as_ref()
                .is_some_and(|_| !context.is_owning_assistant())
        {
            return Err(ToolError::ExecutionFailed(
                "Thread workers report results to their PersonAI; only the owning PersonAI may decide that the task is complete".into(),
            ));
        }

        let output = self
            .tasks
            .update_task_with_status(
                &task_code,
                UpdateTaskCommand {
                    due_at: input.due_at,
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
                status.map(|status| UpdateTaskStatusCommand {
                    status,
                    progress: input.progress,
                }),
            )
            .map_err(service_tool_error)?;
        task_observation(self.name(), &self.tasks, &output)
    }
}

#[derive(Clone)]
struct TaskDeleteTool {
    tasks: TaskService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskCodeInput {
    workspace_code: Option<String>,
    task_code: String,
}

#[derive(Clone)]
struct TaskGetTool {
    tasks: TaskService,
}
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "task_get"
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskCodeInput = parse_tool_input(request.input)?;
        let workspace = resolve_task_workspace(&input.workspace_code, context)?;
        let task = self
            .tasks
            .get_task(input.task_code.trim())
            .map_err(service_tool_error)?;
        ensure_task_access(context, workspace.as_deref(), &task)?;
        task_observation(self.name(), &self.tasks, &task)
    }
}

fn task_observation(
    name: &str,
    tasks: &TaskService,
    task: &TaskDetail,
) -> Result<ToolExecutionResult, ToolError> {
    let flow = tasks
        .get_task_flow(&task.flow_code)
        .map_err(service_tool_error)?;
    let mut value = serde_json::to_value(task).map_err(service_tool_error)?;
    value["taskFlow"] = serde_json::json!({"flowCode":flow.flow_code,"statuses":flow.statuses,"closedStatus":"CLOSED",
        "reopen":"Use task_update (PersonAI: assistant_task_update) with a valid non-CLOSED status. Omitted progress resets to 0 when reopening; archived tasks must first be restored."});
    json_observation(name, &value)
}

impl Tool for TaskDeleteTool {
    fn name(&self) -> &str {
        "task_delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_task_workspace(&input.workspace_code, context)?;
        let current = self
            .tasks
            .get_task(input.task_code.trim())
            .map_err(service_tool_error)?;
        ensure_task_access(context, workspace_code.as_deref(), &current)?;
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
        "task_restore"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_task_workspace(&input.workspace_code, context)?;
        let current = self
            .tasks
            .get_task(input.task_code.trim())
            .map_err(service_tool_error)?;
        ensure_task_access(context, workspace_code.as_deref(), &current)?;
        let output = self
            .tasks
            .restore_task(input.task_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

fn parse_task_status(value: Option<&str>) -> Result<Option<String>, ToolError> {
    Ok(
        normalize_optional_token(value).map(|value| match value.as_str() {
            "open" | "not_started" => "NOT_STARTED".to_string(),
            "completed" | "closed" => "CLOSED".to_string(),
            other => other.to_ascii_uppercase(),
        }),
    )
}

fn parse_task_priority(value: Option<&str>) -> Result<Option<TaskPriority>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("low") => Ok(Some(TaskPriority::Low)),
        Some("medium") => Ok(Some(TaskPriority::Medium)),
        Some("high") => Ok(Some(TaskPriority::High)),
        Some("extra_high" | "extra high") => Ok(Some(TaskPriority::ExtraHigh)),
        Some(other) => Err(enum_tool_error("priority", other)),
    }
}

fn parse_task_type(value: Option<&str>) -> Result<Option<TaskType>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("general") => Ok(Some(TaskType::General)),
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

fn deserialize_optional_deadline<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<i64>>, D::Error> {
    Option::<i64>::deserialize(deserializer).map(Some)
}

fn trusted_assistant_owner(context: &AgentRunContext) -> Result<Option<String>, ToolError> {
    match context
        .collaboration_owner
        .as_ref()
        .filter(|owner| owner.kind == "ASSISTANT")
    {
        Some(owner) => optional_trimmed(Some(owner.code.clone()))
            .map(Some)
            .ok_or_else(|| ToolError::ExecutionFailed("PersonAI owner context is invalid".into())),
        None => Ok(None),
    }
}

fn resolve_task_workspace(
    requested: &Option<String>,
    context: &AgentRunContext,
) -> Result<Option<String>, ToolError> {
    let authorized = optional_trimmed(Some(context.workspace_code.clone()));
    let requested = optional_trimmed(requested.clone());
    if requested.is_some() && requested != authorized {
        return Err(ToolError::ExecutionFailed(
            "workspaceCode is outside the current authorized execution scope".into(),
        ));
    }
    Ok(authorized)
}

fn ensure_task_access(
    context: &AgentRunContext,
    workspace: Option<&str>,
    task: &TaskDetail,
) -> Result<(), ToolError> {
    let owner = trusted_assistant_owner(context)?;
    if task.workspace_code.as_deref() != workspace || task.owner_assistant_code != owner {
        return Err(ToolError::ExecutionFailed(
            "task is outside the current owner or execution scope".into(),
        ));
    }
    Ok(())
}
