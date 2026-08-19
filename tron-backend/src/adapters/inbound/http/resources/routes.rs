use axum::{Router, routing::post};

use crate::{app::AppState, application::resource::ResourceService};

use super::{
    connection_handler::{
        create_data_source, create_server, create_server_account, delete_connection,
        page_connections, update_data_source,
    },
    handler::{
        commit_body_limit, create_resource, delete_resource, download_binary, get_resource,
        page_resources, update_resource, upload_binary, upload_body_limit,
    },
};

pub fn routes(resources: &ResourceService) -> Router<AppState> {
    Router::new()
        .route("/api/v1/resources/page", post(page_resources))
        .route("/api/v1/resources/get", post(get_resource))
        .route(
            "/api/v1/resources/create",
            post(create_resource).layer(commit_body_limit(resources.max_commit_bytes())),
        )
        .route("/api/v1/resources/update", post(update_resource))
        .route("/api/v1/resources/delete", post(delete_resource))
        .route(
            "/api/v1/resources/upload-binary",
            post(upload_binary).layer(upload_body_limit(resources.max_binary_bytes())),
        )
        .route(
            "/api/v1/resources/download-binary",
            post(download_binary),
        )
        .route(
            "/api/v1/resource-connections/page",
            post(page_connections),
        )
        .route(
            "/api/v1/resource-connections/data-sources",
            post(create_data_source),
        )
        .route(
            "/api/v1/resource-connections/data-sources/update",
            post(update_data_source),
        )
        .route("/api/v1/resource-connections/servers", post(create_server))
        .route(
            "/api/v1/resource-connections/server-accounts",
            post(create_server_account),
        )
        .route(
            "/api/v1/resource-connections/delete",
            post(delete_connection),
        )
}
