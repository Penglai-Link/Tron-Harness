use axum::{Router, routing::post};

use crate::{app::AppState, application::resource::ResourceService};

use super::handler::{
    activate_definition_version, activate_resource_version, commit_body_limit,
    create_definition_draft, create_resource, create_resource_draft, delete_definition,
    delete_resource, discard_definition_draft, discard_resource_draft, download_binary,
    get_definition, get_resource, page_definitions, page_resources, publish_definition_draft,
    publish_resource_draft, remove_binary, set_definition_enabled, update_definition_draft,
    update_resource, update_resource_draft, upload_binary, upload_body_limit,
};

pub fn routes(resources: &ResourceService) -> Router<AppState> {
    Router::new()
        .route("/api/v1/resource-definitions/page", post(page_definitions))
        .route("/api/v1/resource-definitions/get", post(get_definition))
        .route(
            "/api/v1/resource-definitions/create-draft",
            post(create_definition_draft),
        )
        .route(
            "/api/v1/resource-definitions/update-draft",
            post(update_definition_draft),
        )
        .route(
            "/api/v1/resource-definitions/discard-draft",
            post(discard_definition_draft),
        )
        .route(
            "/api/v1/resource-definitions/publish-draft",
            post(publish_definition_draft),
        )
        .route(
            "/api/v1/resource-definitions/activate-version",
            post(activate_definition_version),
        )
        .route(
            "/api/v1/resource-definitions/set-enabled",
            post(set_definition_enabled),
        )
        .route(
            "/api/v1/resource-definitions/delete",
            post(delete_definition),
        )
        .route("/api/v1/resources/page", post(page_resources))
        .route("/api/v1/resources/get", post(get_resource))
        .route(
            "/api/v1/resources/create",
            post(create_resource).layer(commit_body_limit(resources.max_commit_bytes())),
        )
        .route(
            "/api/v1/resources/update",
            post(update_resource).layer(commit_body_limit(resources.max_commit_bytes())),
        )
        .route("/api/v1/resources/delete", post(delete_resource))
        .route(
            "/api/v1/resources/create-draft",
            post(create_resource_draft),
        )
        .route(
            "/api/v1/resources/update-draft",
            post(update_resource_draft),
        )
        .route(
            "/api/v1/resources/discard-draft",
            post(discard_resource_draft),
        )
        .route(
            "/api/v1/resources/publish-draft",
            post(publish_resource_draft),
        )
        .route(
            "/api/v1/resources/activate-version",
            post(activate_resource_version),
        )
        .route(
            "/api/v1/resources/upload-binary",
            post(upload_binary).layer(upload_body_limit(resources.max_binary_bytes())),
        )
        .route("/api/v1/resources/remove-binary", post(remove_binary))
        .route("/api/v1/resources/download-binary", post(download_binary))
}
