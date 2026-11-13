use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::resource::{
        ResourceBinaryAgentCommand, ResourceCreateCommand, ResourceDeleteCommand, ResourceGetQuery,
        ResourcePageQuery, ResourceService, ResourceUpdateCommand,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        resource::ResourceKind,
    },
};

use super::common::{json_observation, parse_tool_input, service_tool_error};

const PLATFORM_SCOPE: &str = "PLATFORM";

pub(super) fn tools(resources: ResourceService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(PlatformResourceQueryTool {
            resources: resources.clone(),
        }),
        Arc::new(PlatformResourceGetTool {
            resources: resources.clone(),
        }),
        Arc::new(PlatformResourceCreateTool {
            resources: resources.clone(),
        }),
        Arc::new(PlatformResourceUpdateTool {
            resources: resources.clone(),
        }),
        Arc::new(PlatformResourceDeleteTool {
            resources: resources.clone(),
        }),
        Arc::new(PlatformResourceBinaryTool { resources }),
    ]
}

#[derive(Clone)]
struct PlatformResourceQueryTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformResourceQueryInput {
    search: Option<String>,
    kind: Option<ResourceKind>,
    current: Option<usize>,
    size: Option<usize>,
}

impl Tool for PlatformResourceQueryTool {
    fn name(&self) -> &str {
        "platform_resource_query"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformResourceQueryInput = parse_tool_input(request.input)?;
        let actor = required_actor(context)?;
        let output = self
            .resources
            .for_agent_in_workspace(
                context.agent_code.as_deref(),
                context.execution_workspace_code(),
            )
            .query_resources(
                actor,
                ResourcePageQuery {
                    workspace_code: PLATFORM_SCOPE.to_string(),
                    kinds: input.kind.into_iter().collect(),
                    source_kind: None,
                    search: input.search,
                    current: input.current.unwrap_or(1).max(1),
                    size: input.size.unwrap_or(20).clamp(1, 50),
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformResourceGetTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformResourceCodeInput {
    resource_code: String,
}

impl Tool for PlatformResourceGetTool {
    fn name(&self) -> &str {
        "platform_resource_get"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformResourceCodeInput = parse_tool_input(request.input)?;
        let output = self
            .resources
            .for_agent_in_workspace(
                context.agent_code.as_deref(),
                context.execution_workspace_code(),
            )
            .get_resource(
                required_actor(context)?,
                ResourceGetQuery {
                    workspace_code: PLATFORM_SCOPE.to_string(),
                    resource_code: input.resource_code,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformResourceCreateTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformResourceCreateInput {
    resource_code: String,
    kind: ResourceKind,
    name: String,
    description: Option<String>,
    source_path: String,
}

impl Tool for PlatformResourceCreateTool {
    fn name(&self) -> &str {
        "platform_resource_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformResourceCreateInput = parse_tool_input(request.input)?;
        let output = self
            .resources
            .for_agent_in_workspace(
                context.agent_code.as_deref(),
                context.execution_workspace_code(),
            )
            .create_file_resource_from_agent_path(
                required_actor(context)?,
                ResourceCreateCommand {
                    workspace_code: PLATFORM_SCOPE.to_string(),
                    resource_code: input.resource_code,
                    kind: input.kind,
                    name: input.name,
                    description: input.description.unwrap_or_default(),
                    allow_inheritance: false,
                },
                &input.source_path,
                context.execution_scope.local_directory.as_deref(),
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformResourceUpdateTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformResourceUpdateInput {
    resource_code: String,
    name: Option<String>,
    description: Option<String>,
}

impl Tool for PlatformResourceUpdateTool {
    fn name(&self) -> &str {
        "platform_resource_update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformResourceUpdateInput = parse_tool_input(request.input)?;
        let output = self
            .resources
            .for_agent_in_workspace(
                context.agent_code.as_deref(),
                context.execution_workspace_code(),
            )
            .update_resource(
                required_actor(context)?,
                ResourceUpdateCommand {
                    workspace_code: PLATFORM_SCOPE.to_string(),
                    resource_code: input.resource_code,
                    name: input.name,
                    description: input.description,
                    allow_inheritance: None,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformResourceDeleteTool {
    resources: ResourceService,
}

impl Tool for PlatformResourceDeleteTool {
    fn name(&self) -> &str {
        "platform_resource_delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformResourceCodeInput = parse_tool_input(request.input)?;
        let output = self
            .resources
            .for_agent_in_workspace(
                context.agent_code.as_deref(),
                context.execution_workspace_code(),
            )
            .delete_resource(
                required_actor(context)?,
                ResourceDeleteCommand {
                    workspace_code: PLATFORM_SCOPE.to_string(),
                    resource_code: input.resource_code,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformResourceBinaryTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformResourceBinaryInput {
    action: PlatformResourceBinaryAction,
    resource_code: String,
    source_path: Option<String>,
    target_path: Option<String>,
    overwrite: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PlatformResourceBinaryAction {
    Upload,
    Download,
}

impl Tool for PlatformResourceBinaryTool {
    fn name(&self) -> &str {
        "platform_resource_binary"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformResourceBinaryInput = parse_tool_input(request.input)?;
        let actor = required_actor(context)?;
        let command = ResourceBinaryAgentCommand {
            resource_code: input.resource_code,
            source_path: input.source_path,
            target_path: input.target_path,
            overwrite: input.overwrite.unwrap_or(false),
            agent_local_directory: context.execution_scope.local_directory.clone(),
        };
        let output = match input.action {
            PlatformResourceBinaryAction::Upload => self
                .resources
                .for_agent_in_workspace(
                    context.agent_code.as_deref(),
                    context.execution_workspace_code(),
                )
                .handle_agent_binary_upload(actor, PLATFORM_SCOPE, command),
            PlatformResourceBinaryAction::Download => self
                .resources
                .for_agent_in_workspace(
                    context.agent_code.as_deref(),
                    context.execution_workspace_code(),
                )
                .handle_agent_binary_download(actor, PLATFORM_SCOPE, command),
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
            ToolError::ExecutionFailed(
                "platform resource tools require an authenticated user".into(),
            )
        })
}
