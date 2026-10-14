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
#[path = "host_cell.rs"]
mod host_cell;
use host_cell::RuntimeCell;

#[path = "host_capability_state.rs"]
mod capability_state;
#[path = "host_collaboration.rs"]
pub(super) mod collaboration;
#[path = "host_subagents.rs"]
mod subagents;
pub use subagents::{SubagentRecord, SubagentView};
pub use collaboration::{DudeThreadCatalogItem, ThreadFeedback, ThreadFeedbackSink};
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
#[path = "host_import.rs"]
mod host_import;
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
#[path = "host_remote_tools.rs"]
pub(super) mod remote_tools;
use capability_state::{load_capability_state, persist_capability_state};

#[derive(Clone)]
pub struct FrameworkHost(Arc<Host>);
struct Host {
    runtime_config:
        std::sync::RwLock<Option<crate::application::system_config::BackendRuntimeConfig>>,
    model_client: std::sync::RwLock<Option<Arc<dyn crate::application::agent::ModelClient>>>,
    personai: remote_tools::PersonAiState,
    path: PathBuf,
    state_path: PathBuf,
    pack_root: PathBuf,
    capability_state: std::sync::RwLock<BTreeMap<String, bool>>,
    extension_state: std::sync::RwLock<BTreeMap<String, bool>>,
    config_lock: tokio::sync::Mutex<()>,
    pty: eliteforge_agent_capability_tools::PtyService,
    framework: RuntimeCell<Framework>,
    telemetry: OnceCell<crate::app::telemetry::LocalTelemetry>,
    agent: RuntimeCell<Agent>,
    internal_agents: std::sync::RwLock<BTreeMap<(String, String), Arc<OnceCell<Agent>>>>,
    mcp: OnceCell<crate::application::mcp::McpService>,
    rag: OnceCell<crate::app::AppRagService>,
    skill_registry: OnceCell<crate::application::skill::SkillRegistryService>,
    capability_resources: OnceCell<crate::application::capability::CapabilityResources>,
    skills: std::sync::RwLock<BTreeMap<String, Vec<String>>>,
    installed_packs: std::sync::RwLock<BTreeMap<String, Vec<String>>>,
    tools_enabled: Arc<std::sync::atomic::AtomicBool>,
    observe_enabled: Arc<std::sync::atomic::AtomicBool>,
    external_agent: RuntimeCell<Agent>,
    continuation_agent: RuntimeCell<Agent>,
    database:
        OnceCell<crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase>,
    native_bindings: super::native_sessions::Bindings,
    closed: std::sync::atomic::AtomicBool,
    control: host_control::NativeRuns,
    shutdown: host_shutdown::OwnedShutdown,
    presentation: std::sync::RwLock<Option<crate::application::agent::AgentAguiStreamService>>,
    resources: Option<crate::application::resource::ResourceService>,
    agent_management: OnceCell<crate::application::agent::AgentManagementService>,
    dudes: OnceCell<crate::application::dude::DudeService>,
    knowledge_bases: OnceCell<crate::application::knowledge_base::KnowledgeBaseService>,
    collaboration_builder:
        OnceCell<std::sync::Weak<crate::application::agent::AgentRunContextBuilder>>,
    collaboration_threads: collaboration::Threads,
    thread_feedback: OnceCell<Arc<dyn ThreadFeedbackSink>>,
    _temporary: Option<tempfile::TempDir>,
}
impl FrameworkHost {
    pub(crate) async fn telemetry(
        &self,
    ) -> Result<
        &crate::app::telemetry::LocalTelemetry,
        crate::application::observability::ObservabilityError,
    > {
        self.0
            .telemetry
            .get_or_try_init(|| async {
                crate::app::telemetry::LocalTelemetry::open(
                    &self.0.path.with_extension("telemetry.sqlite3"),
                )
                .await
            })
            .await
    }
    pub async fn observability(
        &self,
    ) -> Result<
        crate::application::observability::ObservabilityService,
        crate::application::observability::ObservabilityError,
    > {
        self.ensure_open()
            .map_err(|_| crate::application::observability::ObservabilityError::Storage)?;
        Ok(self.telemetry().await?.service())
    }

