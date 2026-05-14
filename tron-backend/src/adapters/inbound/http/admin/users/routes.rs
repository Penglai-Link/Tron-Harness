use axum::{
    Router,
    routing::{get, post, put},
};

use crate::app::AppState;

use super::handler::{
    active_users, archive_user, create_user, get_user, list_users, replace_roles, replace_spaces,
    reset_password, restore_user, role_assignments, space_assignments, total_users,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/users", get(list_users).post(create_user))
        .route(
            "/api/v1/admin/users/statistics/total-users",
            get(total_users),
        )
        .route(
            "/api/v1/admin/users/statistics/active-users",
            get(active_users),
        )
        .route(
            "/api/v1/admin/users/statistics/role-assignments",
            get(role_assignments),
        )
        .route(
            "/api/v1/admin/users/statistics/space-assignments",
            get(space_assignments),
        )
        .route(
            "/api/v1/admin/users/{user_code}",
            get(get_user).patch(super::handler::patch_user),
        )
        .route(
            "/api/v1/admin/users/{user_code}/archive",
            post(archive_user),
        )
        .route(
            "/api/v1/admin/users/{user_code}/restore",
            post(restore_user),
        )
        .route("/api/v1/admin/users/{user_code}/roles", put(replace_roles))
        .route(
            "/api/v1/admin/users/{user_code}/spaces",
            put(replace_spaces),
        )
        .route(
            "/api/v1/admin/users/{user_code}/password/reset",
            post(reset_password),
        )
}
