use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::variable::{
        CreateVariableCommand, UpdateVariableCommand, VariableService, VariableValueMode,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        variable::VariableCatalogQuery,
    },
};

use super::common::{
    enum_tool_error, json_observation, normalize_optional_token, page_records, parse_tool_input,
    resolve_workspace_code, service_tool_error,
};

pub(super) fn tools(variables: VariableService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(VariablePageTool {
            variables: variables.clone(),
        }),
        Arc::new(VariableCreateTool {
            variables: variables.clone(),
        }),
        Arc::new(VariableUpdateTool {
            variables: variables.clone(),
        }),
        Arc::new(VariableDeleteTool { variables }),
    ]
}

#[derive(Clone)]
struct VariablePageTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariablePageInput {
    workspace_code: Option<String>,
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
}

impl Tool for VariablePageTool {
    fn name(&self) -> &str {
        "variable_page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: VariablePageInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let records = self
            .variables
            .list_variables(
                workspace_code.as_str(),
                VariableCatalogQuery {
                    search: input.search,
                },
            )
            .map_err(service_tool_error)?;
        let output = page_records(records, input.current, input.size);
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct VariableCreateTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariableCreateInput {
    workspace_code: Option<String>,
    key: String,
    value: String,
    description: Option<String>,
    is_sensitive: Option<bool>,
}

impl Tool for VariableCreateTool {
    fn name(&self) -> &str {
        "variable_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: VariableCreateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .variables
            .create_variable(
                workspace_code.as_str(),
                CreateVariableCommand {
                    key: input.key,
                    value: input.value,
                    description: input.description,
                    is_sensitive: input.is_sensitive,
                    value_mode: Some(VariableValueMode::Replace),
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct VariableUpdateTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariableUpdateInput {
    workspace_code: Option<String>,
    key: String,
    value: Option<String>,
    description: Option<String>,
    is_sensitive: Option<bool>,
    value_mode: Option<String>,
}

impl Tool for VariableUpdateTool {
    fn name(&self) -> &str {
        "variable_update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: VariableUpdateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let current_description = self
            .variables
            .list_variables(
                workspace_code.as_str(),
                VariableCatalogQuery { search: None },
            )
            .map_err(service_tool_error)?
            .into_iter()
            .find(|record| record.key == input.key)
            .and_then(|record| record.description);
        let value_mode = match input.value_mode.as_deref() {
            Some(value) => Some(parse_variable_value_mode(value)?),
            None if input.value.is_none() => Some(VariableValueMode::Preserve),
            None => Some(VariableValueMode::Replace),
        };
        let output = self
            .variables
            .update_variable(
                workspace_code.as_str(),
                input.key.trim(),
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
struct VariableDeleteTool {
    variables: VariableService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariableDeleteInput {
    workspace_code: Option<String>,
    key: String,
}

impl Tool for VariableDeleteTool {
    fn name(&self) -> &str {
        "variable_delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: VariableDeleteInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .variables
            .delete_variable(workspace_code.as_str(), input.key.trim())
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
