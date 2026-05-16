mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use tron_backend::config::AppConfig;

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn task_options_and_default_kanban_exclude_archived_tasks() {
    let router = task_app("options-kanban");
    let options_response =
        authenticated_request(&router, Method::GET, "/api/v1/tasks/options", None).await;
    assert_eq!(options_response.status(), StatusCode::OK);
    let options_json = response_json(options_response).await;
    assert_eq!(
        options_json["data"]["statuses"],
        json!([
            "Backlog",
            "Todo",
            "InProgress",
            "Testing",
            "Review",
            "Reopen",
            "Completed",
            "Archived"
        ])
    );
    assert_eq!(
        options_json["data"]["taskTypes"],
        json!(["Feature", "BugFix", "Refactor"])
    );
    assert_eq!(
        options_json["data"]["sourceTypes"],
        json!(["Internal", "GitHub", "GitLab"])
    );

    let kanban_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/tasks/kanban?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(kanban_response.status(), StatusCode::OK);
    let kanban_json = response_json(kanban_response).await;
    let columns = kanban_json["data"]
        .as_array()
        .expect("kanban data should be an array");
    assert_eq!(columns.len(), 7);
    assert!(!columns.iter().any(|column| column["status"] == "Archived"));

    let archived_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/tasks/kanban?workspaceCode=TRON_CORE&status=Archived&includeArchived=true",
        None,
    )
    .await;
    assert_eq!(archived_response.status(), StatusCode::OK);
    let archived_json = response_json(archived_response).await;
    assert!(
        archived_json["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|column| column["status"] == "Archived")
    );
}

#[tokio::test]
async fn create_task_validates_workspace_assignees_and_optional_source_url() {
    let router = task_app("create-validation");
    let invalid = authenticated_request(&router, Method::POST, "/api/v1/tasks", Some(json!({ "workspaceCode": "TRON_CORE", "title": "Invalid", "priority": "Low", "taskType": "Feature", "sourceType": "Internal", "sourceUrl": "", "assigneeUserCodes": ["USR-ARCHIVED"], "agentCodes": ["agent-main"], "appCodes": ["github"] }))).await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(invalid).await["code"],
        "TASK_ASSIGNEE_INVALID"
    );

    let created = authenticated_request(&router, Method::POST, "/api/v1/tasks", Some(json!({ "workspaceCode": "TRON_CORE", "title": "Ship task management v1", "description": "Persist task planning flows", "priority": "High", "taskType": "BugFix", "sourceType": "GitHub", "sourceUrl": "", "assigneeUserCodes": ["USR-AUTH-0001"], "agentCodes": ["agent-main", "agent-main", "agent-reviewer"], "appCodes": ["github"] }))).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = response_json(created).await;
    assert_eq!(body["data"]["taskType"], "BugFix");
    assert_eq!(body["data"]["sourceType"], "GitHub");
    assert_eq!(body["data"]["sourceUrl"], "");
    assert_eq!(
        body["data"]["agentCodes"],
        json!(["agent-main", "agent-reviewer"])
    );
}

#[tokio::test]
async fn subtask_creation_rejects_cross_workspace_parent() {
    let router = task_app("subtask-parent");
    let parent = create_task(&router, "TRON_CORE", None, "Parent task").await;
    let parent_code = parent["data"]["taskCode"].as_str().unwrap().to_owned();
    let child = create_task(&router, "TRON_CORE", Some(&parent_code), "Child task").await;
    assert_eq!(child["data"]["parentTaskCode"], parent_code);
    assert_eq!(child["data"]["parent"]["taskCode"], parent_code);
    let invalid = authenticated_request(&router, Method::POST, "/api/v1/tasks", Some(json!({ "workspaceCode": "PLATFORM", "parentTaskCode": parent_code, "title": "Cross", "priority": "Medium", "taskType": "Refactor", "sourceType": "Internal", "sourceUrl": "", "assigneeUserCodes": [], "agentCodes": [], "appCodes": [] }))).await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response_json(invalid).await["code"], "TASK_PARENT_INVALID");
}

