use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{
    application::chat_auto_model_router::ChatAutoModelRouter,
    application::short_term_memory::{
        ModelResponseTranscriptFacts, PreparedModelContext, ShortTermMemoryService,
    },
    domain::agent::{
        AgentError, AgentModelConfig, AgentRunContext, AgentRunResult, ModelRequest, ModelResponse,
        ModelStreamEvent, ModelStreamEventKind, ModelTokenUsage, ToolRegistry,
    },
    shared::event_bus::{BusEventDraft, BusOutcome, BusRequestRef, EventBusHandle, PublishOutcome},
};

use super::{
    ActiveRunControl, TurnStateAdapter, TurnStateKind, request_estimate_snapshot,
    usage_audit_snapshot, usage_event_payload,
};

#[path = "../runtime_tool_calls.rs"]
mod runtime_tool_calls;

#[path = "../runtime_helpers.rs"]
mod runtime_helpers;

#[path = "../runtime_compaction.rs"]
mod runtime_compaction;

#[path = "../runtime_model_request.rs"]
mod runtime_model_request;

#[path = "../runtime_steering.rs"]
mod runtime_steering;

use runtime_helpers::{
    agent_kind_label, compact_event_payload_value, compact_event_value,
    model_stream_delta_payloads, record_failure_on_short_term_memory_error,
    short_term_memory_error, validate_context,
};
use runtime_model_request::{model_request_facts, prepared_context_event_payload};
use runtime_steering::RuntimeSteering;

pub type ModelStreamSink = Arc<dyn Fn(ModelStreamEvent) + Send + Sync>;

pub trait ModelClient: Send + Sync {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>>;

    fn generate_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        sink: ModelStreamSink,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let response = self.generate(config, request).await?;
            let output = response.output_text();
            if response.finish == crate::domain::agent::ModelFinish::Final && !output.is_empty() {
                sink(ModelStreamEvent::text_delta(output));
            }
            Ok(response)
        })
    }
}

pub struct ReactAgentBuilder {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
    short_term_memory: Option<ShortTermMemoryService>,
    auto_model_router: Option<ChatAutoModelRouter>,
}

impl ReactAgentBuilder {
    pub fn new(
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            model_client,
            tool_registry,
            event_bus,
            short_term_memory: None,
            auto_model_router: None,
        }
    }

    pub fn with_short_term_memory(mut self, service: Option<ShortTermMemoryService>) -> Self {
        self.short_term_memory = service;
        self
    }

    pub fn with_auto_model_router(mut self, router: Option<ChatAutoModelRouter>) -> Self {
        self.auto_model_router = router;
        self
    }

    pub fn build(self) -> Result<ReactAgent, AgentError> {
        Ok(ReactAgent {
            model_client: self.model_client,
            tool_registry: self.tool_registry,
            event_bus: self.event_bus,
            short_term_memory: self.short_term_memory,
            auto_model_router: self.auto_model_router,
        })
    }
}

#[derive(Clone)]
pub struct ReactAgent {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
    short_term_memory: Option<ShortTermMemoryService>,
    auto_model_router: Option<ChatAutoModelRouter>,
}
