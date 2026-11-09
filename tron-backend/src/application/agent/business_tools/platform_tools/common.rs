use serde::{Deserialize, Serialize};

use crate::domain::agent::{AgentRunContext, ToolError, ToolExecutionResult, ToolObservation};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PagedToolRecords<T> {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<T>,
}

pub(super) fn parse_tool_input<T: for<'de> Deserialize<'de>>(
    input: serde_json::Value,
) -> Result<T, ToolError> {
    serde_json::from_value(input)
        .map_err(|error| ToolError::ExecutionFailed(format!("tool input is invalid: {error}")))
}

pub(super) fn json_observation<T: Serialize>(
    tool_name: &str,
    value: &T,
) -> Result<ToolExecutionResult, ToolError> {
    let output = serde_json::to_string(value).map_err(service_tool_error)?;
    Ok(ToolExecutionResult::observation(ToolObservation {
        tool_name: tool_name.to_string(),
        output,
    }))
}

pub(super) fn page_records<T>(
    records: Vec<T>,
    current: Option<usize>,
    size: Option<usize>,
) -> PagedToolRecords<T> {
    let current = current.unwrap_or(1).max(1);
    let size = size.unwrap_or(50).clamp(1, 100);
    let total = records.len();
    let start = current.saturating_sub(1).saturating_mul(size);

    PagedToolRecords {
        current,
        size,
        total,
        records: records.into_iter().skip(start).take(size).collect(),
    }
}

pub(super) fn ensure_workspace_match(
    expected_workspace_code: &str,
    actual_workspace_code: &str,
    entity_name: &str,
) -> Result<(), ToolError> {
    if actual_workspace_code == expected_workspace_code {
        return Ok(());
    }

    Err(ToolError::ExecutionFailed(format!(
        "workspaceCode `{expected_workspace_code}` does not match {entity_name} workspace `{actual_workspace_code}`"
    )))
}

pub(in crate::application::agent::business_tools) fn resolve_workspace_code(
    requested_workspace_code: &Option<String>,
    context: &AgentRunContext,
) -> Result<String, ToolError> {
    let workspace_code = requested_workspace_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    if workspace_code.trim().is_empty() {
        return Err(ToolError::ExecutionFailed(
            "workspaceCode is required".to_string(),
        ));
    }
    if context.is_owning_dude() || workspace_code != context.execution_workspace_code().trim() {
        return Err(ToolError::ExecutionFailed("workspaceCode is outside the current authorized execution scope".into()));
    }
    Ok(workspace_code.to_string())
}

pub(super) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn normalize_optional_token(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase().replace('-', "_"))
}

pub(super) fn enum_tool_error(field_name: &str, value: &str) -> ToolError {
    ToolError::ExecutionFailed(format!(
        "tool input field `{field_name}` has unsupported value `{value}`"
    ))
}

pub(super) fn service_tool_error(error: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
