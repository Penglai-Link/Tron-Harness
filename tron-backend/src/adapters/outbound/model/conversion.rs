//! Field-exact bridges between Tron's domain DTOs and the model SDK DTOs.
//!
//! The two type systems mirror each other by design; these hand-written
//! conversions replace the previous serde `Value` round trip, which serialized
//! the entire conversation (media base64 included) only to deserialize it back
//! into the SDK shape. Field parity is now enforced by the compiler instead of
//! trusted to runtime JSON.
//!
//! Semantics intentionally preserved from the serde round trip:
//! - `ModelRequest::prepared_tool_schema` is run-local and never crosses the
//!   boundary (`#[serde(skip)]` previously dropped it).
//! - The SDK `EffectiveTool` is a projection: domain-only fields
//!   (`domain_code`, `skill_name`, `required_variable_keys`) are dropped, as
//!   serde ignored them.
use crate::domain::agent::{
    AgentMessage, AgentMessagePart, AgentModelConfig, EffectiveTool, ModelFinish, ModelOutputItem,
    ModelRequest, ModelResponse, ModelStreamEvent, ModelStreamEventKind, ModelTokenUsage,
};
use crate::domain::reasoning::ReasoningEffort;
use eliteforge_agent_official_plugin_model_http as sdk;

use crate::domain::agent::AgentModelProtocol;

// ---------------------------------------------------------------------------
// Shared enums
// ---------------------------------------------------------------------------

impl From<AgentModelProtocol> for sdk::AgentModelProtocol {
    fn from(protocol: AgentModelProtocol) -> Self {
        match protocol {
            AgentModelProtocol::ChatCompletions => Self::ChatCompletions,
            AgentModelProtocol::Responses => Self::Responses,
            AgentModelProtocol::Embeddings => Self::Embeddings,
            AgentModelProtocol::Anthropic => Self::Anthropic,
            AgentModelProtocol::Gemini => Self::Gemini,
        }
    }
}

impl From<sdk::AgentModelProtocol> for AgentModelProtocol {
    fn from(protocol: sdk::AgentModelProtocol) -> Self {
        match protocol {
            sdk::AgentModelProtocol::ChatCompletions => Self::ChatCompletions,
            sdk::AgentModelProtocol::Responses => Self::Responses,
            sdk::AgentModelProtocol::Embeddings => Self::Embeddings,
            sdk::AgentModelProtocol::Anthropic => Self::Anthropic,
            sdk::AgentModelProtocol::Gemini => Self::Gemini,
        }
    }
}

fn sdk_reasoning_effort(effort: ReasoningEffort) -> sdk::ReasoningEffort {
    match effort {
        ReasoningEffort::None => sdk::ReasoningEffort::None,
        ReasoningEffort::Minimal => sdk::ReasoningEffort::Minimal,
        ReasoningEffort::Low => sdk::ReasoningEffort::Low,
        ReasoningEffort::Medium => sdk::ReasoningEffort::Medium,
        ReasoningEffort::High => sdk::ReasoningEffort::High,
        ReasoningEffort::XHigh => sdk::ReasoningEffort::XHigh,
        ReasoningEffort::Max => sdk::ReasoningEffort::Max,
        ReasoningEffort::Ultra => sdk::ReasoningEffort::Ultra,
    }
}

fn domain_reasoning_effort(effort: sdk::ReasoningEffort) -> ReasoningEffort {
    match effort {
        sdk::ReasoningEffort::None => ReasoningEffort::None,
        sdk::ReasoningEffort::Minimal => ReasoningEffort::Minimal,
        sdk::ReasoningEffort::Low => ReasoningEffort::Low,
        sdk::ReasoningEffort::Medium => ReasoningEffort::Medium,
        sdk::ReasoningEffort::High => ReasoningEffort::High,
        sdk::ReasoningEffort::XHigh => ReasoningEffort::XHigh,
        sdk::ReasoningEffort::Max => ReasoningEffort::Max,
        sdk::ReasoningEffort::Ultra => ReasoningEffort::Ultra,
    }
}

fn domain_finish(finish: sdk::ModelFinish) -> ModelFinish {
    match finish {
        sdk::ModelFinish::Final => ModelFinish::Final,
        sdk::ModelFinish::ToolCalls => ModelFinish::ToolCalls,
        sdk::ModelFinish::Continue => ModelFinish::Continue,
    }
}

