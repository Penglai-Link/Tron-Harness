use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::resource::{
        ResourceBinaryAgentCommand, ResourcePageQuery, ResourceService,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        resource::ResourceKind,
    },
};

use super::common::{
    json_observation, parse_tool_input, resolve_workspace_code, service_tool_error,
};

pub(super) fn tools(resources: ResourceService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ResourceQueryTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceBinaryTool { resources }),
    ]
}

#[derive(Clone)]
struct ResourceQueryTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceQueryInput {
    workspace_code: Option<String>,
    search: Option<String>,
    kind: Option<ResourceKind>,
    current: Option<usize>,
    size: Option<usize>,
}

impl Tool for ResourceQueryTool {
    fn name(&self) -> &str {
        "resource.query"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceQueryInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .query_resources(
                actor_user_code,
                ResourcePageQuery {
                    workspace_code,
                    kind: input.kind,
                    search: input.search,
                    current: input.current.unwrap_or(1),
                    size: input.size.unwrap_or(20),
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ResourceBinaryTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceBinaryInput {
    workspace_code: Option<String>,
    action: ResourceBinaryAction,
    resource_code: String,
    source_path: Option<String>,
    target_path: Option<String>,
    overwrite: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ResourceBinaryAction {
    Upload,
    Download,
}

impl Tool for ResourceBinaryTool {
    fn name(&self) -> &str {
        "resource.binary"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceBinaryInput = parse_tool_input(request.input)?;
        validate_resource_binary_input(&input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let command = ResourceBinaryAgentCommand {
            resource_code: input.resource_code,
            source_path: input.source_path,
            target_path: input.target_path,
            overwrite: input.overwrite.unwrap_or(false),
            agent_local_directory: context.execution_scope.local_directory.clone(),
        };
        let output = match input.action {
            ResourceBinaryAction::Upload => self.resources.handle_agent_binary_upload(
                actor_user_code,
                &workspace_code,
                command,
            ),
            ResourceBinaryAction::Download => self.resources.handle_agent_binary_download(
                actor_user_code,
                &workspace_code,
                command,
            ),
        }
        .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

fn required_actor(context: &AgentRunContext) -> Result<&str, ToolError> {
    context
        .user_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ToolError::ExecutionFailed("resource tools require an authenticated user".to_string())
        })
}

fn validate_resource_binary_input(input: &ResourceBinaryInput) -> Result<(), ToolError> {
    match input.action {
        ResourceBinaryAction::Upload => {
            if input.source_path.is_none() || input.target_path.is_some() {
                return Err(invalid_input("upload requires sourcePath only"));
            }
        }
        ResourceBinaryAction::Download => {
            if input.target_path.is_none() || input.source_path.is_some() {
                return Err(invalid_input("download requires targetPath only"));
            }
        }
    }
    Ok(())
}

fn invalid_input(message: &str) -> ToolError {
    ToolError::ExecutionFailed(format!("resource tool input is invalid: {message}"))
}
