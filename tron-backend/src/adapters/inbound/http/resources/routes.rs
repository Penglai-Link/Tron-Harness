use axum::{
    Router,
    routing::post,
};

use crate::{app::AppState, application::resource::ResourceService};

use super::{
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
        .route("/api/v1/resources/download-binary", post(download_binary))

}
