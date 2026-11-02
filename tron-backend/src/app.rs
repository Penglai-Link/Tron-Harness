use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

mod automation_runner;
mod dude_runner;
mod routing;
mod startup;
mod state_extractors;

use axum::{Router, routing::get};
use socketioxide::SocketIo;

use crate::{
    adapters::{
        inbound::http::{
            auth::require_profile, frontend::routes as frontend_routes,
            mcp::routes::transport_routes as mcp_transport_routes,
        },
        inbound::socket::terminal as terminal_socket,
        outbound::{
            config::YamlSystemConfigStore,
            crypto::VariableCipherProvider,
            model::{
                HttpLlmRuntimeClient, HttpModelClient, HttpModelProviderClient,
                PersonAiModelProviderClient,
            },
            persistence::{
                GrafeoStoreCore, SqliteAgentEventReplayStore, SqliteAgentManagementRepository,
                SqliteAgentRunRepository, SqliteAppRepository, SqliteAutomationRepository,
                SqliteChatRepository, SqliteMcpRepository, SqliteMemoryWorkDomainRepository,
                SqliteModelRepository, SqliteNavigationRepository, SqliteProjectRepository,
                SqlitePromptRepository, SqliteRagRepository, SqliteResourceRepository,
                SqliteSkillRegistryRepository, SqliteTaskRepository, SqliteVariableRepository,
                SqliteWorkspaceRepository,
            },
            resource_connectivity::ProtocolResourceConnectionVerifier,
            resource_storage::LocalObjectStorage,
            s3_storage::S3ObjectStorage,
        },
    },
    application::agent::{
        ActiveRunRegistry, AgentAguiReplayStore, AgentAguiStreamService, AgentInvocationService,
        AgentManagementService, AgentRunHistoryService, ModelClient, register_default_agent_tools,
    },
    application::app::AppService,
    application::automation::AutomationService,
    application::chat::ChatService,
    application::graph::{GraphExplorerService, GraphService, OntologyService},
    application::llm::LlmRuntimeService,
    application::mcp::McpService,
    application::memory::{MemoryService, MemoryWorkDomainService},
    application::model::ModelService,
    application::model_provider::{ModelProviderService, ProviderAwareLlmRuntimeResolver},
    application::model_router::LlmModelRouter,
    application::navigation::NavigationService,
    application::profile::ProfileRegistryService,
    application::project::ProjectService,
    application::prompt::PromptService,
    application::rag::{DefaultRagRuntime, RagService},
    application::resource::{ObjectStorage, ResourceService},
    application::short_term_memory::ShortTermMemoryService,
    application::skill::SkillRegistryService,
    application::system_config::SystemConfigService,
    application::task::TaskService,
    application::terminal::TerminalService,
    application::variable::VariableService,
    application::workspace::WorkspaceService,
    config::{AppConfig, ResourceStorageProfile},
    domain::agent::ToolRegistry,
    health::{health, readiness},
    openapi::routes as openapi_routes,
    shared::{
        event_bus::{AguiEventSink, EventBusHandle, EventBusRuntimeBuilder},
        http_client::HttpClientPool,
        id::{IdGenerator, SnowflakeIdProvider, SnowflakedSnowflakeIdProvider},
    },
};

pub use self::startup::{AppBootstrapError, AppReadiness, AppReadinessStatus};
use self::{
    automation_runner::MainAgentAutomationRunner,
    routing::{build_profile_routes, build_protected_routes},
    startup::{build_graph_core, build_graph_services, build_rag_service},
};

static GLOBAL_GRAPH_CORE: OnceLock<GrafeoStoreCore> = OnceLock::new();
static GLOBAL_GRAPH_SERVICES: OnceLock<(GraphService, OntologyService, GraphExplorerService)> =
    OnceLock::new();

pub type AppRagService = RagService<Arc<Mutex<SqliteRagRepository>>, DefaultRagRuntime>;

pub type AppMemoryService = MemoryService<Arc<Mutex<SqliteRagRepository>>, DefaultRagRuntime>;

