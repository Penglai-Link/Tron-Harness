use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    domain::agent::{AgentError, AgentRunContext, AgentRunResult, ToolRegistry},
    shared::event_bus::EventBusHandle,
};

use super::{
    AgentInvocationError, ModelClient, ModelStreamSink, ReactAgentBuilder, SystemAgentCliKind,
    SystemAgentCliModelClient,
};

mod service_mode_runtime;

use service_mode_runtime::ServiceModeRuntime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdapterKey {
    MainReact,
    CodexExec,
    CodexAppServer,
    OpenCodeRun,
    OpenCodeServe,
    OpenCodeAcp,
}

impl RuntimeAdapterKey {
    pub fn code(self) -> &'static str {
        match self {
            Self::MainReact => "MAIN_REACT",
            Self::CodexExec => "CODEX_EXEC",
            Self::CodexAppServer => "CODEX_APP_SERVER",
            Self::OpenCodeRun => "OPENCODE_RUN",
            Self::OpenCodeServe => "OPENCODE_SERVE",
            Self::OpenCodeAcp => "OPENCODE_ACP",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "MAIN_REACT" => Some(Self::MainReact),
            "CODEX_EXEC" => Some(Self::CodexExec),
            "CODEX_APP_SERVER" => Some(Self::CodexAppServer),
            "OPENCODE_RUN" => Some(Self::OpenCodeRun),
            "OPENCODE_SERVE" => Some(Self::OpenCodeServe),
            "OPENCODE_ACP" => Some(Self::OpenCodeAcp),
            _ => None,
        }
    }

    pub fn descriptor(self) -> RuntimeAdapterDescriptor {
        match self {
            Self::MainReact => RuntimeAdapterDescriptor {
                key: self,
                name: "Main React Runtime",
                stage: RuntimeAdapterStage::Ga,
                capabilities: RuntimeCapabilities::main_react(),
            },
            Self::CodexExec => RuntimeAdapterDescriptor {
                key: self,
                name: "Codex Exec Runtime",
                stage: RuntimeAdapterStage::Ga,
                capabilities: RuntimeCapabilities::system_cli(),
            },
            Self::CodexAppServer => RuntimeAdapterDescriptor {
                key: self,
                name: "Codex App Server Runtime",
                stage: RuntimeAdapterStage::Ga,
                capabilities: RuntimeCapabilities::service_mode(),
            },
            Self::OpenCodeRun => RuntimeAdapterDescriptor {
                key: self,
                name: "OpenCode Run Runtime",
                stage: RuntimeAdapterStage::Ga,
                capabilities: RuntimeCapabilities::system_cli(),
            },
            Self::OpenCodeServe => RuntimeAdapterDescriptor {
                key: self,
                name: "OpenCode Serve Runtime",
                stage: RuntimeAdapterStage::Ga,
                capabilities: RuntimeCapabilities::service_mode(),
            },
            Self::OpenCodeAcp => RuntimeAdapterDescriptor {
                key: self,
                name: "OpenCode ACP Runtime",
                stage: RuntimeAdapterStage::Ga,
                capabilities: RuntimeCapabilities::service_mode(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdapterStage {
    Ga,
    Planned,
}

impl RuntimeAdapterStage {
    pub fn code(self) -> &'static str {
        match self {
            Self::Ga => "GA",
            Self::Planned => "PLANNED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeAdapterDescriptor {
    pub key: RuntimeAdapterKey,
    pub name: &'static str,
    pub stage: RuntimeAdapterStage,
    pub capabilities: RuntimeCapabilities,
}

pub fn runtime_adapter_descriptors() -> Vec<RuntimeAdapterDescriptor> {
    [
        RuntimeAdapterKey::MainReact,
        RuntimeAdapterKey::CodexExec,
        RuntimeAdapterKey::CodexAppServer,
        RuntimeAdapterKey::OpenCodeRun,
        RuntimeAdapterKey::OpenCodeServe,
        RuntimeAdapterKey::OpenCodeAcp,
    ]
    .into_iter()
    .map(RuntimeAdapterKey::descriptor)
    .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    pub supports_resume: bool,
    pub supports_cancel: bool,
    pub supports_streaming: bool,
    pub supports_ephemeral: bool,
    pub supports_workdir: bool,
}

impl RuntimeCapabilities {
    fn main_react() -> Self {
        Self {
            supports_resume: false,
            supports_cancel: false,
            supports_streaming: true,
            supports_ephemeral: true,
            supports_workdir: true,
        }
    }

    fn system_cli() -> Self {
        Self {
            supports_resume: true,
            supports_cancel: false,
            supports_streaming: true,
            supports_ephemeral: false,
            supports_workdir: true,
        }
    }

    fn service_mode() -> Self {
        Self {
            supports_resume: true,
            supports_cancel: true,
            supports_streaming: true,
            supports_ephemeral: false,
            supports_workdir: true,
        }
    }
}

pub trait AgentRuntime: Send + Sync {
    fn key(&self) -> RuntimeAdapterKey;
    fn capabilities(&self) -> RuntimeCapabilities;

    fn start<'a>(
        &'a self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>;

    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>;

    fn cancel<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>>;
}

#[derive(Debug, Clone)]
pub enum RuntimeAdapterSelection {
    MainReact,
    CodexExec {
        command_name: String,
        working_directory: Option<String>,
    },
    CodexAppServer {
        endpoint: String,
        working_directory: Option<String>,
    },
    OpenCodeRun {
        command_name: String,
        working_directory: Option<String>,
    },
    OpenCodeServe {
        endpoint: String,
        working_directory: Option<String>,
    },
    OpenCodeAcp {
        endpoint: String,
        working_directory: Option<String>,
    },
}

#[derive(Clone)]
pub struct AgentRuntimeRegistry {
    default_model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
}

impl AgentRuntimeRegistry {
    pub fn new(
        default_model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            default_model_client,
            tool_registry,
            event_bus,
        }
    }

    pub fn resolve(&self, selection: RuntimeAdapterSelection) -> Arc<dyn AgentRuntime> {
        match selection {
            RuntimeAdapterSelection::MainReact => Arc::new(ReactAgentRuntime::new(
                RuntimeAdapterKey::MainReact,
                RuntimeCapabilities::main_react(),
                self.default_model_client.clone(),
                self.tool_registry.clone(),
                self.event_bus.clone(),
            )),
            RuntimeAdapterSelection::CodexExec {
                command_name,
                working_directory,
            } => Arc::new(ReactAgentRuntime::new(
                RuntimeAdapterKey::CodexExec,
                RuntimeCapabilities::system_cli(),
                Arc::new(SystemAgentCliModelClient::new(
                    SystemAgentCliKind::Codex,
                    command_name,
                    working_directory,
                )),
                self.tool_registry.clone(),
                self.event_bus.clone(),
            )),
            RuntimeAdapterSelection::OpenCodeRun {
                command_name,
                working_directory,
            } => Arc::new(ReactAgentRuntime::new(
                RuntimeAdapterKey::OpenCodeRun,
                RuntimeCapabilities::system_cli(),
                Arc::new(SystemAgentCliModelClient::new(
                    SystemAgentCliKind::OpenCode,
                    command_name,
                    working_directory,
                )),
                self.tool_registry.clone(),
                self.event_bus.clone(),
            )),
            RuntimeAdapterSelection::CodexAppServer {
                endpoint,
                working_directory,
            } => Arc::new(ServiceModeRuntime::new(
                RuntimeAdapterKey::CodexAppServer,
                RuntimeCapabilities::service_mode(),
                endpoint,
                working_directory,
            )),
            RuntimeAdapterSelection::OpenCodeServe {
                endpoint,
                working_directory,
            } => Arc::new(ServiceModeRuntime::new(
                RuntimeAdapterKey::OpenCodeServe,
                RuntimeCapabilities::service_mode(),
                endpoint,
                working_directory,
            )),
            RuntimeAdapterSelection::OpenCodeAcp {
                endpoint,
                working_directory,
            } => Arc::new(ServiceModeRuntime::new(
                RuntimeAdapterKey::OpenCodeAcp,
                RuntimeCapabilities::service_mode(),
                endpoint,
                working_directory,
            )),
        }
    }
}

#[derive(Clone)]
struct ReactAgentRuntime {
    key: RuntimeAdapterKey,
    capabilities: RuntimeCapabilities,
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    event_bus: EventBusHandle,
}

impl ReactAgentRuntime {
    fn new(
        key: RuntimeAdapterKey,
        capabilities: RuntimeCapabilities,
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            key,
            capabilities,
            model_client,
            tool_registry,
            event_bus,
        }
    }

    fn run<'a>(
        &'a self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            let agent = ReactAgentBuilder::new(
                self.model_client.clone(),
                self.tool_registry.clone(),
                self.event_bus.clone(),
            )
            .build()
            .map_err(map_agent_error)?;
            match sink {
                Some(sink) => agent.run_stream(context, sink).await,
                None => agent.run(context).await,
            }
            .map_err(map_agent_error)
        })
    }
}

