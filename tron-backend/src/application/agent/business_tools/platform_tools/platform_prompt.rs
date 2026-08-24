use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::prompt::{
        CreatePromptCommand, PromptPageQuery, PromptService, PromptSpecCommand,
        PromptVariableSpecCommand, UpdatePromptCommand,
    },
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        prompt::{PromptCategory, PromptVariableType},
    },
};

use super::common::{json_observation, parse_tool_input, service_tool_error};

pub(super) fn tools(prompts: PromptService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(PlatformPromptPageTool {
            prompts: prompts.clone(),
        }),
        Arc::new(PlatformPromptGetTool {
            prompts: prompts.clone(),
        }),
        Arc::new(PlatformPromptCreateTool {
            prompts: prompts.clone(),
        }),
        Arc::new(PlatformPromptUpdateTool {
            prompts: prompts.clone(),
        }),
        Arc::new(PlatformPromptDeleteTool { prompts }),
    ]
}

#[derive(Clone)]
struct PlatformPromptPageTool {
    prompts: PromptService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformPromptPageInput {
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
    category: Option<PromptCategory>,
    enabled: Option<bool>,
}

impl Tool for PlatformPromptPageTool {
    fn name(&self) -> &str {
        "platform.prompt.page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformPromptPageInput = parse_tool_input(request.input)?;
        let output = self
            .prompts
            .platform_page(PromptPageQuery {
                current: input.current.unwrap_or(1).max(1),
                size: input.size.unwrap_or(20).clamp(1, 100),
                search: input.search,
                category: input.category,
                source_kind: None,
                enabled: input.enabled,
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformPromptGetTool {
    prompts: PromptService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformPromptCodeInput {
    prompt_code: String,
}

impl Tool for PlatformPromptGetTool {
    fn name(&self) -> &str {
        "platform.prompt.get"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformPromptCodeInput = parse_tool_input(request.input)?;
        let output = self
            .prompts
            .platform_detail(input.prompt_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformPromptCreateTool {
    prompts: PromptService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformPromptCreateInput {
    display_name: String,
    category: PromptCategory,
    prompt_body: String,
    description: Option<String>,
    enabled: Option<bool>,
    variables: Vec<PlatformPromptVariableInput>,
}

impl Tool for PlatformPromptCreateTool {
    fn name(&self) -> &str {
        "platform.prompt.create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformPromptCreateInput = parse_tool_input(request.input)?;
        let output = self
            .prompts
            .create_platform_prompt(CreatePromptCommand {
                category: input.category,
                spec: input.into_spec(),
                allow_inheritance: None,
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformPromptUpdateTool {
    prompts: PromptService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformPromptUpdateInput {
    prompt_code: String,
    expected_version: u64,
    display_name: String,
    prompt_body: String,
    description: Option<String>,
    enabled: Option<bool>,
    variables: Vec<PlatformPromptVariableInput>,
}

impl Tool for PlatformPromptUpdateTool {
    fn name(&self) -> &str {
        "platform.prompt.update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformPromptUpdateInput = parse_tool_input(request.input)?;
        let output = self
            .prompts
            .update_platform_prompt(UpdatePromptCommand {
                allow_inheritance: None,
                prompt_code: input.prompt_code,
                expected_version: input.expected_version,
                spec: PromptSpecCommand {
                    display_name: input.display_name,
                    prompt_body: input.prompt_body,
                    description: input.description,
                    enabled: input.enabled.unwrap_or(true),
                    translations: Vec::new(),
                    variables: input
                        .variables
                        .into_iter()
                        .map(PlatformPromptVariableInput::into_command)
                        .collect(),
                },
            })
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct PlatformPromptDeleteTool {
    prompts: PromptService,
}

impl Tool for PlatformPromptDeleteTool {
    fn name(&self) -> &str {
        "platform.prompt.delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: PlatformPromptCodeInput = parse_tool_input(request.input)?;
        let output = self
            .prompts
            .delete_platform_prompt(input.prompt_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlatformPromptVariableInput {
    name: String,
    #[serde(rename = "type", default = "default_prompt_variable_type")]
    variable_type: PromptVariableType,
    description: String,
}

fn default_prompt_variable_type() -> PromptVariableType {
    PromptVariableType::String
}

impl PlatformPromptCreateInput {
    fn into_spec(self) -> PromptSpecCommand {
        PromptSpecCommand {
            display_name: self.display_name,
            prompt_body: self.prompt_body,
            description: self.description,
            enabled: self.enabled.unwrap_or(true),
            translations: Vec::new(),
            variables: self
                .variables
                .into_iter()
                .map(PlatformPromptVariableInput::into_command)
                .collect(),
        }
    }
}

impl PlatformPromptVariableInput {
    fn into_command(self) -> PromptVariableSpecCommand {
        PromptVariableSpecCommand {
            name: self.name,
            variable_type: self.variable_type,
            description: self.description,
        }
    }
}
