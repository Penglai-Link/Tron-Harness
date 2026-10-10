mod common;

use axum::http::{Method, StatusCode};
use common::{authenticated_bearer_token, request_with_headers, response_json};
use serde_json::json;
use tron_agent_harness::ToolPermissionSource;

const TOOL: &str = "framework.fixture.query";
const PLATFORM: &str = "/api/v1/agents/registry/internal/main-agent";
const WORKSPACE: &str = "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent";

#[tokio::test]
async fn discovered_tool_selection_round_trips_through_platform_and_workspace_policies() {
    let app = common::app_with_state_setup(common::prepared_config(Default::default()), |state| {
        state.knowledge_bases.stop_ingestion_workers();
        let descriptor = eliteforge_agent_core::ToolDescriptor::new(
            TOOL,
            "fixture_query",
            "Discovered native tool",
            json!({"type":"object"}),
        );
        state.agent_management.setting(&descriptor).unwrap();
    });
    let auth = format!("Bearer {}", authenticated_bearer_token(&app).await);
    let call = |method, path: String, payload| {
        let app = app.clone();
        let auth = auth.clone();
        async move {
            let response =
                request_with_headers(&app, method, &path, payload, Some("en-US"), Some(&auth))
                    .await;
            let status = response.status();
            let body = response_json(response).await;
            assert_eq!(status, StatusCode::OK, "{path}: {body}");
            body["data"].clone()
        }
    };
    let tools = call(Method::GET, "/api/v1/tools".into(), None).await;
    assert!(
        tools
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["toolCode"] == TOOL)
    );

    // All-tools policy must expose the same catalog that the selection dialog sees.
    let platform = call(Method::GET, "/api/v1/agents/registry/internal".into(), None).await;
    assert!(
        platform.as_array().unwrap()[0]["selectedToolCodes"]
            .as_array()
            .unwrap()
            .contains(&json!(TOOL))
    );

    // The UI saves assignments before switching the policy from all to selected.
    call(
        Method::PUT,
        format!("{PLATFORM}/tool-assignments"),
        Some(json!({"toolCodes":[TOOL]})),
    )
    .await;
    call(
        Method::PATCH,
        format!("{PLATFORM}/policy"),
        Some(json!({"toolAssignmentMode":"selected_tools"})),
    )
    .await;
    let inherited = call(
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/internal".into(),
        None,
    )
    .await;
    let agent = inherited
        .as_array()
        .unwrap()
        .iter()
        .find(|agent| agent["agentCode"] == "main-agent")
        .unwrap();
    assert_eq!(agent["selectedToolCodes"], json!([TOOL]));
    assert_eq!(agent["effectiveToolCount"], 1);

    call(
        Method::PUT,
        format!("{WORKSPACE}/tool-assignments"),
        Some(json!({"toolCodes":[TOOL,"app.page",TOOL]})),
    )
    .await;
    let workspace = call(
        Method::GET,
        "/api/v1/agents/workspaces/TRON_CORE/internal".into(),
        None,
    )
    .await;
    let agent = workspace
        .as_array()
        .unwrap()
        .iter()
        .find(|agent| agent["agentCode"] == "main-agent")
        .unwrap();
    assert_eq!(agent["selectedToolCodes"], json!(["app.page", TOOL]));

    // Unknown tools still fail, preserve the previous selection, and identify the bad code.
    for (prefix, expected) in [
        (PLATFORM, json!([TOOL])),
        (WORKSPACE, json!(["app.page", TOOL])),
    ] {
        for locale in ["zh-CN", "en-US"] {
            let response = request_with_headers(
                &app,
                Method::PUT,
                &format!("{prefix}/tool-assignments"),
                Some(json!({"toolCodes":["app.page","missing.fixture"]})),
                Some(locale),
                Some(&auth),
            )
            .await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            let body = response_json(response).await;
            assert!(
                body["message"]
                    .as_str()
                    .unwrap()
                    .contains("missing.fixture"),
                "{body}"
            );
        }
        let list_path = prefix.rsplit_once('/').unwrap().0;
        let records = call(Method::GET, list_path.into(), None).await;
        let agent = records
            .as_array()
            .unwrap()
            .iter()
            .find(|agent| agent["agentCode"] == "main-agent")
            .unwrap();
        assert_eq!(agent["selectedToolCodes"], expected);
    }
}

#[tokio::test]
async fn discovered_tool_is_in_effective_definitions() {
    common::app_with_state_setup(common::prepared_config(Default::default()), |state| {
        state.knowledge_bases.stop_ingestion_workers();
        state
            .agent_management
            .setting(&eliteforge_agent_core::ToolDescriptor::new(
                TOOL,
                "fixture_query",
                "Discovered native tool",
                json!({"type":"object"}),
            ))
            .unwrap();
        state
            .agent_management
            .replace_platform_internal_agent_tool_assignments("main-agent", vec![TOOL.into()])
            .unwrap();
        state
            .agent_management
            .update_platform_internal_agent_policy(
                "main-agent",
                tron_backend::application::agent::UpdateWorkspaceInternalAgentPolicyCommand {
                    tool_assignment_mode: Some(
                        tron_backend::domain::agent::ToolAssignmentMode::SelectedTools,
                    ),
                    ..Default::default()
                },
            )
            .unwrap();
        let tools = state
            .agent_management
            .resolve_internal_agent_effective_tool_definitions("TRON_CORE", "main-agent", "en-US")
            .unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].tool_code, TOOL);
        assert_eq!(tools[0].tool_name, "fixture_query");
    });
}
