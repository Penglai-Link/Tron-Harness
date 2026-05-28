use serde::Deserialize;

use crate::{
    application::model::{ModelCommand, SetAllowInheritanceCommand},
    domain::model::{ModelCapability, ModelProtocol, ModelType},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelRequest {
    pub display_name: String,
    pub vendor: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub model_capabilities: Option<Vec<ModelCapability>>,
    pub base_url: String,
    pub api_path: Option<String>,
    pub api_key_ref: String,
    pub model_name: String,
    pub enabled: Option<bool>,
    pub description: Option<String>,
    pub thinking_enabled: Option<bool>,
    pub structured_output_enabled: Option<bool>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub allow_inheritance: Option<bool>,
}

impl From<ModelRequest> for ModelCommand {
    fn from(request: ModelRequest) -> Self {
        Self {
            display_name: request.display_name,
            vendor: request.vendor,
            protocol: request.protocol,
            model_type: request.model_type,
            model_capabilities: request.model_capabilities.unwrap_or_default(),
            base_url: request.base_url,
            api_path: request.api_path,
            api_key_ref: request.api_key_ref,
            model_name: request.model_name,
            enabled: request.enabled.unwrap_or(true),
            description: request.description,
            thinking_enabled: request.thinking_enabled.unwrap_or(false),
            structured_output_enabled: request.structured_output_enabled.unwrap_or(false),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            context_window_tokens: request.context_window_tokens,
            top_p: request.top_p,
            timeout_ms: request.timeout_ms,
            max_retries: request.max_retries,
            allow_inheritance: request.allow_inheritance,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetAllowInheritanceRequest {
    pub allow_inheritance: bool,
}

impl From<SetAllowInheritanceRequest> for SetAllowInheritanceCommand {
    fn from(request: SetAllowInheritanceRequest) -> Self {
        Self {
            allow_inheritance: request.allow_inheritance,
        }
    }
}
