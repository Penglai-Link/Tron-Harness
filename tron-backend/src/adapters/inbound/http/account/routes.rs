use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    account_avatar_body_limit, change_password, download_avatar, get_profile, update_profile,
    upload_avatar,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/account/get-profile", get(get_profile))
        .route("/api/v1/account/update-profile", post(update_profile))
        .route("/api/v1/account/change-password", post(change_password))
        .route(
            "/api/v1/account/upload-avatar",
            post(upload_avatar).layer(account_avatar_body_limit()),
        )
        .route("/api/v1/account/download-avatar", get(download_avatar))
}
