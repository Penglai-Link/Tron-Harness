use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    application::{
        app::AppService, model::ModelService, skill::SkillRegistryService, task::CreateTaskCommand,
        task::TaskService, variable::VariableService, workspace::WorkspaceService,
    },
    domain::{
        agent::{
            AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolObservation, ToolRequest,
        },
        app::AppListQuery,
        skill::WorkspaceSkillListQuery,
        task::{TaskListQuery, TaskPriority, TaskSourceType, TaskStatus, TaskType},
        variable::VariableCatalogQuery,
        workspace::{WorkspaceListQuery, WorkspaceStatus},
    },
};

pub(super) fn platform_business_tools(
    workspaces: WorkspaceService,
    apps: AppService,
    tasks: TaskService,
    models: ModelService,
    skills: SkillRegistryService,
    variables: VariableService,
) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(WorkspaceListTool {
            workspaces: workspaces.clone(),
        }),
        Arc::new(WorkspaceGetTool {
            workspaces: workspaces.clone(),
        }),
        Arc::new(AppListTool { apps: apps.clone() }),
        Arc::new(TaskListTool {
            tasks: tasks.clone(),
        }),
        Arc::new(TaskCreateTool {
            tasks: tasks.clone(),
        }),
        Arc::new(ModelListTool {
            models: models.clone(),
        }),
        Arc::new(SkillListTool {
            skills: skills.clone(),
        }),
        Arc::new(SkillViewTool {
            skills: skills.clone(),
        }),
        Arc::new(VariableListTool {
            variables: variables.clone(),
        }),
    ]
}

#[derive(Clone)]
struct WorkspaceListTool {
    workspaces: WorkspaceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceListInput {
    include_archived: Option<bool>,
    search: Option<String>,
    root_only: Option<bool>,
}

impl Tool for WorkspaceListTool {
    fn name(&self) -> &str {
        "workspace.list"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: WorkspaceListInput = parse_tool_input(request.input)?;
        let include_archived = input.include_archived.unwrap_or(false);
        let output = self
            .workspaces
            .list_workspaces(WorkspaceListQuery {
                include_archived: Some(include_archived),
                search: input.search,
                status: (!include_archived).then_some(WorkspaceStatus::Active),
                root_only: input.root_only,
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct WorkspaceGetTool {
    workspaces: WorkspaceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceGetInput {
    workspace_code: String,
}

impl Tool for WorkspaceGetTool {
    fn name(&self) -> &str {
        "workspace.get"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: WorkspaceGetInput = parse_tool_input(request.input)?;
        let output = self
            .workspaces
            .get_workspace(input.workspace_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct AppListTool {
    apps: AppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppListInput {
    workspace_code: Option<String>,
    search: Option<String>,
    include_archived: Option<bool>,
}

impl Tool for AppListTool {
    fn name(&self) -> &str {
        "app.list"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: AppListInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .apps
            .list_apps(
                workspace_code.as_str(),
                AppListQuery {
                    current: Some(1),
                    size: Some(50),
                    search: input.search,
                    runtime_role: None,
                    status: None,
                    include_archived: input.include_archived,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct TaskListTool {
    tasks: TaskService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskListInput {
    workspace_code: Option<String>,
    search: Option<String>,
    status: Option<String>,
}

impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "task.list"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: TaskListInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .tasks
            .list_tasks(TaskListQuery {
                workspace_code: Some(workspace_code),
                current: Some(1),
                size: Some(50),
                search: input.search,
                status: parse_task_status(input.status.as_deref())?,
                priority: None,
                task_type: None,
                source_type: None,
                assignee_user_code: None,
                parent_task_code: None,
                include_archived: Some(false),
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
    title: String,
    description: Option<String>,
    priority: Option<String>,
    task_type: Option<String>,
    source_type: Option<String>,
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
        let output = self
            .tasks
            .create_task(CreateTaskCommand {
                workspace_code,
                parent_task_code: None,
                title: input.title.trim().to_string(),
                description: input.description.map(|value| value.trim().to_string()),
                priority: parse_task_priority(input.priority.as_deref())?
                    .unwrap_or(TaskPriority::Medium),
                task_type: parse_task_type(input.task_type.as_deref())?
                    .unwrap_or(TaskType::Feature),
                source_type: parse_task_source_type(input.source_type.as_deref())?
                    .unwrap_or(TaskSourceType::Internal),
                source_url: None,
                assignee_user_codes: Vec::new(),
                agent_codes: Vec::new(),
                app_codes: input
                    .app_code
                    .map(|code| vec![code.trim().to_string()])
                    .unwrap_or_default(),
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ModelListTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceScopedInput {
    workspace_code: Option<String>,
}

impl Tool for ModelListTool {
    fn name(&self) -> &str {
        "model.list"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: WorkspaceScopedInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .models
            .list_workspace_models(workspace_code.as_str())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct SkillListTool {
    skills: SkillRegistryService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillListInput {
    workspace_code: Option<String>,
    search: Option<String>,
    include_disabled: Option<bool>,
}

impl Tool for SkillListTool {
    fn name(&self) -> &str {
        "skill.list"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: SkillListInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let mut output = self
            .skills
            .list_workspace_skills(
                workspace_code.as_str(),
                WorkspaceSkillListQuery {
                    search: input.search,
                },
            )
            .map_err(service_tool_error)?;
        if !input.include_disabled.unwrap_or(false) {
            output.retain(|item| item.enabled);
        }
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct SkillViewTool {
    skills: SkillRegistryService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillViewInput {
    workspace_code: Option<String>,
    skill_name: String,
}

impl Tool for SkillViewTool {
    fn name(&self) -> &str {
        "skill.view"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: SkillViewInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let skill_name = input.skill_name.trim();
        if skill_name.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "skillName is required".to_string(),
            ));
        }
        let output = self
            .skills
            .get_workspace_skill(workspace_code.as_str(), skill_name)
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct VariableListTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariableListInput {
    workspace_code: Option<String>,
    search: Option<String>,
}

impl Tool for VariableListTool {
    fn name(&self) -> &str {
        "variable.list"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: VariableListInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .variables
            .list_variables(
                workspace_code.as_str(),
                VariableCatalogQuery {
                    search: input.search,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

fn parse_tool_input<T: for<'de> Deserialize<'de>>(
    input: serde_json::Value,
) -> Result<T, ToolError> {
    serde_json::from_value(input)
        .map_err(|error| ToolError::ExecutionFailed(format!("tool input is invalid: {error}")))
}

fn json_observation<T: Serialize>(
    tool_name: &str,
    value: &T,
) -> Result<ToolExecutionResult, ToolError> {
    let output = serde_json::to_string(value).map_err(service_tool_error)?;
    Ok(ToolExecutionResult::observation(ToolObservation {
        tool_name: tool_name.to_string(),
        output,
    }))
}

fn resolve_workspace_code(
    requested_workspace_code: &Option<String>,
    context: &AgentRunContext,
) -> Result<String, ToolError> {
    let workspace_code = requested_workspace_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(context.workspace_code.as_str());
    if workspace_code.trim().is_empty() {
        return Err(ToolError::ExecutionFailed(
            "workspaceCode is required".to_string(),
        ));
    }
    Ok(workspace_code.to_string())
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

fn normalize_optional_token(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase().replace('-', "_"))
}

fn enum_tool_error(field_name: &str, value: &str) -> ToolError {
    ToolError::ExecutionFailed(format!(
        "tool input field `{field_name}` has unsupported value `{value}`"
    ))
}

fn service_tool_error(error: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
