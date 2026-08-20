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

use crate::{
    adapters::{
        inbound::http::{
            auth::routes as auth_routes, frontend::routes as frontend_routes,
            mcp::routes::transport_routes as mcp_transport_routes,
        },
        outbound::{
            config::YamlSystemConfigStore,
            crypto::{BcryptPasswordHasher, Sm4GcmVariableCipher},
            model::{HttpLlmRuntimeClient, HttpModelClient, HttpModelProviderClient},
            persistence::{
                GrafeoStoreCore, GraphBackedRagRepository, SqliteAgentEventReplayStore,
                SqliteAgentManagementRepository, SqliteAgentRunRepository, SqliteAppRepository,
                SqliteAutomationRepository, SqliteChatRepository, SqliteMcpRepository,
                SqliteMemoryWorkDomainRepository, SqliteModelRepository, SqliteOpenAppRepository,
                SqlitePromptRepository, SqliteResourceRepository, SqliteRoleRepository,
                SqliteShortTermMemoryRepository, SqliteSkillRegistryRepository,
                SqliteTaskRepository, SqliteUserRepository, SqliteVariableRepository,
                SqliteWorkspaceRepository, bootstrap_platform_admin,
            },
            resource_connectivity::ProtocolResourceConnectionVerifier,
            resource_storage::LocalObjectStorage,
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
    application::open_app::OpenAppService,
    application::prompt::PromptService,
    application::rag::{DefaultRagRuntime, DefaultRagStrategyRegistry, RagService},
    application::resource::{ObjectStorage, ResourceService},
    application::role::RoleService,
    application::short_term_memory::ShortTermMemoryService,
    application::skill::SkillRegistryService,
    application::system_config::SystemConfigService,
    application::task::TaskService,
    application::user::UserService,
    application::variable::VariableService,
    application::workspace::WorkspaceService,
    config::AppConfig,
    domain::agent::ToolRegistry,
    health::{health, readiness},
    openapi::routes as openapi_routes,
    shared::{
        crypto::Aes256SecretBox,
        event_bus::{AguiEventSink, EventBusHandle, EventBusRuntimeBuilder},
        http_client::HttpClientPool,
        id::{IdGenerator, SnowflakeIdProvider, SnowflakedSnowflakeIdProvider},
    },
};

pub use self::startup::{AppBootstrapError, AppReadiness, AppReadinessStatus};
use self::{
    automation_runner::MainAgentAutomationRunner,
    routing::{build_open_api_routes, build_protected_routes},
    startup::{build_graph_core, build_graph_services, build_rag_service},
};

static GLOBAL_USER_SERVICE: OnceLock<UserService> = OnceLock::new();
static GLOBAL_ROLE_SERVICE: OnceLock<RoleService> = OnceLock::new();
static GLOBAL_WORKSPACE_SERVICE: OnceLock<WorkspaceService> = OnceLock::new();
static GLOBAL_APP_SERVICE: OnceLock<AppService> = OnceLock::new();
static GLOBAL_TASK_SERVICE: OnceLock<TaskService> = OnceLock::new();
static GLOBAL_VARIABLE_SERVICE: OnceLock<VariableService> = OnceLock::new();
static GLOBAL_RESOURCE_SERVICE: OnceLock<ResourceService> = OnceLock::new();
static GLOBAL_SKILL_SERVICE: OnceLock<SkillRegistryService> = OnceLock::new();
static GLOBAL_MODEL_SERVICE: OnceLock<ModelService> = OnceLock::new();
static GLOBAL_CHAT_SERVICE: OnceLock<ChatService> = OnceLock::new();
static GLOBAL_SHORT_TERM_MEMORY_SERVICE: OnceLock<ShortTermMemoryService> = OnceLock::new();
static GLOBAL_OPEN_APP_SERVICE: OnceLock<OpenAppService> = OnceLock::new();
static GLOBAL_MEMORY_WORK_DOMAIN_SERVICE: OnceLock<MemoryWorkDomainService> = OnceLock::new();
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
    pub active_runs: ActiveRunRegistry,
    pub apps: AppService,
    pub automations: AutomationService,
    pub chats: ChatService,
    pub short_term_memory: ShortTermMemoryService,
    pub event_bus: EventBusHandle,
    pub graphs: GraphService,
    pub ontologies: OntologyService,
    pub graph_explorer: GraphExplorerService,
    pub roles: RoleService,
    pub tasks: TaskService,
    pub users: UserService,
    pub variables: VariableService,
    pub resources: ResourceService,
    pub workspaces: WorkspaceService,
    pub skills: SkillRegistryService,
    pub system_config: SystemConfigService,
    pub models: ModelService,
    pub prompts: PromptService,
    pub open_apps: OpenAppService,
    pub llm_runtime: LlmRuntimeService,
    pub mcp: McpService,
    pub memory: AppMemoryService,
    pub memory_work_domains: MemoryWorkDomainService,
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
        let agent_model_client = Arc::new(HttpModelClient::new(Arc::new(
            HttpModelProviderClient::new(model_inference_client.clone()),
        )));
        let users = if config.database_url == ":memory:" {
            GLOBAL_USER_SERVICE
                .get_or_init(|| build_user_service(&config, event_bus.clone()))
                .clone()
        } else {
            build_user_service(&config, event_bus.clone())
        };
        let roles = if config.database_url == ":memory:" {
            GLOBAL_ROLE_SERVICE
                .get_or_init(|| build_role_service(":memory:", event_bus.clone()))
                .clone()
        } else {
            build_role_service(&config.database_url, event_bus.clone())
        };
        let workspaces = if config.database_url == ":memory:" {
            GLOBAL_WORKSPACE_SERVICE
                .get_or_init(|| {
                    build_workspace_service(
                        ":memory:",
                        users.clone(),
                        roles.clone(),
                        event_bus.clone(),
                    )
                })
                .clone()
        } else {
            build_workspace_service(
                &config.database_url,
                users.clone(),
                roles.clone(),
                event_bus.clone(),
            )
        };
        bootstrap_platform_admin(&config.database_url)
            .expect("bootstrap platform admin data should initialize");
        let apps = if config.database_url == ":memory:" {
            GLOBAL_APP_SERVICE
                .get_or_init(|| build_app_service(":memory:", workspaces.clone()))
                .clone()
        } else {
            build_app_service(&config.database_url, workspaces.clone())
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
                        roles.clone(),
                        users.clone(),
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
                roles.clone(),
                users.clone(),
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
        let open_apps = if config.database_url == ":memory:" {
            GLOBAL_OPEN_APP_SERVICE
                .get_or_init(|| {
                    build_open_app_service(":memory:", users.clone(), &config.auth.jwt_secret)
                })
                .clone()
        } else {
            build_open_app_service(&config.database_url, users.clone(), &config.auth.jwt_secret)
        };
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
                .get_or_init(|| build_model_service(":memory:", workspaces.clone()))
                .clone()
        } else {
            build_model_service(&config.database_url, workspaces.clone())
        };
        let prompts = build_prompt_service(
            &config.database_url,
            workspaces.clone(),
            id_provider.clone(),
        );
        let llm_runtime = LlmRuntimeService::new(
            Arc::new(models.clone()),
            Arc::new(HttpLlmRuntimeClient::new(model_inference_client)),
        );
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
                    )
                })
                .clone_with_id_provider(id_provider.clone())
        } else {
            build_short_term_memory_service_with_id_provider(
                &config.database_url,
                &config,
                id_provider.clone(),
            )
        };
        let agent_tool_registry = Arc::new(ToolRegistry::default());
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
            open_apps.clone(),
            memory.clone(),
            short_term_memory.clone(),
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
        .with_resources(resources.clone());
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
            .with_active_runs(active_runs.clone()),
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
            active_runs,
            apps,
            automations,
            chats,
            short_term_memory,
            event_bus,
            graphs,
            ontologies,
            graph_explorer,
            models,
            prompts,
            open_apps,
            llm_runtime,
            mcp,
            memory,
            memory_work_domains,
            rag,
            skills,
            system_config,
            roles,
            tasks,
            users,
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
    let protected_routes = build_protected_routes(&state);
    let open_api_routes = build_open_api_routes(&state);

    Ok(Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/readiness", get(readiness))
        .merge(openapi_routes())
        .merge(auth_routes())
        .merge(mcp_transport_routes())
        .merge(protected_routes)
        .merge(frontend_routes())
        .merge(open_api_routes)
        .with_state(state))
}

include!("app/builders.rs");
