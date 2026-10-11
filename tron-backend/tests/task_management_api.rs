mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use rusqlite::Connection;
use serde_json::{Value, json};
use tron_backend::config::AppConfig;

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn platform_general_tasks_have_nullable_workspace_and_clearable_deadline() {
    let router = task_app("platform-general");
    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/tasks",
        Some(json!({
            "title": "Prepare report", "dueAt": 2_000_000_000_000_i64
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = response_json(response).await;
    assert_eq!(created["data"]["workspaceCode"], Value::Null);
    assert_eq!(created["data"]["taskType"], "General");
    let code = created["data"]["taskCode"].as_str().unwrap();
    let response = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{code}"),
        Some(json!({"dueAt": null})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated = response_json(response).await;
    assert_eq!(updated["data"]["dueAt"], Value::Null);
    let response = authenticated_request(&router, Method::GET, "/api/v1/tasks", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["data"]["total"], 1);
    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/tasks?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["data"]["total"], 0);
}

#[tokio::test]
async fn task_options_and_default_kanban_exclude_archived_tasks_and_seed_data() {
    let router = task_app("options-kanban");

    let options_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/tasks/options?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(options_response.status(), StatusCode::OK);
    let options = response_json(options_response).await;
    assert_eq!(options["data"]["flow"]["flowCode"], "DEFAULT");
    let statuses = options["data"]["statuses"].as_array().unwrap();
    assert_eq!(
        statuses
            .iter()
            .map(|s| s["statusCode"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["NOT_STARTED", "IN_PROGRESS", "REOPEN", "CLOSED"]
    );
    assert_eq!(
        options["data"]["taskTypes"],
        json!(["General", "Feature", "Bug"])
    );
    assert_eq!(
        options["data"]["priorities"],
        json!(["Low", "Medium", "High", "Extra High"])
    );
    for suffix in ["", "&taskType=Feature", "&taskType=Bug"] {
        let response = authenticated_request(
            &router,
            Method::GET,
            &format!("/api/v1/tasks/kanban?workspaceCode=TRON_CORE{suffix}"),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        let columns = body["data"].as_array().unwrap();
        assert_eq!(
            columns
                .iter()
                .map(|c| c["status"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["NOT_STARTED", "IN_PROGRESS", "REOPEN", "CLOSED"]
        );
        assert!(columns.iter().all(|c| c["tasks"] == json!([])));
    }
}

#[tokio::test]
async fn create_task_normalizes_assignments_and_defaults_to_not_started() {
    let router = task_app("create-validation");

    let created = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/tasks",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "title": "Ship task management superpower",
            "description": "Persist the role-aware task workflow",
            "priority": "High",
            "taskType": "Bug",
            "sourceType": "GitHub",
            "sourceUrl": "",
            "assigneeUserCodes": ["USR-AUTH-0001"],
            "agentCodes": ["agent-main", "agent-main", "agent-reviewer"],
            "appCodes": ["APP-TRON-CONSOLE"]
        })),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = response_json(created).await;
    assert_eq!(body["data"]["taskType"], "Bug");
    assert_eq!(body["data"]["status"], "NOT_STARTED");
    assert_eq!(
        body["data"]["description"],
        "Persist the role-aware task workflow"
    );
    assert_eq!(body["data"]["executionResult"], Value::Null);
    assert_eq!(body["data"]["sourceType"], "GitHub");
    assert_eq!(body["data"]["sourceUrl"], "");
    assert_eq!(body["data"]["appCodes"], json!(["APP-TRON-CONSOLE"]));
    assert_eq!(
        body["data"]["agentCodes"],
        json!(["agent-main", "agent-reviewer"])
    );
}

#[tokio::test]
async fn task_app_codes_must_reference_active_apps_in_the_same_workspace() {
    let router = task_app("task-app-validation");

    for invalid_app_codes in [
        json!(["APP-NOT-FOUND"]),
        json!(["APP-TRON-LEGACY"]),
        json!(["APP-COMPLIANCE-API"]),
    ] {
        let response = authenticated_request(
            &router,
            Method::POST,
            "/api/v1/tasks",
            Some(json!({
                "workspaceCode": "TRON_CORE",
                "title": "Invalid app selection",
                "priority": "Medium",
                "taskType": "Feature",
                "sourceType": "Internal",
                "sourceUrl": "",
                "assigneeUserCodes": ["USR-AUTH-0001"],
                "agentCodes": [],
                "appCodes": invalid_app_codes,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response_json(response).await["code"], "TASK_APP_INVALID");
    }

    let created = create_task(&router, "TRON_CORE", None, "Valid app selection", "Feature").await;
    let task_code = created["data"]["taskCode"].as_str().unwrap().to_owned();

    let updated = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{task_code}"),
        Some(json!({
            "appCodes": ["APP-TRON-CONSOLE", "APP-TRON-CONSOLE"]
        })),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(
        response_json(updated).await["data"]["appCodes"],
        json!(["APP-TRON-CONSOLE"])
    );

    let invalid_update = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{task_code}"),
        Some(json!({
            "appCodes": ["APP-COMPLIANCE-API"]
        })),
    )
    .await;
    assert_eq!(invalid_update.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(invalid_update).await["code"],
        "TASK_APP_INVALID"
    );
}

#[tokio::test]
async fn subtask_creation_rejects_cross_workspace_parent() {
    let router = task_app("subtask-parent");
    let parent = create_task(&router, "TRON_CORE", None, "Parent task", "Feature").await;
    let parent_code = parent["data"]["taskCode"].as_str().unwrap().to_owned();
    let child = create_task(
        &router,
        "TRON_CORE",
        Some(&parent_code),
        "Child task",
        "Feature",
    )
    .await;
    assert_eq!(child["data"]["parentTaskCode"], parent_code);
    assert_eq!(child["data"]["parent"]["taskCode"], parent_code);
    assert_eq!(child["data"]["status"], "NOT_STARTED");

    let invalid = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/tasks",
        Some(json!({
            "workspaceCode": "PLATFORM",
            "parentTaskCode": parent_code,
            "title": "Cross",
            "priority": "Medium",
            "taskType": "Bug",
            "sourceType": "Internal",
            "sourceUrl": "",
            "assigneeUserCodes": [],
            "agentCodes": [],
            "appCodes": []
        })),
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response_json(invalid).await["code"], "TASK_PARENT_INVALID");
}

#[tokio::test]
async fn parent_detail_archive_rules_ignore_archived_subtasks_and_restore_to_open() {
    let database_path = database_path("subtask-archive-rules");
    let router = task_app_with_database_path(database_path.clone(), true);

    let parent = create_task(&router, "TRON_CORE", None, "Parent archive rule", "Feature").await;
    let parent_code = parent["data"]["taskCode"].as_str().unwrap().to_owned();
    let active_child = create_task(
        &router,
        "TRON_CORE",
        Some(&parent_code),
        "Active child",
        "Feature",
    )
    .await;
    let active_child_code = active_child["data"]["taskCode"]
        .as_str()
        .unwrap()
        .to_owned();
    let archived_child = create_task(
        &router,
        "TRON_CORE",
        Some(&parent_code),
        "Archived child",
        "Feature",
    )
    .await;
    let archived_child_code = archived_child["data"]["taskCode"]
        .as_str()
        .unwrap()
        .to_owned();

    let blocked = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{parent_code}/archive"),
        None,
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(blocked).await["code"],
        "TASK_ACTIVE_SUBTASK_EXISTS"
    );

    let archived_child_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{archived_child_code}/archive"),
        None,
    )
    .await;
    assert_eq!(archived_child_response.status(), StatusCode::OK);

    let detail = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/tasks/{parent_code}"),
        None,
    )
    .await;
    let detail_json = response_json(detail).await;
    assert_eq!(
        detail_json["data"]["subtasks"],
        json!([{
            "taskCode": active_child_code.clone(),
            "title": "Active child",
            "status": "NOT_STARTED",
            "priority": "High",
            "progress": 0,
            "taskType": "Feature"
        }])
    );

    let list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/tasks?workspaceCode=TRON_CORE&parentTaskCode=__ROOT__&search=Parent%20archive%20rule",
        None,
    )
    .await;
    let list_json = response_json(list).await;
    assert_eq!(list_json["data"]["records"][0]["subtaskCount"], 1);

    let active_child_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{active_child_code}/archive"),
        None,
    )
    .await;
    assert_eq!(active_child_response.status(), StatusCode::OK);

    let parent_archived = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{parent_code}/archive"),
        None,
    )
    .await;
    assert_eq!(
        response_json(parent_archived).await["data"]["previousStatus"],
        "NOT_STARTED"
    );

    clear_previous_status(&database_path, &parent_code);

    let restored = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{parent_code}/restore"),
        None,
    )
    .await;
    assert_eq!(
        response_json(restored).await["data"]["status"],
        "NOT_STARTED"
    );
}

