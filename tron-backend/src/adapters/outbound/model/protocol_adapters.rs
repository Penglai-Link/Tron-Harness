//! Product DTO conversion only. All wire request construction and response
//! parsing belong to the public SDK model plugin.
use super::http_model_client::convert;
use crate::domain::agent::{
    AgentModelConfig, AgentModelProtocol, ModelRequest, ModelResponse, ModelStreamEvent,
    ModelTokenUsage,
};
use eliteforge_agent_official_plugin_model_http as sdk;
use serde_json::Value;
pub trait ModelProtocolAdapter: Send + Sync {
    fn protocol(&self) -> AgentModelProtocol;
    fn default_path(&self) -> &'static str;
    fn request_body(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<Value, String>;
    fn stream_request_body(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<Value, String>;
    fn parse_response(&self, body: &Value) -> Result<ModelResponse, String>;
    fn parse_stream_event(&self, body: &Value) -> Result<Option<ModelStreamEvent>, String>;
    fn parse_stream_terminal_response(&self, body: &Value)
    -> Result<Option<ModelResponse>, String>;
    fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage>;
}
#[cfg(test)]
pub mod registry {
    pub use super::ModelProtocolAdapter;
}
macro_rules! adapter {
    ($name:ident,$kind:ident) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name;
        impl ModelProtocolAdapter for $name {
            fn protocol(&self) -> AgentModelProtocol {
                AgentModelProtocol::$kind
            }
            fn default_path(&self) -> &'static str {
                sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                    .unwrap()
                    .default_path()
            }
            fn request_body(
                &self,
                config: &AgentModelConfig,
                request: &ModelRequest,
            ) -> Result<Value, String> {
                sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                    .unwrap()
                    .request_body(&convert(config)?, &convert(request)?)
            }
            fn stream_request_body(
                &self,
                config: &AgentModelConfig,
                request: &ModelRequest,
            ) -> Result<Value, String> {
                sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                    .unwrap()
                    .stream_request_body(&convert(config)?, &convert(request)?)
            }
            fn parse_response(&self, body: &Value) -> Result<ModelResponse, String> {
                convert(
                    &sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                        .unwrap()
                        .parse_response(body)?,
                )
            }
            fn parse_stream_event(&self, body: &Value) -> Result<Option<ModelStreamEvent>, String> {
                sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                    .unwrap()
                    .parse_stream_event(body)?
                    .as_ref()
                    .map(convert)
                    .transpose()
            }
            fn parse_stream_terminal_response(
                &self,
                body: &Value,
            ) -> Result<Option<ModelResponse>, String> {
                sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                    .unwrap()
                    .parse_stream_terminal_response(body)?
                    .as_ref()
                    .map(convert)
                    .transpose()
            }
            fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage> {
                sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                    .unwrap()
                    .parse_usage(body)
                    .and_then(|usage| convert(&usage).ok())
            }
        }
    };
}
adapter!(ChatCompletionsAdapter, ChatCompletions);
adapter!(ResponsesAdapter, Responses);
adapter!(EmbeddingsAdapter, Embeddings);
pub fn adapter_for_protocol(
    protocol: AgentModelProtocol,
) -> Option<&'static dyn ModelProtocolAdapter> {
    Some(match protocol {
        AgentModelProtocol::ChatCompletions => &ChatCompletionsAdapter,
        AgentModelProtocol::Responses => &ResponsesAdapter,
        AgentModelProtocol::Embeddings => &EmbeddingsAdapter,
    })
}
pub fn chat_completions_request(config: &AgentModelConfig, request: &ModelRequest) -> Value {
    ChatCompletionsAdapter
        .request_body(config, request)
        .expect("serializable model DTO")
}
pub fn responses_request(config: &AgentModelConfig, request: &ModelRequest) -> Value {
    ResponsesAdapter
        .request_body(config, request)
        .expect("serializable model DTO")
}
pub fn embeddings_request(config: &AgentModelConfig, input: &str) -> Value {
    sdk::protocol::embeddings_request(&convert(config).expect("serializable model config"), input)
}

#[cfg(test)]
mod chat_completions_tests;
#[cfg(test)]
mod chat_tool_observation_tests;
#[cfg(test)]
mod externalized_tool_tests;
#[cfg(test)]
mod responses_stateful_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_call_argument_tests;
#[cfg(test)]
mod tool_call_id_tests;
#[cfg(test)]
mod usage_tests;
