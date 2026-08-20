use std::sync::Arc;

use serde::Deserialize;

use crate::{
    application::model::{ModelCommand, ModelService},
    domain::{
        agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest},
        model::{
            ModelCapability, ModelProtocol, ModelType, ModelView,
            default_model_capabilities_for_type,
        },
        reasoning::ReasoningEffort,
    },
};

use super::common::{
    enum_tool_error, json_observation, normalize_optional_token, page_records, parse_tool_input,
    required_string, resolve_workspace_code, service_tool_error,
};

pub(super) fn tools(models: ModelService) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ModelPageTool {
            models: models.clone(),
        }),
        Arc::new(ModelCreateTool {
            models: models.clone(),
        }),
        Arc::new(ModelUpdateTool {
            models: models.clone(),
        }),
        Arc::new(ModelEnableTool {
            models: models.clone(),
        }),
        Arc::new(ModelDisableTool {
            models: models.clone(),
        }),
        Arc::new(ModelDeleteTool { models }),
    ]
}

#[derive(Clone)]
struct ModelPageTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelPageInput {
    workspace_code: Option<String>,
    current: Option<usize>,
    size: Option<usize>,
    search: Option<String>,
    model_type: Option<String>,
    enabled: Option<bool>,
}

impl Tool for ModelPageTool {
    fn name(&self) -> &str {
        "model.page"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ModelPageInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let model_type = parse_model_type(input.model_type.as_deref())?;
        let search = input.search.map(|value| value.trim().to_ascii_lowercase());
        let mut records = self
            .models
            .list_workspace_models(workspace_code.as_str())
            .map_err(service_tool_error)?
            .records;
        records.retain(|record| {
            model_type.is_none_or(|target| record.model_type == target)
                && input
                    .enabled
                    .is_none_or(|enabled| record.enabled == enabled)
                && search.as_ref().is_none_or(|query| {
                    format!(
                        "{} {} {} {} {}",
                        record.model_code,
                        record.display_name,
                        record.vendor,
                        record.model_name,
                        record.source_label
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
struct ModelCreateTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelCreateInput {
    workspace_code: Option<String>,
    #[serde(flatten)]
    spec: ModelSpecInput,
}

impl Tool for ModelCreateTool {
    fn name(&self) -> &str {
        "model.create"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ModelCreateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let command = input.spec.create_command()?;
        let output = self
            .models
            .create_workspace_model(workspace_code.as_str(), command)
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ModelUpdateTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelUpdateInput {
    workspace_code: Option<String>,
    model_code: String,
    #[serde(flatten)]
    spec: ModelSpecInput,
}

impl Tool for ModelUpdateTool {
    fn name(&self) -> &str {
        "model.update"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ModelUpdateInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let current = self
            .models
            .get_workspace_model(workspace_code.as_str(), input.model_code.trim())
            .map_err(service_tool_error)?;
        let command = input.spec.update_command(&current)?;
        let output = self
            .models
            .update_workspace_model(workspace_code.as_str(), input.model_code.trim(), command)
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Clone)]
struct ModelEnableTool {
    models: ModelService,
}

#[derive(Clone)]
struct ModelDisableTool {
    models: ModelService,
}

#[derive(Clone)]
struct ModelDeleteTool {
    models: ModelService,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelCodeInput {
    workspace_code: Option<String>,
    model_code: String,
}

impl Tool for ModelEnableTool {
    fn name(&self) -> &str {
        "model.enable"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ModelCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .models
            .enable_workspace_model(workspace_code.as_str(), input.model_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

impl Tool for ModelDisableTool {
    fn name(&self) -> &str {
        "model.disable"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ModelCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .models
            .disable_workspace_model(workspace_code.as_str(), input.model_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

impl Tool for ModelDeleteTool {
    fn name(&self) -> &str {
        "model.delete"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ModelCodeInput = parse_tool_input(request.input)?;
        let workspace_code = resolve_workspace_code(&input.workspace_code, context)?;
        let output = self
            .models
            .delete_workspace_model(workspace_code.as_str(), input.model_code.trim())
            .map_err(service_tool_error)?;
        json_observation(self.name(), &output)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ModelSpecInput {
    display_name: Option<String>,
    vendor: Option<String>,
    protocol: Option<String>,
    model_type: Option<String>,
    model_capabilities: Option<Vec<String>>,
    base_url: Option<String>,
    api_path: Option<String>,
    api_key_ref: Option<String>,
    model_name: Option<String>,
    enabled: Option<bool>,
    description: Option<String>,
    thinking_enabled: Option<bool>,
    supported_reasoning_efforts: Option<Vec<String>>,
    default_reasoning_effort: Option<String>,
    structured_output_enabled: Option<bool>,
    responses_stateful_enabled: Option<bool>,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    context_window_tokens: Option<u32>,
    top_p: Option<f64>,
    max_retries: Option<u32>,
    allow_inheritance: Option<bool>,
}

impl ModelSpecInput {
    pub(super) fn create_command(self) -> Result<ModelCommand, ToolError> {
        let model_type = parse_required_model_type(self.model_type.as_deref(), "modelType")?;
        let capabilities = parse_model_capabilities(self.model_capabilities, model_type)?;
        let supported_reasoning_efforts =
            parse_reasoning_efforts(self.supported_reasoning_efforts)?;
        let default_reasoning_effort =
            parse_reasoning_effort(self.default_reasoning_effort.as_deref())?;
        Ok(ModelCommand {
            display_name: required_string(self.display_name, "displayName")?,
            vendor: required_string(self.vendor, "vendor")?,
            protocol: parse_required_model_protocol(self.protocol.as_deref(), "protocol")?,
            model_type,
            model_capabilities: capabilities,
            base_url: required_string(self.base_url, "baseUrl")?,
            api_path: self.api_path,
            api_key_ref: required_string(self.api_key_ref, "apiKeyRef")?,
            model_name: required_string(self.model_name, "modelName")?,
            enabled: self.enabled.unwrap_or(true),
            description: self.description,
            thinking_enabled: self.thinking_enabled.unwrap_or(false),
            supported_reasoning_efforts,
            default_reasoning_effort,
            structured_output_enabled: self.structured_output_enabled.unwrap_or(false),
            responses_stateful_enabled: self.responses_stateful_enabled.unwrap_or(false),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            context_window_tokens: self.context_window_tokens,
            top_p: self.top_p,
            max_retries: self.max_retries,
            allow_inheritance: self.allow_inheritance,
        })
    }

    pub(super) fn update_command(self, current: &ModelView) -> Result<ModelCommand, ToolError> {
        let model_type =
            parse_model_type(self.model_type.as_deref())?.unwrap_or(current.model_type);
        let capabilities = match self.model_capabilities {
            Some(values) => parse_model_capabilities(Some(values), model_type)?,
            None => current.model_capabilities.clone(),
        };
        let supported_reasoning_efforts = match self.supported_reasoning_efforts {
            Some(values) => parse_reasoning_efforts(Some(values))?,
            None => current.supported_reasoning_efforts.clone(),
        };
        let default_reasoning_effort = match self.default_reasoning_effort {
            Some(value) => parse_reasoning_effort(Some(value.as_str()))?,
            None => current.default_reasoning_effort,
        };
        Ok(ModelCommand {
            display_name: self
                .display_name
                .unwrap_or_else(|| current.display_name.clone()),
            vendor: self.vendor.unwrap_or_else(|| current.vendor.clone()),
            protocol: parse_model_protocol(self.protocol.as_deref())?.unwrap_or(current.protocol),
            model_type,
            model_capabilities: capabilities,
            base_url: self.base_url.unwrap_or_else(|| current.base_url.clone()),
            api_path: self.api_path.or_else(|| current.api_path.clone()),
            api_key_ref: self
                .api_key_ref
                .unwrap_or_else(|| current.api_key_ref.clone()),
            model_name: self
                .model_name
                .unwrap_or_else(|| current.model_name.clone()),
            enabled: self.enabled.unwrap_or(current.enabled),
            description: self.description.or_else(|| current.description.clone()),
            thinking_enabled: self.thinking_enabled.unwrap_or(current.thinking_enabled),
            supported_reasoning_efforts,
            default_reasoning_effort,
            structured_output_enabled: self
                .structured_output_enabled
                .unwrap_or(current.structured_output_enabled),
            responses_stateful_enabled: self
                .responses_stateful_enabled
                .unwrap_or(current.responses_stateful_enabled),
            temperature: self.temperature.or(current.temperature),
            max_tokens: self.max_tokens.or(current.max_tokens),
            context_window_tokens: self.context_window_tokens.or(current.context_window_tokens),
            top_p: self.top_p.or(current.top_p),
            max_retries: self.max_retries.or(current.max_retries),
            allow_inheritance: self.allow_inheritance.or(current.allow_inheritance),
        })
    }
}

fn parse_reasoning_efforts(values: Option<Vec<String>>) -> Result<Vec<ReasoningEffort>, ToolError> {
    values
        .unwrap_or_default()
        .into_iter()
        .map(|value| parse_reasoning_effort(Some(value.as_str())))
        .collect::<Result<Vec<_>, _>>()
        .map(|values| values.into_iter().flatten().collect())
}

fn parse_reasoning_effort(value: Option<&str>) -> Result<Option<ReasoningEffort>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("none") => Ok(Some(ReasoningEffort::None)),
        Some("minimal") => Ok(Some(ReasoningEffort::Minimal)),
        Some("low") => Ok(Some(ReasoningEffort::Low)),
        Some("medium") => Ok(Some(ReasoningEffort::Medium)),
        Some("high") => Ok(Some(ReasoningEffort::High)),
        Some("xhigh") | Some("x_high") => Ok(Some(ReasoningEffort::XHigh)),
        Some("max") => Ok(Some(ReasoningEffort::Max)),
        Some("ultra") => Ok(Some(ReasoningEffort::Ultra)),
        Some(other) => Err(enum_tool_error("reasoningEffort", other)),
    }
}

fn parse_required_model_protocol(
    value: Option<&str>,
    field_name: &str,
) -> Result<ModelProtocol, ToolError> {
    parse_model_protocol(value)?
        .ok_or_else(|| ToolError::ExecutionFailed(format!("{field_name} is required")))
}

fn parse_required_model_type(
    value: Option<&str>,
    field_name: &str,
) -> Result<ModelType, ToolError> {
    parse_model_type(value)?
        .ok_or_else(|| ToolError::ExecutionFailed(format!("{field_name} is required")))
}

fn parse_model_protocol(value: Option<&str>) -> Result<Option<ModelProtocol>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("openai_compatible") | Some("open_ai_compatible") => {
            Ok(Some(ModelProtocol::OpenAiCompatible))
        }
        Some("responses") => Ok(Some(ModelProtocol::Responses)),
        Some("rerank") => Ok(Some(ModelProtocol::Rerank)),
        Some(other) => Err(enum_tool_error("protocol", other)),
    }
}

fn parse_model_type(value: Option<&str>) -> Result<Option<ModelType>, ToolError> {
    match normalize_optional_token(value).as_deref() {
        None => Ok(None),
        Some("text") => Ok(Some(ModelType::Text)),
        Some("embedding") => Ok(Some(ModelType::Embedding)),
        Some("rerank") => Ok(Some(ModelType::Rerank)),
        Some("image") => Ok(Some(ModelType::Image)),
        Some(other) => Err(enum_tool_error("modelType", other)),
    }
}

fn parse_model_capabilities(
    values: Option<Vec<String>>,
    model_type: ModelType,
) -> Result<Vec<ModelCapability>, ToolError> {
    values
        .filter(|values| !values.is_empty())
        .map(|values| {
            values
                .iter()
                .map(|value| parse_model_capability(value))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_else(|| Ok(default_model_capabilities_for_type(model_type)))
}

fn parse_model_capability(value: &str) -> Result<ModelCapability, ToolError> {
    match normalize_optional_token(Some(value)).as_deref() {
        Some("multimodal_all") => Ok(ModelCapability::MultimodalAll),
        Some("text_generation") => Ok(ModelCapability::TextGeneration),
        Some("deep_thinking") => Ok(ModelCapability::DeepThinking),
        Some("vision_understanding") => Ok(ModelCapability::VisionUnderstanding),
        Some("image_generation") => Ok(ModelCapability::ImageGeneration),
        Some("video_generation") => Ok(ModelCapability::VideoGeneration),
        Some("threed_generation") | Some("3d_generation") => Ok(ModelCapability::ThreedGeneration),
        Some("speech_recognition") => Ok(ModelCapability::SpeechRecognition),
        Some("speech_synthesis") => Ok(ModelCapability::SpeechSynthesis),
        Some("multimodal_vector") => Ok(ModelCapability::MultimodalVector),
        Some("text_vector") => Ok(ModelCapability::TextVector),
        Some("realtime_multimodal") => Ok(ModelCapability::RealtimeMultimodal),
        Some("realtime_speech_synthesis") => Ok(ModelCapability::RealtimeSpeechSynthesis),
        Some("realtime_speech_recognition") => Ok(ModelCapability::RealtimeSpeechRecognition),
        Some("realtime_speech_translation") => Ok(ModelCapability::RealtimeSpeechTranslation),
        Some("rerank") => Ok(ModelCapability::Rerank),
        Some(other) => Err(enum_tool_error("modelCapabilities", other)),
        None => Err(ToolError::ExecutionFailed(
            "modelCapabilities item is required".to_string(),
        )),
    }
}