#[derive(Clone)]
pub struct AppState {
    pub dude_memory: crate::application::dude_memory::DudeMemoryService,
    pub dudes: crate::application::dude::DudeService,
    pub config: AppConfig,
    _temporary_database: Option<Arc<tempfile::TempDir>>,
    _agent_collaboration_builder: Arc<crate::application::agent::AgentRunContextBuilder>,
    pub readiness: AppReadiness,
    pub agui_events: Arc<AguiEventSink>,
    pub agui_replay_store: Arc<dyn AgentAguiReplayStore>,
    pub agent_presentation: AgentAguiStreamService,
    pub agent_model_client: Arc<dyn ModelClient>,
    pub runtime_http_client: reqwest::Client,
    pub agent_management: AgentManagementService,
    pub agent_runs: AgentRunHistoryService,
    pub agent_tool_registry: Arc<ToolRegistry>,
    pub agent_framework: crate::application::agent::FrameworkHost,
    pub agent_terminal: Arc<dyn crate::application::agent::AgentTerminal>,
    pub active_runs: ActiveRunRegistry,
    pub apps: AppService,
    pub projects: ProjectService,
    pub automations: AutomationService,
    pub chats: ChatService,
    pub short_term_memory: ShortTermMemoryService,
    pub event_bus: EventBusHandle,
    pub graphs: GraphService,
    pub ontologies: OntologyService,
    pub graph_explorer: GraphExplorerService,
    pub tasks: TaskService,
    pub terminal: TerminalService,
    pub variables: VariableService,
    pub resources: ResourceService,
    pub workspaces: WorkspaceService,
    pub skills: SkillRegistryService,
    pub system_config: SystemConfigService,
    pub skins: crate::application::skin::SkinService,
    pub models: ModelService,
    pub model_providers: ModelProviderService,
    pub prompts: PromptService,
    pub profiles: ProfileRegistryService,
    pub llm_runtime: LlmRuntimeService,
    pub llm_router: LlmModelRouter,
    pub mcp: McpService,
    pub memory: AppMemoryService,
    pub memory_work_domains: MemoryWorkDomainService,
    pub navigation: NavigationService,
    pub rag: AppRagService,
    pub knowledge_bases: crate::application::knowledge_base::KnowledgeBaseService,
}

impl AppState {
    pub(crate) fn dude_invocation_service(&self) -> AgentInvocationService {
        AgentInvocationService::new(
            self.workspaces.clone(),
            self.apps.clone(),
            self.variables.clone(),
            self.skills.clone(),
            self.agent_management.clone(),
            self.agent_runs.clone(),
            self.mcp.clone(),
            self.event_bus.clone(),
            self.agent_model_client.clone(),
            self.agent_tool_registry.clone(),
            self.runtime_http_client.clone(),
        )
        .with_framework_host(self.agent_framework.clone())
        .with_active_runs(self.active_runs.clone())
        .with_short_term_memory(self.short_term_memory.clone())
        .with_prompt_service(self.prompts.clone())
        .with_platform_scope()
        .with_dude_work_context(self.dudes.clone())
    }
    pub fn new(config: AppConfig) -> Result<Self, AppBootstrapError> {
        Self::new_with_id_provider(config, Arc::new(SnowflakedSnowflakeIdProvider::default()))
    }

    pub fn new_with_id_provider<I>(
        config: AppConfig,
        id_provider: Arc<I>,
    ) -> Result<Self, AppBootstrapError>
    where
        I: SnowflakeIdProvider + IdGenerator + 'static,
    {
        let profiles = if config.database_url == ":memory:" {
            ProfileRegistryService::new(":memory:")
        } else {
            ProfileRegistryService::from_environment()
        }
        .map_err(|source| AppBootstrapError::ProfileRegistryInitFailed { source })?;
        Self::new_with_profiles(config, id_provider, profiles)
    }

