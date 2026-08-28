use std::{collections::BTreeMap, sync::Arc};

use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::mcp::McpService,
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        mcp::{
            CreatePlatformMcpServerCommand, McpServerAuthKind, McpServerTransportKind,
            PlatformMcpServerListQuery, UpdatePlatformMcpServerCommand,
        },
    },
};

use super::common::{json_observation, page_records, parse_tool_input, service_tool_error};

pub(super) fn tools(mcp: McpService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(PlatformMcpPageTool { mcp: mcp.clone() }),
        Arc::new(PlatformMcpGetTool { mcp: mcp.clone() }),
        Arc::new(PlatformMcpCreateTool { mcp: mcp.clone() }),
        Arc::new(PlatformMcpUpdateTool { mcp: mcp.clone() }),
        Arc::new(PlatformMcpDeleteTool { mcp }),
    ]
}

#[derive(Clone)]
struct PlatformMcpPageTool {
    mcp: McpService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformMcpPageInput {
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
}

impl Tool for PlatformMcpPageTool {
    fn name(&self) -> &str {
        "platform_mcp_page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformMcpPageInput = parse_tool_input(request.input)?;
        let records = self
            .mcp
            .list_platform_servers(PlatformMcpServerListQuery {
                search: input.search,
                status: None,
            })
            .map_err(service_tool_error)?;
        let output = page_records(records, input.current, input.size);
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformMcpGetTool {
    mcp: McpService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformMcpCodeInput {
    server_code: String,
}

impl Tool for PlatformMcpGetTool {
    fn name(&self) -> &str {
        "platform_mcp_get"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformMcpCodeInput = parse_tool_input(request.input)?;
        let output = self
            .mcp
            .get_platform_server(input.server_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformMcpCreateTool {
    mcp: McpService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformMcpCreateInput {
    server_code: String,
    name: String,
    description: Option<String>,
    transport_kind: McpServerTransportKind,
    url: Option<String>,
    auth_kind: Option<McpServerAuthKind>,
    auth_payload: Option<Value>,
    headers: Option<BTreeMap<String, String>>,
    headers_variable_refs: Option<BTreeMap<String, String>>,
    command: Option<String>,
    args: Option<Vec<String>>,
    env_variable_refs: Option<BTreeMap<String, String>>,
    env_passthrough_refs: Option<Vec<String>>,
    working_directory: Option<String>,
}

impl Tool for PlatformMcpCreateTool {
    fn name(&self) -> &str {
        "platform_mcp_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformMcpCreateInput = parse_tool_input(request.input)?;
        let output = self
            .mcp
            .create_platform_server(CreatePlatformMcpServerCommand {
                server_code: input.server_code,
                name: input.name,
                description: input.description,
                transport_kind: input.transport_kind,
                url: input.url.unwrap_or_default(),
                auth_kind: input.auth_kind.unwrap_or(McpServerAuthKind::None),
                auth_payload: input.auth_payload,
                headers: input.headers.unwrap_or_default(),
                headers_variable_refs: input.headers_variable_refs.unwrap_or_default(),
                command: input.command.unwrap_or_default(),
                args: input.args.unwrap_or_default(),
                env_variable_refs: input.env_variable_refs.unwrap_or_default(),
                env_passthrough_refs: input.env_passthrough_refs.unwrap_or_default(),
                working_directory: input.working_directory,
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformMcpUpdateTool {
    mcp: McpService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformMcpUpdateInput {
    server_code: String,
    name: String,
    description: Option<String>,
    transport_kind: McpServerTransportKind,
    url: Option<String>,
    auth_kind: Option<McpServerAuthKind>,
    auth_payload: Option<Value>,
    headers: Option<BTreeMap<String, String>>,
    headers_variable_refs: Option<BTreeMap<String, String>>,
    command: Option<String>,
    args: Option<Vec<String>>,
    env_variable_refs: Option<BTreeMap<String, String>>,
    env_passthrough_refs: Option<Vec<String>>,
    working_directory: Option<String>,
}

impl Tool for PlatformMcpUpdateTool {
    fn name(&self) -> &str {
        "platform_mcp_update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformMcpUpdateInput = parse_tool_input(request.input)?;
        let server_code = input.server_code.trim().to_string();
        let output = self
            .mcp
            .update_platform_server(
                &server_code,
                UpdatePlatformMcpServerCommand {
                    name: input.name,
                    description: input.description,
                    transport_kind: input.transport_kind,
                    url: input.url.unwrap_or_default(),
                    auth_kind: input.auth_kind.unwrap_or(McpServerAuthKind::None),
                    auth_payload: input.auth_payload,
                    headers: input.headers.unwrap_or_default(),
                    headers_variable_refs: input.headers_variable_refs.unwrap_or_default(),
                    command: input.command.unwrap_or_default(),
                    args: input.args.unwrap_or_default(),
                    env_variable_refs: input.env_variable_refs.unwrap_or_default(),
                    env_passthrough_refs: input.env_passthrough_refs.unwrap_or_default(),
                    working_directory: input.working_directory,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformMcpDeleteTool {
    mcp: McpService,
}

impl Tool for PlatformMcpDeleteTool {
    fn name(&self) -> &str {
        "platform_mcp_delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformMcpCodeInput = parse_tool_input(request.input)?;
        let output = self
            .mcp
            .delete_platform_server(input.server_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}