#[tokio::test]
async fn parent_progress_is_derived_from_active_subtasks() {
    let router = task_app("parent-progress-derived");
    let parent = create_task(&router, "TRON_CORE", None, "Parent progress", "Feature").await;
    let parent_code = parent["data"]["taskCode"].as_str().unwrap().to_owned();
    let first_child = create_task(
        &router,
        "TRON_CORE",
        Some(&parent_code),
        "Child one",
        "Feature",
    )
    .await;
    let first_child_code = first_child["data"]["taskCode"].as_str().unwrap().to_owned();
    let second_child = create_task(
        &router,
        "TRON_CORE",
        Some(&parent_code),
        "Child two",
        "Feature",
    )
    .await;
    let second_child_code = second_child["data"]["taskCode"]
        .as_str()
        .unwrap()
        .to_owned();

    let first_completed = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{first_child_code}/status"),
        Some(json!({ "status": "CLOSED", "progress": 10 })),
    )
    .await;
    assert_eq!(first_completed.status(), StatusCode::OK);
    let second_in_progress = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{second_child_code}/status"),
        Some(json!({ "status": "IN_PROGRESS", "progress": 0 })),
    )
    .await;
    assert_eq!(second_in_progress.status(), StatusCode::OK);

    let detail = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/tasks/{parent_code}"),
        None,
    )
    .await;
    assert_eq!(response_json(detail).await["data"]["progress"], 50);

    let second_archived = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{second_child_code}/archive"),
        None,
    )
    .await;
    assert_eq!(second_archived.status(), StatusCode::OK);

    let detail_after_archive = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/tasks/{parent_code}"),
        None,
    )
    .await;
    assert_eq!(
        response_json(detail_after_archive).await["data"]["progress"],
        100
    );
}

