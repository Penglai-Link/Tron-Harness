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
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app_with_config, authenticated_bearer_token, request_with_headers, response_json};
use tron_backend::config::{AppConfig, ResourceConfig};

const WORKSPACE: &str = "TRON_CORE";

#[tokio::test]
async fn file_resource_lifecycle_uses_the_management_contract() {
    let (app, _) = resource_app("lifecycle");
    let token = authenticated_bearer_token(&app).await;

    let created = create_file_resource(&app, &token, "release-notes", b"initial notes").await;
    assert_eq!(created.status(), StatusCode::OK);
    let created = response_json(created).await;
    assert_eq!(created["data"]["resourceCode"], "release-notes");
    assert_eq!(created["data"]["kind"], "FILE");

    let page = api_json(
        &app,
        &token,
        "/api/v1/resources/page",
        json!({
            "workspaceCode": WORKSPACE,
            "kind": "FILE",
            "search": null,
            "current": 1,
            "size": 50
        }),
    )
    .await;
    assert_eq!(page["code"], "0");
    assert_eq!(page["data"]["total"], 1);
    let record = &page["data"]["records"][0];
    assert_eq!(record["kind"], "FILE");
    assert_eq!(record["workspaceCode"], WORKSPACE);
    assert_eq!(record["sourceKind"], "LOCAL");
    assert_eq!(record["file"]["fileName"], "notes.txt");
    assert_eq!(record["file"]["byteSize"], 13);
    assert!(record["attributes"].is_null());

    let detail = api_json(
        &app,
        &token,
        "/api/v1/resources/get",
        json!({
            "workspaceCode": WORKSPACE,
            "resourceCode": "release-notes"
        }),
    )
    .await;
    assert_eq!(detail["data"]["name"], "release-notes");

    let updated = api_json(
        &app,
        &token,
        "/api/v1/resources/update",
        json!({
            "workspaceCode": WORKSPACE,
            "resourceCode": "release-notes",
            "name": "Release notes v2",
            "description": "updated",
            "allowInheritance": false
        }),
    )
    .await;
    assert_eq!(updated["code"], "0");
    let detail = api_json(
        &app,
        &token,
        "/api/v1/resources/get",
        json!({
            "workspaceCode": WORKSPACE,
            "resourceCode": "release-notes"
        }),
    )
    .await;
    assert_eq!(detail["data"]["name"], "Release notes v2");
    assert_eq!(detail["data"]["allowInheritance"], false);

    let deleted = api_json(
        &app,
        &token,
        "/api/v1/resources/delete",
        json!({
            "workspaceCode": WORKSPACE,
            "resourceCode": "release-notes"
        }),
    )
    .await;
    assert_eq!(deleted["data"]["deleted"], true);
    let page = api_json(
        &app,
        &token,
        "/api/v1/resources/page",
        json!({
            "workspaceCode": WORKSPACE,
            "kind": "FILE",
            "current": 1,
            "size": 50
        }),
    )
    .await;
    assert_eq!(page["data"]["total"], 0);
}

