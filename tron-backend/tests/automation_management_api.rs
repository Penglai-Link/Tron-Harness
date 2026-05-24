mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use tron_backend::{
    app::AppState,
    application::automation::{CreateAutomationCommand, UpdateAutomationStatusCommand},
    config::AppConfig,
    domain::automation::{AutomationListQuery, AutomationStatus},
};

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn automation_options_and_empty_workspace_list() {
    let router = automation_app("options-list");

    let options_response =
        authenticated_request(&router, Method::GET, "/api/v1/automations/options", None).await;
    assert_eq!(options_response.status(), StatusCode::OK);
    let options_json = response_json(options_response).await;
    assert_eq!(
        options_json["data"]["statuses"],
        json!(["Active", "Paused", "Archived"])
    );
    assert_eq!(
        options_json["data"]["executionStatuses"],
        json!([
            "Pending", "Running", "Success", "Failed", "Canceled", "Skipped"
        ])
    );
    assert_eq!(options_json["data"]["defaultStatus"], "Paused");

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/automations?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    assert_eq!(response_json(list_response).await["data"]["total"], 0);
}

#[tokio::test]
async fn automation_list_stays_workspace_scoped_and_hides_archived_by_default() {
    let database_path = database_path("list-filters");
    let config = config_for_path(database_path);
    let state = AppState::new(config.clone());
    let service = state.automations.clone();

    let active = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Daily summary".into(),
            description: Some("workspace scoped".into()),
            prompt: "Summarize the active workspace state.".into(),
            cron_expression: "0 9 * * *".into(),
            status: Some(AutomationStatus::Active),
        })
        .expect("active automation should be creatable");
    let paused = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Paused digest".into(),
            description: Some("paused record".into()),
            prompt: "Draft the paused digest.".into(),
            cron_expression: "0 10 * * *".into(),
            status: Some(AutomationStatus::Paused),
        })
        .expect("paused automation should be creatable");
    let archived = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Archive me".into(),
            description: Some("archived record".into()),
            prompt: "Archive this automation.".into(),
            cron_expression: "0 11 * * *".into(),
            status: Some(AutomationStatus::Paused),
        })
        .expect("archivable automation should be creatable");
    service
        .archive_automation(&archived.automation_code)
        .expect("archive should succeed");
    service
        .create_automation(CreateAutomationCommand {
            workspace_code: "PLATFORM".into(),
            name: "Platform digest".into(),
            description: Some("other workspace".into()),
            prompt: "Summarize another workspace.".into(),
            cron_expression: "0 12 * * *".into(),
            status: Some(AutomationStatus::Active),
        })
        .expect("other-workspace automation should be creatable");

    let router = app_with_config(config);
    let default_list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/automations?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(default_list.status(), StatusCode::OK);
    let default_json = response_json(default_list).await;
    assert_eq!(default_json["data"]["total"], 2);
    assert!(
        default_json["data"]["records"]
            .as_array()
            .expect("records should be array")
            .iter()
            .all(|item| item["workspaceCode"] == "TRON_CORE" && item["status"] != "Archived")
    );

    let archived_list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/automations?workspaceCode=TRON_CORE&includeArchived=true&status=Archived",
        None,
    )
    .await;
    assert_eq!(archived_list.status(), StatusCode::OK);
    let archived_json = response_json(archived_list).await;
    assert_eq!(archived_json["data"]["total"], 1);
    assert_eq!(
        archived_json["data"]["records"][0]["automationCode"],
        archived.automation_code
    );

    let search_list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/automations?workspaceCode=TRON_CORE&search=paused",
        None,
    )
    .await;
    assert_eq!(search_list.status(), StatusCode::OK);
    assert_eq!(
        response_json(search_list).await["data"]["records"][0]["automationCode"],
        paused.automation_code
    );
    assert_ne!(active.automation_code, paused.automation_code);
}

#[tokio::test]
async fn create_automation_validates_workspace_prompt_and_cron_and_defaults_to_paused() {
    let router = automation_app("create-validation");

    let invalid_cron = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/automations",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "name": "Broken schedule",
            "description": "bad cron",
            "prompt": "Prompt body",
            "cronExpression": "* * *",
        })),
    )
    .await;
    assert_eq!(invalid_cron.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(invalid_cron).await["code"],
        "AUTOMATION_CRON_INVALID"
    );

    let invalid_prompt = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/automations",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "name": "Blank prompt",
            "description": "no body",
            "prompt": "   ",
            "cronExpression": "0 */15 * * *",
        })),
    )
    .await;
    assert_eq!(invalid_prompt.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(invalid_prompt).await["code"],
        "AUTOMATION_PROMPT_REQUIRED"
    );

    let created = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/automations",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "name": "Ship automation loop",
            "description": "Long prompt workflow",
            "prompt": "Write a detailed release digest for the workspace with context, risks, and next actions.",
            "cronExpression": "*/15 * * * *",
        })),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = response_json(created).await;
    assert_eq!(body["data"]["status"], "Paused");
    assert_eq!(body["data"]["schedulerExpression"], "0 */15 * * * *");
    assert_eq!(body["data"]["workspaceCode"], "TRON_CORE");
}