fn domain_stream_kind(kind: sdk::ModelStreamEventKind) -> ModelStreamEventKind {
    match kind {
        sdk::ModelStreamEventKind::Text => ModelStreamEventKind::Text,
        sdk::ModelStreamEventKind::Reasoning => ModelStreamEventKind::Reasoning,
        sdk::ModelStreamEventKind::ReasoningSummary => ModelStreamEventKind::ReasoningSummary,
        sdk::ModelStreamEventKind::MessageStart => ModelStreamEventKind::MessageStart,
        sdk::ModelStreamEventKind::MessageEnd => ModelStreamEventKind::MessageEnd,
        sdk::ModelStreamEventKind::ToolCallDelta => ModelStreamEventKind::ToolCallDelta,
        sdk::ModelStreamEventKind::ToolBoundary => ModelStreamEventKind::ToolBoundary,
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

impl From<&AgentModelConfig> for sdk::AgentModelConfig {
    fn from(config: &AgentModelConfig) -> Self {
        Self {
            model_code: config.model_code.clone(),
            protocol: config.protocol.into(),
            base_url: config.base_url.clone(),
            api_path: config.api_path.clone(),
            api_key: config.api_key.clone(),
            request_headers: config.request_headers.clone(),
            model_name: config.model_name.clone(),
            thinking_enabled: config.thinking_enabled,
            supports_developer_role: config.supports_developer_role,
            reasoning_effort: config.reasoning_effort.map(sdk_reasoning_effort),
            responses_stateful_enabled: config.responses_stateful_enabled,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            context_window_tokens: config.context_window_tokens,
            top_p: config.top_p,
            max_retries: config.max_retries,
        }
    }
}

impl From<&sdk::AgentModelConfig> for AgentModelConfig {
    fn from(config: &sdk::AgentModelConfig) -> Self {
        Self {
            model_code: config.model_code.clone(),
            protocol: config.protocol.into(),
            base_url: config.base_url.clone(),
            api_path: config.api_path.clone(),
            api_key: config.api_key.clone(),
            request_headers: config.request_headers.clone(),
            model_name: config.model_name.clone(),
            thinking_enabled: config.thinking_enabled,
            supports_developer_role: config.supports_developer_role,
            reasoning_effort: config.reasoning_effort.map(domain_reasoning_effort),
            responses_stateful_enabled: config.responses_stateful_enabled,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            context_window_tokens: config.context_window_tokens,
            top_p: config.top_p,
            max_retries: config.max_retries,
        }
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

impl From<&AgentMessagePart> for sdk::AgentMessagePart {
    fn from(part: &AgentMessagePart) -> Self {
        match part {
            AgentMessagePart::Text { text } => Self::Text { text: text.clone() },
            AgentMessagePart::Reasoning { text } => Self::Reasoning { text: text.clone() },
            AgentMessagePart::ReasoningSummary { text } => {
                Self::ReasoningSummary { text: text.clone() }
            }
            AgentMessagePart::ToolCall {
                call_id,
                tool_name,
                input,
                input_error,
            } => Self::ToolCall {
                call_id: call_id.clone(),
                tool_name: tool_name.clone(),
                input: input.clone(),
                input_error: input_error.clone(),
            },
            AgentMessagePart::ToolResult {
                call_id,
                tool_name,
                output,
                externalized,
                metadata,
            } => Self::ToolResult {
                call_id: call_id.clone(),
                tool_name: tool_name.clone(),
                output: output.clone(),
                externalized: *externalized,
                metadata: metadata.clone(),
            },
            AgentMessagePart::Context {
                kind,
                text,
                metadata,
            } => Self::Context {
                kind: kind.clone(),
                text: text.clone(),
                metadata: metadata.clone(),
            },
            AgentMessagePart::Image { mime_type, data } => Self::Image {
                mime_type: mime_type.clone(),
                data: data.clone(),
            },
            AgentMessagePart::CompactionCheckpoint {
                summary,
                recent,
                source_start_seq,
                source_end_seq,
            } => Self::CompactionCheckpoint {
                summary: summary.clone(),
                recent: recent.clone(),
                source_start_seq: *source_start_seq,
                source_end_seq: *source_end_seq,
            },
        }
    }
}

impl From<AgentMessagePart> for sdk::AgentMessagePart {
    fn from(part: AgentMessagePart) -> Self {
        match part {
            AgentMessagePart::Text { text } => Self::Text { text },
            AgentMessagePart::Reasoning { text } => Self::Reasoning { text },
            AgentMessagePart::ReasoningSummary { text } => Self::ReasoningSummary { text },
            AgentMessagePart::ToolCall {
                call_id,
                tool_name,
                input,
                input_error,
            } => Self::ToolCall {
                call_id,
                tool_name,
                input,
                input_error,
            },
            AgentMessagePart::ToolResult {
                call_id,
                tool_name,
                output,
                externalized,
                metadata,
            } => Self::ToolResult {
                call_id,
                tool_name,
                output,
                externalized,
                metadata,
            },
            AgentMessagePart::Context {
                kind,
                text,
                metadata,
            } => Self::Context {
                kind,
                text,
                metadata,
            },
            AgentMessagePart::Image { mime_type, data } => Self::Image { mime_type, data },
            AgentMessagePart::CompactionCheckpoint {
                summary,
                recent,
                source_start_seq,
                source_end_seq,
            } => Self::CompactionCheckpoint {
                summary,
                recent,
                source_start_seq,
                source_end_seq,
            },
        }
    }
}

impl From<sdk::AgentMessagePart> for AgentMessagePart {
    fn from(part: sdk::AgentMessagePart) -> Self {
        match part {
            sdk::AgentMessagePart::Text { text } => Self::Text { text },
            sdk::AgentMessagePart::Reasoning { text } => Self::Reasoning { text },
            sdk::AgentMessagePart::ReasoningSummary { text } => Self::ReasoningSummary { text },
            sdk::AgentMessagePart::ToolCall {
                call_id,
                tool_name,
                input,
                input_error,
            } => Self::ToolCall {
                call_id,
                tool_name,
                input,
                input_error,
            },
            sdk::AgentMessagePart::ToolResult {
                call_id,
                tool_name,
                output,
                externalized,
                metadata,
            } => Self::ToolResult {
                call_id,
                tool_name,
                output,
                externalized,
                metadata,
            },
            sdk::AgentMessagePart::Context {
                kind,
                text,
                metadata,
            } => Self::Context {
                kind,
                text,
                metadata,
            },
            sdk::AgentMessagePart::Image { mime_type, data } => Self::Image { mime_type, data },
            sdk::AgentMessagePart::CompactionCheckpoint {
                summary,
                recent,
                source_start_seq,
                source_end_seq,
            } => Self::CompactionCheckpoint {
                summary,
                recent,
                source_start_seq,
                source_end_seq,
            },
        }
    }
}

impl From<&AgentMessage> for sdk::AgentMessage {
    fn from(message: &AgentMessage) -> Self {
        Self {
            // Both sides share the framework annotation type.
            annotations: message.annotations.clone(),
            instruction_origin: message.instruction_origin,
            role: message.role.clone(),
            phase: message.phase.clone(),
            output_group_id: message.output_group_id.clone(),
            parts: message
                .parts
                .iter()
                .map(sdk::AgentMessagePart::from)
                .collect(),
        }
    }
}

impl From<AgentMessage> for sdk::AgentMessage {
    fn from(message: AgentMessage) -> Self {
        Self {
            annotations: message.annotations,
            instruction_origin: message.instruction_origin,
            role: message.role,
            phase: message.phase,
            output_group_id: message.output_group_id,
            parts: message.parts.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<sdk::AgentMessage> for AgentMessage {
    fn from(message: sdk::AgentMessage) -> Self {
        Self {
            annotations: message.annotations,
            instruction_origin: message.instruction_origin,
            role: message.role,
            phase: message.phase,
            output_group_id: message.output_group_id,
            parts: message.parts.into_iter().map(Into::into).collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

fn sdk_request_parts(request: &ModelRequest) -> (Vec<sdk::AgentMessage>, Vec<sdk::EffectiveTool>) {
    (
        request
            .messages
            .iter()
            .map(sdk::AgentMessage::from)
            .collect(),
        request
            .tools
            .iter()
            .map(|tool| sdk::EffectiveTool {
                tool_name: tool.tool_name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
            })
            .collect(),
    )
}

impl From<&ModelRequest> for sdk::ModelRequest {
    fn from(request: &ModelRequest) -> Self {
        let (messages, tools) = sdk_request_parts(request);
        Self {
            messages,
            tools,
            // Run-local cache; the SDK prepares its own schema.
            prepared_tool_schema: None,
            system_agent_session_id: request.system_agent_session_id.clone(),
            response_input_items: request.response_input_items.clone(),
            previous_response_id: request.previous_response_id.clone(),
        }
    }
}

impl From<ModelRequest> for sdk::ModelRequest {
    fn from(request: ModelRequest) -> Self {
        Self {
            messages: request.messages.into_iter().map(Into::into).collect(),
            tools: request
                .tools
                .into_iter()
                .map(|tool| sdk::EffectiveTool {
                    tool_name: tool.tool_name,
                    description: tool.description,
                    input_schema: tool.input_schema,
                })
                .collect(),
            prepared_tool_schema: None,
            system_agent_session_id: request.system_agent_session_id,
            response_input_items: request.response_input_items,
            previous_response_id: request.previous_response_id,
        }
    }
}

impl From<sdk::ModelRequest> for ModelRequest {
    fn from(request: sdk::ModelRequest) -> Self {
        Self {
            messages: request.messages.into_iter().map(Into::into).collect(),
            tools: request
                .tools
                .into_iter()
                .map(|tool| EffectiveTool {
                    tool_name: tool.tool_name,
                    domain_code: None,
                    description: tool.description,
                    input_schema: tool.input_schema,
                    skill_name: None,
                    required_variable_keys: Vec::new(),
                })
                .collect(),
            prepared_tool_schema: None,
            system_agent_session_id: request.system_agent_session_id,
            response_input_items: request.response_input_items,
            previous_response_id: request.previous_response_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Responses and stream events
// ---------------------------------------------------------------------------

impl From<sdk::ModelOutputItem> for ModelOutputItem {
    fn from(item: sdk::ModelOutputItem) -> Self {
        match item {
            sdk::ModelOutputItem::Message {
                output_index,
                item_id,
                phase,
                text,
            } => Self::Message {
                output_index,
                item_id,
                phase,
                text,
            },
            sdk::ModelOutputItem::ReasoningSummary { output_index, text } => {
                Self::ReasoningSummary { output_index, text }
            }
            sdk::ModelOutputItem::ToolCall { output_index, call } => Self::ToolCall {
                output_index,
                call: crate::domain::agent::ToolCall {
                    call_id: call.call_id,
                    tool_name: call.tool_name,
                    input: call.input,
                    input_error: call.input_error,
                },
            },
        }
    }
}
impl From<ModelOutputItem> for sdk::ModelOutputItem {
    fn from(item: ModelOutputItem) -> Self {
        match item {
            ModelOutputItem::Message {
                output_index,
                item_id,
                phase,
                text,
            } => Self::Message {
                output_index,
                item_id,
                phase,
                text,
            },
            ModelOutputItem::ReasoningSummary { output_index, text } => {
                Self::ReasoningSummary { output_index, text }
            }
            ModelOutputItem::ToolCall { output_index, call } => Self::ToolCall {
                output_index,
                call: sdk::ToolCall {
                    call_id: call.call_id,
                    tool_name: call.tool_name,
                    input: call.input,
                    input_error: call.input_error,
                },
            },
        }
    }
}

impl From<sdk::ModelFinish> for ModelFinish {
    fn from(finish: sdk::ModelFinish) -> Self {
        domain_finish(finish)
    }
}

impl From<ModelFinish> for sdk::ModelFinish {
    fn from(finish: ModelFinish) -> Self {
        match finish {
            ModelFinish::Final => Self::Final,
            ModelFinish::ToolCalls => Self::ToolCalls,
            ModelFinish::Continue => Self::Continue,
        }
    }
}

impl From<sdk::ModelTokenUsage> for ModelTokenUsage {
    fn from(usage: sdk::ModelTokenUsage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            cached_input_tokens: usage.cached_input_tokens,
            reasoning_tokens: usage.reasoning_tokens,
        }
    }
}

impl From<ModelTokenUsage> for sdk::ModelTokenUsage {
    fn from(usage: ModelTokenUsage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            cached_input_tokens: usage.cached_input_tokens,
            reasoning_tokens: usage.reasoning_tokens,
        }
    }
}

impl From<sdk::ModelResponse> for ModelResponse {
    fn from(response: sdk::ModelResponse) -> Self {
        Self {
            assistant: response.assistant.into(),
            finish: response.finish.into(),
            output_items: response.output_items.into_iter().map(Into::into).collect(),
            token_usage: response.token_usage.map(Into::into),
            system_agent_session_id: response.system_agent_session_id,
            response_id: response.response_id,
        }
    }
}

impl From<ModelResponse> for sdk::ModelResponse {
    fn from(response: ModelResponse) -> Self {
        Self {
            assistant: response.assistant.into(),
            finish: response.finish.into(),
            output_items: response.output_items.into_iter().map(Into::into).collect(),
            token_usage: response.token_usage.map(Into::into),
            system_agent_session_id: response.system_agent_session_id,
            response_id: response.response_id,
        }
    }
}

impl From<sdk::ModelStreamEvent> for ModelStreamEvent {
    fn from(event: sdk::ModelStreamEvent) -> Self {
        Self {
            kind: domain_stream_kind(event.kind),
            delta: event.delta,
            output_index: event.output_index,
            item_id: event.item_id,
            phase: event.phase,
        }
    }
}
