use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::resource::{
        ResourceBinaryAction, ResourceBinaryCommand, ResourceDefinitionDraftAction,
        ResourceDefinitionDraftCommand, ResourceDefinitionPublishCommand, ResourceDefinitionQuery,
        ResourceDraftAction, ResourceDraftCommand, ResourcePublishCommand, ResourceQuery,
        ResourceService,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        resource::{ResourceDefinitionStatus, ResourceLifecycleStatus, ResourceScopeKind},
    },
};

use super::common::{
    json_observation, parse_tool_input, resolve_workspace_code, service_tool_error,
};

pub(super) fn tools(resources: ResourceService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ResourceDefinitionQueryTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceDefinitionDraftTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceDefinitionPublishTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceQueryTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceDraftTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourcePublishTool {
            resources: resources.clone(),
        }),
        Arc::new(ResourceBinaryTool { resources }),
    ]
}

#[derive(Clone)]
struct ResourceDefinitionQueryTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceDefinitionQueryInput {
    scope_kind: ResourceScopeKind,
    workspace_code: Option<String>,
    search: Option<String>,
    definition_code: Option<String>,
    definition_version: Option<String>,
    definition_status: Option<ResourceDefinitionStatus>,
    lifecycle_status: Option<ResourceLifecycleStatus>,
    include_history: Option<bool>,
    current: Option<usize>,
    size: Option<usize>,
}