#[tokio::test]
async fn definition_management_endpoints_are_gone() {
    let (app, _) = resource_app("definitions-removed");
    let token = authenticated_bearer_token(&app).await;
    let response = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resource-definitions/page",
        json!({ "current": 1, "size": 50 }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn file_binary_can_be_replaced_and_downloaded() {
    let (app, _) = resource_app("binary-replace");
    let token = authenticated_bearer_token(&app).await;
    let created = create_file_resource(&app, &token, "binary-file", b"first").await;
    assert_eq!(created.status(), StatusCode::OK);

    let upload = multipart_upload(&app, &token, "binary-file", "updated.txt", b"second content")
        .await;
    assert_eq!(upload.status(), StatusCode::OK);
    let upload_body = response_json(upload).await;
    assert_eq!(upload_body["data"]["action"], "UPLOADED");
    assert_eq!(upload_body["data"]["byteSize"], 14);

    let detail = api_json(
        &app,
        &token,
        "/api/v1/resources/get",
        json!({
            "workspaceCode": WORKSPACE,
            "resourceCode": "binary-file"
        }),
    )
    .await;
    assert_eq!(detail["data"]["file"]["fileName"], "updated.txt");
    assert_eq!(detail["data"]["file"]["fileExtension"], "txt");

    let download = authenticated_json_response(
        &app,
        &token,
        "/api/v1/resources/download-binary",
        json!({
            "workspaceCode": WORKSPACE,
            "resourceCode": "binary-file"
        }),
    )
    .await;
    assert_eq!(download.status(), StatusCode::OK);
    let bytes = to_bytes(download.into_body(), usize::MAX)
        .await
        .expect("download body should drain");
    assert_eq!(bytes.as_ref(), b"second content");
}

#[tokio::test]
async fn deleting_a_file_resource_removes_its_stored_object() {
    let (app, storage_root) = resource_app("delete-cleanup");
    let token = authenticated_bearer_token(&app).await;
    let created = create_file_resource(&app, &token, "cleanup-file", b"payload").await;
    assert_eq!(created.status(), StatusCode::OK);
    assert!(!collected_storage_files(&storage_root).is_empty());

    api_json(
        &app,
        &token,
        "/api/v1/resources/delete",
        json!({
            "workspaceCode": WORKSPACE,
            "resourceCode": "cleanup-file"
        }),
    )
    .await;
    assert_storage_root_is_empty(&storage_root);
}

#[tokio::test]
async fn workspace_page_requires_workspace_read_permission() {
    let (app, _) = resource_app("workspace-authorization");
    let token = authenticated_bearer_token(&app).await;
    let created = create_file_resource(&app, &token, "authorized-file", b"body").await;
    assert_eq!(created.status(), StatusCode::OK);

    let viewer_token = signed_access_token("USR-0007", &["VIEWER"], &["SUPPORT", "PLATFORM"]);
    let forbidden = authenticated_json_response(
        &app,
        &viewer_token,
        "/api/v1/resources/page",
        json!({ "workspaceCode": WORKSPACE, "current": 1, "size": 50 }),
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn child_workspace_page_projects_inherited_parent_resources() {
    let (app, _) = resource_app("inheritance-projection");
    let token = authenticated_bearer_token(&app).await;
    let parent_token = signed_access_token(
        "USR-LARGE-ID",
        &["WORKSPACE_ADMIN", "DEVELOPER"],
        &["ENTERPRISE", "PLATFORM"],
    );
    let created =
        create_file_resource_in_workspace(&app, &parent_token, "PLATFORM", "shared-template", b"shared")
            .await;
    let status = created.status();
    let body = response_json(created).await;
    assert_eq!(status, StatusCode::OK, "create failed: {body}");

    let page = api_json(
        &app,
        &token,
        "/api/v1/resources/page",
        json!({
            "workspaceCode": WORKSPACE,
            "kind": "FILE",
            "current": 1,
            "size": 50
        }),
    )
    .await;
    assert_eq!(page["data"]["total"], 1);
    assert_eq!(page["data"]["records"][0]["sourceKind"], "INHERITED");
}

#[tokio::test]
async fn creating_a_duplicate_resource_code_conflicts() {
    let (app, _) = resource_app("duplicate-code");
    let token = authenticated_bearer_token(&app).await;
    let first = create_file_resource(&app, &token, "duplicate", b"one").await;
    assert_eq!(first.status(), StatusCode::OK);
    let second = create_file_resource(&app, &token, "duplicate", b"two").await;
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

async fn create_file_resource(
    app: &Router,
    token: &str,
    resource_code: &str,
    payload: &[u8],
) -> axum::response::Response {
    create_file_resource_in_workspace(app, token, WORKSPACE, resource_code, payload).await
}

async fn create_file_resource_in_workspace(
    app: &Router,
    token: &str,
    workspace_code: &str,
    resource_code: &str,
    payload: &[u8],
) -> axum::response::Response {
    let command = json!({
        "workspaceCode": workspace_code,
        "resourceCode": resource_code,
        "kind": "FILE",
        "name": resource_code,
        "description": "http fixture",
        "allowInheritance": true
    });
    let boundary = "tron-resource-create-boundary";
    let body = create_multipart_body(boundary, &command, payload);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/resources/create")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("create request should build"),
        )
        .await
        .expect("create response should return")
}

async fn multipart_upload(
    app: &Router,
    token: &str,
    resource_code: &str,
    file_name: &str,
    payload: &[u8],
) -> axum::response::Response {
    let boundary = "tron-resource-upload-boundary";
    let body = upload_multipart_body(boundary, resource_code, file_name, payload);
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
                .expect("upload request should build"),
        )
        .await
        .expect("upload response should return")
}

fn create_multipart_body(boundary: &str, command: &Value, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    let command = serde_json::to_string(command).expect("command should serialize");
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"command\"\r\n\r\n");
    body.extend_from_slice(command.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"binary\"; filename=\"notes.txt\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(payload);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

fn upload_multipart_body(
    boundary: &str,
    resource_code: &str,
    file_name: &str,
    payload: &[u8],
) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in [
        ("workspaceCode", WORKSPACE),
        ("resourceCode", resource_code),
    ] {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(payload);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
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

fn assert_storage_root_is_empty(storage_root: &std::path::Path) {
    assert!(
        collected_storage_files(storage_root).is_empty(),
        "deleted resource must not persist an object"
    );
}

fn collected_storage_files(storage_root: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(storage_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collected_storage_files(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
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

#[derive(serde::Serialize)]
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
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
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
