use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

mod automation_runner;
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
                PersonAiAwareLlmRuntimeClient, PersonAiModelProviderClient,
            },
            persistence::{
                GrafeoStoreCore, GraphBackedRagRepository, SqliteAgentEventReplayStore,
                SqliteAgentManagementRepository, SqliteAgentRunRepository, SqliteAppRepository,
                SqliteAutomationRepository, SqliteChatRepository, SqliteMcpRepository,
                SqliteMemoryWorkDomainRepository, SqliteModelRepository,
                SqliteNavigationRepository, SqliteProjectRepository,
                SqlitePromptRepository, SqliteResourceRepository,
                SqliteShortTermMemoryRepository, SqliteSkillRegistryRepository,
                SqliteTaskRepository, SqliteVariableRepository,
                SqliteWorkspaceRepository,
            },
            resource_connectivity::ProtocolResourceConnectionVerifier,
            resource_storage::LocalObjectStorage,
            s3_storage::S3ObjectStorage,
        },
    },
    application::agent::{
        ActiveRunRegistry, AgentAguiReplayStore, AgentInvocationService, AgentManagementService,
        AgentRunHistoryService, ModelClient, register_default_agent_tools,
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
    application::rag::{DefaultRagRuntime, DefaultRagStrategyRegistry, RagService},
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

static GLOBAL_WORKSPACE_SERVICE: OnceLock<WorkspaceService> = OnceLock::new();
static GLOBAL_APP_SERVICE: OnceLock<AppService> = OnceLock::new();
static GLOBAL_PROJECT_SERVICE: OnceLock<ProjectService> = OnceLock::new();
static GLOBAL_TASK_SERVICE: OnceLock<TaskService> = OnceLock::new();
static GLOBAL_VARIABLE_SERVICE: OnceLock<VariableService> = OnceLock::new();
static GLOBAL_RESOURCE_SERVICE: OnceLock<ResourceService> = OnceLock::new();
static GLOBAL_SKILL_SERVICE: OnceLock<SkillRegistryService> = OnceLock::new();
static GLOBAL_MODEL_SERVICE: OnceLock<ModelService> = OnceLock::new();
static GLOBAL_CHAT_SERVICE: OnceLock<ChatService> = OnceLock::new();
static GLOBAL_SHORT_TERM_MEMORY_SERVICE: OnceLock<ShortTermMemoryService> = OnceLock::new();
static GLOBAL_MEMORY_WORK_DOMAIN_SERVICE: OnceLock<MemoryWorkDomainService> = OnceLock::new();
static GLOBAL_NAVIGATION_SERVICE: OnceLock<NavigationService> = OnceLock::new();
static GLOBAL_GRAPH_CORE: OnceLock<GrafeoStoreCore> = OnceLock::new();
static GLOBAL_GRAPH_SERVICES: OnceLock<(GraphService, OntologyService, GraphExplorerService)> =
    OnceLock::new();

pub type AppRagService =
    RagService<Arc<Mutex<GraphBackedRagRepository>>, DefaultRagStrategyRegistry, DefaultRagRuntime>;

pub type AppMemoryService = MemoryService<
    Arc<Mutex<GraphBackedRagRepository>>,
    DefaultRagStrategyRegistry,
    DefaultRagRuntime,
>;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub readiness: AppReadiness,
    pub agui_events: Arc<AguiEventSink>,
    pub agui_replay_store: Arc<dyn AgentAguiReplayStore>,
    pub agent_model_client: Arc<dyn ModelClient>,
    pub runtime_http_client: reqwest::Client,
    pub agent_management: AgentManagementService,
    pub agent_runs: AgentRunHistoryService,
    pub agent_tool_registry: Arc<ToolRegistry>,
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
}

