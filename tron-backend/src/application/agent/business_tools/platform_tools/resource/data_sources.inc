#[derive(Clone)]
struct DataSourceCreateTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DataSourceCreateInput {
    workspace_code: Option<String>,
    resource_code: String,
    kind: ResourceKind,
    name: String,
    description: Option<String>,
    attributes: Value,
    allow_inheritance: bool,
}

impl Tool for DataSourceCreateTool {
    fn name(&self) -> &str {
        "resource_data_source_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: DataSourceCreateInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?.to_string();
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let resources = self.resources.for_agent_in_workspace(context.agent_code.as_deref(),&context.workspace_code);
        let output = run_resource_future(async move {
            resources
                .create_data_source_for_http(
                    &actor_user_code,
                    DataSourceCreateCommand {
                        workspace_code,
                        resource_code: input.resource_code,
                        kind: input.kind,
                        name: input.name,
                        description: input.description.unwrap_or_default(),
                        attributes: input.attributes,
                        allow_inheritance: input.allow_inheritance,
                    },
                )
                .await
        })?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct DataSourceUpdateTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DataSourceUpdateInput {
    workspace_code: Option<String>,
    resource_code: String,
    name: Option<String>,
    description: Option<String>,
    attributes: Option<Value>,
    allow_inheritance: Option<bool>,
}

impl Tool for DataSourceUpdateTool {
    fn name(&self) -> &str {
        "resource_data_source_update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: DataSourceUpdateInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?.to_string();
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let resources = self.resources.for_agent_in_workspace(context.agent_code.as_deref(),&context.workspace_code);
        let output = run_resource_future(async move {
            resources
                .update_data_source_for_http(
                    &actor_user_code,
                    DataSourceUpdateCommand {
                        workspace_code,
                        resource_code: input.resource_code,
                        name: input.name,
                        description: input.description,
                        attributes: input.attributes,
                        allow_inheritance: input.allow_inheritance,
                    },
                )
                .await
        })?;
        json_observation(self.name(), &output)
    }
}
