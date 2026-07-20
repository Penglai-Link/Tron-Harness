mod common;

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app_with_config, authenticated_bearer_token, request_with_headers, response_json};
use tron_backend::config::{AppConfig, ResourceConfig};

const WORKSPACE: &str = "TRON_CORE";

#[tokio::test]
async fn resource_definition_and_resource_lifecycle_use_the_management_contract() {
    let (app, storage_root) = resource_app("lifecycle");
    let token = authenticated_bearer_token(&app).await;

    let platform_page = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/page",
        json!({
            "scopeKind": "PLATFORM",
            "workspaceCode": null,
            "search": null,
            "definitionCode": null,
            "definitionStatus": null,
            "lifecycleStatus": "ACTIVE",
            "validationStatus": "VALID",
            "sourceKind": null,
            "current": 1,
            "size": 50
        }),
    )
    .await;
    assert_eq!(platform_page["code"], "0");
    let file_definition = platform_page["data"]["records"]
        .as_array()
        .expect("platform definition page should contain records")
        .iter()
        .find(|record| record["definitionCode"] == "FILE")
        .expect("FILE should be seeded");
    assert_eq!(file_definition["scopeKind"], "PLATFORM");
    assert!(file_definition["createdAt"].is_i64());
    assert!(file_definition["attributesSchema"].is_null());

    let draft_definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        definition_create_body("HTTP_DOCUMENT"),
    )
    .await;
    let definition_version = draft_definition["data"]["definitionVersion"]
        .as_str()
        .expect("definition version should be present")
        .to_string();
    let definition_revision = draft_definition["data"]["draftRevision"]
        .as_str()
        .expect("definition revision should be a string")
        .to_string();

    let published_definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_DOCUMENT",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision
        }),
    )
    .await;
    assert_eq!(published_definition["data"]["lifecycleStatus"], "ACTIVE");
    assert_eq!(
        published_definition["data"]["activationResult"],
        "DRAFT_PUBLISHED"
    );

    let draft_resource = api_json(
        &app,
        &token,
        "/api/v1/resources/create-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-document",
            "sourceResourceVersion": null,
            "definitionCode": "HTTP_DOCUMENT",
            "name": "HTTP document",
            "description": "created through the management API",
            "attributes": { "title": "First" },
            "allowInheritance": true
        }),
    )
    .await;
    let resource_version = draft_resource["data"]["resourceVersion"]
        .as_str()
        .expect("resource version should be present")
        .to_string();
    let resource_revision = draft_resource["data"]["draftRevision"]
        .as_str()
        .expect("resource revision should be a string")
        .to_string();

    let updated_resource = api_json(
        &app,
        &token,
        "/api/v1/resources/update-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-document",
            "resourceVersion": resource_version,
            "expectedDraftRevision": resource_revision,
            "name": "HTTP document v2",
            "description": "updated through the management API",
            "attributes": { "title": "Second" },
            "allowInheritance": false
        }),
    )
    .await;
    let updated_revision = updated_resource["data"]["draftRevision"]
        .as_str()
        .expect("updated revision should be a string")
        .to_string();

    let stale_update = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resources/update-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-document",
            "resourceVersion": resource_version,
            "expectedDraftRevision": resource_revision,
            "name": "stale update",
            "description": null,
            "attributes": null,
            "allowInheritance": null
        }),
    )
    .await;
    assert_eq!(stale_update.status(), StatusCode::CONFLICT);
    let stale_update_payload = response_json(stale_update).await;
    assert!(stale_update_payload["code"].is_string());
    assert!(stale_update_payload["message"].is_string());
    assert!(stale_update_payload["data"].is_object());

    let published_resource = api_json(
        &app,
        &token,
        "/api/v1/resources/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-document",
            "resourceVersion": resource_version,
            "expectedDraftRevision": updated_revision
        }),
    )
    .await;
    assert_eq!(published_resource["data"]["lifecycleStatus"], "ACTIVE");

    let detail = api_json(
        &app,
        &token,
        "/api/v1/resources/get",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-document",
            "resourceVersion": resource_version
        }),
    )
    .await;
    assert_eq!(detail["data"]["name"], "HTTP document v2");
    assert_eq!(detail["data"]["allowInheritance"], false);
    assert!(detail["data"]["createdAt"].is_i64());
    assert!(detail["data"]["draftRevision"].is_null());
    assert!(detail["data"]["attributesSchema"].is_object());
    assert_eq!(detail["data"]["binaryStorageProfile"], "@default");
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn resource_binary_upload_replace_remove_and_download_never_expose_locator() {
    let (app, storage_root) = resource_app("binary");
    let token = authenticated_bearer_token(&app).await;
    let draft_definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        binary_definition_create_body("HTTP_FILE"),
    )
    .await;
    let definition_version = draft_definition["data"]["definitionVersion"]
        .as_str()
        .expect("definition version should be present")
        .to_string();
    let definition_revision = draft_definition["data"]["draftRevision"]
        .as_str()
        .expect("definition revision should be present")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_FILE",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision
        }),
    )
    .await;

    let draft_resource = api_json(
        &app,
        &token,
        "/api/v1/resources/create-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-file",
            "sourceResourceVersion": null,
            "definitionCode": "HTTP_FILE",
            "name": "HTTP file",
            "description": "binary transport fixture",
            "attributes": { "title": "binary fixture" },
            "allowInheritance": true
        }),
    )
    .await;
    let resource_version = draft_resource["data"]["resourceVersion"]
        .as_str()
        .expect("resource version should be present")
        .to_string();
    let resource_revision = draft_resource["data"]["draftRevision"]
        .as_str()
        .expect("resource revision should be present")
        .to_string();

    let upload = multipart_upload(
        &app,
        &token,
        &resource_version,
        &resource_revision,
        "payload.txt",
        b"first payload",
    )
    .await;
    assert_eq!(upload.status(), StatusCode::OK);
    let upload_json = response_json(upload).await;
    assert_eq!(upload_json["code"], "0");
    assert_eq!(upload_json["data"]["byteSize"], 13);
    assert!(!upload_json.to_string().contains("local://"));
    let updated_revision = upload_json["data"]["draftRevision"]
        .as_str()
        .expect("upload should update the draft revision")
        .to_string();

    let replacement = multipart_upload(
        &app,
        &token,
        &resource_version,
        &updated_revision,
        "payload.txt",
        b"replacement payload",
    )
    .await;
    assert_eq!(replacement.status(), StatusCode::OK);
    let replacement_json = response_json(replacement).await;
    assert_eq!(replacement_json["data"]["byteSize"], 19);
    let replacement_revision = replacement_json["data"]["draftRevision"]
        .as_str()
        .expect("replacement should update the draft revision")
        .to_string();

    let download = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resources/download-binary",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-file",
            "resourceVersion": resource_version,
            "binaryName": "payload.txt"
        }),
    )
    .await;
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(
        download
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/octet-stream"),
    );
    assert!(
        download
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("filename*="))
    );
    let payload = to_bytes(download.into_body(), usize::MAX)
        .await
        .expect("download body should be readable");
    assert_eq!(&payload[..], b"replacement payload");

    let removed = api_json(
        &app,
        &token,
        "/api/v1/resources/remove-binary",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-file",
            "resourceVersion": resource_version,
            "expectedDraftRevision": replacement_revision,
            "binaryName": "payload.txt"
        }),
    )
    .await;
    assert_eq!(removed["data"]["action"], "REMOVED");
    assert!(!removed.to_string().contains("local://"));
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn binary_upload_requires_small_control_fields_before_it_allocates_a_temporary_file() {
    let (app, storage_root) = resource_app("upload-controls");
    let token = authenticated_bearer_token(&app).await;
    let boundary = "tron-resource-http-control-boundary";

    let file_before_controls = multipart_request(
        &app,
        &token,
        boundary,
        multipart_body(boundary, &[], b"the file must not be accepted first"),
    )
    .await;
    assert_eq!(file_before_controls.status(), StatusCode::BAD_REQUEST);
    let file_first_payload = response_json(file_before_controls).await;
    assert!(file_first_payload["code"].is_string());
    assert!(file_first_payload["data"].is_object());
    assert!(!file_first_payload.to_string().contains("local://"));
    assert_storage_root_is_empty(&storage_root);

    let oversized = "x".repeat(8 * 1024 + 1);
    let oversized_control = multipart_request(
        &app,
        &token,
        boundary,
        multipart_body(
            boundary,
            &[("scopeKind", oversized.as_str())],
            b"not written",
        ),
    )
    .await;
    assert_eq!(oversized_control.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let oversized_payload = response_json(oversized_control).await;
    assert!(oversized_payload["code"].is_string());
    assert!(oversized_payload["data"].is_object());
    assert_storage_root_is_empty(&storage_root);

    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn oversized_binary_upload_is_rejected_without_persisting_an_object() {
    let test_name = "oversized-upload";
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_nanos();
    let storage_root = std::env::temp_dir().join(format!(
        "tron-resource-http-api-{test_name}-{}-{nanos}",
        std::process::id(),
    ));
    let mut config = AppConfig::default();
    config.resource = ResourceConfig::with_default_root(storage_root.display().to_string());
    config.resource.limits.max_binary_bytes = 8;
    let app = app_with_config(config);
    let token = authenticated_bearer_token(&app).await;

    let draft_definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        binary_definition_create_body_with_max_size("HTTP_TINY_FILE", 8),
    )
    .await;
    let definition_version = draft_definition["data"]["definitionVersion"]
        .as_str()
        .expect("definition should have a version")
        .to_string();
    let definition_revision = draft_definition["data"]["draftRevision"]
        .as_str()
        .expect("definition should have a revision")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_TINY_FILE",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision
        }),
    )
    .await;
    let draft_resource = api_json(
        &app,
        &token,
        "/api/v1/resources/create-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-tiny-file",
            "sourceResourceVersion": null,
            "definitionCode": "HTTP_TINY_FILE",
            "name": "Tiny HTTP file",
            "description": null,
            "attributes": { "title": "tiny" },
            "allowInheritance": true
        }),
    )
    .await;
    let resource_version = draft_resource["data"]["resourceVersion"]
        .as_str()
        .expect("resource should have a version")
        .to_string();
    let resource_revision = draft_resource["data"]["draftRevision"]
        .as_str()
        .expect("resource should have a revision")
        .to_string();

    let upload = multipart_upload_for_resource(
        &app,
        &token,
        "http-tiny-file",
        &resource_version,
        &resource_revision,
        "payload.txt",
        b"123456789",
    )
    .await;
    assert_eq!(upload.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let payload = response_json(upload).await;
    assert!(payload["code"].is_string());
    assert!(payload["data"].is_object());
    assert_storage_root_is_empty(&storage_root);
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn platform_resource_reads_require_resources_admin_permission() {
    let (app, storage_root) = resource_app("platform-admin");
    let viewer_token = signed_access_token("USR-0007", &["VIEWER"], &["SUPPORT", "PLATFORM"]);
    let response = authenticated_json_response(
        &app,
        &viewer_token,
        "/api/v1/resource-definitions/page",
        json!({
            "scopeKind": "PLATFORM",
            "workspaceCode": null,
            "search": null,
            "definitionCode": null,
            "definitionStatus": null,
            "lifecycleStatus": "ACTIVE",
            "validationStatus": null,
            "sourceKind": null,
            "current": 1,
            "size": 20
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload = response_json(response).await;
    assert!(payload["code"].is_string());
    assert!(payload["message"].is_string());
    assert!(payload["data"].is_object());
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn workspace_active_page_projects_effective_platform_resources() {
    let (app, storage_root) = resource_app("workspace-effective-platform");
    let token = authenticated_bearer_token(&app).await;
    let definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        platform_definition_create_body("HTTP_PLATFORM_DOCUMENT"),
    )
    .await;
    let definition_version = definition["data"]["definitionVersion"]
        .as_str()
        .expect("platform definition draft should have a version")
        .to_string();
    let definition_revision = definition["data"]["draftRevision"]
        .as_str()
        .expect("platform definition draft should have a revision")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "PLATFORM",
            "workspaceCode": null,
            "definitionCode": "HTTP_PLATFORM_DOCUMENT",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision
        }),
    )
    .await;

    let resource = api_json(
        &app,
        &token,
        "/api/v1/resources/create-draft",
        json!({
            "scopeKind": "PLATFORM",
            "workspaceCode": null,
            "resourceCode": "http-platform-document",
            "sourceResourceVersion": null,
            "definitionCode": "HTTP_PLATFORM_DOCUMENT",
            "name": "Platform HTTP document",
            "description": null,
            "attributes": { "title": "shared" },
            "allowInheritance": true
        }),
    )
    .await;
    let resource_version = resource["data"]["resourceVersion"]
        .as_str()
        .expect("platform resource draft should have a version")
        .to_string();
    let resource_revision = resource["data"]["draftRevision"]
        .as_str()
        .expect("platform resource draft should have a revision")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resources/publish-draft",
        json!({
            "scopeKind": "PLATFORM",
            "workspaceCode": null,
            "resourceCode": "http-platform-document",
            "resourceVersion": resource_version,
            "expectedDraftRevision": resource_revision
        }),
    )
    .await;

    let workspace_page = api_json(
        &app,
        &token,
        "/api/v1/resources/page",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "search": null,
            "resourceCode": "http-platform-document",
            "definitionCode": null,
            "definitionStatus": null,
            "lifecycleStatus": "ACTIVE",
            "validationStatus": "VALID",
            "sourceKind": "PLATFORM",
            "current": 1,
            "size": 20
        }),
    )
    .await;
    assert_eq!(workspace_page["data"]["total"], 1);
    let record = &workspace_page["data"]["records"][0];
    assert_eq!(record["sourceKind"], "PLATFORM");
    assert_eq!(record["scopeKind"], "PLATFORM");
    assert_eq!(record["scopeCode"], "PLATFORM");
    assert_eq!(record["resourceCode"], "http-platform-document");
    assert!(record["attributes"].is_null());
    assert!(record["attributesSchema"].is_null());
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn resource_draft_discard_preserves_its_definition_identity_in_the_response() {
    let (app, storage_root) = resource_app("discard-definition-identity");
    let token = authenticated_bearer_token(&app).await;
    let draft = api_json(
        &app,
        &token,
        "/api/v1/resources/create-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-file-discard",
            "sourceResourceVersion": null,
            "definitionCode": "FILE",
            "name": "Discarded file draft",
            "description": null,
            "attributes": {},
            "allowInheritance": true
        }),
    )
    .await;
    let resource_version = draft["data"]["resourceVersion"]
        .as_str()
        .expect("draft should have a resource version")
        .to_string();
    let draft_revision = draft["data"]["draftRevision"]
        .as_str()
        .expect("draft should have a revision")
        .to_string();
    let definition_version = draft["data"]["definitionVersion"]
        .as_str()
        .expect("draft should retain the definition version")
        .to_string();

    let discarded = api_json(
        &app,
        &token,
        "/api/v1/resources/discard-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-file-discard",
            "resourceVersion": resource_version,
            "expectedDraftRevision": draft_revision
        }),
    )
    .await;
    assert_eq!(discarded["data"]["draftResult"], "DISCARDED");
    assert_eq!(discarded["data"]["definitionScopeKind"], "PLATFORM");
    assert_eq!(discarded["data"]["definitionScopeCode"], "PLATFORM");
    assert_eq!(discarded["data"]["definitionCode"], "FILE");
    assert_eq!(discarded["data"]["definitionVersion"], definition_version);
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn direct_resource_commit_creates_and_updates_active_versions_without_drafts() {
    let (app, storage_root) = resource_app("direct-commit");
    let token = authenticated_bearer_token(&app).await;

    let definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        binary_definition_create_body("HTTP_DIRECT_FILE"),
    )
    .await;
    let definition_version = definition["data"]["definitionVersion"]
        .as_str()
        .expect("definition draft should have a version")
        .to_string();
    let definition_revision = definition["data"]["draftRevision"]
        .as_str()
        .expect("definition draft should have a revision")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_DIRECT_FILE",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision,
        }),
    )
    .await;

    let created_response = resource_commit_request(
        &app,
        &token,
        "/api/v1/resources/create",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-direct-file",
            "definitionCode": "HTTP_DIRECT_FILE",
            "expectedActiveResourceVersion": null,
            "name": "Direct file",
            "description": "created in one request",
            "attributes": { "title": "first" },
            "allowInheritance": true,
            "removedBinaryNames": [],
        }),
        &[("payload.txt", b"first payload" as &[u8])],
    )
    .await;
    assert_eq!(created_response.status(), StatusCode::OK);
    let created = response_json(created_response).await;
    assert_eq!(created["code"], "0");
    assert_eq!(created["data"]["lifecycleStatus"], "ACTIVE");
    assert_eq!(created["data"]["activationResult"], "COMMITTED");
    let first_version = created["data"]["resourceVersion"]
        .as_str()
        .expect("direct create should return an active version")
        .to_string();

    let draft_page = api_json(
        &app,
        &token,
        "/api/v1/resources/page",
        resource_page_body("http-direct-file", "DRAFT"),
    )
    .await;
    assert_eq!(draft_page["data"]["total"], 0);

    let updated_response = resource_commit_request(
        &app,
        &token,
        "/api/v1/resources/update",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-direct-file",
            "definitionCode": null,
            "expectedActiveResourceVersion": first_version,
            "name": "Direct file v2",
            "description": "updated in one request",
            "attributes": { "title": "second" },
            "allowInheritance": false,
            "removedBinaryNames": [],
        }),
        &[("payload.txt", b"replacement payload" as &[u8])],
    )
    .await;
    assert_eq!(updated_response.status(), StatusCode::OK);
    let updated = response_json(updated_response).await;
    assert_eq!(updated["code"], "0");
    assert_eq!(updated["data"]["activationResult"], "COMMITTED");
    assert_eq!(
        updated["data"]["previousActiveResourceVersion"],
        first_version
    );
    let second_version = updated["data"]["resourceVersion"]
        .as_str()
        .expect("direct update should return a new active version")
        .to_string();
    assert_ne!(second_version, first_version);

    let first_detail = api_json(
        &app,
        &token,
        "/api/v1/resources/get",
        resource_get_body("http-direct-file", &first_version),
    )
    .await;
    assert_eq!(first_detail["data"]["lifecycleStatus"], "HISTORICAL");

    let second_detail = api_json(
        &app,
        &token,
        "/api/v1/resources/get",
        resource_get_body("http-direct-file", &second_version),
    )
    .await;
    assert_eq!(second_detail["data"]["lifecycleStatus"], "ACTIVE");
    assert_eq!(
        second_detail["data"]["binaries"][0]["binaryName"],
        "payload.txt"
    );
    assert_eq!(second_detail["data"]["binaries"][0]["byteSize"], 19);

    let download = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resources/download-binary",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-direct-file",
            "resourceVersion": second_version,
            "binaryName": "payload.txt",
        }),
    )
    .await;
    assert_eq!(download.status(), StatusCode::OK);
    let payload = to_bytes(download.into_body(), usize::MAX)
        .await
        .expect("directly committed binary should download");
    assert_eq!(&payload[..], b"replacement payload");
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn direct_binary_removal_creates_an_active_snapshot_and_history_can_be_reactivated() {
    let (app, storage_root) = resource_app("direct-binary-removal");
    let token = authenticated_bearer_token(&app).await;
    let definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        optional_binary_definition_create_body("HTTP_OPTIONAL_BINARY"),
    )
    .await;
    let definition_version = definition["data"]["definitionVersion"]
        .as_str()
        .expect("optional binary definition draft should have a version")
        .to_string();
    let definition_revision = definition["data"]["draftRevision"]
        .as_str()
        .expect("optional binary definition draft should have a revision")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_OPTIONAL_BINARY",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision,
        }),
    )
    .await;

    let created = response_json(
        resource_commit_request(
            &app,
            &token,
            "/api/v1/resources/create",
            resource_create_command("http-optional-binary", "HTTP_OPTIONAL_BINARY", "first"),
            &[("payload.txt", b"first payload" as &[u8])],
        )
        .await,
    )
    .await;
    let first_version = created["data"]["resourceVersion"]
        .as_str()
        .expect("binary resource should have an active version")
        .to_string();

    let removed = response_json(
        resource_commit_request(
            &app,
            &token,
            "/api/v1/resources/update",
            json!({
                "scopeKind": "WORKSPACE",
                "workspaceCode": WORKSPACE,
                "resourceCode": "http-optional-binary",
                "definitionCode": null,
                "expectedActiveResourceVersion": first_version,
                "name": "http-optional-binary updated",
                "description": "binary removed in direct save",
                "attributes": { "title": "second" },
                "allowInheritance": true,
                "removedBinaryNames": ["payload.txt"],
            }),
            &[],
        )
        .await,
    )
    .await;
    let removed_version = removed["data"]["resourceVersion"]
        .as_str()
        .expect("binary removal should create a new active version")
        .to_string();

    let removed_detail = api_json(
        &app,
        &token,
        "/api/v1/resources/get",
        resource_get_body("http-optional-binary", &removed_version),
    )
    .await;
    assert_eq!(removed_detail["data"]["lifecycleStatus"], "ACTIVE");
    assert_eq!(removed_detail["data"]["binaries"], json!([]));

    let reactivated = api_json(
        &app,
        &token,
        "/api/v1/resources/activate-version",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-optional-binary",
            "resourceVersion": first_version,
            "expectedActiveResourceVersion": removed_version,
        }),
    )
    .await;
    assert_eq!(
        reactivated["data"]["activationResult"],
        "HISTORICAL_REACTIVATED"
    );

    let restored = api_json(
        &app,
        &token,
        "/api/v1/resources/get",
        resource_get_body("http-optional-binary", &first_version),
    )
    .await;
    assert_eq!(restored["data"]["lifecycleStatus"], "ACTIVE");
    assert_eq!(restored["data"]["binaries"][0]["binaryName"], "payload.txt");
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn deleting_a_workspace_override_restores_the_inherited_resource_and_rejects_pure_inheritance()
 {
    let (app, storage_root) = resource_app("delete-workspace-override");
    let token = authenticated_bearer_token(&app).await;

    let definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        platform_definition_create_body("HTTP_INHERITED_DOCUMENT"),
    )
    .await;
    let definition_version = definition["data"]["definitionVersion"]
        .as_str()
        .expect("platform definition draft should have a version")
        .to_string();
    let definition_revision = definition["data"]["draftRevision"]
        .as_str()
        .expect("platform definition draft should have a revision")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "PLATFORM",
            "workspaceCode": null,
            "definitionCode": "HTTP_INHERITED_DOCUMENT",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision,
        }),
    )
    .await;

    let platform_created = response_json(
        resource_commit_request(
            &app,
            &token,
            "/api/v1/resources/create",
            json!({
                "scopeKind": "PLATFORM",
                "workspaceCode": null,
                "resourceCode": "http-inherited-document",
                "definitionCode": "HTTP_INHERITED_DOCUMENT",
                "expectedActiveResourceVersion": null,
                "name": "Shared document",
                "description": "shared by platform",
                "attributes": { "title": "shared" },
                "allowInheritance": true,
                "removedBinaryNames": [],
            }),
            &[],
        )
        .await,
    )
    .await;
    let platform_resource_version = platform_created["data"]["resourceVersion"]
        .as_str()
        .expect("platform resource should become active")
        .to_string();

    let override_created = response_json(
        resource_commit_request(
            &app,
            &token,
            "/api/v1/resources/update",
            json!({
                "scopeKind": "WORKSPACE",
                "workspaceCode": WORKSPACE,
                "resourceCode": "http-inherited-document",
                "definitionCode": null,
                "expectedActiveResourceVersion": platform_resource_version,
                "name": "Workspace document",
                "description": "workspace override",
                "attributes": { "title": "workspace" },
                "allowInheritance": true,
                "removedBinaryNames": [],
            }),
            &[],
        )
        .await,
    )
    .await;
    let override_version = override_created["data"]["resourceVersion"]
        .as_str()
        .expect("workspace update should create a local active override")
        .to_string();

    let deleted = api_json(
        &app,
        &token,
        "/api/v1/resources/delete",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-inherited-document",
            "expectedActiveResourceVersion": override_version,
            "expectedDraftResourceVersion": null,
            "expectedDraftRevision": null,
        }),
    )
    .await;
    assert_eq!(deleted["data"]["deleted"], true);

    let restored = api_json(
        &app,
        &token,
        "/api/v1/resources/page",
        resource_page_body("http-inherited-document", "ACTIVE"),
    )
    .await;
    assert_eq!(restored["data"]["total"], 1);
    assert_eq!(restored["data"]["records"][0]["sourceKind"], "PLATFORM");
    assert_eq!(
        restored["data"]["records"][0]["resourceVersion"],
        platform_resource_version
    );

    let inherited_delete = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resources/delete",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-inherited-document",
            "expectedActiveResourceVersion": platform_resource_version,
            "expectedDraftResourceVersion": null,
            "expectedDraftRevision": null,
        }),
    )
    .await;
    assert_eq!(inherited_delete.status(), StatusCode::BAD_REQUEST);
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn disabled_definition_blocks_new_direct_resource_but_allows_existing_update_and_delete() {
    let (app, storage_root) = resource_app("disabled-definition-direct-resource");
    let token = authenticated_bearer_token(&app).await;
    let definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        definition_create_body("HTTP_DISABLED_DOCUMENT"),
    )
    .await;
    let definition_version = definition["data"]["definitionVersion"]
        .as_str()
        .expect("definition draft should have a version")
        .to_string();
    let definition_revision = definition["data"]["draftRevision"]
        .as_str()
        .expect("definition draft should have a revision")
        .to_string();
    let published = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_DISABLED_DOCUMENT",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision,
        }),
    )
    .await;
    let active_definition_version = published["data"]["definitionVersion"]
        .as_str()
        .expect("published definition should have an active version")
        .to_string();

    let created = response_json(
        resource_commit_request(
            &app,
            &token,
            "/api/v1/resources/create",
            resource_create_command("http-existing-document", "HTTP_DISABLED_DOCUMENT", "first"),
            &[],
        )
        .await,
    )
    .await;
    let first_resource_version = created["data"]["resourceVersion"]
        .as_str()
        .expect("existing resource should be active")
        .to_string();

    let disabled = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/set-enabled",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_DISABLED_DOCUMENT",
            "enabled": false,
            "expectedActiveDefinitionVersion": active_definition_version,
        }),
    )
    .await;
    assert_eq!(disabled["data"]["activationResult"], "COMMITTED");

    let blocked_create = resource_commit_request(
        &app,
        &token,
        "/api/v1/resources/create",
        resource_create_command("http-blocked-document", "HTTP_DISABLED_DOCUMENT", "blocked"),
        &[],
    )
    .await;
    assert_eq!(blocked_create.status(), StatusCode::CONFLICT);
    let blocked_payload = response_json(blocked_create).await;
    assert_eq!(blocked_payload["code"], "-1");

    let updated = response_json(
        resource_commit_request(
            &app,
            &token,
            "/api/v1/resources/update",
            resource_update_command("http-existing-document", &first_resource_version, "second"),
            &[],
        )
        .await,
    )
    .await;
    assert_eq!(updated["code"], "0");
    assert_eq!(updated["data"]["activationResult"], "COMMITTED");
    let second_resource_version = updated["data"]["resourceVersion"]
        .as_str()
        .expect("existing resource should remain maintainable")
        .to_string();

    let incomplete = api_json(
        &app,
        &token,
        "/api/v1/resources/create-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-existing-document",
            "sourceResourceVersion": second_resource_version,
            "definitionCode": null,
            "name": "Incomplete existing document",
            "description": "explicit unfinished work remains available",
            "attributes": { "title": "unfinished" },
            "allowInheritance": true,
        }),
    )
    .await;
    assert_eq!(incomplete["data"]["draftResult"], "CREATED");
    let incomplete_version = incomplete["data"]["resourceVersion"]
        .as_str()
        .expect("incomplete existing resource should have a draft version")
        .to_string();
    let incomplete_revision = incomplete["data"]["draftRevision"]
        .as_str()
        .expect("incomplete existing resource should have a revision")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resources/discard-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-existing-document",
            "resourceVersion": incomplete_version,
            "expectedDraftRevision": incomplete_revision,
        }),
    )
    .await;

    let deleted = api_json(
        &app,
        &token,
        "/api/v1/resources/delete",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-existing-document",
            "expectedActiveResourceVersion": second_resource_version,
            "expectedDraftResourceVersion": null,
            "expectedDraftRevision": null,
        }),
    )
    .await;
    assert_eq!(deleted["data"]["resourceCode"], "http-existing-document");
    assert_eq!(deleted["data"]["deleted"], true);
    let remaining = api_json(
        &app,
        &token,
        "/api/v1/resources/page",
        resource_page_body("http-existing-document", "ACTIVE"),
    )
    .await;
    assert_eq!(remaining["data"]["total"], 0);
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn definition_enable_state_change_rejects_a_pending_draft_even_when_it_is_a_no_op() {
    let (app, storage_root) = resource_app("definition-enable-draft-guard");
    let token = authenticated_bearer_token(&app).await;
    let definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        definition_create_body("HTTP_ENABLE_DRAFT_GUARD"),
    )
    .await;
    let definition_version = definition["data"]["definitionVersion"]
        .as_str()
        .expect("definition draft should have a version")
        .to_string();
    let definition_revision = definition["data"]["draftRevision"]
        .as_str()
        .expect("definition draft should have a revision")
        .to_string();
    let published = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_ENABLE_DRAFT_GUARD",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision,
        }),
    )
    .await;
    let active_version = published["data"]["definitionVersion"]
        .as_str()
        .expect("published definition should have an active version")
        .to_string();

    let mut next_draft = definition_create_body("HTTP_ENABLE_DRAFT_GUARD");
    next_draft["sourceDefinitionVersion"] = json!(active_version.clone());
    let draft = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        next_draft,
    )
    .await;
    assert_eq!(draft["data"]["lifecycleStatus"], "DRAFT");

    let no_op = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resource-definitions/set-enabled",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_ENABLE_DRAFT_GUARD",
            "enabled": true,
            "expectedActiveDefinitionVersion": active_version,
        }),
    )
    .await;
    assert_eq!(no_op.status(), StatusCode::CONFLICT);
    assert_eq!(response_json(no_op).await["code"], "-1");
    let _ = fs::remove_dir_all(storage_root);
}

