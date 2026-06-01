use std::sync::{Arc, Mutex, OnceLock};

mod routing;

use axum::{Router, extract::FromRef, routing::get};

use crate::{
    adapters::{
        inbound::http::{
            auth::routes as auth_routes, mcp::routes::transport_routes as mcp_transport_routes,
        },
        outbound::{
            crypto::BcryptPasswordHasher,
            model::{HttpLlmRuntimeClient, HttpModelClient, HttpModelProviderClient},
            persistence::{
                GrafeoAdapterConfig, GrafeoGraphStore, GrafeoOntologyStore, GrafeoStoreCore,
                SqliteAgentEventReplayStore, SqliteAgentManagementRepository,
                SqliteAgentRunRepository, SqliteAppRepository, SqliteAutomationRepository,
                SqliteChatRepository, SqliteMcpRepository, SqliteModelRepository,
                SqliteOpenAppRepository, SqliteRoleRepository, SqliteSkillRegistryRepository,
                SqliteTaskRepository, SqliteUserRepository, SqliteVariableRepository,
                SqliteWorkspaceRepository, bootstrap_platform_admin,
                sqlite::SqliteRagRepository,
            },
        },
    },
    application::agent::{
        AgentAguiReplayStore, AgentManagementService, AgentRunHistoryService, ModelClient,
        register_default_agent_tools,
    },
    application::app::AppService,
    application::automation::AutomationService,
    application::chat::ChatService,
    application::graph::{GraphService, OntologyService, ScopeResolver},
    application::llm::LlmRuntimeService,
    application::mcp::McpService,
    application::model::ModelService,
    application::open_app::OpenAppService,
    application::rag::{DefaultRagRuntime, DefaultRagStrategyRegistry, RagService},
    application::role::RoleService,
    application::skill::SkillRegistryService,
    application::task::TaskService,
    application::user::UserService,
    application::variable::VariableService,
    application::workspace::WorkspaceService,
    config::AppConfig,
    domain::agent::ToolRegistry,
    health::health,
    shared::{
        crypto::Aes256SecretBox,
        event_bus::{AguiEventSink, EventBusHandle, EventBusRuntimeBuilder},
        http_client::HttpClientPool,
        id::SnowflakedSnowflakeIdProvider,
    },
};

use self::routing::{build_open_api_routes, build_protected_routes};

static GLOBAL_USER_SERVICE: OnceLock<UserService> = OnceLock::new();
static GLOBAL_ROLE_SERVICE: OnceLock<RoleService> = OnceLock::new();
static GLOBAL_WORKSPACE_SERVICE: OnceLock<WorkspaceService> = OnceLock::new();
static GLOBAL_APP_SERVICE: OnceLock<AppService> = OnceLock::new();
static GLOBAL_TASK_SERVICE: OnceLock<TaskService> = OnceLock::new();
static GLOBAL_VARIABLE_SERVICE: OnceLock<VariableService> = OnceLock::new();
static GLOBAL_SKILL_SERVICE: OnceLock<SkillRegistryService> = OnceLock::new();
static GLOBAL_MODEL_SERVICE: OnceLock<ModelService> = OnceLock::new();
static GLOBAL_CHAT_SERVICE: OnceLock<ChatService> = OnceLock::new();
static GLOBAL_OPEN_APP_SERVICE: OnceLock<OpenAppService> = OnceLock::new();
static GLOBAL_GRAPH_SERVICES: OnceLock<(GraphService, OntologyService)> = OnceLock::new();