#[tokio::test]
async fn parent_detail_and_archive_rules_ignore_archived_subtasks() {
    let router = task_app("subtask-archive-rules");
    let parent = create_task(&router, "TRON_CORE", None, "Parent archive rule").await;
    let parent_code = parent["data"]["taskCode"].as_str().unwrap().to_owned();
    let active_child = create_task(&router, "TRON_CORE", Some(&parent_code), "Active child").await;
    let active_child_code = active_child["data"]["taskCode"]
        .as_str()
        .unwrap()
        .to_owned();
    let archived_child =
        create_task(&router, "TRON_CORE", Some(&parent_code), "Archived child").await;
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
            "status": "Backlog",
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

    let detail_without_active_subtasks = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/tasks/{parent_code}"),
        None,
    )
    .await;
    assert_eq!(
        response_json(detail_without_active_subtasks).await["data"]["subtasks"],
        json!([])
    );

    let parent_archived = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{parent_code}/archive"),
        None,
    )
    .await;
    assert_eq!(parent_archived.status(), StatusCode::OK);
    assert_eq!(
        response_json(parent_archived).await["data"]["status"],
        "Archived"
    );
}

#[tokio::test]
async fn update_status_archive_and_restore_task_lifecycle() {
    let router = task_app("status-archive-restore");
    let created = create_task(&router, "TRON_CORE", None, "Lifecycle task").await;
    let task_code = created["data"]["taskCode"].as_str().unwrap().to_owned();
    let testing = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{task_code}/status"),
        Some(json!({ "status": "Testing", "progress": 55 })),
    )
    .await;
    assert_eq!(testing.status(), StatusCode::OK);
    let testing_body = response_json(testing).await;
    assert_eq!(testing_body["data"]["status"], "Testing");
    assert_eq!(testing_body["data"]["progress"], 55);
    let completed = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{task_code}/status"),
        Some(json!({ "status": "Completed", "progress": 10 })),
    )
    .await;
    assert_eq!(response_json(completed).await["data"]["progress"], 100);
    let reopen = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{task_code}/status"),
        Some(json!({ "status": "Reopen", "progress": 20 })),
    )
    .await;
    assert_eq!(response_json(reopen).await["data"]["status"], "Reopen");
    let patched = authenticated_request(&router, Method::PATCH, &format!("/api/v1/tasks/{task_code}"), Some(json!({ "title": "Lifecycle task updated", "priority": "Medium", "taskType": "Refactor", "sourceType": "GitLab", "sourceUrl": "https://gitlab.com/clouds3n/tron/-/issues/7", "assigneeUserCodes": ["USR-AUTH-0001"], "agentCodes": ["agent-reviewer"], "appCodes": ["gitlab"] }))).await;
    let patched_body = response_json(patched).await;
    assert_eq!(patched_body["data"]["taskType"], "Refactor");
    let archived = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{task_code}/archive"),
        None,
    )
    .await;
    let archived_body = response_json(archived).await;
    assert_eq!(archived_body["data"]["status"], "Archived");
    assert_eq!(archived_body["data"]["previousStatus"], "Reopen");
    let mutation = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/tasks/{task_code}"),
        Some(json!({ "title": "Archived mutation" })),
    )
    .await;
    assert_eq!(mutation.status(), StatusCode::BAD_REQUEST);
    let list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/tasks?workspaceCode=TRON_CORE&search=Lifecycle%20task%20updated",
        None,
    )
    .await;
    assert_eq!(response_json(list).await["data"]["total"], 0);
    let restored = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/tasks/{task_code}/restore"),
        None,
    )
    .await;
    assert_eq!(response_json(restored).await["data"]["status"], "Reopen");
}

async fn create_task(
    router: &axum::Router,
    workspace_code: &str,
    parent_task_code: Option<&str>,
    title: &str,
) -> Value {
    let response = authenticated_request(router, Method::POST, "/api/v1/tasks", Some(json!({ "workspaceCode": workspace_code, "parentTaskCode": parent_task_code, "title": title, "description": format!("{title} description"), "priority": "High", "taskType": "Feature", "sourceType": "Internal", "sourceUrl": "", "assigneeUserCodes": if workspace_code == "TRON_CORE" { json!(["USR-AUTH-0001"]) } else { json!([]) }, "agentCodes": [], "appCodes": [] }))).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}

fn task_app(test_name: &str) -> axum::Router {
    let mut config = AppConfig::default();
    let database_path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("task-management")
        .join(format!("{test_name}.sqlite"));
    if database_path.exists() {
        std::fs::remove_file(&database_path).expect("test database should be resettable");
    }
    config.database_url = database_path.to_string_lossy().into_owned();
    app_with_config(config)
}