#[tokio::test]
async fn definition_delete_requires_a_disabled_unreferenced_custom_definition() {
    let (app, storage_root) = resource_app("definition-delete-protection");
    let token = authenticated_bearer_token(&app).await;
    let definition = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/create-draft",
        definition_create_body("HTTP_GUARDED_DEFINITION"),
    )
    .await;
    let definition_version = definition["data"]["definitionVersion"]
        .as_str()
        .expect("definition draft should have a version")
        .to_string();
    let definition_revision = definition["data"]["draftRevision"]
        .as_str()
        .expect("definition draft should have a revision")
        .to_string();
    let published = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/publish-draft",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_GUARDED_DEFINITION",
            "definitionVersion": definition_version,
            "expectedDraftRevision": definition_revision,
        }),
    )
    .await;
    let enabled_version = published["data"]["definitionVersion"]
        .as_str()
        .expect("published definition should have an active version")
        .to_string();

    let enabled_delete = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resource-definitions/delete",
        definition_delete_body("HTTP_GUARDED_DEFINITION", &enabled_version),
    )
    .await;
    assert_eq!(enabled_delete.status(), StatusCode::BAD_REQUEST);

    let _ = response_json(
        resource_commit_request(
            &app,
            &token,
            "/api/v1/resources/create",
            resource_create_command(
                "http-guarded-resource",
                "HTTP_GUARDED_DEFINITION",
                "referenced",
            ),
            &[],
        )
        .await,
    )
    .await;

    let disabled = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/set-enabled",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "definitionCode": "HTTP_GUARDED_DEFINITION",
            "enabled": false,
            "expectedActiveDefinitionVersion": enabled_version,
        }),
    )
    .await;
    let disabled_version = disabled["data"]["definitionVersion"]
        .as_str()
        .expect("disabled definition should create a new active version")
        .to_string();

    let referenced_delete = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resource-definitions/delete",
        definition_delete_body("HTTP_GUARDED_DEFINITION", &disabled_version),
    )
    .await;
    assert_eq!(referenced_delete.status(), StatusCode::CONFLICT);
    let referenced_payload = response_json(referenced_delete).await;
    assert_eq!(referenced_payload["code"], "-1");

    let guarded_resource = api_json(
        &app,
        &token,
        "/api/v1/resources/page",
        resource_page_body("http-guarded-resource", "ACTIVE"),
    )
    .await;
    let guarded_resource_version = guarded_resource["data"]["records"][0]["resourceVersion"]
        .as_str()
        .expect("referencing resource should be active")
        .to_string();
    let _ = api_json(
        &app,
        &token,
        "/api/v1/resources/delete",
        json!({
            "scopeKind": "WORKSPACE",
            "workspaceCode": WORKSPACE,
            "resourceCode": "http-guarded-resource",
            "expectedActiveResourceVersion": guarded_resource_version,
            "expectedDraftResourceVersion": null,
            "expectedDraftRevision": null,
        }),
    )
    .await;

    let deleted = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/delete",
        definition_delete_body("HTTP_GUARDED_DEFINITION", &disabled_version),
    )
    .await;
    assert_eq!(deleted["data"]["definitionCode"], "HTTP_GUARDED_DEFINITION");
    assert_eq!(deleted["data"]["deleted"], true);

    let platform_definitions = api_json(
        &app,
        &token,
        "/api/v1/resource-definitions/page",
        json!({
            "scopeKind": "PLATFORM",
            "workspaceCode": null,
            "search": null,
            "definitionCode": "FILE",
            "definitionStatus": null,
            "lifecycleStatus": "ACTIVE",
            "validationStatus": null,
            "sourceKind": null,
            "current": 1,
            "size": 50,
        }),
    )
    .await;
    let file_version = platform_definitions["data"]["records"][0]["definitionVersion"]
        .as_str()
        .expect("built-in FILE definition should be present")
        .to_string();
    let builtin_delete = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resource-definitions/delete",
        json!({
            "scopeKind": "PLATFORM",
            "workspaceCode": null,
            "definitionCode": "FILE",
            "expectedActiveDefinitionVersion": file_version,
        }),
    )
    .await;
    assert_eq!(builtin_delete.status(), StatusCode::CONFLICT);
    let _ = fs::remove_dir_all(storage_root);
}

