//! Resource grants use the complete platform identity catalog, not the Agent-only UI list.
mod common;

use axum::{Router, http::Method};
use serde_json::{Value, json};
use tron_backend::config::{AppConfig, ResourceConfig};

async fn data(app: &Router, method: Method, path: &str, body: Option<Value>) -> Value {
    let response = common::authenticated_request(app, method, path, body).await;
    let status = response.status();
    let value = common::response_json(response).await;
    assert!(status.is_success(), "{path}: {status}: {value}");
    value["data"].clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn default_and_custom_dudes_can_assign_revoke_and_use_resources_without_joining_agent_lists()
{
    let directory = tempfile::tempdir().unwrap();
    let skills = directory.path().join("skills");
    let skill = skills.join(".agents/skills/dude-assignment-test");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), "---\nname: dude-assignment-test\ndescription: Resource grant regression\n---\nUse assigned resources.").unwrap();
    let config = common::prepared_config(AppConfig {
        database_url: directory.path().join("db.sqlite3").display().to_string(),
        graph_storage_path: directory.path().join("graph.grafeo").display().to_string(),
        skills_registry_root: skills.display().to_string(),
        resource: ResourceConfig::with_default_root(
            directory.path().join("resources").display().to_string(),
        ),
        ..Default::default()
    });
    let mut captured = None;
    let app = common::app_with_state_setup(config, |state| captured = Some(state.clone()));
    let state = captured.unwrap();
    state.knowledge_bases.stop_ingestion_workers();
    let main = state.dudes.definition("main-dude").unwrap();
    data(
        &app,
        Method::POST,
        "/api/v1/dudes",
        Some(json!({
            "dudeCode":"grant-dude", "name":"Grant Dude", "rolePrompt":"Use grants",
            "enabled":false
        })),
    )
    .await;
    let library = state
        .knowledge_bases
        .create("USR-AUTH-0001", "Dude Library", "")
        .unwrap();
    let foreign = state
        .knowledge_bases
        .create("OTHER-USER", "Foreign Library", "")
        .unwrap();
    for code in ["main-dude", "memory-dude", "grant-dude"] {
        let skills_path = format!("/api/v1/agents/registry/internal/{code}/skills");
        let items = data(&app, Method::GET, &skills_path, None).await;
        assert!(
            items
                .as_array()
                .unwrap()
                .iter()
                .any(|skill| skill["skillName"] == "dude-assignment-test")
        );
        data(
            &app,
            Method::PUT,
            &skills_path,
            Some(json!({"skillNames":["dude-assignment-test"]})),
        )
        .await;
        assert_eq!(
            state
                .skills
                .effective_agent_skills(code)
                .unwrap()
                .iter()
                .map(|s| s.skill_name.as_str())
                .collect::<Vec<_>>(),
            vec!["dude-assignment-test"]
        );
        let knowledge_path = format!("/api/v1/knowledge-bases/agent-grants/{code}");
        data(&app, Method::GET, &knowledge_path, None).await;
        data(
            &app,
            Method::PUT,
            &knowledge_path,
            Some(json!({"libraryIds":[library.id]})),
        )
        .await;
        let listed = data(&app, Method::GET, &knowledge_path, None).await;
        assert!(
            listed
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["id"] == library.id && item["granted"] == true)
        );
        let denied = common::authenticated_request(
            &app,
            Method::PUT,
            &knowledge_path,
            Some(json!({"libraryIds":[foreign.id]})),
        )
        .await;
        assert_eq!(denied.status(), axum::http::StatusCode::NOT_FOUND);
        assert!(
            state
                .knowledge_bases
                .detail("USR-AUTH-0001", &library.id)
                .unwrap()
                .agent_codes
                .contains(&code.to_owned())
        );
        data(
            &app,
            Method::PUT,
            &skills_path,
            Some(json!({"skillNames":[]})),
        )
        .await;
        data(
            &app,
            Method::PUT,
            &knowledge_path,
            Some(json!({"libraryIds":[]})),
        )
        .await;
        assert!(
            state
                .skills
                .effective_agent_skills(code)
                .unwrap()
                .is_empty()
        );
        assert!(
            !state
                .knowledge_bases
                .detail("USR-AUTH-0001", &library.id)
                .unwrap()
                .agent_codes
                .contains(&code.to_owned())
        );
    }
    // Both directions share validation and persistence.
    data(
        &app,
        Method::PUT,
        "/api/v1/skills/registry/dude-assignment-test/agent-grants",
        Some(json!({"agentCodes":["memory-dude"]})),
    )
    .await;
    data(
        &app,
        Method::PUT,
        &format!("/api/v1/knowledge-bases/{}/agent-grants", library.id),
        Some(json!({"agentCodes":["memory-dude"]})),
    )
    .await;
    assert_eq!(
        state
            .skills
            .effective_agent_skills("memory-dude")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        state
            .knowledge_bases
            .library_agent_grants("USR-AUTH-0001", &library.id)
            .unwrap(),
        vec!["memory-dude"]
    );
    for path in [
        "/api/v1/agents/registry/internal/missing-dude/skills",
        "/api/v1/knowledge-bases/agent-grants/missing-dude",
    ] {
        assert!(
            common::authenticated_request(&app, Method::GET, path, None)
                .await
                .status()
                .is_client_error()
        );
    }
    let ordinary = data(&app, Method::GET, "/api/v1/agents/registry/internal", None).await;
    assert!(ordinary.as_array().unwrap().iter().all(|agent| {
        !["main-dude", "memory-dude", "grant-dude"].contains(&agent["agentCode"].as_str().unwrap())
    }));
    data(
        &app,
        Method::PATCH,
        "/api/v1/agents/registry/internal/GRANT-DUDE/definition",
        Some(json!({"groupName":"invalid-group"})),
    )
    .await;
    assert!(
        state
            .dudes
            .definition("grant-dude")
            .unwrap()
            .group_name
            .is_none()
    );
    assert!(
        data(&app, Method::GET, "/api/v1/dudes/grant-dude", None)
            .await
            .get("groupName")
            .is_none()
    );
    assert!(state.agent_framework.close().await.is_success());
}
