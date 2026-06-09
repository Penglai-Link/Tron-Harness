use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    current_session, login, login_ldap, logout, oauth2_authorize, oauth2_callback, providers,
    refresh,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/providers", get(providers))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/login/ldap", post(login_ldap))
        .route("/api/v1/auth/session", get(current_session))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/logout", post(logout))
        .route(
            "/api/v1/auth/oauth2/authorize/{provider_code}",
            get(oauth2_authorize),
        )
        .route(
            "/api/v1/auth/oauth2/callback/{provider_code}",
            get(oauth2_callback),
        )
}