async fn api_json(app: &Router, token: &str, uri: &str, body: Value) -> Value {
    let response = authenticated_json_response(app, token, uri, body).await;
    assert_eq!(response.status(), StatusCode::OK, "{uri} should succeed");
    response_json(response).await
}

async fn authenticated_json_response(
    app: &Router,
    token: &str,
    uri: &str,
    body: Value,
) -> axum::response::Response {
    let authorization = format!("Bearer {token}");
    request_with_headers(
        app,
        axum::http::Method::POST,
        uri,
        Some(body),
        None,
        Some(&authorization),
    )
    .await
}

async fn multipart_upload(
    app: &Router,
    token: &str,
    resource_version: &str,
    expected_draft_revision: &str,
    binary_name: &str,
    payload: &[u8],
) -> axum::response::Response {
    multipart_upload_for_resource(
        app,
        token,
        "http-file",
        resource_version,
        expected_draft_revision,
        binary_name,
        payload,
    )
    .await
}

async fn multipart_upload_for_resource(
    app: &Router,
    token: &str,
    resource_code: &str,
    resource_version: &str,
    expected_draft_revision: &str,
    binary_name: &str,
    payload: &[u8],
) -> axum::response::Response {
    let boundary = "tron-resource-http-boundary";
    let body = multipart_body(
        boundary,
        &[
            ("scopeKind", "WORKSPACE"),
            ("workspaceCode", WORKSPACE),
            ("resourceCode", resource_code),
            ("resourceVersion", resource_version),
            ("expectedDraftRevision", expected_draft_revision),
            ("binaryName", binary_name),
        ],
        payload,
    );
    multipart_request(app, token, boundary, body).await
}