    /// Explicit identity registry for embedded hosts and isolated integration tests.
    pub fn new_with_profiles<I>(
        mut config: AppConfig,
        id_provider: Arc<I>,
        profiles: ProfileRegistryService,
    ) -> Result<Self, AppBootstrapError>
    where
        I: SnowflakeIdProvider + IdGenerator + 'static,
    {
        let temporary_database = if config.database_url == ":memory:" {
            let directory = Arc::new(
                tempfile::tempdir()
                    .map_err(|source| AppBootstrapError::DatabaseInitFailed { source })?,
            );
            config.database_url = directory
                .path()
                .join("tron.sqlite3")
                .to_string_lossy()
                .into_owned();
            Some(directory)
        } else {
            None
        };
        let runtime_config =
            crate::application::system_config::BackendRuntimeConfig::new(config.clone());
        let agui_events = Arc::new(AguiEventSink::new(&config.event_bus));
        let agui_replay_store = Arc::new(
            SqliteAgentEventReplayStore::new().expect("agent event replay store should initialize"),
        );
        let agent_presentation = AgentAguiStreamService::with_replay_store(
            agui_events.clone(),
            agui_replay_store.clone(),
        )
        .with_transient_protocol();
        let event_bus = EventBusRuntimeBuilder::new(config.event_bus.clone(), id_provider.clone())
            .register_sink(agui_events.clone())
            .register_sink(agui_replay_store.clone())
            .build();
        let http_client_pool = HttpClientPool::from_profiles(&config.http_clients)
            .expect("http client pool should initialize");
        let model_inference_client = http_client_pool.model_inference_client();
        let runtime_http_client = http_client_pool.runtime_control_client();
        let workspaces = build_workspace_service(&config.database_url, event_bus.clone());
        let navigation =
            build_navigation_service(&config.database_url, config.navigation.max_depth)
                .with_runtime_config(runtime_config.clone());
        let apps = build_app_service(&config.database_url, workspaces.clone());
        let projects =
            build_project_service(&config.database_url, workspaces.clone(), apps.clone());
        let tasks = build_task_service(
            &config.database_url,
            workspaces.clone(),
            apps.clone(),
            event_bus.clone(),
        );
        let terminal = TerminalService::new(
            apps.clone(),
            workspaces.clone(),
            config.terminal_working_directory.clone(),
        )
        .with_runtime_config(runtime_config.clone());
        let variables = build_variable_service(&config.database_url, workspaces.clone(), &config);
        let resources = build_resource_service(
            &config.database_url,
            workspaces.clone(),
            variables.clone(),
            &config,
            event_bus.clone(),
        );
        let knowledge_base_repository =
            crate::adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository::connect(
                &config.database_url,
            )
            .map_err(|_| AppBootstrapError::RagInitFailed {
                database_url: config.database_url.clone(),
                source: crate::domain::rag::RagError::StorageFailure,
            })?;
        let connection_verifier =
            crate::adapters::outbound::resource_connectivity::ProtocolResourceConnectionVerifier::new(
                std::time::Duration::from_millis(config.resource.connection_verification_timeout_ms),
            )
            .with_runtime_config(runtime_config.clone());
        let resources = resources
            .with_connection_verifier(Arc::new(connection_verifier))
            .with_runtime_config(runtime_config.clone())
            .with_privacy_policy(Arc::new(knowledge_base_repository.clone()));
        let mcp = build_mcp_service(
            &config.database_url,
            workspaces.clone(),
            variables.clone(),
            runtime_http_client.clone(),
        );
        let skills =
            build_skill_registry_service(&config.database_url, &config.skills_registry_root);
        let agent_management = build_agent_management_service(
            &config.database_url,
            workspaces.clone(),
            &config.system_agents,
        );
        agent_management
            .initialize_default_main_agent()
            .expect("default main Agent");
        if let Err(error) = agent_management.bootstrap_registry() {
            tracing::error!("agent management bootstrap scan failed: {error:?}");
        }
        let automations =
            build_automation_service(&config.database_url, workspaces.clone(), event_bus.clone());
        let model_providers = build_model_provider_service(&config.database_url, variables.clone());
        let models = build_model_service(
            &config.database_url,
            variables.clone(),
            model_providers.clone(),
        );
        let agent_model_client = Arc::new(
            HttpModelClient::new(Arc::new(PersonAiModelProviderClient::new(
                HttpModelProviderClient::new(model_inference_client.clone()),
                model_providers.clone(),
            )))
            .with_model_executor(model_providers.model_executor())
            .with_runtime_config(runtime_config.clone()),
        );
        let prompts =
            build_prompt_service(&config.database_url, variables.clone(), id_provider.clone());
        let llm_runtime = LlmRuntimeService::new(
            Arc::new(
                ProviderAwareLlmRuntimeResolver::new(model_providers.clone())
                    .with_configured_providers(models.clone()),
            ),
            Arc::new(
                HttpLlmRuntimeClient::with_model_client(agent_model_client.clone())
                    .with_prompt_service(prompts.clone()),
            ),
        );
        let prompts = prompts.with_llm_runtime(llm_runtime.clone());
        let llm_router = LlmModelRouter::new(model_providers.clone()).with_configured_providers(models.clone());
        let prompts = prompts.with_llm_router(llm_router.clone());
        let graph_core = if config.graph_storage_path == ":memory:" {
            if let Some(core) = GLOBAL_GRAPH_CORE.get() {
                core.clone()
            } else {
                let core = build_graph_core(&config.graph_storage_path)?;
                let _ = GLOBAL_GRAPH_CORE.set(core.clone());
                core
            }
        } else {
            build_graph_core(&config.graph_storage_path)?
        };
        let rag = build_rag_service(
            &config.database_url,
            llm_runtime.clone(),
            llm_router.clone(),
            graph_core.clone(),
        )?
        .with_runtime_config(runtime_config.clone());
        crate::application::rag::RagConfigRepository::recover_record_tasks(
            &rag.repository_handle(),
        )
        .map_err(|source| AppBootstrapError::RagInitFailed {
            database_url: config.database_url.clone(),
            source,
        })?;
        if tokio::runtime::Handle::try_current().is_ok() {
            rag.start_graph_maintenance()
                .map_err(|_| AppBootstrapError::RagInitFailed {
                    database_url: config.database_url.clone(),
                    source: crate::domain::rag::RagError::StorageFailure,
                })?;
        }
        let knowledge_bases = crate::application::knowledge_base::KnowledgeBaseService::new(
            knowledge_base_repository,
            rag.clone(),
            resources.clone(),
            agent_management.clone(),
            llm_runtime.clone(),
        )
        .with_document_parser(Arc::new(
            crate::adapters::outbound::person_ai_documents::PersonAiDocumentParser::new(
                model_providers.clone(),
            ),
        ))
        .with_image_descriptions(config.mineru.image_description.clone())
        .with_visual_context(config.mineru.visual_context_enabled)
        .with_office_renderer(Arc::new(
            crate::adapters::outbound::office_visuals::LocalOfficeChartRenderer,
        ))
        .with_ingestion_config(config.mineru.ingestion.clone())
        .with_runtime_config(runtime_config.clone());
        if tokio::runtime::Handle::try_current().is_ok() {
            knowledge_bases.start_ingestion_workers().map_err(|_| {
                AppBootstrapError::RagInitFailed {
                    database_url: config.database_url.clone(),
                    source: crate::domain::rag::RagError::StorageFailure,
                }
            })?;
        }
        let memory = MemoryService::new(rag.clone());
        let memory_work_domains = build_memory_work_domain_service(&config.database_url);
        let agent_framework = crate::application::agent::FrameworkHost::for_database(
            &config.database_url,
            resources.clone(),
        )
        .expect("initialize Framework storage configuration");
        agent_framework.bind_runtime_config(runtime_config.clone());
        agent_framework.bind_prompt_service(prompts.clone());
        agent_framework
            .bind_personai(model_providers.clone())
            .expect("initial PersonAI tool credentials");
        skills.bind_agents(agent_management.clone());
        knowledge_bases.bind_skills(skills.clone());
        agent_framework.bind_agent_management(agent_management.clone());
        let capability_resources=crate::application::capability::CapabilityResources::new(crate::adapters::outbound::persistence::sqlite::capability_repository::SqliteCapabilityRepository::connect(&config.database_url).expect("capability resource storage"));
        agent_management.bind_capabilities(capability_resources.clone());
        mcp.bind_capabilities(capability_resources.clone());
        agent_management.bind_mcp(mcp.clone());
        skills.bind_capabilities(capability_resources.clone());
        agent_framework
            .bind_skills(
                skills.clone(),
                capability_resources,
                mcp.clone(),
                rag.clone(),
            )
            .expect("capability resource catalog");
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let indexer =
                crate::application::skill::SkillRagIndexService::new(skills.clone(), rag.clone());
            runtime.spawn(async move {
                if let Err(error) = indexer.synchronize_all().await {
                    tracing::error!("platform skill index synchronization failed: {error}");
                }
            });
        }
        agui_replay_store
            .bind_framework(&agent_framework)
            .expect("Framework replay storage");
        agent_framework
            .bind_knowledge_bases(knowledge_bases.clone())
            .expect("initial knowledge-base binding");
        agent_framework
            .bind_presentation(agent_presentation.clone())
            .expect("initial presentation binding");
        agent_framework
            .bind_model_client(agent_model_client.clone())
            .expect("initial model client binding");
        let short_term_memory = ShortTermMemoryService::with_host(
            agent_framework.clone(),
            config.short_term_memory.clone(),
        )
        .with_runtime_config(runtime_config.clone());
        let agent_tool_registry = Arc::new(ToolRegistry::default());
        // The application owns this bridge. The host keeps a weak reference because
        // registered product tools may themselves hold the same FrameworkHost.
        let agent_collaboration_builder = Arc::new(
            crate::application::agent::AgentRunContextBuilder::new(
                workspaces.clone(),
                apps.clone(),
                variables.clone(),
                skills.clone(),
                agent_management.clone(),
                mcp.clone(),
                event_bus.clone(),
                agent_tool_registry.clone(),
                Some(short_term_memory.clone()),
                Some(prompts.clone()),
                None,
                Some(Arc::new(
                    crate::adapters::outbound::application_directory::GitApplicationDirectoryInspector,
                )),
            ).with_framework_host(Some(agent_framework.clone())).with_model_router(crate::application::model_router::AgentModelRouter::new(models.clone(), model_providers.clone()).with_agent_management(agent_management.clone())),
        );
        agent_framework
            .bind_collaboration(&agent_collaboration_builder)
            .expect("initial Agent collaboration binding");
        let agent_terminal: Arc<dyn crate::application::agent::AgentTerminal> = Arc::new(
            crate::adapters::outbound::agent_terminal::LocalAgentTerminal::new(
                agent_framework.pty(),
            ),
        );
        register_default_agent_tools(
            agent_tool_registry.as_ref(),
            workspaces.clone(),
            apps.clone(),
            automations.clone(),
            tasks.clone(),
            skills.clone(),
            variables.clone(),
            resources.clone(),
            prompts.clone(),
            mcp.clone(),
            memory.clone(),
            short_term_memory.clone(),
        )
        .expect("default agent tools should register");
        agent_tool_registry
            .register(crate::application::agent::rag_search_tool(
                rag.clone(),
                knowledge_bases.clone(),
            ))
            .expect("RAG search tool should register");
        for tool in crate::application::agent::rag_graph_tools(rag.clone(), knowledge_bases.clone())
        {
            agent_tool_registry
                .register(tool)
                .expect("RAG graph tools should register");
        }
        let (graphs, ontologies, graph_explorer) = if config.graph_storage_path == ":memory:" {
            if let Some(services) = GLOBAL_GRAPH_SERVICES.get() {
                services.clone()
            } else {
                let services = build_graph_services(
                    graph_core.clone(),
                    workspaces.clone(),
                    llm_runtime.clone(),
                );
                let _ = GLOBAL_GRAPH_SERVICES.set(services.clone());
                services
            }
        } else {
            build_graph_services(graph_core, workspaces.clone(), llm_runtime.clone())
        };

