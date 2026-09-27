mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::config::AppConfig;

use common::{app_with_config, authenticated_request, response_json};

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
    assert_eq!(options["data"]["taskTypes"], json!(["Feature", "Bug"]));
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

#[tokio::test]
async fn task_repository_migrates_completed_description_to_execution_result() {
    let database_path = database_path("execution-result-migration");
    seed_current_task_schema_without_execution_result(&database_path);
    let router = task_app_with_database_path(database_path.clone(), false);

    let response =
        authenticated_request(&router, Method::GET, "/api/v1/tasks/TSK-CURRENT-0001", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["data"]["description"], Value::Null);
    assert_eq!(body["data"]["executionResult"], "Legacy execution markdown");

    let connection = Connection::open(&database_path).expect("task database should open");
    let migrated: (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT description, execution_result FROM tasks WHERE task_code = 'TSK-CURRENT-0001'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("migrated task should exist");
    assert_eq!(migrated, (None, Some("Legacy execution markdown".into())));
}

#[tokio::test]
async fn task_repository_migrates_priority_constraint_for_extra_high() {
    let database_path = database_path("priority-extra-high-migration");
    seed_current_task_schema_without_execution_result(&database_path);
    let router = task_app_with_database_path(database_path.clone(), false);

    let response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/tasks/TSK-CURRENT-0001",
        Some(json!({ "priority": "Extra High" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["data"]["priority"],
        "Extra High"
    );

    let connection = Connection::open(&database_path).expect("task database should open");
    let stored_priority: String = connection
        .query_row(
            "SELECT priority FROM tasks WHERE task_code = 'TSK-CURRENT-0001'",
            [],
            |row| row.get(0),
        )
        .expect("updated task should exist");
    assert_eq!(stored_priority, "Extra High");

    let create_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tasks'",
            [],
            |row| row.get(0),
        )
        .expect("tasks definition should exist");
    assert!(create_sql.contains("'Extra High'"));
}

#[tokio::test]
async fn task_repository_resets_legacy_schema_and_removes_seed_tasks() {
    let database_path = database_path("legacy-reset");
    seed_legacy_task_schema(&database_path);
    let router = task_app_with_database_path(database_path.clone(), false);

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/tasks?workspaceCode=TRON_CORE&includeArchived=true",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    assert_eq!(response_json(list_response).await["data"]["total"], 0);

    let connection = Connection::open(&database_path).expect("legacy reset database should exist");
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
        .expect("tasks count query should succeed");
    assert_eq!(count, 0);
    let create_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tasks'",
            [],
            |row| row.get(0),
        )
        .expect("tasks definition should exist");
    assert!(!create_sql.contains("Backlog"));
    assert!(!create_sql.contains("BugFix"));
    assert!(create_sql.contains("flow_code"));
    assert!(!create_sql.contains("Regressioning"));
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

fn seed_current_task_schema_without_execution_result(database_path: &PathBuf) {
    if database_path.exists() {
        std::fs::remove_file(database_path).expect("task database should be resettable");
    }
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent).expect("task database parent should be creatable");
    }
    let connection = Connection::open(database_path).expect("task database should open");
    connection
        .execute_batch(
            "
            CREATE TABLE tasks (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL UNIQUE,
                workspace_code TEXT NOT NULL,
                parent_task_code TEXT NULL REFERENCES tasks(task_code),
                title TEXT NOT NULL,
                description TEXT NULL,
                status TEXT NOT NULL CHECK (status IN ('Open', 'Designing', 'Red', 'Green', 'Regressioning', 'Review', 'InProgress', 'Reopen', 'Completed', 'Archived')),
                previous_status TEXT NULL CHECK (previous_status IS NULL OR previous_status IN ('Open', 'Designing', 'Red', 'Green', 'Regressioning', 'Review', 'InProgress', 'Reopen', 'Completed')),
                priority TEXT NOT NULL CHECK (priority IN ('Low', 'Medium', 'High')),
                task_type TEXT NOT NULL CHECK (task_type IN ('Feature', 'Bug')),
                source_type TEXT NOT NULL CHECK (source_type IN ('Internal', 'GitHub', 'GitLab')),
                source_url TEXT NULL,
                agent_code TEXT NOT NULL DEFAULT 'main-agent',
                model_code TEXT NULL,
                progress_value INTEGER NOT NULL DEFAULT 0 CHECK (progress_value >= 0 AND progress_value <= 100),
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                archived_at INTEGER NULL
            );
            CREATE TABLE task_user_assignments (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL REFERENCES tasks(task_code),
                user_code TEXT NOT NULL,
                assigned_at INTEGER NOT NULL,
                UNIQUE (task_code, user_code)
            );
            CREATE TABLE task_agent_links (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL REFERENCES tasks(task_code),
                agent_code TEXT NOT NULL,
                linked_at INTEGER NOT NULL,
                UNIQUE (task_code, agent_code)
            );
            CREATE TABLE task_app_links (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL REFERENCES tasks(task_code),
                app_code TEXT NOT NULL,
                linked_at INTEGER NOT NULL,
                UNIQUE (task_code, app_code)
            );
            ",
        )
        .expect("current schema should be creatable");
    connection
        .execute(
            "INSERT INTO tasks (id, task_code, workspace_code, parent_task_code, title, description, status, previous_status, priority, task_type, source_type, source_url, agent_code, model_code, progress_value, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                734_500_000_000_000_001_i64,
                "TSK-CURRENT-0001",
                "TRON_CORE",
                Option::<String>::None,
                "Current task",
                "Legacy execution markdown",
                "Completed",
                Option::<String>::None,
                "Medium",
                "Feature",
                "Internal",
                "",
                "main-agent",
                Option::<String>::None,
                100_i64,
                1_779_830_400_000_i64,
                1_779_830_400_000_i64,
                Option::<i64>::None
            ],
        )
        .expect("current row should be insertable");
}

