mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use fixture::*;
use tower::ServiceExt;
use tron_backend::application::knowledge_base::KnowledgeBaseError;

#[tokio::test]
async fn library_deletion_requires_empty_scope_and_removes_grants_and_index_atomically() {
    let f = Fixture::new();
    let kb = f.service.create(USER, WORKSPACE, "Library", "").unwrap();
    let other = f.service.create(USER, WORKSPACE, "Keep", "").unwrap();
    f.service
        .set_grants(
            USER,
            WORKSPACE,
            &kb.id,
            vec![f.agent.clone(), f.agent.clone()],
        )
        .unwrap();
    assert_eq!(
        f.service
            .list(USER, WORKSPACE)
            .unwrap()
            .iter()
            .find(|item| item.id == kb.id)
            .unwrap()
            .agent_codes
            .len(),
        1
    );
    assert!(
        f.service
            .remove_library("OTHER-USER", WORKSPACE, &kb.id)
            .is_err()
    );
    assert!(
        f.service
            .remove_library(USER, "COMPLIANCE", &kb.id)
            .is_err()
    );
    let doc = f
        .service
        .upload(USER, WORKSPACE, &kb.id, "note.md", b"# Library\norchid")
        .unwrap();
    assert!(matches!(
        f.service.remove_library(USER, WORKSPACE, &kb.id),
        Err(KnowledgeBaseError::NotEmpty)
    ));
    let indexed = f
        .service
        .index(USER, WORKSPACE, &kb.id, &doc.id)
        .await
        .unwrap();
    assert_eq!(indexed.status, "READY");
    assert!(matches!(
        f.service.remove_library(USER, WORKSPACE, &kb.id),
        Err(KnowledgeBaseError::NotEmpty)
    ));
    f.service
        .remove_document(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap();
    f.service.remove_library(USER, WORKSPACE, &kb.id).unwrap();
    assert!(matches!(
        f.service.detail(USER, WORKSPACE, &kb.id),
        Err(KnowledgeBaseError::NotFound)
    ));
    assert!(f.service.detail(USER, WORKSPACE, &other.id).is_ok());
    assert!(
        f.state
            .resources
            .download_binary_to_file(
                USER,
                tron_backend::application::resource::ResourceBinaryDownloadCommand {
                    workspace_code: WORKSPACE.into(),
                    resource_code: doc.resource_code
                }
            )
            .is_ok()
    );
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    for table in ["knowledge_bases", "knowledge_base_grants", "rag_corpora"] {
        let key = if table == "knowledge_bases" {
            "id"
        } else if table == "rag_corpora" {
            "corpus_id"
        } else {
            "knowledge_base_id"
        };
        assert_eq!(
            conn.query_row(
                &format!("SELECT count(*) FROM {table} WHERE {key}=?1"),
                [&kb.id],
                |row| row.get::<_, u64>(0)
            )
            .unwrap(),
            0
        );
    }
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name GLOB ?1",
            [format!("rag_{}_units*", kb.id)],
            |row| row.get::<_, u64>(0)
        )
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn failed_library_deletion_rolls_back_corpus_tables_and_grants() {
    let f = Fixture::new();
    let kb = f
        .service
        .create(USER, WORKSPACE, "Keep on failure", "")
        .unwrap();
    f.service
        .set_grants(USER, WORKSPACE, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_kb_delete BEFORE DELETE ON knowledge_bases BEGIN SELECT RAISE(ABORT, 'test deletion failure'); END;").unwrap();
    assert!(f.service.remove_library(USER, WORKSPACE, &kb.id).is_err());
    assert_eq!(
        f.service
            .detail(USER, WORKSPACE, &kb.id)
            .unwrap()
            .agent_codes,
        vec![f.agent.clone()]
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM rag_corpora WHERE corpus_id=?1",
            [&kb.id],
            |row| row.get::<_, u64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name=?1",
            [format!("rag_{}_units", kb.id)],
            |row| row.get::<_, u64>(0)
        )
        .unwrap(),
        1
    );
}

#[tokio::test]
async fn catalog_count_and_deletion_http_contract_are_localized_and_workspace_scoped() {
    let f = Fixture::new();
    let kb = f.service.create(USER, WORKSPACE, "Library", "").unwrap();
    f.service
        .set_grants(USER, WORKSPACE, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let doc = f
        .service
        .upload(USER, WORKSPACE, &kb.id, "note.md", b"orchid")
        .unwrap();
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let list = common::authenticated_request(
        &app,
        Method::GET,
        &format!("/api/v1/knowledge-bases?workspaceCode={WORKSPACE}"),
        None,
    )
    .await;
    let data = common::response_json(list).await;
    assert_eq!(data["data"][0]["authorizedAgentCount"], 1);
    assert_eq!(data["data"][0]["documentCount"], 1);
    assert!(data["data"][0].get("documents").is_none());
    let uri = format!(
        "/api/v1/knowledge-bases/{}?workspaceCode={WORKSPACE}",
        kb.id
    );
    for (locale, message) in [
        ("en-US", "Delete all resources in the knowledge base first"),
        ("zh-CN", "请先删除知识库内的所有资源"),
    ] {
        let request = Request::builder()
            .method(Method::DELETE)
            .uri(&uri)
            .header("authorization", "Bearer profile-test-token")
            .header("accept-language", locale)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let error = common::response_json(response).await;
        assert_eq!(error["code"], "101027001");
        assert_eq!(error["message"], message);
        assert_eq!(error["data"], serde_json::json!({}));
    }
    let denied = common::authenticated_request(
        &app,
        Method::DELETE,
        &format!("/api/v1/knowledge-bases/{}?workspaceCode=COMPLIANCE", kb.id),
        None,
    )
    .await;
    assert!(!denied.status().is_success());
    f.service
        .remove_document(USER, WORKSPACE, &kb.id, &doc.id)
        .unwrap();
    let response = common::authenticated_request(&app, Method::DELETE, &uri, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        common::response_json(response).await["data"],
        serde_json::json!({})
    );
    assert_eq!(
        common::authenticated_request(&app, Method::DELETE, &uri, None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}
