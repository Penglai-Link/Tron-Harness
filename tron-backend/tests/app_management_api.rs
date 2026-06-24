mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use serde_json::json;
use tron_backend::config::AppConfig;

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn list_apps_is_workspace_scoped_and_supports_filters() {
    let router = app_with_config(AppConfig {
        database_url: temp_database_url("app-management-list")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });

    let active = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/apps/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(active.status(), StatusCode::OK);
    let active_json = response_json(active).await;
    assert_eq!(active_json["code"], "SUCCESS");
    assert_eq!(active_json["data"]["total"], 2);

    let backend_only = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/apps/workspaces/TRON_CORE?runtimeRole=Backend",
        None,
    )
    .await;
    assert_eq!(backend_only.status(), StatusCode::OK);
    assert_eq!(
        response_json(backend_only).await["data"]["records"],
        json!([{
            "appCode": "APP-TRON-API",
            "workspaceCode": "TRON_CORE",
            "name": "Tron API",
            "description": "Core backend serving task and admin workflows.",
            "localDirectory": "/workspace/tron/tron-backend",
            "runtimeRole": "Backend",
            "repositoryUrl": "https://github.com/clouds3n/tron-backend",
            "defaultBranch": "master",
            "tags": ["Core", "Rust"],
            "status": "Active",
            "updatedAt": "2026-05-19T00:00:00Z",
            "archivedAt": null
        }])
    );

    let archived_only = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/apps/workspaces/TRON_CORE?status=Archived",
        None,
    )
    .await;
    assert_eq!(archived_only.status(), StatusCode::OK);
    assert_eq!(response_json(archived_only).await["data"]["total"], 1);

    let include_archived = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/apps/workspaces/TRON_CORE?includeArchived=true",
        None,
    )
    .await;
    assert_eq!(include_archived.status(), StatusCode::OK);
    assert_eq!(response_json(include_archived).await["data"]["total"], 3);
}

#[tokio::test]
async fn create_update_archive_restore_and_options_flow_work_for_apps() {
    let router = app_with_config(AppConfig {
        database_url: temp_database_url("app-management-flow")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });

    let created = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/apps/workspaces/TRON_CORE",
        Some(json!({
            "name": "Task Console",
            "description": "Frontend for task execution oversight.",
            "localDirectory": "/workspace/apps/task-console",
            "runtimeRole": "Mono",
            "repositoryUrl": "https://github.com/clouds3n/task-console",
            "defaultBranch": "master",
            "tags": ["React", "Tasking"]
        })),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_json = response_json(created).await;
    let app_code = created_json["data"]["appCode"].as_str().unwrap().to_owned();
    let app_id = app_code
        .strip_prefix("APP-")
        .expect("created app code should keep APP prefix")
        .parse::<i64>()
        .expect("created app code suffix should be numeric");
    assert!(
        app_id > 734_900_000_000_000_100,
        "created app code should use the shared snowflake provider instead of the SQLite max(id)+1 allocator: {app_code}",
    );
    assert_eq!(created_json["data"]["workspaceCode"], "TRON_CORE");
    assert_eq!(created_json["data"]["runtimeRole"], "Mono");
    assert_eq!(
        created_json["data"]["localDirectory"],
        "/workspace/apps/task-console"
    );
    assert_eq!(created_json["data"]["status"], "Active");

    let options = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/apps/workspaces/TRON_CORE/options",
        None,
    )
    .await;
    assert_eq!(options.status(), StatusCode::OK);
    assert!(
        response_json(options).await["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["appCode"] == app_code)
    );

    let mono_only = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/apps/workspaces/TRON_CORE?runtimeRole=Mono",
        None,
    )
    .await;
    assert_eq!(mono_only.status(), StatusCode::OK);
    assert!(
        response_json(mono_only).await["data"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["appCode"] == app_code)
    );

    let updated = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/apps/{app_code}"),
        Some(json!({
            "name": "Task Runtime API",
            "description": "Backend for task execution oversight.",
            "localDirectory": "/workspace/tron/task-runtime-api",
            "runtimeRole": "Backend",
            "repositoryUrl": "https://github.com/clouds3n/task-runtime-api",
            "defaultBranch": "main",
            "tags": ["Rust", "Tasking"]
        })),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated_json = response_json(updated).await;
    assert_eq!(
        updated_json["data"]["localDirectory"],
        "/workspace/tron/task-runtime-api"
    );
    assert_eq!(updated_json["data"]["runtimeRole"], "Backend");
    assert_eq!(updated_json["data"]["defaultBranch"], "main");

    let archived = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/apps/{app_code}/archive"),
        None,
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    assert_eq!(response_json(archived).await["data"]["status"], "Archived");

    let options_after_archive = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/apps/workspaces/TRON_CORE/options",
        None,
    )
    .await;
    assert_eq!(options_after_archive.status(), StatusCode::OK);
    assert!(
        !response_json(options_after_archive).await["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["appCode"] == app_code)
    );

    let restored = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/apps/{app_code}/restore"),
        None,
    )
    .await;
    assert_eq!(restored.status(), StatusCode::OK);
    assert_eq!(response_json(restored).await["data"]["status"], "Active");
}

#[tokio::test]
async fn duplicate_names_are_rejected_within_a_workspace() {
    let router = app_with_config(AppConfig {
        database_url: temp_database_url("app-management-duplicate-name")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });

    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/apps/workspaces/TRON_CORE",
        Some(json!({
            "name": "Tron API",
            "description": "Duplicate backend name",
            "localDirectory": "/workspace/tron/tron-api-copy",
            "runtimeRole": "Backend",
            "repositoryUrl": null,
            "defaultBranch": null,
            "tags": []
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(response_json(response).await["code"], "APP_NAME_DUPLICATE");
}

#[tokio::test]
async fn apps_allow_missing_repository_url_but_require_local_directory() {
    let router = app_with_config(AppConfig {
        database_url: temp_database_url("app-management-validation")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });

    let missing_repository = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/apps/workspaces/TRON_CORE",
        Some(json!({
            "name": "Repo Optional",
            "description": null,
            "localDirectory": "/workspace/apps/repo-optional",
            "runtimeRole": "Backend",
            "repositoryUrl": null,
            "defaultBranch": "main",
            "tags": []
        })),
    )
    .await;
    assert_eq!(missing_repository.status(), StatusCode::CREATED);
    let missing_repository_json = response_json(missing_repository).await;
    assert_eq!(
        missing_repository_json["data"]["repositoryUrl"],
        json!(null)
    );

    let missing_directory = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/apps/workspaces/TRON_CORE",
        Some(json!({
            "name": "Local Without Directory",
            "description": null,
            "localDirectory": "   ",
            "runtimeRole": "Frontend",
            "repositoryUrl": null,
            "defaultBranch": null,
            "tags": []
        })),
    )
    .await;
    assert_eq!(missing_directory.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(missing_directory).await["code"],
        "APP_LOCAL_DIRECTORY_REQUIRED"
    );
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
