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
        "resource_query"
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
            .for_agent_in_workspace(context.agent_code.as_deref(),&context.workspace_code)
            .query_resources(
                actor_user_code,
                ResourcePageQuery {
                    workspace_code,
                    kinds: input.kind.into_iter().collect(),
                    source_kind: None,
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
struct ResourceGetTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceGetInput {
    workspace_code: Option<String>,
    resource_code: String,
}

impl Tool for ResourceGetTool {
    fn name(&self) -> &str {
        "resource_get"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceGetInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .for_agent_in_workspace(context.agent_code.as_deref(),&context.workspace_code)
            .get_resource(
                actor_user_code,
                ResourceGetQuery {
                    workspace_code,
                    resource_code: input.resource_code,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ResourceCreateTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceCreateInput {
    workspace_code: Option<String>,
    resource_code: String,
    kind: ResourceKind,
    name: String,
    description: Option<String>,
    allow_inheritance: bool,
    source_path: String,
}

impl Tool for ResourceCreateTool {
    fn name(&self) -> &str {
        "resource_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceCreateInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .for_agent_in_workspace(context.agent_code.as_deref(),&context.workspace_code)
            .create_file_resource_from_agent_path(
                actor_user_code,
                ResourceCreateCommand {
                    workspace_code,
                    resource_code: input.resource_code,
                    kind: input.kind,
                    name: input.name,
                    description: input.description.unwrap_or_default(),
                    allow_inheritance: input.allow_inheritance,
                },
                &input.source_path,
                context.execution_scope.local_directory.as_deref(),
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ResourceUpdateTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceUpdateInput {
    workspace_code: Option<String>,
    resource_code: String,
    name: Option<String>,
    description: Option<String>,
    allow_inheritance: Option<bool>,
}

impl Tool for ResourceUpdateTool {
    fn name(&self) -> &str {
        "resource_update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceUpdateInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .for_agent_in_workspace(context.agent_code.as_deref(),&context.workspace_code)
            .update_resource(
                actor_user_code,
                ResourceUpdateCommand {
                    workspace_code,
                    resource_code: input.resource_code,
                    name: input.name,
                    description: input.description,
                    allow_inheritance: input.allow_inheritance,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}
