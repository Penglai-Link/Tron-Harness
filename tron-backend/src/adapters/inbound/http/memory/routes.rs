use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, patch, post},
};

use crate::app::AppState;

use super::handler::{
    create_memory_record, create_work_domain, delete_memory_record, disable_work_domain,
    enable_work_domain, get_memory_record, list_work_domains, patch_memory_record,
    patch_work_domain, query_memory_records,
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
        .route("/api/v1/memory/records/query", post(query_memory_records))
        .route("/api/v1/memory/records", post(create_memory_record))
        .route(
            "/api/v1/memory/records/{memory_code}",
            get(get_memory_record)
                .patch(patch_memory_record)
                .delete(delete_memory_record),
        )
        .layer(DefaultBodyLimit::max(3 * 1024 * 1024))
}