#[tokio::test]
async fn flow_status_validation_is_independent_of_task_type_and_parentage() {
    let router = task_app("flow-validation");
    let parent = create_task(&router, "TRON_CORE", None, "Parent", "Feature").await;
    let parent_code = parent["data"]["taskCode"].as_str().unwrap();
    let child = create_task(&router, "TRON_CORE", Some(parent_code), "Child", "Feature").await;
    for task in [&parent, &child] {
        let code = task["data"]["taskCode"].as_str().unwrap();
        let valid = authenticated_request(
            &router,
            Method::PATCH,
            &format!("/api/v1/tasks/{code}/status"),
            Some(json!({"status":"IN_PROGRESS","progress":35})),
        )
        .await;
        assert_eq!(valid.status(), StatusCode::OK);
        let invalid = authenticated_request(
            &router,
            Method::PATCH,
            &format!("/api/v1/tasks/{code}/status"),
            Some(json!({"status":"NOT_IN_THIS_FLOW","progress":60})),
        )
        .await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        let changed_type = authenticated_request(
            &router,
            Method::PATCH,
            &format!("/api/v1/tasks/{code}"),
            Some(json!({"taskType":"Bug"})),
        )
        .await;
        assert_eq!(changed_type.status(), StatusCode::OK);
        let body = response_json(changed_type).await;
        assert_eq!(body["data"]["status"], "IN_PROGRESS");
        assert_eq!(body["data"]["taskType"], "Bug");
    }
}

async fn create_task(
    router: &axum::Router,
    workspace_code: &str,
    parent_task_code: Option<&str>,
    title: &str,
    task_type: &str,
) -> Value {
    let response = authenticated_request(
        router,
        Method::POST,
        "/api/v1/tasks",
        Some(json!({
            "workspaceCode": workspace_code,
            "parentTaskCode": parent_task_code,
            "title": title,
            "description": format!("{title} description"),
            "priority": "High",
            "taskType": task_type,
            "sourceType": "Internal",
            "sourceUrl": "",
            "assigneeUserCodes": if workspace_code == "TRON_CORE" { json!(["USR-AUTH-0001"]) } else { json!([]) },
            "agentCodes": [],
            "appCodes": []
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}

fn task_app(test_name: &str) -> axum::Router {
    task_app_with_database_path(database_path(test_name), true)
}

fn task_app_with_database_path(database_path: PathBuf, reset_before_boot: bool) -> axum::Router {
    if reset_before_boot && database_path.exists() {
        std::fs::remove_file(&database_path).expect("test database should be resettable");
    }
    let mut config = AppConfig::default();
    config.database_url = database_path.to_string_lossy().into_owned();
    app_with_config(config)
}

fn database_path(test_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("task-management")
        .join(format!("{test_name}.sqlite"))
}

fn clear_previous_status(database_path: &PathBuf, task_code: &str) {
    let connection = Connection::open(database_path).expect("task database should be writable");
    connection
        .execute(
            "UPDATE tasks SET previous_status = NULL WHERE task_code = ?1",
            [task_code],
        )
        .expect("previous status should be clearable");
}