async fn multipart_request(
    app: &Router,
    token: &str,
    boundary: &str,
    body: Vec<u8>,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/resources/upload-binary")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("multipart request should build"),
        )
        .await
        .expect("multipart response should return")
}

async fn resource_commit_request(
    app: &Router,
    token: &str,
    uri: &str,
    command: Value,
    binaries: &[(&str, &[u8])],
) -> axum::response::Response {
    let boundary = "tron-resource-http-commit-boundary";
    let body = resource_commit_multipart_body(boundary, &command, binaries);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("resource commit request should build"),
        )
        .await
        .expect("resource commit response should return")
}

fn assert_storage_root_is_empty(storage_root: &std::path::Path) {
    let entries = fs::read_dir(storage_root)
        .expect("Resource storage root should exist")
        .collect::<Result<Vec<_>, _>>()
        .expect("Resource storage root should be readable");
    assert!(
        entries.is_empty(),
        "rejected upload must not persist an object"
    );
}

fn multipart_body(boundary: &str, values: &[(&str, &str)], payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in values {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"payload.txt\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(payload);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

fn resource_commit_multipart_body(
    boundary: &str,
    command: &Value,
    binaries: &[(&str, &[u8])],
) -> Vec<u8> {
    let mut body = Vec::new();
    let command = serde_json::to_string(command).expect("resource commit command should serialize");
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"command\"\r\n\r\n");
    body.extend_from_slice(command.as_bytes());
    body.extend_from_slice(b"\r\n");
    for (binary_name, payload) in binaries {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"binary\"; filename=\"{binary_name}\"\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        body.extend_from_slice(payload);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

fn resource_create_command(resource_code: &str, definition_code: &str, title: &str) -> Value {
    json!({
        "scopeKind": "WORKSPACE",
        "workspaceCode": WORKSPACE,
        "resourceCode": resource_code,
        "definitionCode": definition_code,
        "expectedActiveResourceVersion": null,
        "name": resource_code,
        "description": "direct resource",
        "attributes": { "title": title },
        "allowInheritance": true,
        "removedBinaryNames": [],
    })
}

fn resource_update_command(resource_code: &str, version: &str, title: &str) -> Value {
    json!({
        "scopeKind": "WORKSPACE",
        "workspaceCode": WORKSPACE,
        "resourceCode": resource_code,
        "definitionCode": null,
        "expectedActiveResourceVersion": version,
        "name": format!("{resource_code} updated"),
        "description": "direct update",
        "attributes": { "title": title },
        "allowInheritance": false,
        "removedBinaryNames": [],
    })
}

fn resource_get_body(resource_code: &str, resource_version: &str) -> Value {
    json!({
        "scopeKind": "WORKSPACE",
        "workspaceCode": WORKSPACE,
        "resourceCode": resource_code,
        "resourceVersion": resource_version,
    })
}

fn resource_page_body(resource_code: &str, lifecycle_status: &str) -> Value {
    json!({
        "scopeKind": "WORKSPACE",
        "workspaceCode": WORKSPACE,
        "search": null,
        "resourceCode": resource_code,
        "definitionCode": null,
        "definitionStatus": null,
        "lifecycleStatus": lifecycle_status,
        "validationStatus": null,
        "sourceKind": null,
        "current": 1,
        "size": 50,
    })
}

fn definition_delete_body(definition_code: &str, definition_version: &str) -> Value {
    json!({
        "scopeKind": "WORKSPACE",
        "workspaceCode": WORKSPACE,
        "definitionCode": definition_code,
        "expectedActiveDefinitionVersion": definition_version,
    })
}

fn definition_create_body(definition_code: &str) -> Value {
    json!({
        "scopeKind": "WORKSPACE",
        "workspaceCode": WORKSPACE,
        "definitionCode": definition_code,
        "sourceDefinitionVersion": null,
        "name": "HTTP document definition",
        "description": "HTTP integration fixture",
        "definitionStatus": "ENABLED",
        "allowInheritance": true,
        "attributesSchema": {
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": { "title": { "type": "string", "minLength": 1 } },
            "required": ["title"],
            "additionalProperties": false
        },
        "binarySchema": { "minCount": 0, "maxCount": 0 },
        "binaryStorageProfile": null
    })
}

fn binary_definition_create_body(definition_code: &str) -> Value {
    binary_definition_create_body_with_max_size(definition_code, 64)
}

fn optional_binary_definition_create_body(definition_code: &str) -> Value {
    let mut body = binary_definition_create_body(definition_code);
    body["binarySchema"]["minCount"] = json!(0);
    body
}

fn binary_definition_create_body_with_max_size(
    definition_code: &str,
    max_size_bytes: u64,
) -> Value {
    let mut body = definition_create_body(definition_code);
    body["name"] = Value::String("HTTP file definition".to_string());
    body["binarySchema"] = json!({
        "minCount": 1,
        "maxCount": 1,
        "maxSizeBytes": max_size_bytes,
        "namePattern": "^[a-z]+\\.txt$"
    });
    body
}

fn platform_definition_create_body(definition_code: &str) -> Value {
    let mut body = definition_create_body(definition_code);
    body["scopeKind"] = Value::String("PLATFORM".to_string());
    body["workspaceCode"] = Value::Null;
    body
}

fn resource_app(test_name: &str) -> (Router, PathBuf) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_nanos();
    let storage_root = std::env::temp_dir().join(format!(
        "tron-resource-http-api-{test_name}-{}-{nanos}",
        std::process::id(),
    ));
    let mut config = AppConfig::default();
    config.resource = ResourceConfig::with_default_root(storage_root.display().to_string());
    (app_with_config(config), storage_root)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TestAccessTokenClaims {
    sub: String,
    sid: String,
    user_code: String,
    session_code: String,
    auth_method: String,
    provider_code: Option<String>,
    role_codes: Vec<String>,
    space_codes: Vec<String>,
    exp: usize,
    iat: usize,
    kind: String,
}

fn signed_access_token(user_code: &str, role_codes: &[&str], space_codes: &[&str]) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_secs() as usize;
    let claims = TestAccessTokenClaims {
        sub: user_code.to_string(),
        sid: format!("TEST-{user_code}"),
        user_code: user_code.to_string(),
        session_code: format!("TEST-{user_code}"),
        auth_method: "internal".to_string(),
        provider_code: None,
        role_codes: role_codes
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        space_codes: space_codes
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        exp: now + 300,
        iat: now,
        kind: "access".to_string(),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(AppConfig::default().auth.jwt_secret.as_bytes()),
    )
    .expect("viewer access token should be signed")
}
