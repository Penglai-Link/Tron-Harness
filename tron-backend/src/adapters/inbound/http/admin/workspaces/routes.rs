use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    active_workspaces, archive_workspace, create_workspace, direct_user_grants, get_workspace,
    list_effective_users, list_role_access, list_user_access, list_workspace_options,
    list_workspaces, move_workspace, patch_workspace, replace_role_access,
    replace_user_access, restore_workspace, role_grants, total_workspaces,
    workspace_switcher,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/workspaces", get(list_workspaces).post(create_workspace))
        .route("/api/v1/admin/workspaces/options", get(list_workspace_options))
        .route("/api/v1/admin/workspaces/switcher", get(workspace_switcher))
        .route(
            "/api/v1/admin/workspaces/statistics/total-workspaces",
            get(total_workspaces),
        )
        .route(
            "/api/v1/admin/workspaces/statistics/active-workspaces",
            get(active_workspaces),
        )
        .route(
            "/api/v1/admin/workspaces/statistics/direct-user-grants",
            get(direct_user_grants),
        )
        .route(
            "/api/v1/admin/workspaces/statistics/role-grants",
            get(role_grants),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}",
            get(get_workspace).patch(patch_workspace),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/move",
            post(move_workspace),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/archive",
            post(archive_workspace),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/restore",
            post(restore_workspace),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/access/users",
            get(list_user_access).put(replace_user_access),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/access/roles",
            get(list_role_access).put(replace_role_access),
        )
        .route(
            "/api/v1/admin/workspaces/{workspace_code}/access/effective-users",
            get(list_effective_users),
        )
}