pub type AppRagService =
    RagService<Arc<Mutex<SqliteRagRepository>>, DefaultRagStrategyRegistry, DefaultRagRuntime>;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub agui_events: Arc<AguiEventSink>,
    pub agui_replay_store: Arc<dyn AgentAguiReplayStore>,
    pub agent_model_client: Arc<dyn ModelClient>,
    pub runtime_http_client: reqwest::Client,
    pub agent_management: AgentManagementService,
    pub agent_runs: AgentRunHistoryService,
    pub agent_tool_registry: Arc<ToolRegistry>,
    pub apps: AppService,
    pub automations: AutomationService,
    pub chats: ChatService,
    pub event_bus: EventBusHandle,
    pub graphs: GraphService,
    pub ontologies: OntologyService,
    pub roles: RoleService,
    pub tasks: TaskService,
    pub users: UserService,
    pub variables: VariableService,
    pub workspaces: WorkspaceService,
    pub skills: SkillRegistryService,
    pub models: ModelService,
    pub open_apps: OpenAppService,
    pub llm_runtime: LlmRuntimeService,
    pub mcp: McpService,
    pub rag: AppRagService,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let agui_events = Arc::new(AguiEventSink::new(&config.event_bus));
        let agui_replay_store = Arc::new(
            SqliteAgentEventReplayStore::connect(&config.database_url)
                .expect("agent event replay store should initialize"),
        );
        let event_bus = EventBusRuntimeBuilder::new(
            config.event_bus.clone(),
            Arc::new(SnowflakedSnowflakeIdProvider::default()),
        )
        .register_sink(agui_events.clone())
        .register_sink(agui_replay_store.clone())
        .build();
        let http_client_pool = HttpClientPool::from_profiles(&config.http_clients)
            .expect("http client pool should initialize");
        let model_inference_client = http_client_pool.model_inference_client();
        let runtime_http_client = http_client_pool.runtime_control_client();
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
                .get_or_init(|| build_variable_service(":memory:", workspaces.clone()))
                .clone()
        } else {
            build_variable_service(&config.database_url, workspaces.clone())
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
        automations
            .bootstrap_scheduler()
            .expect("automation scheduler should initialize");
        let models = if config.database_url == ":memory:" {
            GLOBAL_MODEL_SERVICE
                .get_or_init(|| build_model_service(":memory:", workspaces.clone()))
                .clone()
        } else {
            build_model_service(&config.database_url, workspaces.clone())
        };
        let agent_tool_registry = Arc::new(ToolRegistry::default());
        register_default_agent_tools(
            agent_tool_registry.as_ref(),
            workspaces.clone(),
            apps.clone(),
            tasks.clone(),
            models.clone(),
            skills.clone(),
            variables.clone(),
        )
        .expect("default agent tools should register");
        let llm_runtime = LlmRuntimeService::new(
            Arc::new(models.clone()),
            Arc::new(HttpLlmRuntimeClient::new(model_inference_client)),
        );
        let rag = build_rag_service(
            &config.database_url,
            llm_runtime.clone(),
            &config.project_doc_root,
            event_bus.clone(),
        );
        let (graphs, ontologies) = if config.graph_storage_path == ":memory:" {
            GLOBAL_GRAPH_SERVICES
                .get_or_init(|| {
                    build_graph_services(
                        &config.graph_storage_path,
                        workspaces.clone(),
                        llm_runtime.clone(),
                    )
                })
                .clone()
        } else {
            build_graph_services(
                &config.graph_storage_path,
                workspaces.clone(),
                llm_runtime.clone(),
            )
        };
        let agent_runs = build_agent_run_history_service(&config.database_url);
        let chats = if config.database_url == ":memory:" {
            GLOBAL_CHAT_SERVICE
                .get_or_init(|| build_chat_service(":memory:"))
                .clone()
        } else {
            build_chat_service(&config.database_url)
        };

        Self {
            config,
            agui_events,
            agui_replay_store,
            agent_model_client,
            runtime_http_client,
            agent_management,
            agent_runs,
            agent_tool_registry,
            apps,
            automations,
            chats,
            event_bus,
            graphs,
            ontologies,
            models,
            open_apps,
            llm_runtime,
            mcp,
            rag,
            skills,
            roles,
            tasks,
            users,
            variables,
            workspaces,
        }
    }
}

impl FromRef<AppState> for AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for UserService {
    fn from_ref(state: &AppState) -> Self {
        state.users.clone()
    }
}

impl FromRef<AppState> for AppService {
    fn from_ref(state: &AppState) -> Self {
        state.apps.clone()
    }
}

impl FromRef<AppState> for EventBusHandle {
    fn from_ref(state: &AppState) -> Self {
        state.event_bus.clone()
    }
}

