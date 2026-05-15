use std::sync::OnceLock;

use axum::{Router, extract::FromRef, middleware, routing::get};

use crate::{
    adapters::{
        inbound::http::{
            admin::{roles::routes as role_routes, users::routes as user_routes},
            auth::{require_auth, routes as auth_routes},
        },
        outbound::{
            crypto::BcryptPasswordHasher,
            persistence::{SqliteRoleRepository, SqliteUserRepository},
        },
    },
    application::role::RoleService,
    application::user::UserService,
    config::AppConfig,
    health::health,
};

static GLOBAL_USER_SERVICE: OnceLock<UserService> = OnceLock::new();
static GLOBAL_ROLE_SERVICE: OnceLock<RoleService> = OnceLock::new();

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub roles: RoleService,
    pub users: UserService,
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

        Self {
            config,
            roles,
            users,
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

pub fn build_router(config: AppConfig) -> Router {
    let state = AppState::new(config);
    let protected_admin_routes = user_routes()
        .merge(role_routes())
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