fn seed_legacy_task_schema(database_path: &PathBuf) {
    if database_path.exists() {
        std::fs::remove_file(database_path).expect("legacy database should be resettable");
    }
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent).expect("legacy database parent should be creatable");
    }
    let connection = Connection::open(database_path).expect("legacy database should open");
    connection
        .execute_batch(
            "
            CREATE TABLE tasks (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL UNIQUE,
                workspace_code TEXT NOT NULL,
                parent_task_code TEXT NULL REFERENCES tasks(task_code),
                title TEXT NOT NULL,
                description TEXT NULL,
                status TEXT NOT NULL CHECK (status IN ('Backlog', 'Todo', 'InProgress', 'Testing', 'Review', 'Reopen', 'Completed', 'Archived')),
                previous_status TEXT NULL CHECK (previous_status IS NULL OR previous_status IN ('Backlog', 'Todo', 'InProgress', 'Testing', 'Review', 'Reopen', 'Completed')),
                priority TEXT NOT NULL CHECK (priority IN ('Low', 'Medium', 'High')),
                task_type TEXT NOT NULL CHECK (task_type IN ('Feature', 'BugFix', 'Refactor')),
                source_type TEXT NOT NULL CHECK (source_type IN ('Internal', 'GitHub', 'GitLab')),
                source_url TEXT NULL,
                progress_value INTEGER NOT NULL DEFAULT 0 CHECK (progress_value >= 0 AND progress_value <= 100),
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                archived_at INTEGER NULL
            );
            CREATE TABLE task_user_assignments (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL REFERENCES tasks(task_code),
                user_code TEXT NOT NULL,
                assigned_at INTEGER NOT NULL,
                UNIQUE (task_code, user_code)
            );
            CREATE TABLE task_agent_links (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL REFERENCES tasks(task_code),
                agent_code TEXT NOT NULL,
                linked_at INTEGER NOT NULL,
                UNIQUE (task_code, agent_code)
            );
            CREATE TABLE task_app_links (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL REFERENCES tasks(task_code),
                app_code TEXT NOT NULL,
                linked_at INTEGER NOT NULL,
                UNIQUE (task_code, app_code)
            );
            ",
        )
        .expect("legacy schema should be creatable");
    connection
        .execute(
            "INSERT INTO tasks (id, task_code, workspace_code, parent_task_code, title, description, status, previous_status, priority, task_type, source_type, source_url, progress_value, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                734_500_000_000_000_001_i64,
                "TSK-LEGACY-0001",
                "TRON_CORE",
                Option::<String>::None,
                "Legacy task",
                "Legacy task description",
                "Backlog",
                Option::<String>::None,
                "Medium",
                "BugFix",
                "Internal",
                "",
                0_i64,
                1_779_830_400_000_i64,
                1_779_830_400_000_i64,
                Option::<i64>::None
            ],
        )
        .expect("legacy row should be insertable");
}
