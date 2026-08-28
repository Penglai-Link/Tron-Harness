use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::open_app::{CreateOpenAppCommand, OpenAppService, UpdateOpenAppCommand},
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        open_app::OpenAppStatus,
    },
};

use super::common::{json_observation, page_records, parse_tool_input, service_tool_error};

pub(super) fn tools(open_apps: OpenAppService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(PlatformOpenAppPageTool {
            open_apps: open_apps.clone(),
        }),
        Arc::new(PlatformOpenAppGetTool {
            open_apps: open_apps.clone(),
        }),
        Arc::new(PlatformOpenAppCreateTool {
            open_apps: open_apps.clone(),
        }),
        Arc::new(PlatformOpenAppUpdateTool {
            open_apps: open_apps.clone(),
        }),
        Arc::new(PlatformOpenAppDisableTool { open_apps }),
    ]
}

#[derive(Clone)]
struct PlatformOpenAppPageTool {
    open_apps: OpenAppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformOpenAppPageInput {
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
    status: Option<OpenAppStatus>,
}

impl Tool for PlatformOpenAppPageTool {
    fn name(&self) -> &str {
        "platform_open_app_page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformOpenAppPageInput = parse_tool_input(request.input)?;
        let search = input
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
        let mut records = self
            .open_apps
            .list_open_apps()
            .map_err(service_tool_error)?;
        records.retain(|record| {
            input.status.is_none_or(|status| record.status == status)
                && search.as_ref().is_none_or(|query| {
                    format!("{} {}", record.open_app_code, record.name)
                        .to_ascii_lowercase()
                        .contains(query)
                })
        });
        let output = page_records(records, input.current, input.size);
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformOpenAppGetTool {
    open_apps: OpenAppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformOpenAppCodeInput {
    open_app_code: String,
}

impl Tool for PlatformOpenAppGetTool {
    fn name(&self) -> &str {
        "platform_open_app_get"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformOpenAppCodeInput = parse_tool_input(request.input)?;
        let output = self
            .open_apps
            .get_open_app_detail(input.open_app_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformOpenAppCreateTool {
    open_apps: OpenAppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformOpenAppCreateInput {
    name: String,
    description: Option<String>,
}

impl Tool for PlatformOpenAppCreateTool {
    fn name(&self) -> &str {
        "platform_open_app_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformOpenAppCreateInput = parse_tool_input(request.input)?;
        let created = self
            .open_apps
            .create_open_app(CreateOpenAppCommand {
                name: input.name,
                description: input.description,
            })
            .map_err(service_tool_error)?;
        let output = self
            .open_apps
            .get_open_app_detail(&created.open_app.open_app_code)
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformOpenAppUpdateTool {
    open_apps: OpenAppService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformOpenAppUpdateInput {
    open_app_code: String,
    name: String,
    description: Option<String>,
    status: OpenAppStatus,
}

impl Tool for PlatformOpenAppUpdateTool {
    fn name(&self) -> &str {
        "platform_open_app_update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformOpenAppUpdateInput = parse_tool_input(request.input)?;
        let output = self
            .open_apps
            .update_open_app(
                input.open_app_code.trim(),
                UpdateOpenAppCommand {
                    name: input.name,
                    description: input.description,
                    status: input.status,
                },
            )
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformOpenAppDisableTool {
    open_apps: OpenAppService,
}

impl Tool for PlatformOpenAppDisableTool {
    fn name(&self) -> &str {
        "platform_open_app_disable"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformOpenAppCodeInput = parse_tool_input(request.input)?;
        self.open_apps
            .disable_open_app(input.open_app_code.trim())
            .map_err(service_tool_error)?;
        let output = self
            .open_apps
            .get_open_app_detail(input.open_app_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}
