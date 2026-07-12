use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{change_password, get_profile, update_profile};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/account/get-profile", get(get_profile))
        .route("/api/v1/account/update-profile", post(update_profile))
        .route("/api/v1/account/change-password", post(change_password))
}
