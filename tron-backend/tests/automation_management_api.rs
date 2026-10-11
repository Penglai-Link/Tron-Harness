mod common;

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::{Value, json};
use tokio::{
    net::TcpListener,
    task::JoinHandle,
    time::{Instant, sleep},
};
use tron_backend::{
    adapters::outbound::persistence::{SqliteAutomationRepository, SqliteChatRepository},
    app::AppState,
    application::{
        agent::{BusinessToolCatalog, tool_record_to_effective_tool},
        automation::{
            AutomationRepository, AutomationService, CreateAutomationCommand,
            UpdateAutomationStatusCommand,
        },
    },
    config::AppConfig,
    domain::{
        agent::{
            AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
            AgentRunContext, AgentRunLimits, EffectiveSkill, EffectiveTool, EffectiveVariable,
            ToolRequest,
        },
        automation::{
            Automation, AutomationExecutionDraft, AutomationExecutionListItem,
            AutomationExecutionListQuery, AutomationExecutionStatus, AutomationListQuery,
            AutomationStatus, AutomationTriggerSource,
        },
    },
    shared::id::SnowflakeId,
};

use common::{
    app_with_config, authenticated_bearer_token, authenticated_request, prepared_config,
    request_with_headers, response_json,
};

struct RecordingAgentModelServer {
    base_url: String,
    request_count: Arc<AtomicUsize>,
    _task: JoinHandle<()>,
}

async fn recording_agent_model_server() -> RecordingAgentModelServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("agent mock server should bind");
    let address = listener
        .local_addr()
        .expect("agent mock server should expose address");
    let request_count = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route("/v1/chat/completions", post(agent_chat_completion))
        .with_state(request_count.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("agent mock server should stay available");
    });
    RecordingAgentModelServer {
        base_url: format!("http://{address}/v1"),
        request_count,
        _task: task,
    }
}

