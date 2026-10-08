use super::{
    context::TronContext,
    model::{HostExecution, HostRunState},
    tools::{AttachmentReader, HarnessTools, LocalScope, ToolAudit},
};
use crate::application::agent::{ActiveRunControl, ModelStreamSink, runtime::ReactAgent};
use crate::domain::agent::{AgentError, AgentRunContext, AgentRunResult};
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::{Agent, AgentSelection, Framework};

use eliteforge_agent_official_plugin_tools_local::FileReferenceReaderService;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, OnceCell};

#[path = "host_capability_state.rs"]
mod capability_state;
#[path = "host_continuation.rs"]
mod host_continuation;
#[path = "host_control.rs"]
mod host_control;
#[path = "host_driver.rs"]
mod host_driver;
#[path = "host_environment.rs"]
mod host_environment;
#[path = "host_execution.rs"]
mod host_execution;
#[path = "host_external.rs"]
mod host_external;
#[path = "host_interactions.rs"]
mod host_interactions;
#[path = "host_run.rs"]
mod host_run;
#[path = "host_shutdown.rs"]
mod host_shutdown;
#[path = "host_snapshot.rs"]
mod host_snapshot;
#[path = "host_storage.rs"]
mod host_storage;
#[path = "host_summary.rs"]
mod host_summary;
use capability_state::{load_capability_state, persist_capability_state};

#[derive(Clone)]
pub struct FrameworkHost(Arc<Host>);
struct Host {
    model_client: std::sync::RwLock<Option<Arc<dyn crate::application::agent::ModelClient>>>,
    path: PathBuf,
    state_path: PathBuf,
    pack_root: PathBuf,
    capability_state: std::sync::RwLock<BTreeMap<String, bool>>,
    extension_state: std::sync::RwLock<BTreeMap<String, bool>>,
    config_lock: tokio::sync::Mutex<()>,
    pty: eliteforge_agent_capability_tools::PtyService,
    framework: OnceCell<Framework>,
    agent: OnceCell<Agent>,
    internal_agents: std::sync::RwLock<BTreeMap<(String, String), Arc<OnceCell<Agent>>>>,
    skills: std::sync::RwLock<BTreeMap<String, Vec<String>>>,
    installed_packs: std::sync::RwLock<BTreeMap<String, Vec<String>>>,
    tools_enabled: Arc<std::sync::atomic::AtomicBool>,
    observe_enabled: Arc<std::sync::atomic::AtomicBool>,
    external_agent: OnceCell<Agent>,
    continuation_agent: OnceCell<Agent>,
    database:
        OnceCell<crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase>,
    native_bindings: super::native_sessions::Bindings,
    closed: std::sync::atomic::AtomicBool,
    control: host_control::NativeRuns,
    shutdown: host_shutdown::OwnedShutdown,
    presentation: std::sync::RwLock<Option<crate::application::agent::AgentAguiStreamService>>,
    resources: Option<crate::application::resource::ResourceService>,
    knowledge_bases: OnceCell<crate::application::knowledge_base::KnowledgeBaseService>,
    _temporary: Option<tempfile::TempDir>,
}
impl FrameworkHost {
    /// Drain owned runs and command processes before shutting down a profile.
    pub fn pty(&self) -> eliteforge_agent_capability_tools::PtyService {
        self.0.pty.clone()
    }

    pub fn bind_knowledge_bases(
        &self,
        service: crate::application::knowledge_base::KnowledgeBaseService,
    ) -> Result<(), AgentError> {
        self.0.knowledge_bases.set(service).map_err(|_| {
            AgentError::InvalidConfiguration("knowledge base service is already bound".into())
        })
    }

    pub(crate) fn knowledge_bases(
        &self,
    ) -> Option<crate::application::knowledge_base::KnowledgeBaseService> {
        self.0.knowledge_bases.get().cloned()
    }

    pub fn capability_state(&self) -> (BTreeMap<String, bool>, BTreeMap<String, bool>) {
        (
            self.0.capability_state.read().unwrap().clone(),
            self.0.extension_state.read().unwrap().clone(),
        )
    }

    pub fn pack_root(&self) -> &Path {
        &self.0.pack_root
    }

