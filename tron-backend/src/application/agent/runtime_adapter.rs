use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    application::short_term_memory::ShortTermMemoryService,
    domain::agent::{AgentRunContext, AgentRunResult, ToolRegistry},
    shared::event_bus::EventBusHandle,
};

use super::{ActiveRunRegistry, AgentInvocationError, ModelClient, ModelStreamSink};

mod react_runtime;
mod service_mode_runtime;
mod system_cli_runtime;

use react_runtime::ReactRuntime;
use service_mode_runtime::ServiceModeRuntime;
pub(crate) use system_cli_runtime::SystemCliRuntime;

pub(crate) use system_cli_runtime::SystemAgentCliKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdapterKey {
    React,
    CodexExec,
    CodexAppServer,
    OpenCodeRun,
    DshHeadless,
    OpenCodeServe,
    OpenCodeAcp,
}

impl RuntimeAdapterKey {
    pub fn code(self) -> &'static str {
        match self {
            Self::React => "REACT",
            Self::CodexExec => "CODEX_EXEC",
            Self::CodexAppServer => "CODEX_APP_SERVER",
            Self::OpenCodeRun => "OPENCODE_RUN",
            Self::DshHeadless => "DSH_HEADLESS",
            Self::OpenCodeServe => "OPENCODE_SERVE",
            Self::OpenCodeAcp => "OPENCODE_ACP",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "REACT" => Some(Self::React),
            "CODEX_EXEC" => Some(Self::CodexExec),
            "CODEX_APP_SERVER" => Some(Self::CodexAppServer),
            "OPENCODE_RUN" => Some(Self::OpenCodeRun),
            "DSH_HEADLESS" => Some(Self::DshHeadless),
            "OPENCODE_SERVE" => Some(Self::OpenCodeServe),
            "OPENCODE_ACP" => Some(Self::OpenCodeAcp),
            _ => None,
        }
    }

    pub fn descriptor(self) -> RuntimeAdapterDescriptor {
        match self {
            Self::React => RuntimeAdapterDescriptor {
                key: self,
                name: "React Runtime",
                stage: RuntimeAdapterStage::Ga,
                capabilities: RuntimeCapabilities::react(),
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
            Self::DshHeadless => RuntimeAdapterDescriptor {
                key: self,
                name: "DeepSeek Harness Headless Runtime",
                stage: RuntimeAdapterStage::Ga,
                capabilities: RuntimeCapabilities::system_cli(),
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
        RuntimeAdapterKey::React,
        RuntimeAdapterKey::CodexExec,
        RuntimeAdapterKey::CodexAppServer,
        RuntimeAdapterKey::DshHeadless,
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
    pub supports_steer: bool,
    pub supports_streaming: bool,
    pub supports_ephemeral: bool,
    pub supports_workdir: bool,
}

impl RuntimeCapabilities {
    fn react() -> Self {
        Self {
            supports_resume: false,
            supports_cancel: true,
            supports_steer: true,
            supports_streaming: true,
            supports_ephemeral: true,
            supports_workdir: true,
        }
    }

    pub(crate) fn system_cli() -> Self {
        Self {
            supports_resume: true,
            supports_cancel: false,
            supports_steer: false,
            supports_streaming: true,
            supports_ephemeral: false,
            supports_workdir: true,
        }
    }

    fn service_mode() -> Self {
        Self {
            supports_resume: true,
            supports_cancel: true,
            supports_steer: false,
            supports_streaming: false,
            supports_ephemeral: false,
            supports_workdir: true,
        }
    }
}

pub trait AgentRuntime: Send + Sync {
    /// Native adapters must be drained after cancellation before product admission releases.
    fn owns_native_lifecycle(&self) -> bool {
        false
    }
    /// Framework-managed runs must never be replayed wholesale after side effects.
    fn key(&self) -> RuntimeAdapterKey;
    /// Stable transport configuration identity for durable remote-session reuse.
    fn session_scope(&self) -> Option<String> {
        None
    }
    /// Explicit transport directory, used only when the run has no assigned directory.
    fn configured_working_directory(&self) -> Option<String> {
        None
    }
    /// Native observations travel separately from model text and reasoning.
    fn run_observed<'a>(
        &'a self,
        session: Option<&'a str>,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
        _observations: Arc<dyn Fn(crate::domain::agent::ToolObservation) + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        match session {
            Some(session) => self.resume(session, context, sink),
            None => self.start(context, sink),
        }
    }
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
    React,
    CodexExec {
        command_name: String,
        working_directory: Option<String>,
    },
    CodexAppServer {
        endpoint: String,
        working_directory: Option<String>,
    },
    DshHeadless {
        command_name: String,
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
    runtime_http_client: reqwest::Client,
    directory_inspector:
        Option<Arc<dyn crate::application::project::ApplicationDirectoryInspector>>,
    short_term_memory: Option<ShortTermMemoryService>,
    active_runs: ActiveRunRegistry,
    framework_host: Option<super::FrameworkHost>,
    auto_model_router: Option<crate::application::model_router::AgentModelRouter>,
}

impl AgentRuntimeRegistry {
    pub fn new(
        default_model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        event_bus: EventBusHandle,
        runtime_http_client: reqwest::Client,
    ) -> Self {
        Self {
            default_model_client,
            tool_registry,
            event_bus,
            runtime_http_client,
            directory_inspector: None,
            short_term_memory: None,
            active_runs: ActiveRunRegistry::new(),
            framework_host: Some(
                super::FrameworkHost::temporary(None)
                    .expect("standalone Framework storage should initialize"),
            ),
            auto_model_router: None,
        }
    }

    pub fn with_framework_host(mut self, host: Option<super::FrameworkHost>) -> Self {
        if host.is_some() {
            self.framework_host = host;
        }
        self
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

    pub fn with_active_runs(mut self, active_runs: ActiveRunRegistry) -> Self {
        self.active_runs = active_runs;
        self
    }

    pub fn with_auto_model_router(
        mut self,
        router: Option<crate::application::model_router::AgentModelRouter>,
    ) -> Self {
        self.auto_model_router = router;
        self
    }

    pub fn active_runs(&self) -> ActiveRunRegistry {
        self.active_runs.clone()
    }

    pub fn resolve(&self, selection: RuntimeAdapterSelection) -> Arc<dyn AgentRuntime> {
        let runtime: Arc<dyn AgentRuntime> = match selection {
            RuntimeAdapterSelection::React => Arc::new(ReactRuntime::new(
                RuntimeAdapterKey::React,
                RuntimeCapabilities::react(),
                self.framework_host.clone(),
                self.default_model_client.clone(),
                self.tool_registry.clone(),
                self.event_bus.clone(),
                self.short_term_memory.clone(),
                self.active_runs.clone(),
                self.auto_model_router.clone(),
                self.directory_inspector.clone(),
            )),
            RuntimeAdapterSelection::CodexExec {
                command_name,
                working_directory,
            } => Arc::new(SystemCliRuntime::new(
                RuntimeAdapterKey::CodexExec,
                RuntimeCapabilities::system_cli(),
                SystemAgentCliKind::Codex,
                command_name,
                working_directory,
            )),
            RuntimeAdapterSelection::DshHeadless {
                command_name,
                working_directory,
            } => Arc::new(
                SystemCliRuntime::new(
                    RuntimeAdapterKey::DshHeadless,
                    RuntimeCapabilities::system_cli(),
                    SystemAgentCliKind::Dsh,
                    command_name,
                    working_directory,
                )
                .with_dsh_workspace(
                    self.framework_host
                        .as_ref()
                        .map(|host| host.dsh_workspace_options()),
                ),
            ),
            RuntimeAdapterSelection::OpenCodeRun {
                command_name,
                working_directory,
            } => Arc::new(SystemCliRuntime::new(
                RuntimeAdapterKey::OpenCodeRun,
                RuntimeCapabilities::system_cli(),
                SystemAgentCliKind::OpenCode,
                command_name,
                working_directory,
            )),
            RuntimeAdapterSelection::CodexAppServer {
                endpoint,
                working_directory,
            } => Arc::new(ServiceModeRuntime::new(
                RuntimeAdapterKey::CodexAppServer,
                RuntimeCapabilities::service_mode(),
                endpoint,
                working_directory,
                self.runtime_http_client.clone(),
            )),
            RuntimeAdapterSelection::OpenCodeServe {
                endpoint,
                working_directory,
            } => Arc::new(ServiceModeRuntime::new(
                RuntimeAdapterKey::OpenCodeServe,
                RuntimeCapabilities::service_mode(),
                endpoint,
                working_directory,
                self.runtime_http_client.clone(),
            )),
            RuntimeAdapterSelection::OpenCodeAcp {
                endpoint,
                working_directory,
            } => Arc::new(ServiceModeRuntime::new(
                RuntimeAdapterKey::OpenCodeAcp,
                RuntimeCapabilities::service_mode(),
                endpoint,
                working_directory,
                self.runtime_http_client.clone(),
            )),
        };
        if runtime.key() == RuntimeAdapterKey::React {
            return runtime;
        }
        Arc::new(super::framework_runtime::FrameworkExternalRuntime {
            host: self
                .framework_host
                .clone()
                .expect("Framework host is initialized"),
            transport: runtime,
        })
    }
}