async fn agent_chat_completion(
    axum::extract::State(request_count): axum::extract::State<Arc<AtomicUsize>>,
    Json(payload): Json<Value>,
) -> Response {
    request_count.fetch_add(1, Ordering::SeqCst);
    let prompt = payload["messages"]
        .as_array()
        .and_then(|messages| {
            messages
                .iter()
                .rev()
                .find_map(|message| message["content"].as_str())
        })
        .unwrap_or_default();
    Json(json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": format!("automation reply: {prompt}")
            }
        }]
    }))
    .into_response()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn automation_list_reports_execution_success_rate() {
    let database_path = database_path("list-success-rate");
    let config = prepared_config(config_for_path(database_path));
    let database_url = config.database_url.clone();
    let mut repository =
        SqliteAutomationRepository::connect(&database_url).expect("automation repo should connect");
    let now = "2026-06-24T12:00:00Z".to_string();
    let automation = Automation {
        id: SnowflakeId::new(734_600_000_000_300_001),
        automation_code: "AUT-SUCCESS-RATE".into(),
        workspace_code: "TRON_CORE".into(),
        name: "Success rate".into(),
        description: None,
        agent_code: "main-agent".into(),
        model_code: None,
        prompt_text: "Track execution stats.".into(),
        cron_expression: "0 8 * * *".into(),
        scheduler_expression: "0 0 8 * * * *".into(),
        timezone: "UTC".into(),
        status: AutomationStatus::Paused,
        previous_status: None,
        last_execution_status: None,
        last_execution_code: None,
        last_execution_summary: None,
        last_started_at: None,
        last_finished_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        archived_at: None,
    };
    repository
        .create_automation(&automation)
        .expect("automation should be inserted");
    for status in [
        AutomationExecutionStatus::Success,
        AutomationExecutionStatus::Success,
        AutomationExecutionStatus::Failed,
        AutomationExecutionStatus::Running,
    ] {
        repository
            .record_execution(
                &automation,
                &AutomationExecutionDraft {
                    automation_code: automation.automation_code.clone(),
                    workspace_code: automation.workspace_code.clone(),
                    session_code: None,
                    trigger_source: AutomationTriggerSource::Manual,
                    status,
                    summary: None,
                    scheduled_at: None,
                    started_at: now.clone(),
                    finished_at: Some(now.clone()),
                    created_at: now.clone(),
                },
            )
            .expect("execution should record");
    }

    let router = app_with_config(config);
    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/automations?workspaceCode=TRON_CORE",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let item = body["data"]["records"]
        .as_array()
        .expect("records should be array")
        .iter()
        .find(|item| item["automationCode"] == "AUT-SUCCESS-RATE")
        .expect("automation should be listed");
    assert_eq!(item["executionSuccessCount"], 2);
    assert_eq!(item["executionTotalCount"], 3);
    let rate = item["executionSuccessRate"]
        .as_f64()
        .expect("success rate should be numeric");
    assert!((rate - (2.0 / 3.0)).abs() < f64::EPSILON);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn automation_list_stays_workspace_scoped_and_hides_archived_by_default() {
    let database_path = database_path("list-filters");
    let config = prepared_config(config_for_path(database_path));
    let state = AppState::new(config.clone()).expect("app state should build");
    let service = state.automations.clone();

    let active = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Daily summary".into(),
            description: Some("workspace scoped".into()),
            agent_code: Some("main-agent".into()),
            model_code: None,
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
            agent_code: Some("main-agent".into()),
            model_code: None,
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
            agent_code: Some("main-agent".into()),
            model_code: None,
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
            agent_code: Some("main-agent".into()),
            model_code: None,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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
            "agentCode": "main-agent",
            "modelCode": "MODEL-1",
            "prompt": "Write a detailed release digest for the workspace with context, risks, and next actions.",
            "cronExpression": "*/15 * * * *",
        })),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = response_json(created).await;
    assert_eq!(body["data"]["status"], "Paused");
    assert_eq!(body["data"]["agentCode"], "main-agent");
    assert_eq!(body["data"]["modelCode"], "MODEL-1");
    assert_eq!(body["data"]["schedulerExpression"], "0 */15 * * * *");
    assert_eq!(body["data"]["workspaceCode"], "TRON_CORE");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_execution_persists_history_and_rolls_up_latest_result() {
    let server = recording_agent_model_server().await;
    let database_path = database_path("manual-execution");
    let config = prepared_config(config_for_path(database_path));
    seed_default_text_model(config.clone(), &server.base_url).await;
    let state = AppState::new(config.clone()).expect("app state should build");
    let router = app_with_config(config);
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
    assert_eq!(executed_json["data"]["lastExecutionStatus"], "Running");

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
    let execution_code = execution_json["data"]["records"][0]["executionCode"]
        .as_str()
        .expect("execution code should be returned")
        .to_string();
    assert_eq!(
        execution_json["data"]["records"][0]["triggerSource"],
        "Manual"
    );
    let record = wait_for_final_execution_status(
        &state.automations,
        &automation_code,
        &execution_code,
        AutomationExecutionStatus::Success,
        Duration::from_secs(3),
    )
    .await;
    assert_eq!(
        record.summary.as_deref(),
        Some("automation reply: Capture a manual execution record.")
    );
    wait_for_model_requests(&server, 1, Duration::from_secs(3)).await;
    let groups = state
        .chats
        .list_session_groups(tron_backend::application::chat::ChatSessionQuery {
            workspace_code: Some("TRON_CORE".into()),
        })
        .expect("chat groups should list");
    assert!(groups.iter().any(|group| {
        group.source_type == tron_backend::domain::chat::ChatSessionSourceType::Automation
            && group.source_code.as_deref() == Some(automation_code.as_str())
    }));

    let deleted = authenticated_request(
        &router,
        Method::DELETE,
        &format!("/api/v1/automations/{automation_code}/executions/{execution_code}"),
        None,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::OK);
    let deleted_json = response_json(deleted).await;
    assert_eq!(deleted_json["data"]["executionCode"], execution_code);

    let executions_after_delete = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/automations/{automation_code}/executions"),
        None,
    )
    .await;
    assert_eq!(executions_after_delete.status(), StatusCode::OK);
    let execution_after_delete_json = response_json(executions_after_delete).await;
    assert_eq!(execution_after_delete_json["data"]["total"], 0);

    let detail_after_delete = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/automations/{automation_code}"),
        None,
    )
    .await;
    assert_eq!(detail_after_delete.status(), StatusCode::OK);
    let detail_after_delete_json = response_json(detail_after_delete).await;
    assert!(detail_after_delete_json["data"]["lastExecutionCode"].is_null());
}

