use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    create_authorization_session, disconnect_provider, get_authorization_session,
    get_provider_account_balance, get_provider_connection, list_catalog_models,
    list_provider_connections, list_sync_runs, poll_authorization_session, sync_provider,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/model-providers", get(list_provider_connections))
        .route(
            "/api/v1/admin/model-providers/{provider_code}",
            get(get_provider_connection).delete(disconnect_provider),
        )
        .route(
            "/api/v1/admin/model-providers/{provider_code}/catalog",
            get(list_catalog_models),
        )
        .route(
            "/api/v1/admin/model-providers/{provider_code}/account-balance",
            get(get_provider_account_balance),
        )
        .route(
            "/api/v1/admin/model-providers/{provider_code}/sync-runs",
            get(list_sync_runs),
        )
        .route(
            "/api/v1/admin/model-providers/{provider_code}/authorization-sessions",
            post(create_authorization_session),
        )
        .route(
            "/api/v1/admin/model-providers/{provider_code}/authorization-sessions/{session_id}",
            get(get_authorization_session),
        )
        .route(
            "/api/v1/admin/model-providers/{provider_code}/authorization-sessions/{session_id}/poll",
            post(poll_authorization_session),
        )
        .route(
            "/api/v1/admin/model-providers/{provider_code}/sync",
            post(sync_provider),
        )
}

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/model-providers/{provider_code}/authorization-sessions",
            post(create_authorization_session),
        )
        .route(
            "/api/v1/model-providers/{provider_code}/authorization-sessions/{session_id}",
            get(get_authorization_session),
        )
        .route(
            "/api/v1/model-providers/{provider_code}/authorization-sessions/{session_id}/poll",
            post(poll_authorization_session),
        )
}

pub fn profile_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/model-providers/configured",
            get(super::configured::list).post(super::configured::create),
        )
        .route(
            "/api/v1/model-providers/configured/{provider_id}",
            get(super::configured::get)
                .put(super::configured::update)
                .delete(super::configured::delete),
        )
        .route("/api/v1/model-providers/presets", get(super::presets::list))
        .route(
            "/api/v1/model-providers/presets/{preset_id}/models",
            get(super::presets::models),
        )
        .route("/api/v1/model-providers", get(list_provider_connections))
        .route(
            "/api/v1/model-providers/{provider_code}",
            get(get_provider_connection).delete(disconnect_provider),
        )
        .route(
            "/api/v1/model-providers/{provider_code}/catalog",
            get(list_catalog_models),
        )
        .route(
            "/api/v1/model-providers/{provider_code}/account-balance",
            get(get_provider_account_balance),
        )
        .route(
            "/api/v1/model-providers/{provider_code}/sync-runs",
            get(list_sync_runs),
        )
        .route(
            "/api/v1/model-providers/{provider_code}/sync",
            post(sync_provider),
        )
}
