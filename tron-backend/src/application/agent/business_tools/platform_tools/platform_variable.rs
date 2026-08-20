use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::variable::{
        CreateVariableCommand, UpdateVariableCommand, VariableService, VariableValueMode,
    },
    domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
};

use super::common::{
    enum_tool_error, json_observation, normalize_optional_token, page_records, parse_tool_input,
    service_tool_error,
};

/// Agent tools for the platform settings variable catalog.
///
/// These tools intentionally do not accept `workspaceCode`: platform variables
/// belong to the administrator-managed platform scope, not to a workspace.
pub(super) fn tools(variables: VariableService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(PlatformVariablePageTool {
            variables: variables.clone(),
        }),
        Arc::new(PlatformVariableCreateTool {
            variables: variables.clone(),
        }),
        Arc::new(PlatformVariableUpdateTool {
            variables: variables.clone(),
        }),
        Arc::new(PlatformVariableDeleteTool { variables }),
    ]
}

#[derive(Clone)]
struct PlatformVariablePageTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformVariablePageInput {
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
}

impl Tool for PlatformVariablePageTool {
    fn name(&self) -> &str {
        "platform.variable.page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformVariablePageInput = parse_tool_input(request.input)?;
        let search = input
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
        let mut records = self
            .variables
            .list_platform_variables()
            .map_err(service_tool_error)?;
        if let Some(search) = search {
            records.retain(|record| {
                record.key.to_ascii_lowercase().contains(&search)
                    || record
                        .description
                        .as_deref()
                        .map(str::to_ascii_lowercase)
                        .is_some_and(|description| description.contains(&search))
            });
        }
        let output = page_records(records, input.current, input.size);
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformVariableCreateTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformVariableCreateInput {
    key: String,
    value: String,
    description: Option<String>,
    is_sensitive: Option<bool>,
}

impl Tool for PlatformVariableCreateTool {
    fn name(&self) -> &str {
        "platform.variable.create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformVariableCreateInput = parse_tool_input(request.input)?;
        let output = self
            .variables
            .create_platform_variable(CreateVariableCommand {
                key: input.key,
                value: input.value,
                description: input.description,
                is_sensitive: input.is_sensitive,
                value_mode: Some(VariableValueMode::Replace),
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformVariableUpdateTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformVariableUpdateInput {
    key: String,
    value: Option<String>,
    description: Option<String>,
    is_sensitive: Option<bool>,
    value_mode: Option<String>,
}

impl Tool for PlatformVariableUpdateTool {
    fn name(&self) -> &str {
        "platform.variable.update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformVariableUpdateInput = parse_tool_input(request.input)?;
        let key = input.key.trim().to_string();
        let current_description = self
            .variables
            .list_platform_variables()
            .map_err(service_tool_error)?
            .into_iter()
            .find(|record| record.key == key)
            .and_then(|record| record.description);
        let value_mode = match input.value_mode.as_deref() {
            Some(value) => Some(parse_variable_value_mode(value)?),
            None if input.value.is_none() => Some(VariableValueMode::Preserve),
            None => Some(VariableValueMode::Replace),
        };
        let output = self
            .variables
            .update_platform_variable(
                &key,
                UpdateVariableCommand {
                    value: input.value,
                    description: input.description.or(current_description),
                    is_sensitive: input.is_sensitive,
                    value_mode,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformVariableDeleteTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformVariableDeleteInput {
    key: String,
}

impl Tool for PlatformVariableDeleteTool {
    fn name(&self) -> &str {
        "platform.variable.delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformVariableDeleteInput = parse_tool_input(request.input)?;
        let output = self
            .variables
            .delete_platform_variable(input.key.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

fn parse_variable_value_mode(value: &str) -> Result<VariableValueMode, ToolError> {
    match normalize_optional_token(Some(value)).as_deref() {
        Some("replace") => Ok(VariableValueMode::Replace),
        Some("preserve") => Ok(VariableValueMode::Preserve),
        Some(other) => Err(enum_tool_error("valueMode", other)),
        None => Ok(VariableValueMode::Replace),
    }
}
