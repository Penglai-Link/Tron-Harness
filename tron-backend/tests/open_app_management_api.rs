mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use common::{OPEN_API_TEST_APP_CODE, app, authenticated_request, response_json};

#[tokio::test]
async fn list_open_apps_and_catalog_expose_platform_management_views() {
    let router = app();

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/platform/open-apps",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    assert_eq!(list_json["code"], json!("SUCCESS"));
    let records = list_json["data"]
        .as_array()
        .expect("open-app list should return an array");
    let seeded = records
        .iter()
        .find(|item| item["openAppCode"] == json!(OPEN_API_TEST_APP_CODE))
        .expect("seeded open-app should be visible");
    assert!(seeded.get("appKey").is_none());
    assert_eq!(
        seeded["appKeyFingerprint"]
            .as_str()
            .expect("seeded fingerprint should exist")
            .len(),
        12
    );

    let catalog_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/platform/open-apps/catalog",
        None,
    )
    .await;
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog_json = response_json(catalog_response).await;
    let groups = catalog_json["data"]
        .as_array()
        .expect("catalog should return grouped capabilities");
    let agent_chat = groups
        .iter()
        .find(|item| item["capabilityCode"] == json!("agent-chat"))
        .expect("agent-chat catalog group should exist");
    assert_eq!(agent_chat["capabilityName"], json!("Agent Chat OpenAPI"));
    let operations = agent_chat["operations"]
        .as_array()
        .expect("agent-chat should include operations");
    assert!(
        operations
            .iter()
            .any(|item| item["operationCode"] == json!("session.read"))
    );
}

#[tokio::test]
async fn create_patch_and_rotate_open_app_keep_secret_reveal_one_time_only() {
    let router = app();

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/open-apps",
        Some(json!({
            "name": "CRM Bridge",
            "description": "External CRM integration"
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let create_json = response_json(create_response).await;
    let open_app_code = create_json["data"]["openApp"]["openAppCode"]
        .as_str()
        .expect("create should return an open-app code")
        .to_owned();
    let created_key = create_json["data"]["secretReveal"]["appKey"]
        .as_str()
        .expect("create should reveal the app key")
        .to_owned();
    let created_fingerprint = create_json["data"]["secretReveal"]["appKeyFingerprint"]
        .as_str()
        .expect("create should return the fingerprint")
        .to_owned();
    assert_eq!(
        create_json["data"]["openApp"]["appKeyFingerprint"],
        json!(created_fingerprint.clone())
    );

    let patch_response = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}"),
        Some(json!({
            "name": "CRM Bridge Disabled",
            "description": "Disabled for maintenance",
            "status": "Disabled"
        })),
    )
    .await;
    assert_eq!(patch_response.status(), StatusCode::OK);
    let patch_json = response_json(patch_response).await;
    assert_eq!(patch_json["data"]["name"], json!("CRM Bridge Disabled"));
    assert_eq!(patch_json["data"]["status"], json!("Disabled"));

    let rotate_disabled = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}/rotate-key"),
        None,
    )
    .await;
    assert_eq!(rotate_disabled.status(), StatusCode::CONFLICT);
    let rotate_disabled_json = response_json(rotate_disabled).await;
    assert_eq!(rotate_disabled_json["code"], json!("OPEN_APP_DISABLED"));

    let enable_response = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}"),
        Some(json!({
            "name": "CRM Bridge",
            "description": "External CRM integration",
            "status": "Enabled"
        })),
    )
    .await;
    assert_eq!(enable_response.status(), StatusCode::OK);

    let rotate_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}/rotate-key"),
        None,
    )
    .await;
    assert_eq!(rotate_response.status(), StatusCode::OK);
    let rotate_json = response_json(rotate_response).await;
    let rotated_key = rotate_json["data"]["secretReveal"]["appKey"]
        .as_str()
        .expect("rotate should reveal a new app key")
        .to_owned();
    let rotated_fingerprint = rotate_json["data"]["secretReveal"]["appKeyFingerprint"]
        .as_str()
        .expect("rotate should reveal the new fingerprint")
        .to_owned();
    assert_ne!(rotated_key, created_key);
    assert_ne!(rotated_fingerprint, created_fingerprint);

    let detail_response = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}"),
        None,
    )
    .await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_json = response_json(detail_response).await;
    let detail = &detail_json["data"];
    assert_eq!(detail["appKeyFingerprint"], json!(rotated_fingerprint));
    assert!(detail.get("secretReveal").is_none());
    assert!(detail.get("appKey").is_none());

    let archive_response = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}"),
        Some(json!({
            "name": "CRM Bridge",
            "description": "External CRM integration",
            "status": "Archived"
        })),
    )
    .await;
    assert_eq!(archive_response.status(), StatusCode::OK);
    let archive_json = response_json(archive_response).await;
    assert_eq!(archive_json["data"]["status"], json!("Archived"));

    let rotate_archived = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}/rotate-key"),
        None,
    )
    .await;
    assert_eq!(rotate_archived.status(), StatusCode::CONFLICT);
    let rotate_archived_json = response_json(rotate_archived).await;
    assert_eq!(rotate_archived_json["code"], json!("OPEN_APP_DISABLED"));
}

