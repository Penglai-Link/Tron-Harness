use std::sync::OnceLock;

use axum::{Router, extract::FromRef, middleware, routing::get};

use crate::{
    adapters::{
        inbound::http::{
            admin::{roles::routes as role_routes, users::routes as user_routes, workspaces::routes as workspace_routes},
            auth::{require_auth, routes as auth_routes},
        },
        outbound::{
            crypto::BcryptPasswordHasher,
            persistence::{SqliteRoleRepository, SqliteUserRepository, SqliteWorkspaceRepository},
        },
    },
    application::role::RoleService,
    application::user::UserService,
    application::workspace::WorkspaceService,
    config::AppConfig,
    health::health,
};

static GLOBAL_USER_SERVICE: OnceLock<UserService> = OnceLock::new();
static GLOBAL_ROLE_SERVICE: OnceLock<RoleService> = OnceLock::new();
static GLOBAL_WORKSPACE_SERVICE: OnceLock<WorkspaceService> = OnceLock::new();

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub roles: RoleService,
    pub users: UserService,
    pub workspaces: WorkspaceService,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let users = if config.database_url == ":memory:" {
            GLOBAL_USER_SERVICE
                .get_or_init(|| build_user_service(&config))
                .clone()
        } else {
            build_user_service(&config)
        };
        let roles = if config.database_url == ":memory:" {
            GLOBAL_ROLE_SERVICE
                .get_or_init(|| build_role_service(":memory:"))
                .clone()
        } else {
            build_role_service(&config.database_url)
        };
        let workspaces = if config.database_url == ":memory:" {
            GLOBAL_WORKSPACE_SERVICE
                .get_or_init(|| build_workspace_service(":memory:", users.clone(), roles.clone()))
                .clone()
        } else {
            build_workspace_service(&config.database_url, users.clone(), roles.clone())
        };

        Self {
            config,
            roles,
            users,
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

pub fn build_router(config: AppConfig) -> Router {
    let state = AppState::new(config);
    let protected_admin_routes = user_routes()
        .merge(role_routes())
        .merge(workspace_routes())
        .route_layer(middleware::from_fn_with_state(state.users.clone(), require_auth));

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(auth_routes())
        .merge(protected_admin_routes)
        .with_state(state)
}

fn build_user_service(config: &AppConfig) -> UserService {
    let repository = SqliteUserRepository::connect(&config.database_url)
        .expect("user SQLite repository should initialize");
    UserService::new(repository, BcryptPasswordHasher, config.auth.clone())
}

fn build_role_service(database_url: &str) -> RoleService {
    let repository = SqliteRoleRepository::connect(database_url)
        .expect("role SQLite repository should initialize");
    RoleService::new(repository)
}

fn build_workspace_service(
    database_url: &str,
    users: UserService,
    roles: RoleService,
) -> WorkspaceService {
    let repository = SqliteWorkspaceRepository::connect(database_url)
        .expect("workspace SQLite repository should initialize");
    WorkspaceService::new(repository, users, roles)
}
