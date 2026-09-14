mod common;

use std::{path::PathBuf, time::Duration};

use axum::http::{Method, StatusCode};
use serde_json::json;
use tokio::time::timeout;
use tron_backend::config::AppConfig;

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn saving_a_project_and_listing_it_returns_without_blocking() {
    let router = app_with_config(AppConfig {
        database_url: temp_database_url("project-management-flow")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });

    let created = timeout(
        Duration::from_secs(1),
        authenticated_request(
            &router,
            Method::POST,
            "/api/v1/projects/save",
            Some(json!({
                "workspaceCode": "TRON_CORE",
                "name": "Tron workspace",
                "appCodes": ["APP-TRON-API", "APP-TRON-CONSOLE"],
                "primaryAppCode": "APP-TRON-API"
            })),
        ),
    )
    .await
    .expect("project save should not block");
    assert_eq!(created.status(), StatusCode::OK);
    let created_json = response_json(created).await;
    assert_eq!(created_json["code"], "SUCCESS");
    assert_eq!(created_json["data"]["name"], "Tron workspace");
    assert_eq!(created_json["data"]["primaryAppCode"], "APP-TRON-API");

    let project_code = created_json["data"]["projectCode"]
        .as_str()
        .expect("created project should expose its code");
    let first_session = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "projectCode": project_code,
            "title": "New chat",
            "agentCode": "main-agent"
        })),
    )
    .await;
    assert_eq!(first_session.status(), StatusCode::CREATED);
    let first_session_json = response_json(first_session).await;

    let repeated_session = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "projectCode": project_code,
            "title": "New chat",
            "agentCode": "main-agent"
        })),
    )
    .await;
    assert_eq!(repeated_session.status(), StatusCode::CREATED);
    let repeated_session_json = response_json(repeated_session).await;
    assert_eq!(
        repeated_session_json["data"]["sessionCode"],
        first_session_json["data"]["sessionCode"],
        "a project can have only one active empty chat session"
    );

    let listed = timeout(
        Duration::from_secs(1),
        authenticated_request(
            &router,
            Method::POST,
            "/api/v1/projects/list",
            Some(json!({ "workspaceCode": "TRON_CORE" })),
        ),
    )
    .await
    .expect("project list should not block");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_json = response_json(listed).await;
    assert_eq!(listed_json["code"], "SUCCESS");
    assert_eq!(listed_json["data"][0]["name"], "Tron workspace");
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let unique = format!(
        "tron-project-management-{test_name}-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos(),
    );
    std::env::temp_dir().join(unique)
}
