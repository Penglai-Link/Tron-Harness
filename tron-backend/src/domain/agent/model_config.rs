use serde::{Deserialize, Serialize};

use crate::domain::reasoning::ReasoningEffort;

use super::AgentModelProtocol;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelConfig {
    pub model_code: String,
    pub protocol: AgentModelProtocol,
    pub base_url: String,
    pub api_path: Option<String>,
    pub api_key: String,
    #[serde(default)]
    pub request_headers: std::collections::BTreeMap<String, String>,
    pub model_name: String,
    #[serde(default)]
    pub thinking_enabled: bool,
    #[serde(default)]
    pub supports_developer_role: bool,
    pub reasoning_effort: Option<ReasoningEffort>,
    #[serde(default)]
    pub responses_stateful_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub max_retries: Option<u32>,
}

/// Credential-free identity. Provider-specific routing dimensions are optional
/// and are not requirements for ordinary model connections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentModelIdentity {
    pub base_url: String,
    pub api_path: Option<String>,
    pub protocol: AgentModelProtocol,
    pub model_code: String,
    pub provider: Option<String>,
    pub upstream_vendor: Option<String>,
    pub quality: Option<String>,
}

impl AgentModelConfig {
    pub fn identity(&self) -> AgentModelIdentity {
        let mut identity = AgentModelIdentity {
            base_url: self.base_url.clone(),
            api_path: self.api_path.clone(),
            protocol: self.protocol,
            model_code: self.model_code.clone(),
            provider: None,
            upstream_vendor: None,
            quality: None,
        };
        if let Some(fixed) = self.model_code.strip_prefix("PERSON_AI:FIXED:")
            && let Some((quality, model)) = fixed.split_once(':')
        {
            identity.provider = Some("PersonAi".into());
            identity.quality = Some(quality.into());
            identity.model_code = model.into();
            if let Some((vendor, model)) = model.split_once('/') {
                identity.upstream_vendor = Some(vendor.into());
                identity.model_code = model.into();
            }
            // PersonAI request paths contain conversation/run routing rather
            // than a connection identity.
            identity.api_path = None;
        }
        identity
    }
}
