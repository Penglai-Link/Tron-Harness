use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::app::{AppService, CreateAppCommand, UpdateAppCommand},
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        app::{AppListQuery, AppRuntimeRole},
    },
};

use super::common::{
    ensure_workspace_match, enum_tool_error, json_observation, normalize_optional_token,
    parse_tool_input, resolve_workspace_code, service_tool_error,
};

pub(super) fn tools(apps: AppService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(AppPageTool { apps: apps.clone() }),
        Arc::new(AppCreateTool { apps: apps.clone() }),
        Arc::new(AppUpdateTool { apps: apps.clone() }),
        Arc::new(AppDeleteTool { apps: apps.clone() }),
        Arc::new(AppRestoreTool { apps }),
    ]
}

#[derive(Clone)]
struct AppPageTool {
    apps: AppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppPageInput {
    workspace_code: Option<String>,
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
    include_archived: Option<bool>,
}

impl Tool for AppPageTool {
    fn name(&self) -> &str {
        "app.page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: AppPageInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .apps
            .list_apps(
                workspace_code.as_str(),
                AppListQuery {
                    current: input.current,
                    size: input.size,
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
struct AppCreateTool {
    apps: AppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppCreateInput {
    workspace_code: Option<String>,
    name: String,
    description: Option<String>,
    local_directory: String,
    runtime_role: String,
    repository_url: Option<String>,
    default_branch: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

impl Tool for AppCreateTool {
    fn name(&self) -> &str {
        "app.create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: AppCreateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .apps
            .create_app(
                workspace_code.as_str(),
                CreateAppCommand {
                    name: input.name,
                    description: input.description,
                    local_directory: input.local_directory,
                    runtime_role: parse_app_runtime_role(input.runtime_role.as_str())?,
                    repository_url: input.repository_url,
                    default_branch: input.default_branch,
                    tags: input.tags,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct AppUpdateTool {
    apps: AppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppUpdateInput {
    workspace_code: Option<String>,
    app_code: String,
    name: Option<String>,
    description: Option<String>,
    local_directory: Option<String>,
    runtime_role: Option<String>,
    repository_url: Option<String>,
    default_branch: Option<String>,
    tags: Option<Vec<String>>,
}

impl Tool for AppUpdateTool {
    fn name(&self) -> &str {
        "app.update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: AppUpdateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let current = self
            .apps
            .get_app(input.app_code.trim())
            .map_err(service_tool_error)?;
        ensure_workspace_match(&workspace_code, &current.workspace_code, "app")?;
        let runtime_role = match input.runtime_role {
            Some(role) => parse_app_runtime_role(role.as_str())?,
            None => current.runtime_role,
        };
        let output = self
            .apps
            .update_app(
                input.app_code.trim(),
                UpdateAppCommand {
                    name: input.name.unwrap_or(current.name),
                    description: input.description.or(current.description),
                    local_directory: input.local_directory.unwrap_or(current.local_directory),
                    runtime_role,
                    repository_url: input.repository_url.or(current.repository_url),
                    default_branch: input.default_branch.or(current.default_branch),
                    tags: input.tags.unwrap_or(current.tags),
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct AppDeleteTool {
    apps: AppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppCodeInput {
    workspace_code: Option<String>,
    app_code: String,
}

impl Tool for AppDeleteTool {
    fn name(&self) -> &str {
        "app.delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: AppCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let current = self
            .apps
            .get_app(input.app_code.trim())
            .map_err(service_tool_error)?;
        ensure_workspace_match(&workspace_code, &current.workspace_code, "app")?;
        let output = self
            .apps
            .archive_app(input.app_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct AppRestoreTool {
    apps: AppService,
}

impl Tool for AppRestoreTool {
    fn name(&self) -> &str {
        "app.restore"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: AppCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let current = self
            .apps
            .get_app(input.app_code.trim())
            .map_err(service_tool_error)?;
        ensure_workspace_match(&workspace_code, &current.workspace_code, "app")?;
        let output = self
            .apps
            .restore_app(input.app_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

fn parse_app_runtime_role(value: &str) -> Result<AppRuntimeRole, ToolError> {
    match normalize_optional_token(Some(value)).as_deref() {
        Some("frontend") => Ok(AppRuntimeRole::Frontend),
        Some("backend") => Ok(AppRuntimeRole::Backend),
        Some(other) => Err(enum_tool_error("runtimeRole", other)),
        None => Err(ToolError::ExecutionFailed(
            "runtimeRole is required".to_string(),
        )),
    }
}
