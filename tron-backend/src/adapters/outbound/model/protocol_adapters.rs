//! Product DTO conversion only. All wire request construction and response
//! parsing belong to the public SDK model plugin.
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
                    .request_body(
                        &sdk::AgentModelConfig::from(config),
                        &sdk::ModelRequest::from(request),
                    )
            }
            fn stream_request_body(
                &self,
                config: &AgentModelConfig,
                request: &ModelRequest,
            ) -> Result<Value, String> {
                sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                    .unwrap()
                    .stream_request_body(
                        &sdk::AgentModelConfig::from(config),
                        &sdk::ModelRequest::from(request),
                    )
            }
            fn parse_response(&self, body: &Value) -> Result<ModelResponse, String> {
                Ok(ModelResponse::from(
                    sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                        .unwrap()
                        .parse_response(body)?,
                ))
            }
            fn parse_stream_event(&self, body: &Value) -> Result<Option<ModelStreamEvent>, String> {
                Ok(
                    sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                        .unwrap()
                        .parse_stream_event(body)?
                        .map(ModelStreamEvent::from),
                )
            }
            fn parse_stream_terminal_response(
                &self,
                body: &Value,
            ) -> Result<Option<ModelResponse>, String> {
                Ok(
                    sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                        .unwrap()
                        .parse_stream_terminal_response(body)?
                        .map(ModelResponse::from),
                )
            }
            fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage> {
                sdk::protocol::adapter_for_protocol(sdk::AgentModelProtocol::$kind)
                    .unwrap()
                    .parse_usage(body)
                    .map(ModelTokenUsage::from)
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
        // These protocols are executed by the SDK's GenAI adapter. They do not
        // expose a second product-level wire encoder or response parser.
        AgentModelProtocol::Anthropic | AgentModelProtocol::Gemini => return None,
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
    sdk::protocol::embeddings_request(&sdk::AgentModelConfig::from(config), input)
}