    pub async fn collector_connection(
        &self,
    ) -> Result<
        crate::application::observability::CollectorConnection,
        crate::application::observability::ObservabilityError,
    > {
        self.ensure_open()
            .map_err(|_| crate::application::observability::ObservabilityError::Storage)?;
        Ok(self.telemetry().await?.connection())
    }
    pub fn bind_agent_management(
        &self,
        service: crate::application::agent::AgentManagementService,
    ) {
        self.0
            .agent_management
            .set(service)
            .unwrap_or_else(|_| panic!("agent management already bound"));
    }
    pub fn bind_skills(
        &self,
        service: crate::application::skill::SkillRegistryService,
        resources: crate::application::capability::CapabilityResources,
        mcp: crate::application::mcp::McpService,
        rag: crate::app::AppRagService,
    ) -> Result<(), String> {
        self.0.mcp.set(mcp).map_err(|_| "MCP already bound")?;
        self.0.rag.set(rag).map_err(|_| "RAG already bound")?;
        self.0
            .skill_registry
            .set(service)
            .map_err(|_| "skills already bound")?;
        self.0
            .capability_resources
            .set(resources)
            .map_err(|_| "capability resources already bound")?;
        self.sync_capability_resources()
    }
    pub(crate) fn bind_collaboration(
        &self,
        builder: &Arc<crate::application::agent::AgentRunContextBuilder>,
    ) -> Result<(), AgentError> {
        self.0
            .collaboration_builder
            .set(Arc::downgrade(builder))
            .map_err(|_| AgentError::InvalidConfiguration("collaboration is already bound".into()))
    }
    pub fn bind_runtime_config(
        &self,
        runtime: crate::application::system_config::BackendRuntimeConfig,
    ) {
        *self
            .0
            .runtime_config
            .write()
            .unwrap_or_else(|error| error.into_inner()) = Some(runtime);
    }
    fn invocation_config(&self) -> crate::config::ModelInvocationConfig {
        self.0
            .runtime_config
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .map(|runtime| runtime.snapshot().model_invocation)
            .unwrap_or_default()
    }

    /// Drain owned runs and command processes before shutting down a profile.
    pub fn pty(&self) -> eliteforge_agent_capability_tools::PtyService {
        self.0.pty.clone()
    }

    pub(crate) fn bind_dudes(&self, service:crate::application::dude::DudeService) {
        let _ = self.0.dudes.set(service);
    }
    pub(crate) fn dudes(&self)->Option<crate::application::dude::DudeService> {self.0.dudes.get().cloned()}
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

