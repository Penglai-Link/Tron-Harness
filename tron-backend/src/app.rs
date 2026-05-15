use std::sync::OnceLock;

use axum::{Router, extract::FromRef, middleware, routing::get};

use crate::{
    adapters::{
        inbound::http::{
            admin::users::routes as user_routes,
            auth::{require_auth, routes as auth_routes},
        },
        outbound::{crypto::BcryptPasswordHasher, persistence::SqliteUserRepository},
    },
    application::user::UserService,
    config::AppConfig,
    health::health,
};

static GLOBAL_USER_SERVICE: OnceLock<UserService> = OnceLock::new();

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
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

        Self { config, users }
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

pub fn build_router(config: AppConfig) -> Router {
    let state = AppState::new(config);
    let protected_user_routes =
        user_routes().route_layer(middleware::from_fn_with_state(state.users.clone(), require_auth));

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(auth_routes())
        .merge(protected_user_routes)
        .with_state(state)
}

fn build_user_service(config: &AppConfig) -> UserService {
    let repository = SqliteUserRepository::connect(&config.database_url)
        .expect("user SQLite repository should initialize");
    UserService::new(repository, BcryptPasswordHasher, config.auth.clone())
}
