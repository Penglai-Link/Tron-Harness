mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use tron_backend::application::{
    agent::CreateWorkspaceInternalAgentCommand, knowledge_base::KnowledgeBaseError,
};
use tron_backend::domain::agent::loop_design::DEFAULT_REACT_LOOP_CODE;

#[tokio::test]
async fn knowledge_authorization_follows_agent_assignments_without_library_space_grants() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Shared", "").unwrap();
    let doc = f
        .service
        .upload(USER, &kb.id, "note.md", b"orchid shared platform evidence")
        .unwrap();
    f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    let agent = f
        .state
        .agent_management
        .create_platform_internal_agent(CreateWorkspaceInternalAgentCommand {
            description: String::new(),
            candidate_model_codes: Vec::new(),
            avatar_id: None,
            loop_code: Some(DEFAULT_REACT_LOOP_CODE.into()),
            agent_code: "knowledge-expert".into(),
            name: "Expert".into(),
            role_prompt: "Help".into(),
            group_name: None,
        })
        .unwrap()
        .agent_code;
    let read_resource = |workspace: &str| {
        f.state
            .resources
            .for_agent_in_workspace(Some(&agent), workspace)
            .download_binary_to_file(
                USER,
                tron_backend::application::resource::ResourceBinaryDownloadCommand {
                    workspace_code: "PLATFORM".into(),
                    resource_code: doc.resource_code.clone(),
                },
            )
    };
    // Knowledge configuration is available before the new Agent has any spaces.
    assert!(
        f.state
            .agent_management
            .agent_workspace_grants(&agent)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.service
            .list_agent_knowledge_libraries(USER, &agent)
            .unwrap()
            .len(),
        1
    );
    f.service
        .set_agent_knowledge_libraries(USER, &agent, &[kb.id.clone(), kb.id.clone()])
        .unwrap();
    assert_eq!(
        f.service.platform_libraries(USER).unwrap()[0].agent_codes,
        vec![agent.clone()]
    );
    assert!(
        f.service
            .search_for_agent(USER, WORKSPACE, &agent, "orchid", 1000)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(read_resource(WORKSPACE).is_err());
    f.state
        .agent_management
        .assign_agent_workspaces(&agent, vec![WORKSPACE.into(), "COMPLIANCE".into()])
        .unwrap();
    for workspace in [WORKSPACE, "COMPLIANCE"] {
        assert!(read_resource(workspace).is_ok());
        assert!(
            f.service
                .list_agent_knowledge_libraries(USER, &agent)
                .unwrap()[0]
                .agent_codes
                .contains(&agent)
        );
        assert!(
            f.service
                .search_for_agent(USER, workspace, &agent, "orchid", 1000)
                .await
                .unwrap()
                .text
                .contains("orchid shared platform evidence")
        );
    }
    let indexed = f.service.detail(USER, &kb.id).unwrap().documents[0].clone();
    f.state
        .agent_management
        .assign_agent_workspaces(&agent, vec!["COMPLIANCE".into()])
        .unwrap();
    assert!(
        f.service
            .search_for_agent(USER, WORKSPACE, &agent, "orchid", 1000)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(
        !f.service
            .search_for_agent(USER, "COMPLIANCE", &agent, "orchid", 1000)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert!(read_resource(WORKSPACE).is_err());
    assert!(read_resource("COMPLIANCE").is_ok());
    f.service.assign_library_agents(USER, &kb.id, &[]).unwrap();
    assert!(read_resource("COMPLIANCE").is_err());
    assert!(
        f.service
            .search_for_agent(USER, "COMPLIANCE", &agent, "orchid", 1000)
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    let after = f.service.detail(USER, &kb.id).unwrap().documents[0].clone();
    assert_eq!(indexed.active_source_code, after.active_source_code);
    assert_eq!(indexed.index_revision, after.index_revision);
}

#[test]
fn global_library_grants_are_owner_scoped_validated_atomic_and_persist_on_reconnect() {
    use tron_backend::adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository;
    let f = Fixture::new();
    let kb = f.service.create(USER, "Global", "").unwrap();
    f.service
        .assign_library_agents(USER, &kb.id, &[f.agent.clone()])
        .unwrap();
    assert!(matches!(
        f.service.assign_library_agents("OTHER-USER", &kb.id, &[]),
        Err(KnowledgeBaseError::NotFound)
    ));
    assert!(matches!(
        f.service
            .assign_library_agents(USER, &kb.id, &["missing".into()]),
        Err(KnowledgeBaseError::Invalid(_))
    ));
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_acl BEFORE UPDATE OF acl_revision ON knowledge_bases BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
    assert!(f.service.assign_library_agents(USER, &kb.id, &[]).is_err());
    assert_eq!(
        f.service.library_agent_grants(USER, &kb.id).unwrap(),
        vec![f.agent.clone()]
    );
    conn.execute_batch("DROP TRIGGER fail_acl;").unwrap();
    let repository = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    assert_eq!(repository.grants(&kb.id).unwrap(), vec![f.agent.clone()]);
    let column_exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('knowledge_bases') WHERE name='workspace_code')", [], |row| row.get(0)).unwrap();
    assert!(!column_exists);
    let stale_tables: u32 = conn.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('knowledge_base_scoped_agent_grants','knowledge_base_workspace_grants','knowledge_base_migration_markers')", [], |row| row.get(0)).unwrap();
    assert_eq!(stale_tables, 0);
    let payload: String = conn
        .query_row(
            "SELECT payload_json FROM knowledge_bases WHERE id=?1",
            [&kb.id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!payload.contains("workspaceCode"));

    f.service.assign_library_agents(USER, &kb.id, &[]).unwrap();
    let reopened = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    assert!(reopened.grants(&kb.id).unwrap().is_empty());
}

#[tokio::test]
async fn platform_agent_assignment_controls_catalog_and_runtime() {
    let f = Fixture::new();
    let service = &f.state.agent_management;
    let agent = service
        .create_platform_internal_agent(CreateWorkspaceInternalAgentCommand {
            description: String::new(),
            candidate_model_codes: Vec::new(),
            avatar_id: None,
            loop_code: Some(DEFAULT_REACT_LOOP_CODE.into()),
            agent_code: "shared-expert".into(),
            name: "Expert".into(),
            role_prompt: "Help".into(),
            group_name: None,
        })
        .unwrap();
    assert!(
        service
            .agent_workspace_grants(&agent.agent_code)
            .unwrap()
            .is_empty()
    );
    assert!(
        service
            .resolve_workspace_agent_runtime(WORKSPACE, Some(&agent.agent_code))
            .is_err()
    );
    service
        .assign_agent_workspaces(&agent.agent_code, vec![WORKSPACE.into()])
        .unwrap();
    assert!(
        service
            .resolve_workspace_agent_runtime(WORKSPACE, Some(&agent.agent_code))
            .is_ok()
    );
    assert!(
        service
            .resolve_workspace_agent_runtime("COMPLIANCE", Some(&agent.agent_code))
            .is_err()
    );
    assert!(
        !service
            .list_workspace_internal_agents("COMPLIANCE")
            .unwrap()
            .iter()
            .any(|record| record.agent_code == agent.agent_code)
    );
    assert!(
        service
            .assign_agent_workspaces(&agent.agent_code, vec!["missing".into()])
            .is_err()
    );
    assert_eq!(
        service.agent_workspace_grants(&agent.agent_code).unwrap(),
        vec![WORKSPACE]
    );
    service
        .assign_agent_workspaces(&agent.agent_code, vec![])
        .unwrap();
    assert!(
        service
            .resolve_workspace_agent_runtime(WORKSPACE, Some(&agent.agent_code))
            .is_err()
    );
    assert!(
        service
            .resolve_workspace_agent_runtime(WORKSPACE, Some("main-agent"))
            .is_ok()
    );
}

#[tokio::test]
async fn platform_knowledge_agent_http_contracts_count_and_roundtrip() {
    use axum::http::{Method, StatusCode};
    use serde_json::json;
    let f = Fixture::new();
    let library = f.service.create(USER, "Shared", "Documents").unwrap();
    let other = f.service.create("OTHER-USER", "Private", "").unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let path = format!("/api/v1/knowledge-bases/{}/agent-grants", library.id);
    let response = common::authenticated_request(
        &app,
        Method::PUT,
        &path,
        Some(json!({"agentCodes":[f.agent, f.agent]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        common::response_json(response).await,
        json!({"code":"0","message":"ok","data":{}})
    );
    let response = common::authenticated_request(&app, Method::GET, &path, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        common::response_json(response).await["data"],
        json!({"agentCodes":[f.agent]})
    );
    let response =
        common::authenticated_request(&app, Method::GET, "/api/v1/knowledge-bases/platform", None)
            .await;
    let body = common::response_json(response).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert!(!body.to_string().contains(&other.id));
    assert_eq!(body["data"][0]["authorizedAgentCount"], 1);
    assert!(body["data"][0].get("workspaceCodes").is_none());
    assert!(body["data"][0].get("workspaceCode").is_none());
    for _ in [WORKSPACE, "COMPLIANCE"] {
        let response = common::authenticated_request(
            &app,
            Method::GET,
            &format!("/api/v1/knowledge-bases/agent-grants/{}", f.agent),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            common::response_json(response).await["data"][0]["granted"],
            true
        );
    }
    let response = common::authenticated_request(
        &app,
        Method::PUT,
        &format!("/api/v1/knowledge-bases/{}/agent-grants", other.id),
        Some(json!({"agentCodes":[]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = common::authenticated_request(
        &app,
        Method::PUT,
        &path,
        Some(json!({"agentCodes":["missing"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response =
        common::authenticated_request(&app, Method::PUT, &path, Some(json!({"agentCodes":[]})))
            .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response =
        common::authenticated_request(&app, Method::GET, "/api/v1/knowledge-bases/platform", None)
            .await;
    assert_eq!(
        common::response_json(response).await["data"][0]["authorizedAgentCount"],
        0
    );
}

#[test]
fn system_agent_assignment_controls_runtime_and_preserves_global_enablement() {
    use tron_backend::{
        adapters::outbound::persistence::sqlite::SqliteAgentManagementRepository,
        application::agent::AgentManagementRepository,
        domain::agent::{AgentRuntimeType, ManagedAgentSource, SystemAgentRegistryEntry},
    };
    let f = Fixture::new();
    let mut repository =
        SqliteAgentManagementRepository::connect(&f.state.config.database_url).unwrap();
    repository
        .upsert_system_agent(&SystemAgentRegistryEntry {
            fast_mode: false,
            reasoning_effort: None,
            avatar_id: None,
            base_agent_code: None,
            model_id: None,
            agent_code: "system-expert".into(),
            name: "System Expert".into(),
            description: "".into(),
            runtime_type: AgentRuntimeType::CodexExec,
            command_name: "fixture-command".into(),
            source: ManagedAgentSource::System,
            enabled: true,
            available: true,
            last_scanned_at: None,
            last_seen_at: None,
            last_error: None,
        })
        .unwrap();
    let service = &f.state.agent_management;
    service
        .assign_agent_workspaces("system-expert", vec![WORKSPACE.into()])
        .unwrap();
    assert!(
        service
            .resolve_workspace_agent_runtime(WORKSPACE, Some("system-expert"))
            .is_ok()
    );
    assert!(
        service
            .resolve_workspace_agent_runtime("COMPLIANCE", Some("system-expert"))
            .is_err()
    );
    assert!(
        service
            .list_workspace_system_agents("COMPLIANCE")
            .unwrap()
            .iter()
            .all(|agent| agent.agent_code != "system-expert")
    );
    service
        .assign_agent_workspaces("system-expert", vec![])
        .unwrap();
    assert!(
        service
            .resolve_workspace_agent_runtime(WORKSPACE, Some("system-expert"))
            .is_err()
    );
    assert!(
        repository
            .list_system_agents()
            .unwrap()
            .iter()
            .find(|agent| agent.agent_code == "system-expert")
            .unwrap()
            .enabled
    );
}