impl AppState {
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
        let agui_events = Arc::new(AguiEventSink::new(&config.event_bus));
        let agui_replay_store = Arc::new(
            SqliteAgentEventReplayStore::connect(&config.database_url)
                .expect("agent event replay store should initialize"),
        );
        let event_bus = EventBusRuntimeBuilder::new(config.event_bus.clone(), id_provider.clone())
            .register_sink(agui_events.clone())
            .register_sink(agui_replay_store.clone())
            .build();
        let http_client_pool = HttpClientPool::from_profiles(&config.http_clients)
            .expect("http client pool should initialize");
        let model_inference_client = http_client_pool.model_inference_client();
        let runtime_http_client = http_client_pool.runtime_control_client();
        let document_parsing_client = http_client_pool.document_parsing_client();
        let workspaces = if config.database_url == ":memory:" {
            GLOBAL_WORKSPACE_SERVICE
                .get_or_init(|| {
                    build_workspace_service(
                        ":memory:",
                        event_bus.clone(),
                    )
                })
                .clone()
        } else {
            build_workspace_service(
                &config.database_url,
                event_bus.clone(),
            )
        };
        let navigation = if config.database_url == ":memory:" {
            GLOBAL_NAVIGATION_SERVICE
                .get_or_init(|| build_navigation_service(":memory:", config.navigation.max_depth))
                .clone()
        } else {
            build_navigation_service(&config.database_url, config.navigation.max_depth)
        };
        let apps = if config.database_url == ":memory:" {
            GLOBAL_APP_SERVICE
                .get_or_init(|| build_app_service(":memory:", workspaces.clone()))
                .clone()
        } else {
            build_app_service(&config.database_url, workspaces.clone())
        };
        let projects = if config.database_url == ":memory:" {
            GLOBAL_PROJECT_SERVICE
                .get_or_init(|| build_project_service(":memory:", workspaces.clone(), apps.clone()))
                .clone()
        } else {
            build_project_service(&config.database_url, workspaces.clone(), apps.clone())
        };
        let tasks = if config.database_url == ":memory:" {
            GLOBAL_TASK_SERVICE
                .get_or_init(|| {
                    build_task_service(
                        ":memory:",
                        workspaces.clone(),
                        apps.clone(),
                        event_bus.clone(),
                    )
                })
                .clone()
        } else {
            build_task_service(
                &config.database_url,
                workspaces.clone(),
                apps.clone(),
                event_bus.clone(),
            )
        };
        let terminal = TerminalService::new(
            apps.clone(),
            workspaces.clone(),
            config.terminal_working_directory.clone(),
        );
        let variables = if config.database_url == ":memory:" {
            GLOBAL_VARIABLE_SERVICE
                .get_or_init(|| build_variable_service(":memory:", workspaces.clone(), &config))
                .clone()
        } else {
            build_variable_service(&config.database_url, workspaces.clone(), &config)
        };
        let resources = if config.database_url == ":memory:" {
            GLOBAL_RESOURCE_SERVICE
                .get_or_init(|| {
                    build_resource_service(
                        ":memory:",
                        workspaces.clone(),
                        variables.clone(),
                        &config,
                        event_bus.clone(),
                    )
                })
                .clone()
        } else {
            build_resource_service(
                &config.database_url,
                workspaces.clone(),
                variables.clone(),
                &config,
                event_bus.clone(),
            )
        };
        let mcp = build_mcp_service(
            &config.database_url,
            workspaces.clone(),
            variables.clone(),
            runtime_http_client.clone(),
        );
        let skills = if config.database_url == ":memory:" {
            GLOBAL_SKILL_SERVICE
                .get_or_init(|| {
                    build_skill_registry_service(
                        ":memory:",
                        workspaces.clone(),
                        &config.skills_registry_root,
                    )
                })
                .clone()
        } else {
            build_skill_registry_service(
                &config.database_url,
                workspaces.clone(),
                &config.skills_registry_root,
            )
        };
        let agent_management = build_agent_management_service(
            &config.database_url,
            workspaces.clone(),
            &config.system_agents,
        );
        if let Err(error) = agent_management.bootstrap_registry() {
            eprintln!("agent management bootstrap scan failed: {error:?}");
        }
        let automations =
            build_automation_service(&config.database_url, workspaces.clone(), event_bus.clone());
        let models = if config.database_url == ":memory:" {
            GLOBAL_MODEL_SERVICE
                .get_or_init(|| {
                    build_model_service(":memory:", workspaces.clone(), variables.clone())
                })
                .clone()
        } else {
            build_model_service(&config.database_url, workspaces.clone(), variables.clone())
        };
        let model_providers = build_model_provider_service(&config.database_url, variables.clone());
        let agent_model_client = Arc::new(HttpModelClient::new(Arc::new(
            PersonAiModelProviderClient::new(
                HttpModelProviderClient::new(model_inference_client.clone()),
                model_providers.clone(),
            ),
        )));
        let prompts = build_prompt_service(
            &config.database_url,
            workspaces.clone(),
            variables.clone(),
            id_provider.clone(),
        );
        let llm_runtime = LlmRuntimeService::new(
            Arc::new(ProviderAwareLlmRuntimeResolver::new(
                models.clone(),
                model_providers.clone(),
            )),
            Arc::new(PersonAiAwareLlmRuntimeClient::new(
                HttpLlmRuntimeClient::new(model_inference_client),
                model_providers.clone(),
            )),
        );
        let prompts = prompts.with_llm_runtime(llm_runtime.clone());
        let llm_router = LlmModelRouter::new(model_providers.clone());
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
            graph_core.clone(),
            workspaces.clone(),
            llm_runtime.clone(),
            llm_router.clone(),
            prompts.clone(),
            &config.project_doc_root,
            config.mineru.clone(),
            document_parsing_client,
            event_bus.clone(),
        )?;
        let memory = MemoryService::new(rag.clone());
        let memory_work_domains = if config.database_url == ":memory:" {
            GLOBAL_MEMORY_WORK_DOMAIN_SERVICE
                .get_or_init(|| build_memory_work_domain_service(":memory:"))
                .clone()
        } else {
            build_memory_work_domain_service(&config.database_url)
        };
        let short_term_memory = if config.database_url == ":memory:" {
            GLOBAL_SHORT_TERM_MEMORY_SERVICE
                .get_or_init(|| {
                    build_short_term_memory_service_with_id_provider(
                        ":memory:",
                        &config,
                        id_provider.clone(),
                        variables.cipher(),
                    )
                })
                .clone_with_id_provider(id_provider.clone())
        } else {
            build_short_term_memory_service_with_id_provider(
                &config.database_url,
                &config,
                id_provider.clone(),
                variables.cipher(),
            )
        };
        let short_term_memory = short_term_memory.with_event_bus(event_bus.clone());
        let agent_tool_registry = Arc::new(ToolRegistry::default());
        let agent_terminal: Arc<dyn crate::application::agent::AgentTerminal> = Arc::new(crate::adapters::outbound::agent_terminal::LocalAgentTerminal::default());
        register_default_agent_tools(
            agent_tool_registry.as_ref(),
            workspaces.clone(),
            apps.clone(),
            automations.clone(),
            tasks.clone(),
            models.clone(),
            skills.clone(),
            variables.clone(),
            resources.clone(),
            prompts.clone(),
            mcp.clone(),
            memory.clone(),
            short_term_memory.clone(),
            Arc::new(crate::adapters::outbound::code_file_editor::LocalCodeFileEditor),
            agent_terminal.clone(),
        )
        .expect("default agent tools should register");
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
        let agent_runs = build_agent_run_history_service(&config.database_url);
        let active_runs = ActiveRunRegistry::new();
        let chats = if config.database_url == ":memory:" {
            GLOBAL_CHAT_SERVICE
                .get_or_init(|| {
                    build_chat_service_with_id_provider(":memory:", id_provider.clone())
                })
                .clone_with_id_provider(id_provider.clone())
        } else {
            build_chat_service_with_id_provider(&config.database_url, id_provider)
        }
        .with_active_runs(active_runs.clone())
        .with_projects(projects.clone())
        .with_execution_memory(short_term_memory.clone())
        .with_resources(resources.clone())
        .with_prompt_service(prompts.clone())
        .with_llm_router(llm_router.clone())
        .with_event_bus(event_bus.clone());
        automations.set_execution_runner(Arc::new(MainAgentAutomationRunner::new(
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
            .with_directory_inspector(Some(std::sync::Arc::new(
                crate::adapters::outbound::application_directory::GitApplicationDirectoryInspector,
            )))
            .with_active_runs(active_runs.clone())
            .with_prompt_service(prompts.clone()),
        )));
        automations
            .bootstrap_scheduler()
            .expect("automation scheduler should initialize");
        let readiness = AppReadiness::validated_ready();
        let system_config = SystemConfigService::new(Arc::new(YamlSystemConfigStore::new(
            config.application_config_path.clone(),
        )));
        Ok(Self {
            config,
            readiness,
            agui_events,
            agui_replay_store,
            agent_model_client,
            runtime_http_client,
            agent_management,
            agent_runs,
            agent_tool_registry,
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
            skills,
            system_config,
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
    state.model_providers.start_background_sync();
    let protected_routes = build_protected_routes(&state);
    let profile_routes = build_profile_routes();
    let terminal_gateway = terminal_socket::TerminalSocketGateway::new(state.terminal.clone());
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
        .with_state(state)
        .layer(socket_layer))
}

include!("app/builders.rs");
