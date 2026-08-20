mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use common::{app, authenticated_request, response_json, signed_open_api_request};

#[tokio::test]
async fn internal_terminal_ticket_without_app_uses_the_configured_working_directory() {
    let router = app();

    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/terminals/socket-tickets",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "appCode": null
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert!(
        body["data"]["ticket"]
            .as_str()
            .is_some_and(|ticket| !ticket.is_empty())
    );
    assert!(
        body["data"]["expiresAt"]
            .as_i64()
            .is_some_and(|value| value > 0)
    );
    assert_eq!(body["data"]["socketPath"], json!("/socket.io"));
    assert_eq!(body["data"]["socketNamespace"], json!("/terminals"));
}

#[tokio::test]
async fn terminal_ticket_rejects_mismatched_or_inaccessible_workspaces() {
    let router = app();

    let mismatched = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/terminals/socket-tickets",
        Some(json!({
            "workspaceCode": "COMPLIANCE",
            "appCode": "APP-TRON-CONSOLE"
        })),
    )
    .await;
    assert_eq!(mismatched.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(mismatched).await["code"],
        json!("TERMINAL_APP_WORKSPACE_INVALID")
    );

    let inaccessible = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/terminals/socket-tickets",
        Some(json!({
            "workspaceCode": "COMPLIANCE",
            "appCode": "APP-COMPLIANCE-API"
        })),
    )
    .await;
    assert_eq!(inaccessible.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_json(inaccessible).await["code"],
        json!("TERMINAL_WORKSPACE_FORBIDDEN")
    );
}

#[tokio::test]
async fn terminal_request_rejections_keep_the_api_envelope_contract() {
    let router = app();

    let internal = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/terminals/socket-tickets",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "unexpected": true
        })),
    )
    .await;
    assert_eq!(internal.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(internal).await["code"],
        json!("TERMINAL_SOCKET_TICKET_REQUEST_INVALID")
    );

    let open_api = signed_open_api_request(
        &router,
        Method::POST,
        "/open-api/v1/terminals/commands/generate",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "prompt": "show changes",
            "unexpected": true
        })),
    )
    .await;
    assert_eq!(open_api.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(open_api).await["code"],
        json!("TERMINAL_COMMAND_GENERATE_REQUEST_INVALID")
    );
}

#[tokio::test]
async fn open_api_terminal_ticket_uses_the_public_contract() {
    let router = app();

    let response = signed_open_api_request(
        &router,
        Method::POST,
        "/open-api/v1/terminals/socket-tickets",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "appCode": null
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["code"], json!("SUCCESS"));
    assert!(
        body["data"]["ticket"]
            .as_str()
            .is_some_and(|ticket| !ticket.is_empty())
    );
    assert_eq!(body["data"]["socketPath"], json!("/socket.io"));
    assert_eq!(body["data"]["socketNamespace"], json!("/terminals"));
}

#[tokio::test]
async fn terminal_command_generation_uses_the_workspace_llm_runtime_contract() {
    let router = app();

    let internal = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/terminals/commands/generate",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "modelCode": null,
            "prompt": "show the current changes"
        })),
    )
    .await;
    assert_eq!(internal.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(internal).await["code"],
        json!("TERMINAL_COMMAND_MODEL_UNAVAILABLE")
    );

    let open_api = signed_open_api_request(
        &router,
        Method::POST,
        "/open-api/v1/terminals/commands/generate",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "modelCode": null,
            "prompt": "show the current changes"
        })),
    )
    .await;
    assert_eq!(open_api.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(open_api).await["code"],
        json!("TERMINAL_COMMAND_MODEL_UNAVAILABLE")
    );
}
