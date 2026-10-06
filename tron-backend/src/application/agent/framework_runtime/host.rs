use super::{
    context::TronContext,
    model::{HostExecution, HostRunState},
    tools::{AttachmentReader, HarnessTools, LocalScope, ToolAudit},
};
use crate::application::agent::{ActiveRunControl, ModelStreamSink, runtime::ReactAgent};
use crate::domain::agent::{AgentError, AgentRunContext, AgentRunResult};
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::{Agent, AgentBuilder};
use eliteforge_agent_official_plugin_storage_local::{LocalStoragePlugin, SqliteConversationStore};
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
    agent: OnceCell<Agent>,
    external_agent: OnceCell<Agent>,
    continuation_agent: OnceCell<Agent>,
    native_sessions: OnceCell<SqliteConversationStore>,
    summaries: OnceCell<Arc<eliteforge_agent_official_plugin_storage_local::SqliteSummaryStore>>,
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
        persist_capability_state(&self.0.state_path, &packs, &extensions)?;
        if let Some(agent) = self.0.agent.get() {
            let target = self.agent_builder(&packs, &extensions)?;
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
        let packs = self.0.capability_state.read().unwrap().clone();
        let mut extensions = self.0.extension_state.read().unwrap().clone();
        let previous = extensions.clone();
        extensions.insert(point.to_owned(), enabled);
        persist_capability_state(&self.0.state_path, &packs, &extensions)?;
        if let Some(agent) = self.0.agent.get() {
            let target = self.agent_builder(&packs, &extensions)?;
            if let Err(error) = agent.reconfigure(target).await {
                let _ = persist_capability_state(&self.0.state_path, &packs, &previous);
                return Err(format!("{error:?}"));
            }
        }
        *self.0.extension_state.write().unwrap() = extensions;
        Ok(())
    }

    fn agent_builder(
        &self,
        packs: &BTreeMap<String, bool>,
        extensions: &BTreeMap<String, bool>,
    ) -> Result<AgentBuilder, String> {
        let enabled = |point: &str| extensions.get(point).copied().unwrap_or(true);
        let builder = AgentBuilder::new()
            .default_models(false)
            .plugin(eliteforge_agent_official_plugin_agui::AgUiPlugin::default());
        let builder = if enabled("storage") {
            builder.plugin(LocalStoragePlugin::new(
                &self.0.path,
                self.0.path.with_extension("artifacts"),
            ))
        } else {
            builder
        };
        let builder = if enabled("model") {
            builder.plugin(eliteforge_agent_framework::ModelRuntimePlugin::new(
                eliteforge_agent_framework::ModelRuntime::new(
                    eliteforge_agent_framework::ModelRuntimeUpdate::new("", 0, vec![]),
                )
                .map_err(|error| error.to_string())?,
            ))
        } else {
            builder
        };
        // Product authority is a host invariant, independent of the optional
        // harness pack. Managed packs may keep model/context capabilities while
        // the tool extension is disabled, but cannot admit any tool executor.
        let builder = builder.plugin(fw::ToolPolicyPlugin::new(
            "tron.harness.tool-authority",
            super::tools::ToolCatalogPolicy::new(enabled("tool")),
        ));
        let builder = if enabled("observe") {
            builder.plugin(fw::ToolMiddlewarePlugin::new(
                "tron.harness.audit",
                ToolAudit,
            ))
        } else {
            builder
        };
        let builder =
            if packs.get("framework.local.scoped").copied().unwrap_or(true) && enabled("tool") {
                builder.capability_pack(eliteforge_agent_capability_tools::scoped_local_tools_pack(
                    Arc::new(LocalScope),
                    self.pty(),
                ))
            } else {
                builder
            };
        let mut builder = if packs.get("tron.harness").copied().unwrap_or(true)
            && enabled("context")
            && enabled("tool")
        {
            builder.capability_pack(
                tron_agent_harness::TronHarness::new(Arc::new(HarnessTools), [])
                    .with_context_preparation(TronContext, 32)
                    .capability_pack(),
            )
        } else {
            builder
        };
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|error| error.to_string())?;
        for (package, active) in packs {
            if !active
                || matches!(
                    package.as_str(),
                    "framework.external-agent" | "framework.local.scoped" | "tron.harness"
                )
            {
                continue;
            }
            let version = semver::VersionReq::STAR;
            let managed = manager
                .select(
                    package,
                    &version,
                    "tron",
                    "default",
                    serde_json::Value::Null,
                )
                .map_err(|error| error.to_string())?;
            builder = builder.capability_pack_arc(Arc::new(managed));
        }
        Ok(builder)
    }

    fn ensure_external_idle(&self) -> Result<(), String> {
        self.ensure_open()?;
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
            agent: OnceCell::new(),
            external_agent: OnceCell::new(),
            continuation_agent: OnceCell::new(),
            native_sessions: OnceCell::new(),
            summaries: OnceCell::new(),
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
            agent: OnceCell::new(),
            external_agent: OnceCell::new(),
            continuation_agent: OnceCell::new(),
            native_sessions: OnceCell::new(),
            summaries: OnceCell::new(),
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
        let path = Path::new(database).with_extension("agent-framework.sqlite3");
        Ok(Self::persistent(path, Some(resources)))
    }
    async fn agent(&self) -> Result<&Agent, fw::AgentError> {
        self.0
            .agent
            .get_or_try_init(|| async {
                if let Some(parent) = self
                    .0
                    .path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    std::fs::create_dir_all(parent)
                        .map_err(|error| fw::AgentError::InvalidConfiguration(error.to_string()))?;
                }
                self.prepare_artifacts()
                    .map_err(fw::AgentError::InvalidConfiguration)?;
                let (packs, extensions) = self.capability_state();
                self.agent_builder(&packs, &extensions)
                    .map_err(fw::AgentError::InvalidConfiguration)?
                    .build()
                    .await
            })
            .await
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