impl FromRef<AppState> for RoleService {
    fn from_ref(state: &AppState) -> Self {
        state.roles.clone()
    }
}

impl FromRef<AppState> for WorkspaceService {
    fn from_ref(state: &AppState) -> Self {
        state.workspaces.clone()
    }
}

impl FromRef<AppState> for GraphService {
    fn from_ref(state: &AppState) -> Self {
        state.graphs.clone()
    }
}

impl FromRef<AppState> for OntologyService {
    fn from_ref(state: &AppState) -> Self {
        state.ontologies.clone()
    }
}

impl FromRef<AppState> for TaskService {
    fn from_ref(state: &AppState) -> Self {
        state.tasks.clone()
    }
}

impl FromRef<AppState> for AutomationService {
    fn from_ref(state: &AppState) -> Self {
        state.automations.clone()
    }
}

impl FromRef<AppState> for ChatService {
    fn from_ref(state: &AppState) -> Self {
        state.chats.clone()
    }
}

impl FromRef<AppState> for VariableService {
    fn from_ref(state: &AppState) -> Self {
        state.variables.clone()
    }
}

impl FromRef<AppState> for SkillRegistryService {
    fn from_ref(state: &AppState) -> Self {
        state.skills.clone()
    }
}

impl FromRef<AppState> for ModelService {
    fn from_ref(state: &AppState) -> Self {
        state.models.clone()
    }
}

impl FromRef<AppState> for OpenAppService {
    fn from_ref(state: &AppState) -> Self {
        state.open_apps.clone()
    }
}

impl FromRef<AppState> for LlmRuntimeService {
    fn from_ref(state: &AppState) -> Self {
        state.llm_runtime.clone()
    }
}

impl FromRef<AppState> for AppRagService {
    fn from_ref(state: &AppState) -> Self {
        state.rag.clone()
    }
}

impl FromRef<AppState> for AgentRunHistoryService {
    fn from_ref(state: &AppState) -> Self {
        state.agent_runs.clone()
    }
}

impl FromRef<AppState> for AgentManagementService {
    fn from_ref(state: &AppState) -> Self {
        state.agent_management.clone()
    }
}

impl FromRef<AppState> for McpService {
    fn from_ref(state: &AppState) -> Self {
        state.mcp.clone()
    }
}

pub fn build_router(config: AppConfig) -> Router {
    let state = AppState::new(config);
    let protected_routes = build_protected_routes(&state);
    let open_api_routes = build_open_api_routes(&state);

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(auth_routes())
        .merge(mcp_transport_routes())
        .merge(protected_routes)
        .merge(open_api_routes)
        .with_state(state)
}

fn build_user_service(config: &AppConfig, event_bus: EventBusHandle) -> UserService {
    let repository = SqliteUserRepository::connect(&config.database_url)
        .expect("user SQLite repository should initialize");
    UserService::with_event_bus(
        repository,
        BcryptPasswordHasher,
        config.auth.clone(),
        event_bus,
    )
}

fn build_role_service(database_url: &str, event_bus: EventBusHandle) -> RoleService {
    let repository = SqliteRoleRepository::connect(database_url)
        .expect("role SQLite repository should initialize");
    RoleService::with_event_bus(repository, event_bus)
}

fn build_workspace_service(
    database_url: &str,
    users: UserService,
    roles: RoleService,
    event_bus: EventBusHandle,
) -> WorkspaceService {
    let repository = SqliteWorkspaceRepository::connect(database_url)
        .expect("workspace SQLite repository should initialize");
    WorkspaceService::with_event_bus(repository, users, roles, event_bus)
}

fn build_app_service(database_url: &str, workspaces: WorkspaceService) -> AppService {
    let repository = SqliteAppRepository::connect(database_url)
        .expect("app SQLite repository should initialize");
    AppService::new(repository, workspaces)
}

fn build_task_service(
    database_url: &str,
    workspaces: WorkspaceService,
    apps: AppService,
    event_bus: EventBusHandle,
) -> TaskService {
    let repository = SqliteTaskRepository::connect(database_url)
        .expect("task SQLite repository should initialize");
    TaskService::with_event_bus(repository, workspaces, apps, event_bus)
}

