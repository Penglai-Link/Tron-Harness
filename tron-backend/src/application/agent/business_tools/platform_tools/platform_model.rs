use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::model::ModelService,
    domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
};

use super::{
    common::{json_observation, page_records, parse_tool_input, service_tool_error},
    model::ModelSpecInput,
};

pub(super) fn tools(models: ModelService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(PlatformModelPageTool {
            models: models.clone(),
        }),
        Arc::new(PlatformModelGetTool {
            models: models.clone(),
        }),
        Arc::new(PlatformModelCreateTool {
            models: models.clone(),
        }),
        Arc::new(PlatformModelUpdateTool {
            models: models.clone(),
        }),
        Arc::new(PlatformModelEnableTool {
            models: models.clone(),
        }),
        Arc::new(PlatformModelDisableTool {
            models: models.clone(),
        }),
        Arc::new(PlatformModelDeleteTool { models }),
    ]
}

#[derive(Clone)]
struct PlatformModelPageTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformModelPageInput {
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
    enabled: Option<bool>,
}

impl Tool for PlatformModelPageTool {
    fn name(&self) -> &str {
        "platform_model_page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformModelPageInput = parse_tool_input(request.input)?;
        let search = input
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
        let mut records = self
            .models
            .list_platform_models()
            .map_err(service_tool_error)?;
        records.retain(|record| {
            input
                .enabled
                .is_none_or(|enabled| record.enabled == enabled)
                && search.as_ref().is_none_or(|query| {
                    format!(
                        "{} {} {} {}",
                        record.model_code, record.display_name, record.vendor, record.model_name
                    )
                    .to_ascii_lowercase()
                    .contains(query)
                })
        });
        let output = page_records(records, input.current, input.size);
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformModelGetTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformModelCodeInput {
    model_code: String,
}

impl Tool for PlatformModelGetTool {
    fn name(&self) -> &str {
        "platform_model_get"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformModelCodeInput = parse_tool_input(request.input)?;
        let output = self
            .models
            .get_platform_model(input.model_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformModelCreateTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformModelCreateInput {
    #[serde(flatten)]
    spec: ModelSpecInput,
}

impl Tool for PlatformModelCreateTool {
    fn name(&self) -> &str {
        "platform_model_create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformModelCreateInput = parse_tool_input(request.input)?;
        let output = self
            .models
            .create_platform_model(input.spec.create_command()?)
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformModelUpdateTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformModelUpdateInput {
    model_code: String,
    #[serde(flatten)]
    spec: ModelSpecInput,
}

impl Tool for PlatformModelUpdateTool {
    fn name(&self) -> &str {
        "platform_model_update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformModelUpdateInput = parse_tool_input(request.input)?;
        let model_code = input.model_code.trim().to_string();
        let current = self
            .models
            .get_platform_model(&model_code)
            .map_err(service_tool_error)?;
        let output = self
            .models
            .update_platform_model(&model_code, input.spec.update_command(&current)?)
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformModelEnableTool {
    models: ModelService,
}

#[derive(Clone)]
struct PlatformModelDisableTool {
    models: ModelService,
}

#[derive(Clone)]
struct PlatformModelDeleteTool {
    models: ModelService,
}

impl Tool for PlatformModelEnableTool {
    fn name(&self) -> &str {
        "platform_model_enable"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformModelCodeInput = parse_tool_input(request.input)?;
        let output = self
            .models
            .enable_platform_model(input.model_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

impl Tool for PlatformModelDisableTool {
    fn name(&self) -> &str {
        "platform_model_disable"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformModelCodeInput = parse_tool_input(request.input)?;
        let output = self
            .models
            .disable_platform_model(input.model_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

impl Tool for PlatformModelDeleteTool {
    fn name(&self) -> &str {
        "platform_model_delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformModelCodeInput = parse_tool_input(request.input)?;
        let output = self
            .models
            .delete_platform_model(input.model_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}
