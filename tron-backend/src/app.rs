use axum::{Router, extract::FromRef, routing::get};

use crate::{
    config::AppConfig, domain::user::UserService, health::health,
    http::admin::users::routes as user_routes,
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub users: UserService,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            users: UserService::global(),
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

pub fn build_router(config: AppConfig) -> Router {
    let state = AppState::new(config);

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(user_routes())
        .with_state(state)
}