impl AgentRuntime for ReactAgentRuntime {
    fn key(&self) -> RuntimeAdapterKey {
        self.key
    }

    fn capabilities(&self) -> RuntimeCapabilities {
        self.capabilities
    }

    fn start<'a>(
        &'a self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        self.run(context, sink)
    }

    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        mut context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        if !self.capabilities.supports_resume {
            return Box::pin(async move {
                Err(AgentInvocationError::RuntimeFailed(format!(
                    "runtime adapter `{}` does not support resume",
                    self.key.code()
                )))
            });
        }
        context.system_agent_session_id = Some(session_id.to_string());
        self.run(context, sink)
    }

    fn cancel<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        Box::pin(async move {
            Err(AgentInvocationError::RuntimeFailed(format!(
                "runtime adapter `{}` does not support cancel for run `{run_code}`",
                self.key.code()
            )))
        })
    }
}

fn map_agent_error(error: AgentError) -> AgentInvocationError {
    match error {
        AgentError::ModelFailure(message) => AgentInvocationError::ModelFailed(message),
        AgentError::ToolFailure(message)
        | AgentError::MissingTool(message)
        | AgentError::UnauthorizedTool(message) => AgentInvocationError::ToolFailed(message),
        AgentError::MaxStepsExceeded => AgentInvocationError::MaxStepsExceeded,
        AgentError::InvalidConfiguration(message) | AgentError::EventPublishFailed(message) => {
            AgentInvocationError::RuntimeFailed(message)
        }
    }
}