        let agent_runs = AgentRunHistoryService::new(
            SqliteAgentRunRepository::with_host(agent_framework.clone())
                .expect("Framework run history"),
        );
        let active_runs = ActiveRunRegistry::new();
        let chats = ChatService::with_id_provider(
            SqliteChatRepository::with_host(agent_framework.clone())
                .expect("Framework chat storage"),
            id_provider,
        )
        .with_active_runs(active_runs.clone())
        .with_projects(projects.clone())
        .with_execution_memory(short_term_memory.clone())
        .with_skill_registry(skills.clone())
        .with_resources(resources.clone())
        .with_prompt_service(prompts.clone())
        .with_llm_router(llm_router.clone())
        .with_event_bus(event_bus.clone());
        automations.set_execution_runner(Arc::new(MainAgentAutomationRunner::new(
            agent_framework.clone(),
            automations.clone(),
            chats.clone(),
            agent_management.clone(),
            models.clone(),
            AgentInvocationService::new(
                workspaces.clone(),
                apps.clone(),
                variables.clone(),
                skills.clone(),
                agent_management.clone(),
                agent_runs.clone(),
                mcp.clone(),
                event_bus.clone(),
                agent_model_client.clone(),
                agent_tool_registry.clone(),
                runtime_http_client.clone(),
            )
            .with_framework_host(agent_framework.clone())
            .with_directory_inspector(Some(std::sync::Arc::new(
                crate::adapters::outbound::application_directory::GitApplicationDirectoryInspector,
            )))
            .with_active_runs(active_runs.clone())
            .with_prompt_service(prompts.clone()),
        )));
        automations
            .bootstrap_scheduler()
            .expect("automation scheduler should initialize");
        let dudes = crate::application::dude::DudeService::new(
            crate::adapters::outbound::persistence::sqlite::SqliteDudeRepository::connect(
                &config.database_url,
            )
            .expect("Dude storage"),
            agent_management.clone(),
        );
        dudes.bootstrap().expect("Dude defaults");
        agent_framework.bind_dudes(dudes.clone());
        chats.bind_dudes(dudes.clone());
        chats.bind_knowledge_quality(knowledge_bases.clone());
        knowledge_bases
            .recover_quality_optimizations()
            .map_err(|e| AppBootstrapError::DatabaseInitFailed {
                source: std::io::Error::other(e.to_string()),
            })?;
        for tool in crate::application::agent::knowledge_quality_tools(
            crate::application::knowledge_quality::KnowledgeQualityService::new(
                knowledge_bases.clone(),
            ),
            chats.clone(),
        ) {
            agent_tool_registry
                .register(tool)
                .expect("knowledge evaluation tools should register");
        }
        crate::application::dude::register_dude_tools(
            agent_tool_registry.as_ref(),
            dudes.clone(),
            tasks.clone(),
            agent_framework.clone(),
        )
        .expect("Dude tools should register");
        let dude_memory = crate::application::dude_memory::DudeMemoryService::new(
            crate::adapters::outbound::persistence::sqlite::SqliteDudeMemoryRepository::connect(
                &config.database_url,
            )
            .expect("Dude memory storage"),
            skills.clone(),
            std::path::Path::new(&config.skills_registry_root).join(".dude-candidates"),
        );
        dude_memory.set_notifications(Arc::new(dude_runner::MemoryNotifications(dudes.clone())));