    async fn invalidate_environment(&self) -> Result<(), String> {
        // Preflight has excluded active and durable users. Release package runtimes and
        // MCP leases before removing package files; session data remains in shared storage.
        for agent in self.initialized_internal_agents().into_iter().chain(
            [self.0.external_agent.get(), self.0.continuation_agent.get()]
                .into_iter()
                .flatten(),
        ) {
            let report = agent.close().await;
            if !report.is_success() {
                return Err(format!("Agent cleanup failed: {report:?}"));
            }
        }
        self.0.agent.take();
        self.0.external_agent.take();
        self.0.continuation_agent.take();
        self.0.internal_agents.write().unwrap().clear();
        if let Some(framework) = self.0.framework.get() {
            let report = framework.close().await;
            if !report.is_success() {
                return Err(format!("capability runtime cleanup failed: {report:?}"));
            }
            self.0.framework.take();
        }
        Ok(())
    }
    pub async fn import_pack(
        &self,
        bytes: Vec<u8>,
    ) -> Result<crate::application::agent::CapabilityPackInfo, String> {
        let _guard = self.0.config_lock.lock().await;
        self.ensure_external_idle()?;
        let package_id = {
            use std::io::Read;
            let mut archive =
                zip::ZipArchive::new(std::io::Cursor::new(&bytes)).map_err(|e| e.to_string())?;
            let mut data = String::new();
            archive
                .by_name("manifest.json")
                .map_err(|e| e.to_string())?
                .take(1024 * 1024)
                .read_to_string(&mut data)
                .map_err(|e| e.to_string())?;
            let manifest: serde_json::Value =
                serde_json::from_str(&data).map_err(|e| e.to_string())?;
            manifest
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("missing capability package id")?
                .to_owned()
        };
        if Self::builtin_pack(&package_id) {
            return Err("bundled capability package ids cannot be imported".into());
        }
        let bytes = self.validate_pack_import(bytes).await?;
        self.check_pack_references(&package_id)?;
        self.invalidate_environment().await?;
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|e| e.to_string())?;
        for version in manager
            .installed()
            .map_err(|e| e.to_string())?
            .iter()
            .filter(|v| v.manifest.id == package_id)
        {
            manager
                .check_remove(&package_id, &version.manifest.version)
                .map_err(|e| e.to_string())?;
        }
        let root = self.0.pack_root.clone();
        let pack = tokio::task::spawn_blocking(move || {
            crate::application::agent::install_capability_pack(&root, &bytes)
        })
        .await
        .map_err(|e| e.to_string())??;
        {
            let mut packs = self.0.capability_state.write().unwrap();
            packs.insert(pack.id.clone(), false);
            persist_capability_state(
                &self.0.state_path,
                &packs,
                &self.0.extension_state.read().unwrap(),
            )?;
        }
        if let Some(resources) = self.0.capability_resources.get() {
            resources.transition(&pack.id, false, &[])?;
        }
        self.sync_pack_indexes(&pack.id, false).await?;
        if let (Some(skills), Some(rag)) = (self.0.skill_registry.get(), self.0.rag.get()) {
            let indexer =
                crate::application::skill::SkillRagIndexService::new(skills.clone(), rag.clone());
            indexer
                .remove_pack_skills(&pack.id)
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some(resources) = self.0.capability_resources.get() {
            resources.remove(&pack.id)?;
        }
        if let Err(error) = self.sync_capability_resources() {
            // Conflicting imports remain disabled and can be removed or retried.
            self.invalidate_environment().await?;
            return Err(error);
        }
        self.sync_pack_indexes(&pack.id, false).await?;
        self.invalidate_environment().await?;
        self.framework().await.map_err(|e| e.to_string())?;
        Ok(pack)
    }
    async fn sync_pack_indexes(&self, id: &str, enabled: bool) -> Result<(), String> {
        if let (Some(skills), Some(rag)) = (self.0.skill_registry.get(), self.0.rag.get()) {
            let indexer =
                crate::application::skill::SkillRagIndexService::new(skills.clone(), rag.clone());
            for skill in skills
                .list_registry(crate::domain::skill::RegistrySkillListQuery { search: None })
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|s| s.provider_pack_id.as_deref() == Some(id))
            {
                let state = if enabled {
                    indexer.index_registry_skill(&skill.skill_name).await
                } else {
                    indexer.disable_skill(&skill.skill_name).await
                }
                .map_err(|e| e.to_string())?;
                if state.vector_status == crate::domain::skill::SkillRagIndexStatus::Failed
                    || state.graph_status == crate::domain::skill::SkillRagIndexStatus::Failed
                {
                    return Err(state
                        .last_error
                        .unwrap_or_else(|| "skill index update failed".into()));
                }
            }
        }
        Ok(())
    }
    fn check_pack_references(&self, id: &str) -> Result<(), String> {
        self.ensure_external_idle()?;
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|e| e.to_string())?;
        if manager
            .durable_pins()
            .map_err(|e| e.to_string())?
            .iter()
            .any(|p| p.versions.iter().any(|v| v.package == id))
        {
            return Err("capability pack is referenced by a durable run".into());
        }
        Ok(())
    }
    pub async fn set_pack_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let _guard = self.0.config_lock.lock().await;
        self.check_pack_references(id)?;
        let mut packs = self.0.capability_state.read().unwrap().clone();
        let extensions = self.0.extension_state.read().unwrap().clone();
        let was_changed = packs.get(id).copied().unwrap_or(Self::builtin_pack(id)) != enabled;
        packs.insert(id.into(), enabled);
        if let Some(f) = self.0.framework.get() {
            self.main_selection(&f, &packs, &extensions)?;
        }
        let agents = if let Some(m) = self.0.agent_management.get() {
            m.persist_internal_agent_defaults()
                .map_err(|e| format!("{e:?}"))?;
            m.list_platform_internal_agents()
                .map_err(|e| format!("{e:?}"))?
                .into_iter()
                .map(|a| a.agent_code)
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let result = async {
            if let Some(resources) = self.0.capability_resources.get() {
                resources.transition(id, enabled, &agents)?;
            }
            if !enabled || was_changed {
                self.sync_pack_indexes(id, enabled).await?;
            }
            // Both durable configuration and native Agent selections must succeed.
            persist_capability_state(&self.0.state_path, &packs, &extensions)?;
            for agent in self.initialized_internal_agents() {
                let selection = self.main_selection(&agent.framework(), &packs, &extensions)?;
                agent
                    .reconfigure(agent.framework().agent(selection))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Ok::<(), String>(())
        }
        .await;
        if let Err(error) = result {
            packs.insert(id.into(), false);
            if let Some(resources) = self.0.capability_resources.get() {
                let _ = resources.transition(id, false, &agents);
            }
            let _ = persist_capability_state(&self.0.state_path, &packs, &extensions);
            *self.0.capability_state.write().unwrap() = packs;
            return Err(error);
        }
        *self.0.capability_state.write().unwrap() = packs;
        Ok(())
    }
    pub async fn uninstall_pack(&self, id: &str) -> Result<(), String> {
        let _guard = self.0.config_lock.lock().await;
        self.check_pack_references(id)?;
        if Self::builtin_pack(id) {
            return Err("bundled capability packs cannot be uninstalled".into());
        }
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|e| e.to_string())?;
        let versions = manager
            .installed()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|p| p.manifest.id == id)
            .collect::<Vec<_>>();
        // Release this host's idle leases before checking cross-process version locks.
        // No persisted package state or assignments change when preflight rejects.
        self.invalidate_environment().await?;
        for version in &versions {
            manager
                .check_remove(id, &version.manifest.version)
                .map_err(|e| e.to_string())?;
        }
        let mut packs = self.0.capability_state.read().unwrap().clone();
        let extensions = self.0.extension_state.read().unwrap().clone();
        packs.insert(id.into(), false);
        persist_capability_state(&self.0.state_path, &packs, &extensions)?;
        *self.0.capability_state.write().unwrap() = packs;
        if let Some(resources) = self.0.capability_resources.get() {
            resources.transition(id, false, &[])?;
        }
        if let (Some(skills), Some(rag)) = (self.0.skill_registry.get(), self.0.rag.get()) {
            let names = skills
                .list_registry(crate::domain::skill::RegistrySkillListQuery { search: None })
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|s| s.provider_pack_id.as_deref() == Some(id))
                .map(|s| s.skill_name)
                .collect::<Vec<_>>();
            let indexer =
                crate::application::skill::SkillRagIndexService::new(skills.clone(), rag.clone());
            for name in names {
                indexer
                    .remove_skill_index(&name)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        self.invalidate_environment().await?;
        for version in versions {
            manager
                .remove(id, &version.manifest.version)
                .map_err(|e| e.to_string())?;
        }
        if let Some(skills) = self.0.skill_registry.get() {
            if let Some(rag) = self.0.rag.get() {
                crate::application::skill::SkillRagIndexService::new(skills.clone(), rag.clone())
                    .remove_pack_skills(id)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                skills.delete_pack_skills(id).map_err(|e| e.to_string())?;
            }
        }
        if let Some(resources) = self.0.capability_resources.get() {
            resources.remove(id)?;
        }
        self.0.skills.write().unwrap().remove(id);
        // Retain provider IDs as a disabled deny list until the initialized environment closes.
        let safe = id
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let cache = self.0.path.with_extension("skills-catalog").join(&safe);
        if cache.exists() {
            fs::remove_dir_all(cache).map_err(|e| e.to_string())?;
        }
        let cache = self
            .0
            .path
            .with_extension("skills")
            .join("capability-packs")
            .join(safe);
        if cache.exists() {
            fs::remove_dir_all(cache).map_err(|e| e.to_string())?;
        }
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
            personai: Default::default(),
            runtime_config: Default::default(),
            model_client: Default::default(),
            path,
            state_path,
            pack_root,
            capability_state: std::sync::RwLock::new(packs),
            extension_state: std::sync::RwLock::new(extensions),
            config_lock: tokio::sync::Mutex::new(()),
            framework: RuntimeCell::new(),
            telemetry: OnceCell::new(),
            agent: RuntimeCell::new(),
            internal_agents: Default::default(),
            mcp: OnceCell::new(),
            rag: OnceCell::new(),
            skill_registry: OnceCell::new(),
            capability_resources: OnceCell::new(),
            skills: Default::default(),
            installed_packs: Default::default(),
            tools_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            observe_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            external_agent: RuntimeCell::new(),
            continuation_agent: RuntimeCell::new(),
            database: OnceCell::new(),
            native_bindings: Default::default(),
            closed: Default::default(),
            control: Default::default(),
            shutdown: Default::default(),
            presentation: Default::default(),
            pty: Default::default(),
            resources,
            dudes: OnceCell::new(),
            knowledge_bases: OnceCell::new(),
            agent_management: OnceCell::new(),
            collaboration_builder: OnceCell::new(),
            collaboration_threads: Default::default(),
            thread_feedback: OnceCell::new(),
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
            personai: Default::default(),
            runtime_config: Default::default(),
            model_client: Default::default(),
            state_path: path.with_extension("capabilities.json"),
            pack_root: path.with_extension("capability-packs"),
            path,
            capability_state: std::sync::RwLock::new(BTreeMap::new()),
            extension_state: std::sync::RwLock::new(BTreeMap::new()),
            config_lock: tokio::sync::Mutex::new(()),
            framework: RuntimeCell::new(),
            telemetry: OnceCell::new(),
            agent: RuntimeCell::new(),
            internal_agents: Default::default(),
            mcp: OnceCell::new(),
            rag: OnceCell::new(),
            skill_registry: OnceCell::new(),
            capability_resources: OnceCell::new(),
            skills: Default::default(),
            installed_packs: Default::default(),
            tools_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            observe_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            external_agent: RuntimeCell::new(),
            continuation_agent: RuntimeCell::new(),
            database: OnceCell::new(),
            native_bindings: Default::default(),
            closed: Default::default(),
            control: Default::default(),
            shutdown: Default::default(),
            presentation: Default::default(),
            pty: Default::default(),
            resources,
            dudes: OnceCell::new(),
            knowledge_bases: OnceCell::new(),
            agent_management: OnceCell::new(),
            collaboration_builder: OnceCell::new(),
            collaboration_threads: Default::default(),
            thread_feedback: OnceCell::new(),
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
    async fn agent(&self) -> Result<Agent, fw::AgentError> {
        self.0
            .agent
            .get_or_try_init(|| async {
                let framework = self.framework().await?;
                let (packs, extensions) = self.capability_state();
                let selection = self
                    .main_selection(&framework, &packs, &extensions)
                    .map_err(fw::AgentError::InvalidConfiguration)?;
                framework.agent(selection).build().await
            })
            .await
    }

    fn initialized_internal_agents(&self) -> Vec<Agent> {
        self.0
            .agent
            .get()
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
                .main_selection(&framework, &packs, &extensions)
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
    async fn drain(&self, run: &fw::RunContext, seal: bool) -> Result<Vec<fw::Content>, fw::AgentError> {
        Ok(self.drain_inputs(run, seal).await?.into_iter().map(|input| input.content).collect())
    }
    async fn drain_inputs(&self, run: &fw::RunContext, seal: bool) -> Result<Vec<fw::LiveInput>, fw::AgentError> {
        let inputs = if seal { self.control.take_pending_or_begin_finishing() } else { self.control.take_pending() }
            .map_err(|error| fw::AgentError::Run(error.to_string()))?;
        let selected = inputs.iter().map(|input| (input.requested_skills.clone(), input.annotations.clone())).collect::<Vec<_>>();
        let host = HostExecution::from_extensions(&run.extensions)?;
        let state = host.state.lock().await;
        let messages = host.harness.framework_steering(&state.context, inputs)
            .map_err(|error| fw::AgentError::Run(error.to_string()))?;
        messages.iter().zip(selected).map(|(message, (requested_skills, annotations))| {
            super::messages::encode(message).map(|message| fw::LiveInput { content: message.content, requested_skills, annotations })
                .map_err(fw::AgentError::InvalidConfiguration)
        }).collect()
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