#[test]
fn automation_execution_summary_persists_the_runner_final_reply() {
    let database_path = database_path("execution-summary-session-reply");
    let config = prepared_config(config_for_path(database_path));
    let database_url = config.database_url.clone();
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat schema should initialize");
    drop(chat_repository);

    let mut repository =
        SqliteAutomationRepository::connect(&database_url).expect("automation repo should connect");
    let now = "2026-06-24T12:00:00Z".to_string();
    let automation = Automation {
        id: SnowflakeId::new(734_600_000_000_200_001),
        automation_code: "AUT-SUMMARY-FINAL-REPLY".into(),
        workspace_code: "TRON_CORE".into(),
        name: "Final reply summary".into(),
        description: None,
        agent_code: "main-agent".into(),
        model_code: None,
        prompt_text: "Return the final answer.".into(),
        cron_expression: "0 8 * * *".into(),
        scheduler_expression: "0 0 8 * * * *".into(),
        timezone: "UTC".into(),
        status: AutomationStatus::Paused,
        previous_status: None,
        last_execution_status: None,
        last_execution_code: None,
        last_execution_summary: None,
        last_started_at: None,
        last_finished_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        archived_at: None,
    };
    repository
        .create_automation(&automation)
        .expect("automation should be inserted");

    let session_code = "SESSION-SUMMARY-FINAL-REPLY";
    let fallback_summary = format!(
        "Agent main-agent completed automation execution AEX-placeholder in session {session_code}."
    );
    let record = repository
        .record_execution(
            &automation,
            &AutomationExecutionDraft {
                automation_code: automation.automation_code.clone(),
                workspace_code: automation.workspace_code.clone(),
                session_code: Some(session_code.into()),
                trigger_source: AutomationTriggerSource::Manual,
                status: AutomationExecutionStatus::Running,
                summary: Some(fallback_summary.clone()),
                scheduled_at: None,
                started_at: now.clone(),
                finished_at: None,
                created_at: now.clone(),
            },
        )
        .expect("execution should record")
        .expect("execution should insert");

    repository
        .complete_execution_record(
            &automation.automation_code,
            &record.execution_code,
            AutomationExecutionStatus::Success,
            Some("Chat session final reply.".into()),
            Some(session_code.into()),
            "2026-06-24T12:01:00Z",
        )
        .expect("execution should complete");

    let records = repository
        .list_execution_records(&automation.automation_code)
        .expect("execution records should list");
    assert_eq!(
        records[0].summary.as_deref(),
        Some("Chat session final reply.")
    );
    let automation = repository
        .list_automations()
        .expect("automations should list")
        .into_iter()
        .find(|item| item.automation_code == "AUT-SUMMARY-FINAL-REPLY")
        .expect("automation should exist");
    assert_eq!(
        automation.last_execution_summary.as_deref(),
        Some("Chat session final reply.")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn internal_agent_automation_tools_are_cataloged_and_executable() {
    let server = recording_agent_model_server().await;
    let database_path = database_path("internal-tools");
    let config = prepared_config(config_for_path(database_path));
    let text_model_code = seed_default_text_model(config.clone(), &server.base_url).await;
    let state = AppState::new(config).expect("app state should build");
    let automation_tools = BusinessToolCatalog::list_tools(Some("automation"), None, "en-US");
    let automation_tool_codes = automation_tools
        .iter()
        .map(|tool| tool.tool_code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        automation_tool_codes,
        vec![
            "automation.page",
            "automation.get",
            "automation.create",
            "automation.update",
            "automation.delete",
            "automation.restore",
            "automation.execute",
        ]
    );
    let context = automation_tool_context(
        automation_tools
            .into_iter()
            .map(tool_record_to_effective_tool)
            .collect(),
    );

    let created = execute_tool_json(
        &state,
        &context,
        "automation_create",
        json!({
            "workspaceCode": "TRON_CORE",
            "name": "Agent-created automation",
            "description": "created through internal tool",
            "agentCode": "main-agent",
            "prompt": "Summarize the workspace.",
            "modelCode": text_model_code.clone(),
            "cronExpression": "0 8 * * *",
            "status": "paused",
        }),
    );
    let automation_code = created["automationCode"]
        .as_str()
        .expect("created automation should expose automationCode")
        .to_string();

    let fetched = execute_tool_json(
        &state,
        &context,
        "automation_get",
        json!({ "workspaceCode": "TRON_CORE", "automationCode": automation_code.clone() }),
    );
    assert_eq!(fetched["status"], json!("Paused"));
    assert_eq!(fetched["agentCode"], json!("main-agent"));
    assert_eq!(fetched["modelCode"], json!(text_model_code.clone()));

    let updated = execute_tool_json(
        &state,
        &context,
        "automation_update",
        json!({
            "workspaceCode": "TRON_CORE",
            "automationCode": automation_code.clone(),
            "name": "Agent-updated automation",
            "agentCode": "main-agent",
            "modelCode": text_model_code.clone(),
            "status": "paused",
        }),
    );
    assert_eq!(updated["name"], json!("Agent-updated automation"));

    let listed = execute_tool_json(
        &state,
        &context,
        "automation_page",
        json!({ "workspaceCode": "TRON_CORE", "search": "Agent-updated" }),
    );
    assert_eq!(listed["total"], json!(1));

    let executed = execute_tool_json(
        &state,
        &context,
        "automation_execute",
        json!({ "workspaceCode": "TRON_CORE", "automationCode": automation_code.clone() }),
    );
    assert_eq!(executed["lastExecutionStatus"], json!("Running"));
    let execution_code = executed["lastExecutionCode"]
        .as_str()
        .expect("execute should return lastExecutionCode")
        .to_string();
    wait_for_final_execution_status(
        &state.automations,
        &automation_code,
        &execution_code,
        AutomationExecutionStatus::Success,
        Duration::from_secs(3),
    )
    .await;

    let deleted = execute_tool_json(
        &state,
        &context,
        "automation_delete",
        json!({ "workspaceCode": "TRON_CORE", "automationCode": automation_code.clone() }),
    );
    assert_eq!(deleted["status"], json!("Archived"));

    let restored = execute_tool_json(
        &state,
        &context,
        "automation_restore",
        json!({ "workspaceCode": "TRON_CORE", "automationCode": automation_code }),
    );
    assert_eq!(restored["status"], json!("Paused"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_automation_scheduler_persists_real_tick() {
    let server = recording_agent_model_server().await;
    let database_path = database_path("scheduler-real-tick");
    let config = prepared_config(config_for_path(database_path));
    seed_default_text_model(config.clone(), &server.base_url).await;
    let state = AppState::new(config).expect("app state should build");
    let service = state.automations.clone();
    let automation = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Real scheduler tick".into(),
            description: Some("active every second".into()),
            agent_code: Some("main-agent".into()),
            model_code: None,
            prompt: "Record a scheduler tick.".into(),
            cron_expression: "*/1 * * * * *".into(),
            status: Some(AutomationStatus::Active),
        })
        .expect("active automation should be created");

    let records = wait_for_scheduler_records(
        &service,
        &automation.automation_code,
        1,
        Duration::from_secs(3),
    )
    .await;

    assert_eq!(
        records[0].trigger_source,
        AutomationTriggerSource::Scheduler
    );
    let record = wait_for_final_execution_status(
        &service,
        &automation.automation_code,
        &records[0].execution_code,
        AutomationExecutionStatus::Success,
        Duration::from_secs(3),
    )
    .await;
    assert_eq!(record.status, AutomationExecutionStatus::Success);
    assert!(
        records[0].scheduled_at.is_some(),
        "scheduler execution should persist its planned tick"
    );
    wait_for_model_requests(&server, 1, Duration::from_secs(3)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduler_ticks_are_claimed_once_across_multiple_instances() {
    let server = recording_agent_model_server().await;
    let database_path = database_path("scheduler-multi-instance");
    let config = prepared_config(config_for_path(database_path));
    seed_default_text_model(config.clone(), &server.base_url).await;
    let first_state = AppState::new(config.clone()).expect("first app state should build");
    let service = first_state.automations.clone();
    let automation = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Multi instance scheduler".into(),
            description: Some("active every second".into()),
            agent_code: Some("main-agent".into()),
            model_code: None,
            prompt: "Record only one execution per scheduler tick.".into(),
            cron_expression: "*/1 * * * * *".into(),
            status: Some(AutomationStatus::Active),
        })
        .expect("active automation should be created");
    let second_state = AppState::new(config).expect("second app state should build");

    let _records = wait_for_scheduler_records(
        &service,
        &automation.automation_code,
        2,
        Duration::from_secs(4),
    )
    .await;
    // Freeze the schedule before counting. Otherwise a new second can add an
    // execution while the assertion is waiting for earlier model requests.
    service
        .update_status(
            &automation.automation_code,
            UpdateAutomationStatusCommand {
                status: AutomationStatus::Paused,
            },
        )
        .unwrap();
    second_state
        .automations
        .update_status(
            &automation.automation_code,
            UpdateAutomationStatusCommand {
                status: AutomationStatus::Paused,
            },
        )
        .unwrap();
    let records = service
        .list_execution_records(
            &automation.automation_code,
            AutomationExecutionListQuery {
                current: Some(1),
                size: Some(100),
            },
        )
        .unwrap()
        .records;
    let scheduler_ticks = records
        .iter()
        .filter_map(|record| record.scheduled_at.clone())
        .collect::<Vec<_>>();
    let unique_ticks = scheduler_ticks.iter().collect::<HashSet<_>>();
    assert_eq!(
        scheduler_ticks.len(),
        unique_ticks.len(),
        "each tick must be claimed once across both instances"
    );
    for record in &records {
        wait_for_final_execution_status(
            &service,
            &automation.automation_code,
            &record.execution_code,
            AutomationExecutionStatus::Success,
            Duration::from_secs(30),
        )
        .await;
    }
    assert_eq!(
        server.request_count.load(Ordering::SeqCst),
        unique_ticks.len(),
        "each claimed tick must invoke the model exactly once"
    );
    drop(second_state);
    drop(first_state);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lifecycle_and_bootstrap_scheduler_only_track_active_automations() {
    let database_path = database_path("scheduler-bootstrap");
    let config = prepared_config(config_for_path(database_path));
    let state = AppState::new(config.clone()).expect("app state should build");
    let service = state.automations.clone();

    let active = service
        .create_automation(CreateAutomationCommand {
            workspace_code: "TRON_CORE".into(),
            name: "Active cadence".into(),
            description: Some("active".into()),
            agent_code: Some("main-agent".into()),
            model_code: None,
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
            agent_code: Some("main-agent".into()),
            model_code: None,
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

    let rebooted = AppState::new(config).expect("app state should reboot");
    assert!(rebooted.automations.scheduler_snapshot().is_empty());
}

async fn wait_for_scheduler_records(
    service: &AutomationService,
    automation_code: &str,
    minimum: usize,
    timeout: Duration,
) -> Vec<AutomationExecutionListItem> {
    let deadline = Instant::now() + timeout;
    loop {
        let records = service
            .list_execution_records(
                automation_code,
                AutomationExecutionListQuery {
                    current: Some(1),
                    size: Some(20),
                },
            )
            .expect("execution records should list")
            .records;
        let scheduler_records = records
            .iter()
            .filter(|record| record.trigger_source == AutomationTriggerSource::Scheduler)
            .count();
        if scheduler_records >= minimum {
            return records;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {minimum} scheduler records; saw {scheduler_records}"
        );
        sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_for_final_execution_status(
    service: &AutomationService,
    automation_code: &str,
    execution_code: &str,
    expected_status: AutomationExecutionStatus,
    timeout: Duration,
) -> AutomationExecutionListItem {
    let deadline = Instant::now() + timeout;
    loop {
        let records = service
            .list_execution_records(
                automation_code,
                AutomationExecutionListQuery {
                    current: Some(1),
                    size: Some(20),
                },
            )
            .expect("execution records should list")
            .records;
        let latest = records
            .into_iter()
            .find(|record| record.execution_code == execution_code);
        if let Some(record) = &latest {
            if record.status == expected_status {
                return record.clone();
            }
            assert!(
                record.status != AutomationExecutionStatus::Failed,
                "execution {execution_code} failed: {:?}",
                record.summary
            );
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for execution {execution_code} to reach {expected_status:?}; latest: {latest:?}"
        );
        sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_for_model_requests(
    server: &RecordingAgentModelServer,
    minimum: usize,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    loop {
        let count = server.request_count.load(Ordering::SeqCst);
        if count >= minimum {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {minimum} model requests; saw {count}"
        );
        sleep(Duration::from_millis(100)).await;
    }
}

async fn seed_default_text_model(config: AppConfig, base_url: &str) -> String {
    let router = app_with_config(config);
    let token = authenticated_bearer_token(&router).await;
    let authorization = format!("Bearer {token}");
    let response = request_with_headers(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Automation Default",
            "vendor": "OpenAI",
            "protocol": "OPENAI_COMPATIBLE",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": base_url,
            "apiPath": "/chat/completions",
            "apiKeyRef": "sk-automation-api",
            "modelName": "gpt-automation",
            "enabled": true,
            "description": "Automation model",
            "thinkingEnabled": false,
            "structuredOutputEnabled": false,
            "temperature": 0.2,
            "maxTokens": 1024,
            "topP": 1.0,
            "maxRetries": 0
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await["data"]["modelCode"]
        .as_str()
        .expect("seeded model should expose modelCode")
        .to_string()
}

fn execute_tool_json(
    state: &AppState,
    context: &AgentRunContext,
    tool_name: &str,
    input: Value,
) -> Value {
    let result = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: tool_name.to_string(),
                input,
                invocation_id: None,
            },
            context,
        )
        .expect("tool should execute");
    serde_json::from_str(&result.observation.output).expect("tool output should be JSON")
}

fn automation_tool_context(tools: Vec<EffectiveTool>) -> AgentRunContext {
    AgentRunContext {
        collaboration_owner: None,
        agent_code: None,
        conversation_code: Some("CONV-AUTOMATION-TOOLS".into()),
        run_code: "RUN-AUTOMATION-TOOLS".into(),
        workspace_code: "TRON_CORE".into(),
        user_code: Some("USR-1".into()),
        locale: "en-US".into(),
        request_id: Some("REQ-AUTOMATION-TOOLS".into()),
        stream_key: Some("STREAM-AUTOMATION-TOOLS".into()),
        agent_kind: AgentKind::React,
        system_agent_session_id: None,
        model: AgentModelConfig {
            model_code: "MODEL-1".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "gpt-test".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: Some(0.2),
            max_tokens: Some(128),
            context_window_tokens: Some(8_192),
            top_p: Some(1.0),
            max_retries: Some(0),
        },
        variables: vec![EffectiveVariable {
            key: "REGION".into(),
            value: "us".into(),
            source: "LOCAL".into(),
            sensitive: false,
        }],
        skills: vec![EffectiveSkill {
            skill_name: "automation-manager".into(),
            description: "Manage automations".into(),
            instruction: None,
            tool_names: tools.iter().map(|tool| tool.tool_name.clone()).collect(),
        }],
        tools,
        effective_tools: Vec::new(),
        tool_exposure: Default::default(),
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits {
            max_steps: 10,
            max_tool_calls: 10,
            max_parallel_tool_calls: 4,
        },
        messages: vec![AgentMessage::user("manage automations")],
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-06-22T00:00:00Z".into(),
        auto_model_route: None,
    }
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
        .join(format!(
            "{test_name}-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
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