fn build_automation_service(
    database_url: &str,
    workspaces: WorkspaceService,
    event_bus: EventBusHandle,
) -> AutomationService {
    let repository = SqliteAutomationRepository::connect(database_url)
        .expect("automation SQLite repository should initialize");
    AutomationService::with_event_bus(repository, workspaces, event_bus)
}

fn build_chat_service(database_url: &str) -> ChatService {
    let repository = SqliteChatRepository::connect(database_url)
        .expect("chat SQLite repository should initialize");
    ChatService::new(repository)
}

fn build_variable_service(database_url: &str, workspaces: WorkspaceService) -> VariableService {
    let repository = SqliteVariableRepository::connect(database_url)
        .expect("variable SQLite repository should initialize");
    VariableService::new(repository, workspaces)
}

fn build_mcp_service(
    database_url: &str,
    workspaces: WorkspaceService,
    variables: VariableService,
    runtime_http_client: reqwest::Client,
) -> McpService {
    let repository = SqliteMcpRepository::connect(database_url)
        .expect("MCP SQLite repository should initialize");
    McpService::new(repository, workspaces, variables, runtime_http_client)
}

fn build_skill_registry_service(
    database_url: &str,
    workspaces: WorkspaceService,
    registry_root: &str,
) -> SkillRegistryService {
    let repository = SqliteSkillRegistryRepository::connect(database_url)
        .expect("skill registry SQLite repository should initialize");
    SkillRegistryService::new(repository, workspaces, registry_root)
}

fn build_model_service(database_url: &str, workspaces: WorkspaceService) -> ModelService {
    let repository = SqliteModelRepository::connect(database_url)
        .expect("model SQLite repository should initialize");
    ModelService::new(repository, workspaces)
}

fn build_open_app_service(
    database_url: &str,
    users: UserService,
    jwt_secret: &str,
) -> OpenAppService {
    let repository = SqliteOpenAppRepository::connect(database_url)
        .expect("open-app SQLite repository should initialize");
    OpenAppService::new(repository, users, Aes256SecretBox::new(jwt_secret))
}

fn build_rag_service(
    database_url: &str,
    llm_runtime: LlmRuntimeService,
    project_doc_root: &str,
    event_bus: EventBusHandle,
) -> AppRagService {
    let repository = SqliteRagRepository::connect(database_url)
        .expect("RAG SQLite repository should initialize");
    RagService::new(
        Arc::new(Mutex::new(repository)),
        DefaultRagStrategyRegistry::default(),
        DefaultRagRuntime::new(llm_runtime, project_doc_root),
    )
    .with_event_bus(event_bus)
}

fn build_graph_services(
    graph_storage_path: &str,
    workspaces: WorkspaceService,
    llm_runtime: LlmRuntimeService,
) -> (GraphService, OntologyService) {
    let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
        storage_path: graph_storage_path.to_owned(),
    })
    .expect("graph Grafeo adapter should initialize");
    let graphs = GraphService::with_embedding_runtime(
        GrafeoGraphStore::new(core.clone()),
        ScopeResolver::new(workspaces.clone()),
        Arc::new(llm_runtime),
    );
    let ontologies = OntologyService::new(
        GrafeoOntologyStore::new(core),
        ScopeResolver::new(workspaces),
    );

    (graphs, ontologies)
}

fn build_agent_run_history_service(database_url: &str) -> AgentRunHistoryService {
    let repository = SqliteAgentRunRepository::connect(database_url)
        .expect("agent run SQLite repository should initialize");
    AgentRunHistoryService::new(repository)
}

fn build_agent_management_service(
    database_url: &str,
    workspaces: WorkspaceService,
    system_agents: &crate::config::SystemAgentConfig,
) -> AgentManagementService {
    let repository = SqliteAgentManagementRepository::connect(database_url)
        .expect("agent management SQLite repository should initialize");
    AgentManagementService::new(repository, workspaces, system_agents)
}
