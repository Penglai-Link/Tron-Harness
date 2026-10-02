use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{
    application::model_router::AgentModelRouter,
    application::short_term_memory::{
        ModelResponseTranscriptFacts, PreparedModelContext, ShortTermMemoryService,
    },
    domain::agent::{
        AgentError, AgentModelConfig, AgentRunContext, AgentRunResult, ModelRequest, ModelResponse,
        ModelStreamEvent, ModelTokenUsage, ToolRegistry,
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
    record_failure_on_short_term_memory_error,
    short_term_memory_error, validate_context,
};
use runtime_model_request::{model_request_facts, prepared_context_event_payload};

pub type ModelStreamSink = Arc<dyn Fn(ModelStreamEvent) + Send + Sync>;

pub trait ModelClient: Send + Sync {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>>;

    /// Bind immutable authorization for a native run. Stateless adapters may
    /// return None; credential-backed adapters must capture the account scope.
    fn bind_for_run(&self,_:&AgentModelConfig)->Result<Option<Arc<dyn ModelClient>>,String> {Ok(None)}
    fn authorization_scope(&self,_:&AgentModelConfig)->Result<Option<String>,String> {Ok(None)}
    fn close_observed<'a>(&'a self)->Pin<Box<dyn Future<Output=Result<eliteforge_agent_plugin_core::ShutdownReport,String>>+Send+'a>> {Box::pin(async {Err("model client does not declare lifecycle ownership".into())})}
    fn generate_observed<'a>(&'a self,_:&'a AgentModelConfig,_:&'a ModelRequest)->Pin<Box<dyn Future<Output=eliteforge_agent_framework::ObservedModelResult<ModelResponse>>+Send+'a>> {Box::pin(async {eliteforge_agent_framework::ObservedModelResult{result:Err(eliteforge_agent_core::AgentError::InvalidConfiguration("observed model invocation is unavailable".into())),report:Default::default()}})}
    fn generate_goal_judge_child<'a>(&'a self,_:&'a AgentModelConfig,_:&'a ModelRequest,_:Arc<dyn eliteforge_agent_framework::ModelJobRepair>,_:usize,_:&'a eliteforge_agent_core::RunContext)->Pin<Box<dyn Future<Output=eliteforge_agent_framework::ObservedModelResult<ModelResponse>>+Send+'a>> {Box::pin(async {eliteforge_agent_framework::ObservedModelResult{result:Err(eliteforge_agent_core::AgentError::InvalidConfiguration("model client does not support native child jobs".into())),report:Default::default()}})}
    /// Exact admitted native content is separate from the product audit projection.
    fn generate_native_once<'a>(&'a self,config:&'a AgentModelConfig,_:&'a eliteforge_agent_core::ChatRequest,product:&'a ModelRequest,sink:Option<ModelStreamSink>)->Pin<Box<dyn Future<Output=Result<ModelResponse,String>>+Send+'a>> { match sink {Some(sink)=>self.generate_stream(config,product,sink),None=>self.generate_once(config,product)} }
    fn compact_memory<'a>(&'a self, _: &'a AgentModelConfig, _: &'a [String], _:Option<&'a str>)->Pin<Box<dyn Future<Output=Result<eliteforge_agent_memory::MemoryCompactionResult,String>>+Send+'a>> {Box::pin(async {Err("memory compaction is unavailable".into())})}
    fn on_model_failure(&self, _: &AgentModelConfig, _: &eliteforge_agent_core::ModelError) {}
    /// Used only inside an SDK-controlled turn or auxiliary call.
    fn generate_once<'a>(&'a self, config: &'a AgentModelConfig, request: &'a ModelRequest) -> Pin<Box<dyn Future<Output = Result<ModelResponse,String>> + Send + 'a>> { let _=(config,request); Box::pin(async {Err("single-attempt model invocation must be explicitly implemented".into())}) }
    fn recover_authentication<'a>(&'a self, _: &'a AgentModelConfig, _: &'a eliteforge_agent_core::ModelError) -> Pin<Box<dyn Future<Output=Result<bool,eliteforge_agent_core::ModelError>> + Send + 'a>> { Box::pin(async { Ok(false) }) }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output=()> + Send + 'a>>;
    fn generate_with_repair<'a>(&'a self, config: &'a AgentModelConfig, request: &'a ModelRequest, _: Arc<dyn eliteforge_agent_framework::ModelJobRepair>, _: usize) -> Pin<Box<dyn Future<Output=Result<ModelResponse,String>> + Send + 'a>> { let _=(config,request); Box::pin(async {Err("controlled repair must be explicitly implemented".into())}) }

    fn generate_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        sink: ModelStreamSink,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let response = self.generate_once(config, request).await?;
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
    directory_inspector:
        Option<Arc<dyn crate::application::project::ApplicationDirectoryInspector>>,
    short_term_memory: Option<ShortTermMemoryService>,
    auto_model_router: Option<AgentModelRouter>,
    framework_host: Option<super::FrameworkHost>,
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
            directory_inspector: None,
            short_term_memory: None,
            auto_model_router: None,
            framework_host: None,
        }
    }

    pub fn with_directory_inspector(
        mut self,
        inspector: Option<Arc<dyn crate::application::project::ApplicationDirectoryInspector>>,
    ) -> Self {
        self.directory_inspector = inspector;
        self
    }

    pub fn with_short_term_memory(mut self, service: Option<ShortTermMemoryService>) -> Self {
        self.short_term_memory = service;
        self
    }

    pub fn with_auto_model_router(mut self, router: Option<AgentModelRouter>) -> Self {
        self.auto_model_router = router;
        self
    }

    pub fn with_framework_host(mut self, host: Option<super::FrameworkHost>) -> Self {
        self.framework_host = host;
        self
    }

    pub fn build(self) -> Result<ReactAgent, AgentError> {
        Ok(ReactAgent {
            model_client: self.model_client,
            tool_registry: self.tool_registry,
            event_bus: self.event_bus,
            directory_inspector: self.directory_inspector,
            short_term_memory: self.short_term_memory,
            auto_model_router: self.auto_model_router,
            framework_host: Some(match self.framework_host { Some(host) => host, None => super::FrameworkHost::temporary(None)? }),
        })
    }
}

#[derive(Clone)]
pub struct ReactAgent {
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
    directory_inspector:
        Option<Arc<dyn crate::application::project::ApplicationDirectoryInspector>>,
    short_term_memory: Option<ShortTermMemoryService>,
    auto_model_router: Option<AgentModelRouter>,
    framework_host: Option<super::FrameworkHost>,
}