        let readiness = AppReadiness::validated_ready();
        let system_config = SystemConfigService::new(Arc::new(YamlSystemConfigStore::new(
            config.application_config_path.clone(),
        )))
        .with_runtime_config(runtime_config);
        let skin_root = std::path::Path::new(&config.application_config_path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("skins");
        let skins = crate::application::skin::SkinService::new(Arc::new(
            crate::adapters::outbound::skin::FileSkinStore::new(skin_root),
        ));
        Ok(Self {
            dude_memory,
            dudes,
            config,
            _temporary_database: temporary_database,
            _agent_collaboration_builder: agent_collaboration_builder,
            readiness,
            agui_events,
            agui_replay_store,
            agent_presentation,
            agent_model_client,
            runtime_http_client,
            agent_management,
            agent_runs,
            agent_tool_registry,
            agent_framework,
            agent_terminal,
            active_runs,
            apps,
            projects,
            automations,
            chats,
            short_term_memory,
            event_bus,
            graphs,
            ontologies,
            graph_explorer,
            models,
            model_providers,
            prompts,
            profiles,
            llm_runtime,
            llm_router,
            mcp,
            memory,
            memory_work_domains,
            navigation,
            rag,
            knowledge_bases,
            skills,
            system_config,
            skins,
            tasks,
            terminal,
            variables,
            resources,
            workspaces,
        })
    }
}

