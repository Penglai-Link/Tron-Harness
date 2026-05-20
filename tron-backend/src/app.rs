use std::sync::{Arc, OnceLock};

use axum::{Router, extract::FromRef, middleware, routing::get};

use crate::{
    adapters::{
        inbound::http::{
            admin::{
                roles::routes as role_routes, users::routes as user_routes,
                workspaces::routes as workspace_routes,
            },
            auth::{require_auth, routes as auth_routes},
            skills::routes::routes as skill_routes,
            tasks::routes as task_routes,
            variables::routes::routes as variable_routes,
        },
        outbound::{
            crypto::BcryptPasswordHasher,
            persistence::{
                SqliteRoleRepository, SqliteSkillRegistryRepository, SqliteTaskRepository,
                SqliteUserRepository, SqliteVariableRepository, SqliteWorkspaceRepository,
            },
        },
    },
    application::role::RoleService,
    application::skill::SkillRegistryService,
    application::task::TaskService,
    application::user::UserService,
    application::variable::VariableService,
    application::workspace::WorkspaceService,
    config::AppConfig,
    health::health,
    shared::{
        event_bus::{AguiEventSink, EventBusHandle, EventBusRuntimeBuilder},
        id::SnowflakedSnowflakeIdProvider,
    },
};

static GLOBAL_USER_SERVICE: OnceLock<UserService> = OnceLock::new();
static GLOBAL_ROLE_SERVICE: OnceLock<RoleService> = OnceLock::new();
static GLOBAL_WORKSPACE_SERVICE: OnceLock<WorkspaceService> = OnceLock::new();
static GLOBAL_TASK_SERVICE: OnceLock<TaskService> = OnceLock::new();
static GLOBAL_VARIABLE_SERVICE: OnceLock<VariableService> = OnceLock::new();
static GLOBAL_SKILL_SERVICE: OnceLock<SkillRegistryService> = OnceLock::new();

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub agui_events: Arc<AguiEventSink>,
    pub event_bus: EventBusHandle,
    pub roles: RoleService,
    pub tasks: TaskService,
    pub users: UserService,
    pub variables: VariableService,
    pub workspaces: WorkspaceService,
    pub skills: SkillRegistryService,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let agui_events = Arc::new(AguiEventSink::new(&config.event_bus));
        let event_bus = EventBusRuntimeBuilder::new(
            config.event_bus.clone(),
            Arc::new(SnowflakedSnowflakeIdProvider::default()),
        )
        .register_sink(agui_events.clone())
        .build();
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
        let tasks = if config.database_url == ":memory:" {
            GLOBAL_TASK_SERVICE
                .get_or_init(|| {
                    build_task_service(":memory:", workspaces.clone(), event_bus.clone())
                })
                .clone()
        } else {
            build_task_service(&config.database_url, workspaces.clone(), event_bus.clone())
        };
        let variables = if config.database_url == ":memory:" {
            GLOBAL_VARIABLE_SERVICE
                .get_or_init(|| build_variable_service(":memory:", workspaces.clone()))
                .clone()
        } else {
            build_variable_service(&config.database_url, workspaces.clone())
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

        Self {
            config,
            agui_events,
            event_bus,
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

impl FromRef<AppState> for TaskService {
    fn from_ref(state: &AppState) -> Self {
        state.tasks.clone()
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

pub fn build_router(config: AppConfig) -> Router {
    let state = AppState::new(config);
    let protected_routes = user_routes()
        .merge(role_routes())
        .merge(workspace_routes())
        .merge(task_routes())
        .merge(variable_routes())
        .merge(skill_routes())
        .route_layer(middleware::from_fn_with_state(
            state.users.clone(),
            require_auth,
        ));

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(auth_routes())
        .merge(protected_routes)
        .with_state(state)
}

fn build_user_service(config: &AppConfig, event_bus: EventBusHandle) -> UserService {
    let repository = SqliteUserRepository::connect(&config.database_url)
        .expect("user SQLite repository should initialize");
    UserService::with_event_bus(repository, BcryptPasswordHasher, config.auth.clone(), event_bus)
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

fn build_task_service(
    database_url: &str,
    workspaces: WorkspaceService,
    event_bus: EventBusHandle,
) -> TaskService {
    let repository = SqliteTaskRepository::connect(database_url)
        .expect("task SQLite repository should initialize");
    TaskService::with_event_bus(repository, workspaces, event_bus)
}

fn build_variable_service(database_url: &str, workspaces: WorkspaceService) -> VariableService {
    let repository = SqliteVariableRepository::connect(database_url)
        .expect("variable SQLite repository should initialize");
    VariableService::new(repository, workspaces)
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
