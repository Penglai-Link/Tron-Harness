use std::{future::Future, sync::Arc, thread};

use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::resource::{
        DataSourceCreateCommand, DataSourceUpdateCommand, ResourceBinaryAgentCommand,
        ResourceCreateCommand, ResourceDeleteCommand, ResourceGetQuery, ResourcePageQuery,
        ResourceService, ResourceUpdateCommand, ServerAccountCreateCommand,
        ServerAccountCreateInput, ServerCreateCommand,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        resource::{ResourceError, ResourceKind, ServerAuthenticationType},
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
        Arc::new(ResourceGetTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceCreateTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceUpdateTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceDeleteTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceBinaryTool {
            resources: resources.clone(),
        }),
        Arc::new(DataSourceCreateTool {
            resources: resources.clone(),
        }),
        Arc::new(DataSourceUpdateTool {
            resources: resources.clone(),
        }),
        Arc::new(ServerCreateTool {
            resources: resources.clone(),
        }),
        Arc::new(ServerAccountCreateTool { resources }),
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
        "resource.get"
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
        "resource.create"
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
        "resource.update"
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
        "resource.delete"
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
        "resource.data_source.create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: DataSourceCreateInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?.to_string();
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let resources = self.resources.clone();
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
        "resource.data_source.update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: DataSourceUpdateInput = parse_tool_input(request.input)?;
        let actor_user_code = required_actor(context)?.to_string();
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let resources = self.resources.clone();
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
        "resource.server.create"
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
        "resource.server_account.create"
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

fn run_resource_future<F, T>(future: F) -> Result<T, ToolError>
where
    F: Future<Output = Result<T, ResourceError>> + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?
            .block_on(future)
            .map_err(service_tool_error)
    })
    .join()
    .map_err(|_| ToolError::ExecutionFailed("resource tool worker panicked".into()))?
}
