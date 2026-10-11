mod common;
use axum::http::{Method, StatusCode};
use common::{authenticated_bearer_token, request_with_headers, response_json};
use serde_json::{Value, json};
#[tokio::test]
async fn tool_permissions_round_trip_validate_atomically_and_reject_unknown_enum_values() {
    let app = common::app_with_state_setup(common::prepared_config(Default::default()), |s| {
        s.knowledge_bases.stop_ingestion_workers()
    });
    let token = authenticated_bearer_token(&app).await;
    let auth = format!("Bearer {token}");
    let response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/tools/permissions",
        None,
        None,
        Some(&auth),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let settings = body["data"]["permissions"].as_array().unwrap();
    assert!(!settings.is_empty());
    assert!(
        settings
            .iter()
            .all(|s| s["permission"] == "Run" && s["confirmLevel"] == "Low")
    );
    let code = settings[0]["toolCode"].clone();
    let setting = json!({"toolCode":code,"permission":"Ask","confirmLevel":"High"});
    let response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/tools/permissions",
        Some(json!({"permissions":[setting]})),
        None,
        Some(&auth),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    for invalid in [
        json!({"permissions":[{"toolCode":code,"permission":"run","confirmLevel":"Low"}]}),
        json!({"permissions":[{"toolCode":"unknown.tool","permission":"No","confirmLevel":"Low"}]}),
        json!({"permissions":[setting,setting]}),
        json!({"permissions":[{"toolCode":code,"permission":"No","confirmLevel":"Critical"},{"toolCode":"unknown.tool","permission":"Run","confirmLevel":"Low"}]}),
    ] {
        let response = request_with_headers(
            &app,
            Method::PUT,
            "/api/v1/tools/permissions",
            Some(invalid),
            None,
            Some(&auth),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response_json(response).await["code"], "101028015");
    }
    let body = response_json(
        request_with_headers(
            &app,
            Method::GET,
            "/api/v1/tools/permissions",
            None,
            None,
            Some(&auth),
        )
        .await,
    )
    .await;
    assert_eq!(
        body["data"]["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["toolCode"] == code)
            .unwrap(),
        &setting
    );
    let response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/tools/permissions",
        Some(json!({"permissions":[]})),
        None,
        Some(&auth),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response_json(response).await;
    assert!(
        body["data"]["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["permission"] == "Run")
    );
}

#[tokio::test]
async fn discovered_native_mcp_tools_are_listed_in_domains_catalog_and_permissions() {
    use tron_harness_capability::ToolPermissionSource;
    let app = common::app_with_state_setup(common::prepared_config(Default::default()), |s| {
        s.knowledge_bases.stop_ingestion_workers();
        let descriptor = eliteforge_agent_core::ToolDescriptor::new(
            "mcp/fixture/query",
            "mcp_fixture_query",
            "Trusted provider discovery",
            json!({"type":"object"}),
        );
        // The source is called by the real provider's ToolPolicy admission, not an HTTP projection.
        assert_eq!(
            s.agent_management.setting(&descriptor).unwrap().tool_code,
            "mcp/fixture/query"
        );
    });
    let token = authenticated_bearer_token(&app).await;
    let auth = format!("Bearer {token}");
    let body = response_json(
        request_with_headers(
            &app,
            Method::GET,
            "/api/v1/tools/domains",
            None,
            None,
            Some(&auth),
        )
        .await,
    )
    .await;
    let domain = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["domainCode"] == "mcp")
        .unwrap();
    let domain_count = domain["toolCount"].as_u64().unwrap();
    let body = response_json(
        request_with_headers(
            &app,
            Method::GET,
            "/api/v1/tools?domainCode=mcp",
            None,
            None,
            Some(&auth),
        )
        .await,
    )
    .await;
    let tools = body["data"].as_array().unwrap();
    assert_eq!(domain_count, tools.len() as u64);
    assert!(
        tools
            .iter()
            .any(|tool| tool["toolCode"] == "mcp/fixture/query")
    );
    let body = response_json(
        request_with_headers(
            &app,
            Method::GET,
            "/api/v1/tools/permissions",
            None,
            None,
            Some(&auth),
        )
        .await,
    )
    .await;
    assert!(
        body["data"]["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["toolCode"] == "mcp/fixture/query")
    );
    let response=request_with_headers(&app,Method::PUT,"/api/v1/tools/permissions",Some(json!({"permissions":[{"toolCode":"mcp/fixture/query","permission":"No","confirmLevel":"High"}]})),None,Some(&auth)).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn tool_permissions_persist_and_failed_full_replace_rolls_back() {
    use tron_backend::{
        adapters::outbound::persistence::sqlite::SqliteAgentManagementRepository,
        application::agent::AgentManagementRepository,
    };
    use tron_harness_capability::{ToolPermission, ToolPermissionSetting, ToolRisk};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("permissions.sqlite");
    let mut repository = SqliteAgentManagementRepository::connect(path.to_str().unwrap()).unwrap();
    let original = ToolPermissionSetting {
        tool_code: "filesystem.write_text".into(),
        permission: ToolPermission::Ask,
        confirm_level: ToolRisk::Medium,
    };
    repository
        .replace_tool_permissions(std::slice::from_ref(&original))
        .unwrap();
    let mut replacement = original.clone();
    replacement.permission = ToolPermission::No;
    assert!(
        repository
            .replace_tool_permissions(&[replacement.clone(), replacement])
            .is_err()
    );
    assert_eq!(
        repository.list_tool_permissions().unwrap(),
        vec![original.clone()]
    );
    drop(repository);
    let mut restored = SqliteAgentManagementRepository::connect(path.to_str().unwrap()).unwrap();
    assert_eq!(restored.list_tool_permissions().unwrap(), vec![original]);
}

#[test]
fn interactive_input_and_process_stop_keep_their_side_effect_risk() {
    use eliteforge_agent_core::ToolDescriptor;
    use tron_harness_capability::{ToolRisk, identify_tool_risk};
    for id in ["system.command.task.write", "framework.terminal.write"] {
        let descriptor = ToolDescriptor::new(id, "write", "", json!({}));
        assert_eq!(
            identify_tool_risk(&descriptor, &json!({"data":"rm -rf /\n"}), Some(true)),
            ToolRisk::Critical
        );
    }
    for id in ["system.command.task.stop", "framework.terminal.stop"] {
        let descriptor = ToolDescriptor::new(id, "stop", "", json!({}));
        assert_eq!(
            identify_tool_risk(&descriptor, &json!({}), Some(true)),
            ToolRisk::High
        );
    }
}