pub fn build_router(config: AppConfig) -> Result<Router, AppBootstrapError> {
    build_router_with_id_provider(config, Arc::new(SnowflakedSnowflakeIdProvider::default()))
}

pub fn build_router_with_id_provider<I>(
    config: AppConfig,
    id_provider: Arc<I>,
) -> Result<Router, AppBootstrapError>
where
    I: SnowflakeIdProvider + IdGenerator + 'static,
{
    let state = AppState::new_with_id_provider(config, id_provider)?;
    build_router_with_state(state)
}

/// Assemble routes from the same state owned by the host lifecycle.
pub fn build_router_with_state(state: AppState) -> Result<Router, AppBootstrapError> {
    dude_runner::start(state.clone());
    state.model_providers.start_background_sync();
    state.model_providers.start_person_ai_heartbeat();
    let protected_routes = build_protected_routes(&state);
    let profile_routes = build_profile_routes();
    let terminal_gateway = terminal_socket::TerminalSocketGateway::new(state.terminal.clone())
        .with_pty(state.agent_framework.pty());
    let (socket_layer, socket_io) = SocketIo::builder()
        .with_state(terminal_gateway)
        .max_payload(64 * 1024)
        .build_layer();
    terminal_socket::configure(&socket_io);

    Ok(Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/readiness", get(readiness))
        .merge(openapi_routes())
        .merge(profile_routes)
        .merge(
            mcp_transport_routes().route_layer(axum::middleware::from_fn_with_state(
                state.profiles.clone(),
                require_profile,
            )),
        )
        .merge(protected_routes)
        .merge(frontend_routes())
        .with_state(state.clone())
        .layer(socket_layer)
        .layer(axum::middleware::from_fn_with_state(
            state,
            crate::adapters::inbound::http::observability::log_application_request,
        )))
}

include!("app/builders.rs");

pub mod telemetry;