#[tokio::test]
async fn replacing_grants_updates_detail_and_rejects_invalid_targets() {
    let router = app();

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/open-apps",
        Some(json!({
            "name": "Ops Bridge"
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let create_json = response_json(create_response).await;
    let open_app_code = create_json["data"]["openApp"]["openAppCode"]
        .as_str()
        .expect("create should return an open-app code")
        .to_owned();

    let user_grants_response = authenticated_request(
        &router,
        Method::PUT,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}/user-grants"),
        Some(json!({
            "userCodes": ["USR-0002", "USR-0001", "USR-0002"]
        })),
    )
    .await;
    assert_eq!(user_grants_response.status(), StatusCode::OK);
    let user_grants_json = response_json(user_grants_response).await;
    assert_eq!(
        user_grants_json["data"]["userGrants"],
        json!([
            { "userCode": "USR-0001" },
            { "userCode": "USR-0002" }
        ])
    );

    let capability_grants_response = authenticated_request(
        &router,
        Method::PUT,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}/capability-grants"),
        Some(json!({
            "grants": [
                { "capabilityCode": "agent-chat", "operationCode": "message.write" },
                { "capabilityCode": "agent-chat", "operationCode": "session.read" },
                { "capabilityCode": "agent-chat", "operationCode": "message.write" }
            ]
        })),
    )
    .await;
    assert_eq!(capability_grants_response.status(), StatusCode::OK);
    let capability_grants_json = response_json(capability_grants_response).await;
    assert_eq!(
        capability_grants_json["data"]["capabilityGrants"],
        json!([
            { "capabilityCode": "agent-chat", "operationCode": "message.write" },
            { "capabilityCode": "agent-chat", "operationCode": "session.read" }
        ])
    );

    let invalid_user_response = authenticated_request(
        &router,
        Method::PUT,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}/user-grants"),
        Some(json!({
            "userCodes": ["USR-MISSING"]
        })),
    )
    .await;
    assert_eq!(invalid_user_response.status(), StatusCode::BAD_REQUEST);
    let invalid_user_json = response_json(invalid_user_response).await;
    assert_eq!(
        invalid_user_json["code"],
        json!("OPEN_APP_USER_CODE_INVALID")
    );

    let invalid_capability_response = authenticated_request(
        &router,
        Method::PUT,
        &format!("/api/v1/admin/platform/open-apps/{open_app_code}/capability-grants"),
        Some(json!({
            "grants": [
                { "capabilityCode": "agent-chat", "operationCode": "not-real" }
            ]
        })),
    )
    .await;
    assert_eq!(
        invalid_capability_response.status(),
        StatusCode::BAD_REQUEST
    );
    let invalid_capability_json = response_json(invalid_capability_response).await;
    assert_eq!(
        invalid_capability_json["code"],
        json!("OPEN_APP_CAPABILITY_INVALID")
    );
}
