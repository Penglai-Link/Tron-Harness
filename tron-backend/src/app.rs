use std::sync::OnceLock;

use axum::{Router, extract::FromRef, routing::get};

use crate::{
    adapters::{
        inbound::http::admin::users::routes as user_routes,
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
                .get_or_init(|| build_user_service(":memory:"))
                .clone()
        } else {
            build_user_service(&config.database_url)
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

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(user_routes())
        .with_state(state)
}

fn build_user_service(database_url: &str) -> UserService {
    let repository = SqliteUserRepository::connect(database_url)
        .expect("user SQLite repository should initialize");
    UserService::new(repository, BcryptPasswordHasher)
}