    pub async fn set_pack_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let _guard = self.0.config_lock.lock().await;
        self.ensure_external_idle()?;
        let mut packs = self.0.capability_state.read().unwrap().clone();
        let previous = packs.clone();
        let extensions = self.0.extension_state.read().unwrap().clone();
        packs.insert(id.to_owned(), enabled);
        if let Some(framework) = self.0.framework.get() {
            self.main_selection(framework, &packs, &extensions)?;
        }
        persist_capability_state(&self.0.state_path, &packs, &extensions)?;
        for agent in self.initialized_internal_agents() {
            let selection = match self.main_selection(&agent.framework(), &packs, &extensions) {
                Ok(selection) => selection,
                Err(error) => {
                    let _ = persist_capability_state(&self.0.state_path, &previous, &extensions);
                    return Err(error);
                }
            };
            let target = agent.framework().agent(selection);
            if let Err(error) = agent.reconfigure(target).await {
                let _ = persist_capability_state(&self.0.state_path, &previous, &extensions);
                return Err(format!("{error:?}"));
            }
        }
        *self.0.capability_state.write().unwrap() = packs;
        Ok(())
    }

    pub async fn set_extension_enabled(&self, point: &str, enabled: bool) -> Result<(), String> {
        let _guard = self.0.config_lock.lock().await;
        self.ensure_external_idle()?;
        if matches!(point, "storage" | "model") && !enabled {
            return Err(format!("{point} is a required Framework service"));
        }
        let packs = self.0.capability_state.read().unwrap().clone();
        let mut extensions = self.0.extension_state.read().unwrap().clone();
        let previous = extensions.clone();
        extensions.insert(point.to_owned(), enabled);
        persist_capability_state(&self.0.state_path, &packs, &extensions)?;
        for agent in self.initialized_internal_agents() {
            let selection = match self.main_selection(&agent.framework(), &packs, &extensions) {
                Ok(selection) => selection,
                Err(error) => {
                    let _ = persist_capability_state(&self.0.state_path, &packs, &previous);
                    return Err(error);
                }
            };
            let target = agent.framework().agent(selection);
            if let Err(error) = agent.reconfigure(target).await {
                let _ = persist_capability_state(&self.0.state_path, &packs, &previous);
                return Err(format!("{error:?}"));
            }
        }
        self.0.tools_enabled.store(
            extensions.get("tool").copied().unwrap_or(true),
            std::sync::atomic::Ordering::Release,
        );
        self.0.observe_enabled.store(
            extensions.get("observe").copied().unwrap_or(true),
            std::sync::atomic::Ordering::Release,
        );
        *self.0.extension_state.write().unwrap() = extensions;
        Ok(())
    }

    fn ensure_external_idle(&self) -> Result<(), String> {
        self.ensure_open()?;
        for agent in self.initialized_internal_agents() {
            if !agent
                .reconfiguration_blockers()
                .map_err(|error| error.to_string())?
                .is_empty()
            {
                return Err("internal agent is busy".into());
            }
        }
        if let Some(agent) = self.0.continuation_agent.get() {
            if !agent
                .reconfiguration_blockers()
                .map_err(|error| error.to_string())?
                .is_empty()
            {
                return Err("native continuation is busy".into());
            }
        }
        if let Some(agent) = self.0.external_agent.get()
            && !agent
                .reconfiguration_blockers()
                .map_err(|e| e.to_string())?
                .is_empty()
        {
            return Err("external runtime is busy".into());
        }
        Ok(())
    }
    fn prepare_artifacts(&self) -> Result<(), String> {
        self.prepare_artifacts_at(&self.0.path.with_extension("artifacts"))
    }
    fn prepare_artifacts_at(&self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn ensure_open(&self) -> Result<(), String> {
        if self.0.closed.load(std::sync::atomic::Ordering::Acquire) {
            return Err("framework host is closed".into());
        }
        Ok(())
    }

    pub fn persistent(
        path: impl Into<PathBuf>,
        resources: Option<crate::application::resource::ResourceService>,
    ) -> Self {
        let path = path.into();
        let state_path = path.with_extension("capabilities.json");
        let pack_root = path.with_extension("capability-packs");
        let (packs, extensions) = load_capability_state(&state_path);
        Self(Arc::new(Host {
            model_client: Default::default(),
            path,
            state_path,
            pack_root,
            capability_state: std::sync::RwLock::new(packs),
            extension_state: std::sync::RwLock::new(extensions),
            config_lock: tokio::sync::Mutex::new(()),
            framework: OnceCell::new(),
            agent: OnceCell::new(),
            internal_agents: Default::default(),
            skills: Default::default(),
            installed_packs: Default::default(),
            tools_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            observe_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            external_agent: OnceCell::new(),
            continuation_agent: OnceCell::new(),
            database: OnceCell::new(),
            native_bindings: Default::default(),
            closed: Default::default(),
            control: Default::default(),
            shutdown: Default::default(),
            presentation: Default::default(),
            pty: Default::default(),
            resources,
            knowledge_bases: OnceCell::new(),
            _temporary: None,
        }))
    }
    pub fn temporary(
        resources: Option<crate::application::resource::ResourceService>,
    ) -> Result<Self, AgentError> {
        let directory = tempfile::tempdir()
            .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
        let path = directory.path().join("framework.sqlite3");
        Ok(Self(Arc::new(Host {
            model_client: Default::default(),
            state_path: path.with_extension("capabilities.json"),
            pack_root: path.with_extension("capability-packs"),
            path,
            capability_state: std::sync::RwLock::new(BTreeMap::new()),
            extension_state: std::sync::RwLock::new(BTreeMap::new()),
            config_lock: tokio::sync::Mutex::new(()),
            framework: OnceCell::new(),
            agent: OnceCell::new(),
            internal_agents: Default::default(),
            skills: Default::default(),
            installed_packs: Default::default(),
            tools_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            observe_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            external_agent: OnceCell::new(),
            continuation_agent: OnceCell::new(),
            database: OnceCell::new(),
            native_bindings: Default::default(),
            closed: Default::default(),
            control: Default::default(),
            shutdown: Default::default(),
            presentation: Default::default(),
            pty: Default::default(),
            resources,
            knowledge_bases: OnceCell::new(),
            _temporary: Some(directory),
        })))
    }
    pub fn for_database(
        database: &str,
        resources: crate::application::resource::ResourceService,
    ) -> Result<Self, AgentError> {
        if database == ":memory:" {
            return Self::temporary(Some(resources));
        }
        if database.trim().is_empty() {
            return Err(AgentError::InvalidConfiguration(
                "database path is required".into(),
            ));
        }
        let path = PathBuf::from(database);
        Ok(Self::persistent(path, Some(resources)))
    }
    async fn agent(&self) -> Result<&Agent, fw::AgentError> {
        self.0
            .agent
            .get_or_try_init(|| async {
                let framework = self.framework().await?;
                let (packs, extensions) = self.capability_state();
                let selection = self
                    .main_selection(framework, &packs, &extensions)
                    .map_err(fw::AgentError::InvalidConfiguration)?;
                framework.agent(selection).build().await
            })
            .await
    }

    fn initialized_internal_agents(&self) -> Vec<Agent> {
        self.0
            .agent
            .get()
            .cloned()
            .into_iter()
            .chain(
                self.0
                    .internal_agents
                    .read()
                    .unwrap()
                    .values()
                    .filter_map(|cell| cell.get().cloned()),
            )
            .collect()
    }

    async fn internal_agent(&self, workspace: &str, code: &str) -> Result<Agent, fw::AgentError> {
        let key = (workspace.to_owned(), code.to_ascii_lowercase());
        let cell = self
            .0
            .internal_agents
            .write()
            .unwrap()
            .entry(key)
            .or_insert_with(|| Arc::new(OnceCell::new()))
            .clone();
        cell.get_or_try_init(|| async {
            self.ensure_open()
                .map_err(fw::AgentError::InvalidConfiguration)?;
            let framework = self.framework().await?;
            let (packs, extensions) = self.capability_state();
            let selection = self
                .main_selection(framework, &packs, &extensions)
                .map_err(fw::AgentError::InvalidConfiguration)?;
            framework.agent(selection).build().await
        })
        .await
        .cloned()
    }
}
struct HostSteering {
    control: ActiveRunControl,
}
#[async_trait::async_trait]
impl fw::LiveInputSource for HostSteering {
    async fn drain(
        &self,
        run: &fw::RunContext,
        seal: bool,
    ) -> Result<Vec<fw::Content>, fw::AgentError> {
        let inputs = if seal {
            self.control.take_pending_or_begin_finishing()
        } else {
            self.control.take_pending()
        }
        .map_err(|error| fw::AgentError::Run(error.to_string()))?;
        let host = HostExecution::from_extensions(&run.extensions)?;
        let state = host.state.lock().await;
        let messages = host
            .harness
            .framework_steering(&state.context, inputs)
            .map_err(|error| fw::AgentError::Run(error.to_string()))?;
        messages
            .iter()
            .map(|message| {
                super::messages::encode(message)
                    .map(|message| message.content)
                    .map_err(fw::AgentError::InvalidConfiguration)
            })
            .collect()
    }
}
fn map_error(error: fw::AgentError) -> AgentError {
    map_error_ref(&error)
}
fn map_error_ref(error: &fw::AgentError) -> AgentError {
    match error.primary() {
        fw::AgentError::SessionBusy(session) => AgentError::SessionBusy(session.clone()),
        fw::AgentError::Cancelled => AgentError::Cancelled,
        _ => AgentError::InvalidConfiguration(error.to_string()),
    }
}

#[cfg(test)]
#[path = "host_lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "context_seed_budget_tests.rs"]
mod context_seed_budget_tests;

#[cfg(test)]
#[path = "host_environment_tests.rs"]
mod environment_tests;
