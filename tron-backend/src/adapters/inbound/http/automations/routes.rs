use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    archive_automation, create_automation, execute_automation, get_automation,
    list_automations, list_execution_records, options, patch_automation, patch_status,
    restore_automation,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/automations/options", get(options))
        .route("/api/v1/automations", get(list_automations).post(create_automation))
        .route(
            "/api/v1/automations/{automation_code}",
            get(get_automation).patch(patch_automation),
        )
        .route(
            "/api/v1/automations/{automation_code}/status",
            post(patch_status).patch(patch_status),
        )
        .route(
            "/api/v1/automations/{automation_code}/archive",
            post(archive_automation),
        )
        .route(
            "/api/v1/automations/{automation_code}/restore",
            post(restore_automation),
        )
        .route(
            "/api/v1/automations/{automation_code}/execute",
            post(execute_automation),
        )
        .route(
            "/api/v1/automations/{automation_code}/executions",
            get(list_execution_records),
        )
}