impl Tool for ResourceDefinitionQueryTool {
    fn name(&self) -> &str {
        "resource_definition.query"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceDefinitionQueryInput = parse_tool_input(request.input)?;
        validate_definition_query_input(&input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code =
            resolve_definition_workspace(input.scope_kind, &input.workspace_code, context)?;
        let output = self
            .resources
            .query_definitions(
                actor_user_code,
                ResourceDefinitionQuery {
                    scope_kind: input.scope_kind,
                    workspace_code,
                    search: input.search,
                    definition_code: input.definition_code,
                    definition_version: input.definition_version,
                    definition_status: input.definition_status,
                    lifecycle_status: input.lifecycle_status,
                    include_history: input.include_history.unwrap_or(false),
                    current: input.current.unwrap_or(1),
                    size: input.size.unwrap_or(20),
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ResourceDefinitionDraftTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceDefinitionDraftInput {
    scope_kind: ResourceScopeKind,
    workspace_code: Option<String>,
    action: ResourceDefinitionDraftAction,
    definition_code: String,
    definition_version: Option<String>,
    expected_draft_revision: Option<i64>,
    name: Option<String>,
    description: Option<String>,
    definition_status: Option<ResourceDefinitionStatus>,
    allow_inheritance: Option<bool>,
    attributes_schema: Option<Value>,
    binary_schema: Option<Value>,
    binary_storage_profile: Option<String>,
}

impl Tool for ResourceDefinitionDraftTool {
    fn name(&self) -> &str {
        "resource_definition.draft"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceDefinitionDraftInput = parse_tool_input(request.input)?;
        validate_definition_draft_input(&input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code =
            resolve_definition_workspace(input.scope_kind, &input.workspace_code, context)?;
        let output = self
            .resources
            .draft_definition(
                actor_user_code,
                ResourceDefinitionDraftCommand {
                    scope_kind: input.scope_kind,
                    workspace_code,
                    action: input.action,
                    definition_code: input.definition_code,
                    definition_version: input.definition_version,
                    expected_draft_revision: input.expected_draft_revision,
                    name: input.name,
                    description: input.description,
                    definition_status: input.definition_status,
                    allow_inheritance: input.allow_inheritance,
                    attributes_schema: input.attributes_schema,
                    binary_schema: input.binary_schema,
                    binary_storage_profile: input.binary_storage_profile,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ResourceDefinitionPublishTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceDefinitionPublishInput {
    scope_kind: ResourceScopeKind,
    workspace_code: Option<String>,
    definition_code: String,
    definition_version: String,
    expected_draft_revision: Option<i64>,
    expected_active_definition_version: Option<String>,
}

impl Tool for ResourceDefinitionPublishTool {
    fn name(&self) -> &str {
        "resource_definition.publish"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceDefinitionPublishInput = parse_tool_input(request.input)?;
        if input.expected_draft_revision.is_some()
            && input.expected_active_definition_version.is_some()
        {
            return Err(invalid_input(
                "publish expects either draft or active version concurrency data",
            ));
        }
        let actor_user_code = required_actor(context)?;
        let workspace_code =
            resolve_definition_workspace(input.scope_kind, &input.workspace_code, context)?;
        let output = self
            .resources
            .publish_definition(
                actor_user_code,
                ResourceDefinitionPublishCommand {
                    scope_kind: input.scope_kind,
                    workspace_code,
                    definition_code: input.definition_code,
                    definition_version: input.definition_version,
                    expected_draft_revision: input.expected_draft_revision,
                    expected_active_definition_version: input.expected_active_definition_version,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
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
    definition_code: Option<String>,
    resource_code: Option<String>,
    resource_version: Option<String>,
    include_history: Option<bool>,
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
        validate_resource_query_input(&input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .query_resources(
                actor_user_code,
                ResourceQuery {
                    workspace_code: Some(workspace_code),
                    search: input.search,
                    definition_code: input.definition_code,
                    resource_code: input.resource_code,
                    resource_version: input.resource_version,
                    include_history: input.include_history.unwrap_or(false),
                    current: input.current.unwrap_or(1),
                    size: input.size.unwrap_or(20),
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ResourceDraftTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceDraftInput {
    workspace_code: Option<String>,
    action: ResourceDraftAction,
    resource_code: String,
    resource_version: Option<String>,
    expected_draft_revision: Option<i64>,
    definition_code: Option<String>,
    name: Option<String>,
    description: Option<String>,
    attributes: Option<Value>,
    allow_inheritance: Option<bool>,
}

impl Tool for ResourceDraftTool {
    fn name(&self) -> &str {
        "resource.draft"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourceDraftInput = parse_tool_input(request.input)?;
        validate_resource_draft_input(&input)?;
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .draft_resource(
                actor_user_code,
                ResourceDraftCommand {
                    action: input.action,
                    workspace_code: Some(workspace_code),
                    resource_code: input.resource_code,
                    resource_version: input.resource_version,
                    expected_draft_revision: input.expected_draft_revision,
                    definition_code: input.definition_code,
                    name: input.name,
                    description: input.description,
                    attributes: input.attributes,
                    allow_inheritance: input.allow_inheritance,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ResourcePublishTool {
    resources: ResourceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourcePublishInput {
    workspace_code: Option<String>,
    resource_code: String,
    resource_version: String,
    expected_draft_revision: Option<i64>,
    expected_active_resource_version: Option<String>,
}

impl Tool for ResourcePublishTool {
    fn name(&self) -> &str {
        "resource.publish"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ResourcePublishInput = parse_tool_input(request.input)?;
        if input.expected_draft_revision.is_some()
            && input.expected_active_resource_version.is_some()
        {
            return Err(invalid_input(
                "publish expects either draft or active version concurrency data",
            ));
        }
        let actor_user_code = required_actor(context)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .resources
            .publish_resource(
                actor_user_code,
                ResourcePublishCommand {
                    workspace_code: Some(workspace_code),
                    resource_code: input.resource_code,
                    resource_version: input.resource_version,
                    expected_draft_revision: input.expected_draft_revision,
                    expected_active_resource_version: input.expected_active_resource_version,
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
    resource_version: Option<String>,
    expected_draft_revision: Option<i64>,
    binary_name: String,
    source_path: Option<String>,
    target_path: Option<String>,
    overwrite: Option<bool>,
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
        let output = self
            .resources
            .handle_resource_binary(
                actor_user_code,
                ResourceBinaryCommand {
                    action: input.action,
                    workspace_code: Some(workspace_code),
                    resource_code: input.resource_code,
                    resource_version: input.resource_version,
                    expected_draft_revision: input.expected_draft_revision,
                    binary_name: input.binary_name,
                    source_path: input.source_path,
                    target_path: input.target_path,
                    overwrite: input.overwrite.unwrap_or(false),
                    agent_local_directory: context.execution_scope.local_directory.clone(),
                },
            )
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

fn resolve_definition_workspace(
    scope_kind: ResourceScopeKind,
    requested_workspace_code: &Option<String>,
    context: &AgentRunContext,
) -> Result<Option<String>, ToolError> {
    match scope_kind {
        ResourceScopeKind::Platform => {
            if requested_workspace_code.is_some() {
                return Err(invalid_input(
                    "workspaceCode is not allowed for PLATFORM scope",
                ));
            }
            Ok(None)
        }
        ResourceScopeKind::Workspace => Ok(Some(resolve_workspace_code(
            requested_workspace_code,
            context,
        )?)),
    }
}

fn validate_definition_draft_input(input: &ResourceDefinitionDraftInput) -> Result<(), ToolError> {
    match input.action {
        ResourceDefinitionDraftAction::Create => {
            if input.definition_version.is_some() || input.expected_draft_revision.is_some() {
                return Err(invalid_input(
                    "create does not accept definitionVersion or expectedDraftRevision",
                ));
            }
            if input
                .name
                .as_deref()
                .is_none_or(|name| name.trim().is_empty())
            {
                return Err(invalid_input("create requires a non-blank name"));
            }
        }
        ResourceDefinitionDraftAction::Update => {
            if input.definition_version.is_none() {
                return Err(invalid_input("update requires definitionVersion"));
            }
            if input.name.is_none()
                && input.description.is_none()
                && input.definition_status.is_none()
                && input.allow_inheritance.is_none()
                && input.attributes_schema.is_none()
                && input.binary_schema.is_none()
                && input.binary_storage_profile.is_none()
            {
                return Err(invalid_input("update requires definition content"));
            }
        }
        ResourceDefinitionDraftAction::Discard => {
            if input.definition_version.is_none() || input.expected_draft_revision.is_none() {
                return Err(invalid_input(
                    "discard requires definitionVersion and expectedDraftRevision",
                ));
            }
            if input.name.is_some()
                || input.description.is_some()
                || input.definition_status.is_some()
                || input.allow_inheritance.is_some()
                || input.attributes_schema.is_some()
                || input.binary_schema.is_some()
                || input.binary_storage_profile.is_some()
            {
                return Err(invalid_input(
                    "discard does not accept definition content fields",
                ));
            }
        }
    }
    Ok(())
}

fn validate_resource_draft_input(input: &ResourceDraftInput) -> Result<(), ToolError> {
    match input.action {
        ResourceDraftAction::Create => {
            if input.definition_code.is_none()
                || input.resource_version.is_some()
                || input.expected_draft_revision.is_some()
            {
                return Err(invalid_input(
                    "create requires definitionCode and no version concurrency fields",
                ));
            }
            if input
                .name
                .as_deref()
                .is_none_or(|name| name.trim().is_empty())
            {
                return Err(invalid_input("create requires a non-blank name"));
            }
        }
        ResourceDraftAction::Update => {
            if input.resource_version.is_none() {
                return Err(invalid_input("update requires resourceVersion"));
            }
            if input.definition_code.is_some() {
                return Err(invalid_input("update does not accept definitionCode"));
            }
        }
        ResourceDraftAction::Discard => {
            if input.resource_version.is_none() || input.expected_draft_revision.is_none() {
                return Err(invalid_input(
                    "discard requires resourceVersion and expectedDraftRevision",
                ));
            }
            if input.definition_code.is_some()
                || input.name.is_some()
                || input.description.is_some()
                || input.attributes.is_some()
                || input.allow_inheritance.is_some()
            {
                return Err(invalid_input(
                    "discard does not accept resource content fields",
                ));
            }
        }
    }
    Ok(())
}

fn validate_definition_query_input(input: &ResourceDefinitionQueryInput) -> Result<(), ToolError> {
    if input.definition_version.is_some() && input.definition_code.is_none() {
        return Err(invalid_input("definitionVersion requires definitionCode"));
    }
    if input.definition_version.is_some() && input.include_history == Some(true) {
        return Err(invalid_input(
            "definitionVersion cannot be combined with includeHistory",
        ));
    }
    if input.lifecycle_status.is_some() && input.include_history != Some(true) {
        return Err(invalid_input("lifecycleStatus requires includeHistory"));
    }
    Ok(())
}

fn validate_resource_query_input(input: &ResourceQueryInput) -> Result<(), ToolError> {
    if input.resource_version.is_some() && input.resource_code.is_none() {
        return Err(invalid_input("resourceVersion requires resourceCode"));
    }
    if input.resource_version.is_some() && input.include_history == Some(true) {
        return Err(invalid_input(
            "resourceVersion cannot be combined with includeHistory",
        ));
    }
    Ok(())
}

fn validate_resource_binary_input(input: &ResourceBinaryInput) -> Result<(), ToolError> {
    match input.action {
        ResourceBinaryAction::Upload => {
            if input.resource_version.is_none()
                || input.expected_draft_revision.is_none()
                || input.source_path.is_none()
                || input.target_path.is_some()
            {
                return Err(invalid_input(
                    "upload requires draft version, revision, and sourcePath only",
                ));
            }
        }
        ResourceBinaryAction::Download => {
            if input.target_path.is_none()
                || input.source_path.is_some()
                || input.expected_draft_revision.is_some()
            {
                return Err(invalid_input(
                    "download requires targetPath and no draft mutation fields",
                ));
            }
        }
        ResourceBinaryAction::Remove => {
            if input.resource_version.is_none()
                || input.expected_draft_revision.is_none()
                || input.source_path.is_some()
                || input.target_path.is_some()
            {
                return Err(invalid_input(
                    "remove requires draft version and revision only",
                ));
            }
        }
    }
    Ok(())
}

fn invalid_input(message: &str) -> ToolError {
    ToolError::ExecutionFailed(format!("resource tool input is invalid: {message}"))
}