#[tokio::test]
async fn manual_execution_persists_history_and_rolls_up_latest_result() {
    let router = automation_app("manual-execution");
    let created = create_automation(
        &router,
        "TRON_CORE",
        "Manual execution",
        "Capture a manual execution record.",
        "0 8 * * *",
        Some("Paused"),
    )
    .await;
    let automation_code = created["data"]["automationCode"]
        .as_str()
        .expect("create should return automationCode")
        .to_owned();

    let executed = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/automations/{automation_code}/execute"),
        None,
    )
    .await;
    assert_eq!(executed.status(), StatusCode::OK);
    let executed_json = response_json(executed).await;
    assert_eq!(executed_json["data"]["lastExecutionStatus"], "Success");
    assert_eq!(
        executed_json["data"]["lastExecutionSummary"],
        "Prompt dispatch recorded for the automation."
    );

    let executions = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/automations/{automation_code}/executions"),
        None,
    )
    .await;
    assert_eq!(executions.status(), StatusCode::OK);
    let execution_json = response_json(executions).await;
    assert_eq!(execution_json["data"]["total"], 1);
    assert_eq!(
        execution_json["data"]["records"][0]["triggerSource"],
        "Manual"
    );
    assert_eq!(execution_json["data"]["records"][0]["status"], "Success");
}

#[tokio::test]
async fn lifecycle_and_bootstrap_scheduler_only_track_active_automations() {
    let database_path = database_path("scheduler-bootstrap");
    let config = config_for_path(database_path);
    let state = AppState::new(config.clone());
    let service = state.automations.clone();

    let active = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Active cadence".into(),
            description: Some("active".into()),
            prompt: "Keep the active schedule.".into(),
            cron_expression: "0 1 * * *".into(),
            status: Some(AutomationStatus::Active),
        })
        .expect("active automation should be created");
    let paused = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Paused cadence".into(),
            description: Some("paused".into()),
            prompt: "Keep the paused schedule.".into(),
            cron_expression: "0 2 * * *".into(),
            status: Some(AutomationStatus::Paused),
        })
        .expect("paused automation should be created");

    assert_eq!(
        service.scheduler_snapshot(),
        vec![active.automation_code.clone()]
    );

    service
        .update_status(
            &active.automation_code,
            UpdateAutomationStatusCommand {
                status: AutomationStatus::Paused,
            },
        )
        .expect("pause should remove from scheduler");
    assert!(service.scheduler_snapshot().is_empty());

    service
        .update_status(
            &paused.automation_code,
            UpdateAutomationStatusCommand {
                status: AutomationStatus::Active,
            },
        )
        .expect("activate should add to scheduler");
    assert_eq!(
        service.scheduler_snapshot(),
        vec![paused.automation_code.clone()]
    );

    service
        .archive_automation(&paused.automation_code)
        .expect("archive should remove from scheduler");
    assert!(service.scheduler_snapshot().is_empty());

    drop(state);

    let rebooted = AppState::new(config);
    assert!(rebooted.automations.scheduler_snapshot().is_empty());
}

fn automation_app(test_name: &str) -> axum::Router {
    app_with_config(config_for_path(database_path(test_name)))
}

fn config_for_path(database_path: PathBuf) -> AppConfig {
    if database_path.exists() {
        std::fs::remove_file(&database_path).expect("test database should be resettable");
    }
    let mut config = AppConfig::default();
    config.database_url = database_path.to_string_lossy().into_owned();
    config
}

fn database_path(test_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("automation-management")
        .join(format!("{test_name}.sqlite"))
}

async fn create_automation(
    router: &axum::Router,
    workspace_code: &str,
    name: &str,
    prompt: &str,
    cron_expression: &str,
    status: Option<&str>,
) -> Value {
    let response = authenticated_request(
        router,
        Method::POST,
        "/api/v1/automations",
        Some(json!({
            "workspaceCode": workspace_code,
            "name": name,
            "description": "integration helper",
            "prompt": prompt,
            "cronExpression": cron_expression,
            "status": status,
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}

#[test]
fn automation_list_query_type_is_public() {
    let _ = AutomationListQuery {
        workspace_code: Some("TRON_CORE".into()),
        current: Some(1),
        size: Some(20),
        search: None,
        status: None,
        include_archived: Some(false),
    };
}
