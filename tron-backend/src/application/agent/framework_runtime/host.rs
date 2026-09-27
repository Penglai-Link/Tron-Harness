use super::{
    context::{TronContext, TronTokenEstimator},
    model::{HostExecution, HostRunState, TronChatModel},
    tools::{AttachmentReader, HarnessTools, LocalScope, ToolAudit},
};
use crate::application::agent::{ActiveRunControl, ModelStreamSink, runtime::ReactAgent};
use crate::domain::agent::{AgentError, AgentRunContext, AgentRunResult};
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::{Agent, AgentBuilder};
use eliteforge_agent_official_plugin_storage_sqlite::SqliteStoragePlugin;
use eliteforge_agent_official_plugin_tools_local::FileReferenceReaderService;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, OnceCell};

#[derive(Clone)]
pub struct FrameworkHost(Arc<Host>);
struct Host {
    path: PathBuf,
    state_path: PathBuf,
    pack_root: PathBuf,
    capability_state: std::sync::RwLock<BTreeMap<String, bool>>,
    extension_state: std::sync::RwLock<BTreeMap<String, bool>>,
    config_lock: tokio::sync::Mutex<()>,
    pty: eliteforge_agent_capability_tools::PtyService,
    agent: OnceCell<Agent>,
    resources: Option<crate::application::resource::ResourceService>,
    _temporary: Option<tempfile::TempDir>,
}
impl FrameworkHost {
    /// Drain owned runs and command processes before shutting down a profile.
    pub fn pty(&self) -> eliteforge_agent_capability_tools::PtyService {
        self.0.pty.clone()
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
        let mut descriptor = fw::ModelDescriptor::new("route")
            .with_input_modalities([fw::Modality::Text, fw::Modality::Image]);
        descriptor.supports_tools = Some(true);
        let builder = AgentBuilder::new().default_models(false);
        let builder = if packs
            .get("framework.external-agent")
            .copied()
            .unwrap_or(true)
            && enabled("runtime")
        {
            builder.capability_pack(eliteforge_agent_capability_external::external_agent_pack())
        } else {
            builder
        };
        let builder = if enabled("storage") {
            builder.plugin(SqliteStoragePlugin::new(&self.0.path))
        } else {
            builder
        };
        let builder = if enabled("model") {
            builder.plugin(fw::ModelConnectionPlugin::new(
                fw::ModelConnection::new(
                    "tron",
                    Arc::new(TronChatModel),
                    vec![descriptor, fw::ModelDescriptor::new("compaction")],
                )
                .with_protocol("tron-host")
                .with_token_estimator(Arc::new(TronTokenEstimator)),
            ))
        } else {
            builder
        };
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
                tron_agent_harness::TronHarness::new(
                    Arc::new(TronContext),
                    Arc::new(HarnessTools),
                    [],
                )
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

    pub async fn close(&self) {
        if let Some(agent) = self.0.agent.get() {
            agent.close().await;
        }
        let pty = self.pty();
        let _ = tokio::task::spawn_blocking(move || pty.close()).await;
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
            path,
            state_path,
            pack_root,
            capability_state: std::sync::RwLock::new(packs),
            extension_state: std::sync::RwLock::new(extensions),
            config_lock: tokio::sync::Mutex::new(()),
            agent: OnceCell::new(),
            pty: Default::default(),
            resources,
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
            state_path: path.with_extension("capabilities.json"),
            pack_root: path.with_extension("capability-packs"),
            path,
            capability_state: std::sync::RwLock::new(BTreeMap::new()),
            extension_state: std::sync::RwLock::new(BTreeMap::new()),
            config_lock: tokio::sync::Mutex::new(()),
            agent: OnceCell::new(),
            pty: Default::default(),
            resources,
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
                let (packs, extensions) = self.capability_state();
                self.agent_builder(&packs, &extensions)
                    .map_err(fw::AgentError::InvalidConfiguration)?
                    .build()
                    .await
            })
            .await
    }
    pub(crate) async fn run_external(
        &self,
        transport: Arc<dyn crate::application::agent::AgentRuntime>,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
        resume: bool,
    ) -> Result<AgentRunResult, crate::application::agent::AgentInvocationError> {
        use crate::application::agent::AgentInvocationError;
        let map = |error: fw::AgentError| AgentInvocationError::RuntimeFailed(error.to_string());
        let agent = self.agent().await.map_err(map)?;
        let session = context
            .conversation_code
            .clone()
            .unwrap_or_else(|| context.run_code.clone());
        let input = context
            .messages
            .iter()
            .rfind(|message| message.role == "user")
            .ok_or(AgentInvocationError::PromptRequired)?;
        let input = super::messages::encode(input)
            .map_err(AgentInvocationError::RuntimeFailed)?
            .content;
        let mut request = fw::RunRequest::new(&session, input);
        request.model = Some(fw::ModelRef::new(
            eliteforge_agent_capability_external::CONNECTION,
            "agent",
        ));
        request.limits.max_steps = 1;
        request.limits.max_model_retries = 0;
        request.limits.max_context_tokens =
            context.model.context_window_tokens.unwrap_or(128_000) as usize;
        request.limits.max_model_output_tokens = context
            .model
            .max_tokens
            .filter(|value| *value > 0)
            .unwrap_or(4096);
        request.limits.model_timeout = Duration::from_secs(3600);
        request.limits.run_timeout = Duration::from_secs(3600);
        request
            .extensions
            .insert(super::external::ExternalExecution {
                transport,
                context: context.clone(),
                sink,
                result: std::sync::Mutex::new(None),
            });
        tron_agent_harness::TronRunScope {
            user_code: context.user_code.unwrap_or_else(|| "system".into()),
            workspace_code: context.workspace_code,
            session_code: session,
            run_code: context.run_code,
            app_code: context.execution_scope.app_code,
            task_code: context.execution_scope.task_code,
            agent_code: context
                .short_term_memory
                .and_then(|memory| memory.agent_code),
        }
        .attach(&mut request)
        .map_err(map)?;
        let execution = request
            .extensions
            .get::<super::external::ExternalExecution>()
            .expect("registered external execution");
        request
            .extensions
            .insert(eliteforge_agent_capability_external::ExternalStage {
                scope: execution.session_scope().ok_or_else(|| {
                    AgentInvocationError::RuntimeFailed("external scope is missing".into())
                })?,
                session_id: execution.context.system_agent_session_id.clone(),
                require_resume: resume,
                invocation: execution.clone(),
            });
        let result = agent.run(request).await;
        let native = execution
            .result
            .lock()
            .map_err(|_| {
                AgentInvocationError::RuntimeFailed("external result lock unavailable".into())
            })?
            .take();
        if let Some(Err(error)) = native {
            return Err(error);
        }
        result.map_err(map)?;
        native.ok_or_else(|| {
            AgentInvocationError::RuntimeFailed(
                "external runtime completed without a result".into(),
            )
        })?
    }

    pub(crate) async fn run(
        &self,
        harness: ReactAgent,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
        steering: Option<ActiveRunControl>,
    ) -> Result<AgentRunResult, AgentError> {
        harness.framework_started(&context)?;
        let agent = self.agent().await.map_err(map_error)?;
        let session = context
            .conversation_code
            .clone()
            .unwrap_or_else(|| context.run_code.clone());
        let memory_input = context
            .short_term_memory
            .as_ref()
            .and_then(|memory| memory.current_input_message.as_ref());
        let (history, current) = if let Some(input) = memory_input {
            let history = if context.messages.last().is_some_and(|last| last == input) {
                &context.messages[..context.messages.len() - 1]
            } else {
                &context.messages[..]
            };
            (history, input)
        } else {
            let current = context
                .messages
                .iter()
                .rposition(|message| message.role == "user")
                .ok_or_else(|| {
                    AgentError::InvalidConfiguration("run requires a current user message".into())
                })?;
            if current + 1 != context.messages.len() {
                return Err(AgentError::InvalidConfiguration(
                    "current input must follow the prepared conversation history".into(),
                ));
            }
            (&context.messages[..current], &context.messages[current])
        };
        let history =
            super::messages::encode_history(history).map_err(AgentError::InvalidConfiguration)?;
        let input = super::messages::encode(current)
            .map_err(AgentError::InvalidConfiguration)?
            .content;
        let answer = current.parts.iter().find_map(|part| match part {
            crate::domain::agent::AgentMessagePart::Context {
                kind,
                metadata: Some(metadata),
                ..
            } if kind == "tron_interaction_response" => metadata
                .get("resume")
                .and_then(|value| value.as_array())
                .and_then(|entries| entries.first())
                .and_then(|entry| {
                    entry
                        .get("interruptId")
                        .and_then(|id| id.as_str())
                        .and_then(|id| id.strip_prefix("INPUT-FW-"))
                        .map(|id| {
                            (
                                id.to_owned(),
                                entry.get("payload").cloned().unwrap_or_default(),
                            )
                        })
                }),
            _ => None,
        });
        let key = fw::ConversationKey::new(&session);
        let snapshot = agent.conversation(&key).await.map_err(map_error)?;
        if snapshot.revision == 0 && !history.is_empty() {
            agent
                .append_conversation(&key, 0, history)
                .await
                .map_err(map_error)?;
        }
        let mut request = fw::RunRequest::new(&session, input);
        if let Some((answer_id, payload)) = answer {
            let id = fw::RunId::parse_str(&answer_id)
                .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
            let resumed = agent
                .answer(&session, id, super::question::framework_answer(&payload)?)
                .await
                .map_err(map_error)?;
            request.resume = resumed.resume;
        }
        request.model = Some(fw::ModelRef::new("tron", "route"));
        request.limits.max_steps = context.limits.max_steps as usize;
        request.limits.max_tool_calls = context.limits.max_tool_calls as usize;
        request.limits.tool_batch_budget_policy = fw::ToolBatchBudgetPolicy::ExecuteAvailable;
        // Domain discovery controls schema exposure; the effective catalog owns authority.
        request.limits.allow_unexposed_tools = true;
        request.limits.max_parallel_tools = context.limits.max_parallel_tool_calls.max(1) as usize;
        request.limits.max_context_messages = context
            .messages
            .len()
            .max(256)
            .saturating_add(request.limits.max_steps + request.limits.max_tool_calls * 2);
        request.limits.max_context_tokens =
            context.model.context_window_tokens.unwrap_or(128_000) as usize;
        request.limits.max_model_output_tokens = context
            .model
            .max_tokens
            .filter(|value| *value > 0)
            .unwrap_or(4096);
        request.limits.max_total_tokens = (request.limits.max_context_tokens as u64
            + u64::from(request.limits.max_model_output_tokens))
        .saturating_mul(request.limits.max_steps as u64);
        request.limits.model_timeout = Duration::from_secs(300);
        request.limits.run_timeout =
            Duration::from_secs(300 * context.limits.max_steps.max(1) as u64);
        request.limits.max_model_retries = if context.auto_model_route.is_some() {
            2
        } else {
            context.model.max_retries.unwrap_or(0).min(3) as usize
        };
        request.limits.max_model_calls_per_step =
            Some((request.limits.max_model_retries + 1).max(3));
        request.limits.max_overflow_recoveries = harness.framework_overflow_limit();
        request.generation.max_output_tokens = context.model.max_tokens.filter(|value| *value > 0);
        let state = Arc::new(Mutex::new(HostRunState::new(context.clone())));
        request
            .extensions
            .insert(fw::ToolResultProcessorService(Arc::new(ToolAudit)));
        request.extensions.insert(
            eliteforge_agent_official_plugin_tools_local::CommandProgressSinkService(Arc::new(
                super::tools::CommandProgress {
                    harness: harness.clone(),
                    context: context.clone(),
                },
            )),
        );
        request.extensions.insert(HostExecution {
            harness: harness.clone(),
            state: state.clone(),
            sink,
        });
        tron_agent_harness::TronRunScope {
            user_code: context.user_code.clone().unwrap_or_else(|| "system".into()),
            workspace_code: context.workspace_code.clone(),
            session_code: session,
            run_code: context.run_code.clone(),
            app_code: context.execution_scope.app_code.clone(),
            task_code: context.execution_scope.task_code.clone(),
            agent_code: context
                .short_term_memory
                .as_ref()
                .and_then(|memory| memory.agent_code.clone()),
        }
        .attach(&mut request)
        .map_err(map_error)?;
        if let Some(resources) = &self.0.resources {
            request
                .extensions
                .insert(FileReferenceReaderService(Arc::new(AttachmentReader(
                    resources.clone(),
                ))));
        }
        if let Some(control) = steering {
            request
                .extensions
                .insert(fw::LiveInputSourceService(Arc::new(HostSteering {
                    control,
                })));
        }
        let result = agent.run(request).await;
        let state = state.lock().await;
        let failure = match &result {
            Ok(result) if result.status == fw::RunStatus::StepLimit => {
                Some(AgentError::MaxStepsExceeded)
            }
            Ok(result) if result.status == fw::RunStatus::ToolCallLimit => {
                Some(AgentError::MaxToolCallsExceeded)
            }
            Err(error) => Some(
                state
                    .error
                    .clone()
                    .unwrap_or_else(|| AgentError::InvalidConfiguration(error.to_string())),
            ),
            _ => None,
        };
        if let Some(error) = failure {
            harness
                .framework_finished(&state.context, &Err(fw::AgentError::Run(error.to_string())))?;
            return Err(error);
        }
        harness.framework_finished(&state.context, &result)?;
        let result = result.map_err(map_error)?;
        let output = state.final_output.clone();
        Ok(AgentRunResult {
            run_code: context.run_code,
            conversation_code: context.conversation_code,
            workspace_code: context.workspace_code,
            final_output: output,
            step_count: result.steps as u32,
            token_usage: state.usage,
            latest_request_estimate: state.estimate.clone(),
            latest_usage_audit: state.audit.clone(),
            system_agent_session_id: state.context.system_agent_session_id.clone(),
            observations: state.context.observations.clone(),
        })
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
    AgentError::InvalidConfiguration(error.to_string())
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct CapabilityState {
    packs: BTreeMap<String, bool>,
    extensions: BTreeMap<String, bool>,
}
fn load_capability_state(path: &Path) -> (BTreeMap<String, bool>, BTreeMap<String, bool>) {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CapabilityState>(&bytes).ok())
        .map(|state| (state.packs, state.extensions))
        .unwrap_or_default()
}
fn persist_capability_state(
    path: &Path,
    packs: &BTreeMap<String, bool>,
    extensions: &BTreeMap<String, bool>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec(&CapabilityState {
        packs: packs.clone(),
        extensions: extensions.clone(),
    })
    .map_err(|error| error.to_string())?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(|error| error.to_string())?;
    fs::rename(temp, path).map_err(|error| error.to_string())
}
