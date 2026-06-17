use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    archive_memory_record, create_memory_record, create_work_domain, disable_memory_record,
    disable_work_domain, enable_work_domain, get_memory_record, list_memory_records,
    list_work_domains, patch_memory_record, patch_work_domain,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/memory/domains",
            get(list_work_domains).post(create_work_domain),
        )
        .route(
            "/api/v1/memory/domains/{domain_code}",
            patch(patch_work_domain),
        )
        .route(
            "/api/v1/memory/domains/{domain_code}/enable",
            post(enable_work_domain),
        )
        .route(
            "/api/v1/memory/domains/{domain_code}/disable",
            post(disable_work_domain),
        )
        .route(
            "/api/v1/memory/workspaces/{workspace_code}/records",
            get(list_memory_records).post(create_memory_record),
        )
        .route(
            "/api/v1/memory/workspaces/{workspace_code}/records/{memory_code}",
            get(get_memory_record).patch(patch_memory_record),
        )
        .route(
            "/api/v1/memory/workspaces/{workspace_code}/records/{memory_code}/archive",
            post(archive_memory_record),
        )
        .route(
            "/api/v1/memory/workspaces/{workspace_code}/records/{memory_code}/disable",
            post(disable_memory_record),
        )
}
