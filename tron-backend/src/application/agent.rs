mod agui_stream;
pub(crate) mod agui_translate;
mod canonical_event;
mod dto;
mod error;
mod history;
mod invocation;
mod management;
mod orchestrator;
mod port;
mod runtime;
mod runtime_adapter;
mod runtime_capability;
mod system_agent_model_probe;
mod system_cli_model_client;
mod system_cli_output_parser;

pub use agui_stream::{AgentAguiReplayStore, AgentAguiStreamError, AgentAguiStreamService};
pub use canonical_event::CanonicalRuntimeEvent;
pub use dto::{AgentInvocationCommand, AgentInvocationResult};
pub use error::AgentInvocationError;
pub use history::AgentRunHistoryService;
pub use invocation::{AgentInvocationService, generate_agent_run_code};
pub use management::{
    AgentManagementRepository, AgentManagementService, ResolvedSystemAgentRuntime,
    UpdatePlatformMainAgentCommand,
};
pub use orchestrator::{AgentOrchestrator, OrchestratedRunResult, RunLifecycleState};
pub use port::AgentRunHistoryRepository;
pub use runtime::{ModelClient, ModelStreamSink, ReactAgentBuilder};
pub use runtime_adapter::{
    AgentRuntime, AgentRuntimeRegistry, RuntimeAdapterDescriptor, RuntimeAdapterKey,
    RuntimeAdapterSelection, RuntimeAdapterStage, RuntimeCapabilities, runtime_adapter_descriptors,
};
pub(crate) use system_cli_model_client::{SystemAgentCliKind, SystemAgentCliModelClient};
