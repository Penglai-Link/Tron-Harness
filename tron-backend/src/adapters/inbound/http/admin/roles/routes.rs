use axum::{Router, routing::get};

use crate::app::AppState;

use super::handler::{list_permission_domains, list_roles, replace_roles};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/permission-domains",
            get(list_permission_domains),
        )
        .route("/api/v1/admin/roles", get(list_roles).put(replace_roles))
}
