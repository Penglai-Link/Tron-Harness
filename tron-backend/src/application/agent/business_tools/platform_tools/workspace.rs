use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        workspace::{WorkspaceListQuery, WorkspaceStatus},
    },
};

use super::common::{json_observation, page_records, parse_tool_input, service_tool_error};

pub(super) fn tools(workspaces: WorkspaceService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(WorkspacePageTool {
            workspaces: workspaces.clone(),
        }),
        Arc::new(WorkspaceGetTool { workspaces }),
    ]
}

#[derive(Clone)]
struct WorkspacePageTool {
    workspaces: WorkspaceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspacePageInput {
    current: Option<usize>,
    size: Option<usize>,
    include_archived: Option<bool>,
    search: Option<String>,
    root_only: Option<bool>,
}

impl Tool for WorkspacePageTool {
    fn name(&self) -> &str {
        "workspace_page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: WorkspacePageInput = parse_tool_input(request.input)?;
        let include_archived = input.include_archived.unwrap_or(false);
        let records = self
            .workspaces
            .list_workspaces(WorkspaceListQuery {
                include_archived: Some(include_archived),
                search: input.search,
                status: (!include_archived).then_some(WorkspaceStatus::Active),
                root_only: Some(input.root_only.unwrap_or(true)),
            })
            .map_err(service_tool_error)?;
        let output = page_records(records, input.current, input.size);
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct WorkspaceGetTool {
    workspaces: WorkspaceService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceGetInput {
    workspace_code: String,
}

impl Tool for WorkspaceGetTool {
    fn name(&self) -> &str {
        "workspace_get"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: WorkspaceGetInput = parse_tool_input(request.input)?;
        let output = self
            .workspaces
            .get_workspace(input.workspace_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}
