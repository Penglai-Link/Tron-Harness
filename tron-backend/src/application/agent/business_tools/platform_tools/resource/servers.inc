#[derive(Clone)]
struct ServerCreateTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerCreateInput {
    workspace_code: Option<String>,
    server_code: String,
    name: String,
    description: Option<String>,
    host: String,
    port: u16,
    allow_inheritance: bool,
    initial_account: ResourceServerAccountInput,
}

impl Tool for ServerCreateTool {
    fn name(&self) -> &str {
        "resource_server_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ServerCreateInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?.to_string();
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let resources = self.resources.clone();
        let output = run_resource_future(async move {
            resources
                .create_server_for_http(
                    &actor_user_code,
                    ServerCreateCommand {
                        workspace_code,
                        server_code: input.server_code,
                        name: input.name,
                        description: input.description.unwrap_or_default(),
                        host: input.host,
                        port: input.port,
                        allow_inheritance: input.allow_inheritance,
                        initial_account: Some(input.initial_account.into_command()),
                    },
                )
                .await
        })?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ServerAccountCreateTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerAccountCreateToolInput {
    workspace_code: Option<String>,
    server_code: String,
    account: ResourceServerAccountInput,
}

impl Tool for ServerAccountCreateTool {
    fn name(&self) -> &str {
        "resource_server_account_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ServerAccountCreateToolInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .create_server_account_for_http(
                actor_user_code,
                ServerAccountCreateCommand {
                    workspace_code,
                    server_code: input.server_code,
                    account: input.account.into_command(),
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceServerAccountInput {
    account_code: String,
    name: String,
    description: Option<String>,
    username: String,
    authentication_type: ServerAuthenticationType,
    password_variable_key: Option<String>,
    private_key_variable_key: Option<String>,
    private_key_passphrase_variable_key: Option<String>,
    allow_inheritance: bool,
}

impl ResourceServerAccountInput {
    fn into_command(self) -> ServerAccountCreateInput {
        ServerAccountCreateInput {
            account_code: self.account_code,
            name: self.name,
            description: self.description.unwrap_or_default(),
            username: self.username,
            authentication_type: self.authentication_type,
            password_variable_key: self.password_variable_key,
            private_key_variable_key: self.private_key_variable_key,
            private_key_passphrase_variable_key: self.private_key_passphrase_variable_key,
            allow_inheritance: self.allow_inheritance,
        }
    }
}
