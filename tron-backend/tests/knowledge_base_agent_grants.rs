mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use fixture::*;
use serde_json::json;
use tower::ServiceExt;
use tron_backend::application::knowledge_base::KnowledgeBaseError;

#[tokio::test]
async fn agent_selection_preserves_other_agents_users_and_workspaces_and_rolls_back() {
    let f = Fixture::new();
    let a = f.service.create(USER, WORKSPACE, "A", "").unwrap();
    let b = f.service.create(USER, WORKSPACE, "B", "").unwrap();
    let foreign = f
        .service
        .create("OTHER-USER", WORKSPACE, "Foreign", "")
        .unwrap();
    let elsewhere = f
        .service
        .create(USER, "COMPLIANCE", "Elsewhere", "")
        .unwrap();
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    for id in [&a.id, &b.id, &foreign.id, &elsewhere.id] {
        conn.execute(
            "INSERT INTO knowledge_base_grants VALUES(?1,?2)",
            rusqlite::params![id, f.agent],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO knowledge_base_grants VALUES(?1,'another-agent')",
            [id],
        )
        .unwrap();
    }
    f.service
        .set_agent_knowledge_libraries(USER, WORKSPACE, &f.agent, &[b.id.clone(), b.id.clone()])
        .unwrap();
    assert_eq!(
        f.service
            .detail(USER, WORKSPACE, &a.id)
            .unwrap()
            .agent_codes,
        vec!["another-agent"]
    );
    assert!(
        f.service
            .detail(USER, WORKSPACE, &b.id)
            .unwrap()
            .agent_codes
            .contains(&f.agent)
    );
    assert!(
        f.service
            .detail("OTHER-USER", WORKSPACE, &foreign.id)
            .unwrap()
            .agent_codes
            .contains(&f.agent)
    );
    assert!(
        f.service
            .detail(USER, "COMPLIANCE", &elsewhere.id)
            .unwrap()
            .agent_codes
            .contains(&f.agent)
    );
    for invalid in [&foreign.id, &elsewhere.id, &"missing".to_string()] {
        assert!(matches!(
            f.service
                .set_agent_knowledge_libraries(USER, WORKSPACE, &f.agent, &[invalid.clone()]),
            Err(KnowledgeBaseError::NotFound)
        ));
        assert!(
            f.service
                .detail(USER, WORKSPACE, &b.id)
                .unwrap()
                .agent_codes
                .contains(&f.agent)
        );
    }
    assert!(matches!(
        f.service
            .set_agent_knowledge_libraries(USER, WORKSPACE, "unknown-agent", &[]),
        Err(KnowledgeBaseError::Invalid(_))
    ));
    conn.execute_batch("CREATE TRIGGER fail_grant_revision BEFORE UPDATE OF acl_revision ON knowledge_bases BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
    assert!(
        f.service
            .set_agent_knowledge_libraries(USER, WORKSPACE, &f.agent, &[])
            .is_err()
    );
    assert!(
        f.service
            .detail(USER, WORKSPACE, &b.id)
            .unwrap()
            .agent_codes
            .contains(&f.agent)
    );
    conn.execute_batch("DROP TRIGGER fail_grant_revision")
        .unwrap();
    f.service
        .set_agent_knowledge_libraries(USER, WORKSPACE, &f.agent, &[])
        .unwrap();
    assert_eq!(
        f.service
            .detail(USER, WORKSPACE, &b.id)
            .unwrap()
            .agent_codes,
        vec!["another-agent"]
    );
}

#[tokio::test]
async fn agent_grants_api_roundtrips_with_library_grants_and_localized_errors() {
    let f = Fixture::new();
    let library = f
        .service
        .create(USER, WORKSPACE, "Library", "Documents")
        .unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let path = format!("/api/v1/knowledge-bases/agent-grants/{}", f.agent);
    let query = format!("{path}?workspaceCode={WORKSPACE}");
    let response = common::authenticated_request(&app, Method::GET, &query, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let list = common::response_json(response).await;
    assert_eq!(list["code"], "0");
    assert_eq!(
        list["data"][0],
        json!({"id": library.id, "name":"Library", "description":"Documents", "documentCount":0, "granted":false})
    );
    let response = common::authenticated_request(
        &app,
        Method::PUT,
        &path,
        Some(json!({"workspaceCode":WORKSPACE,"libraryIds":[library.id]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let detail_path = format!(
        "/api/v1/knowledge-bases/{}?workspaceCode={WORKSPACE}",
        library.id
    );
    let response = common::authenticated_request(&app, Method::GET, &detail_path, None).await;
    assert_eq!(
        common::response_json(response).await["data"]["agentCodes"],
        json!([f.agent])
    );
    let grant_path = format!("/api/v1/knowledge-bases/{}/grants", library.id);
    let response = common::authenticated_request(
        &app,
        Method::PUT,
        &grant_path,
        Some(json!({"workspaceCode":WORKSPACE,"agentCodes":[]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = common::authenticated_request(&app, Method::GET, &query, None).await;
    assert_eq!(
        common::response_json(response).await["data"][0]["granted"],
        false
    );
    for (locale, message) in [
        ("zh-CN", "知识库不存在或不属于当前工作区"),
        ("en-US", "Knowledge base not found in this workspace"),
    ] {
        let request = Request::builder()
            .method(Method::PUT)
            .uri(&path)
            .header("authorization", "Bearer profile-test-token")
            .header("accept-language", locale)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"workspaceCode":WORKSPACE,"libraryIds":["missing"]}).to_string(),
            ))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let error = common::response_json(response).await;
        assert_eq!(error["code"], "101027002");
        assert_eq!(error["message"], message);
        assert_eq!(error["data"], json!({}));
    }
}
