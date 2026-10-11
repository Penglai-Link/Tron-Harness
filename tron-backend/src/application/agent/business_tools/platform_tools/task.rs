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
    enum_tool_error, json_observation, normalize_optional_token,
    optional_trimmed, parse_tool_input, service_tool_error,
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
                owner_dude_code: trusted_dude_owner(context)?,
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
                owner_dude_code: trusted_dude_owner(context)?,
                due_at: input.due_at,
                workspace_code,
                parent_task_code: input.parent_task_code,
                title: input.title.trim().to_string(),
                description: input.description.map(|value| value.trim().to_string()),
                priority: parse_task_priority(input.priority.as_deref())?
                    .unwrap_or(TaskPriority::Medium),
                task_type: parse_task_type(input.task_type.as_deref())?
                    .unwrap_or(if trusted_dude_owner(context)?.is_some() { TaskType::General } else { TaskType::Feature }),
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
        json_observation(self.name(), &output)
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

fn deserialize_optional_deadline<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Option<i64>>, D::Error> {
    Option::<i64>::deserialize(deserializer).map(Some)
}

fn trusted_dude_owner(context: &AgentRunContext) -> Result<Option<String>, ToolError> {
    match context.collaboration_owner.as_ref().filter(|owner| owner.kind == "DUDE") {
        Some(owner) => optional_trimmed(Some(owner.code.clone())).map(Some)
            .ok_or_else(|| ToolError::ExecutionFailed("Dude owner context is invalid".into())),
        None => Ok(None),
    }
}

fn resolve_task_workspace(requested: &Option<String>, context: &AgentRunContext) -> Result<Option<String>, ToolError> {
    let authorized = optional_trimmed(Some(context.workspace_code.clone()));
    let requested = optional_trimmed(requested.clone());
    if requested.is_some() && requested != authorized {
        return Err(ToolError::ExecutionFailed("workspaceCode is outside the current authorized execution scope".into()));
    }
    Ok(authorized)
}

fn ensure_task_access(context: &AgentRunContext, workspace: Option<&str>, task: &TaskDetail) -> Result<(), ToolError> {
    let owner = trusted_dude_owner(context)?;
    if task.workspace_code.as_deref() != workspace || task.owner_dude_code != owner {
        return Err(ToolError::ExecutionFailed("task is outside the current owner or execution scope".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::domain::agent::{AgentCollaborationOwner, AgentExecutionScope, AgentKind, AgentModelConfig, AgentModelProtocol, AgentRunLimits, AgentToolExposureState};

    fn context(owner: Option<&str>, workspace: &str) -> AgentRunContext {
        AgentRunContext {
            collaboration_owner: owner.map(|code| AgentCollaborationOwner { kind: "DUDE".into(), code: code.into(), origin_session_code: None, task_code: None }),
            agent_code: None, conversation_code: None, run_code: "run".into(), workspace_code: workspace.into(),
            user_code: None, locale: "en-US".into(), request_id: None, stream_key: None,
            agent_kind: AgentKind::React, system_agent_session_id: None,
            model: AgentModelConfig { model_code: "test".into(), protocol: AgentModelProtocol::ChatCompletions,
                base_url: String::new(), api_path: None, api_key: String::new(), model_name: "test".into(),
                thinking_enabled: false, reasoning_effort: None, responses_stateful_enabled: false,
                temperature: None, max_tokens: None, context_window_tokens: None, top_p: None, max_retries: None },
            auto_model_route: None, variables: vec![], skills: vec![], tools: vec![], effective_tools: vec![],
            tool_exposure: AgentToolExposureState::default(), execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(), messages: vec![], observations: vec![], short_term_memory: None,
            started_at: "2026-10-11T00:00:00Z".into(),
        }
    }

    fn service() -> TaskService {
        use crate::adapters::outbound::persistence::{SqliteAppRepository, SqliteTaskRepository, SqliteWorkspaceRepository};
        use crate::application::{app::AppService, workspace::WorkspaceService};
        let workspaces = WorkspaceService::new(SqliteWorkspaceRepository::connect(":memory:").unwrap());
        let apps = AppService::new(SqliteAppRepository::connect(":memory:").unwrap(), workspaces.clone());
        TaskService::new(SqliteTaskRepository::connect(":memory:").unwrap(), workspaces, apps)
    }

    fn request(name: &str, input: serde_json::Value) -> ToolRequest {
        ToolRequest { tool_name: name.into(), input, invocation_id: None }
    }

    #[test]
    fn task_tools_inject_dude_owner_and_reject_spoofed_owners_and_scopes() {
        let tasks = service();
        let creator = TaskCreateTool { tasks: tasks.clone() };
        let dude = context(Some("dude-a"), "");
        let result = creator.exec(request("task_create", json!({"title":"Report","dueAt":2_000_000_000_000_i64})), &dude).unwrap();
        let created: serde_json::Value = serde_json::from_str(&result.observation.output).unwrap();
        assert_eq!(created["ownerDudeCode"], "dude-a");
        assert_eq!(created["workspaceCode"], serde_json::Value::Null);
        assert_eq!(created["taskType"], "General");
        assert!(creator.exec(request("task_create", json!({"title":"Spoof","ownerDudeCode":"dude-b"})), &dude).is_err());
        assert!(creator.exec(request("task_create", json!({"title":"Spoof","workspaceCode":"foreign"})), &dude).is_err());
        let code = created["taskCode"].as_str().unwrap();
        let updater = TaskUpdateTool { tasks: tasks.clone() };
        for context in [context(Some("dude-b"), ""), context(None, "")] {
            assert!(updater.exec(request("task_update", json!({"taskCode":code,"title":"Hijack"})), &context).is_err());
        }
        assert!(updater.exec(request("task_update", json!({"taskCode":code,"dueAt":null})), &dude).is_ok());
        assert_eq!(tasks.get_task(code).unwrap().due_at, None);
        let archived = TaskDeleteTool { tasks: tasks.clone() };
        assert!(archived.exec(request("task_delete", json!({"taskCode":code})), &context(Some("dude-b"), "")).is_err());
        assert!(archived.exec(request("task_delete", json!({"taskCode":code})), &dude).is_ok());
        let restore = TaskRestoreTool { tasks };
        assert!(restore.exec(request("task_restore", json!({"taskCode":code})), &context(None, "")).is_err());
        assert!(restore.exec(request("task_restore", json!({"taskCode":code})), &dude).is_ok());
    }
}
