
#[derive(Clone)]
struct ResourceDeleteTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceDeleteInput {
    workspace_code: Option<String>,
    resource_code: String,
}

impl Tool for ResourceDeleteTool {
    fn name(&self) -> &str {
        "resource_delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceDeleteInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .delete_resource(
                actor_user_code,
                ResourceDeleteCommand {
                    workspace_code,
                    resource_code: input.resource_code,
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
        "resource_binary"
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
            ResourceBinaryAction::Upload => {
                self.resources
                    .handle_agent_binary_upload(actor_user_code, &workspace_code, command)
            }
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
