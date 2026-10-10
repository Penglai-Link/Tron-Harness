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
async fn global_agent_selection_preserves_other_agents_and_users_and_rolls_back() {
    let f = Fixture::new();
    let a = f.service.create(USER, "A", "").unwrap();
    let b = f.service.create(USER, "B", "").unwrap();
    let foreign = f
        .service
        .create("OTHER-USER", "Foreign", "")
        .unwrap();
    let elsewhere = f
        .service
        .create(USER, "Elsewhere", "")
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
        .set_agent_knowledge_libraries(USER, &f.agent, &[b.id.clone(), b.id.clone()])
        .unwrap();
    assert_eq!(
        f.service
            .detail(USER, &a.id)
            .unwrap()
            .agent_codes,
        vec!["another-agent"]
    );
    assert!(
        f.service
            .detail(USER, &b.id)
            .unwrap()
            .agent_codes
            .contains(&f.agent)
    );
    assert!(
        f.service
            .detail("OTHER-USER", &foreign.id)
            .unwrap()
            .agent_codes
            .contains(&f.agent)
    );
    assert_eq!(
        f.service
            .detail(USER, &elsewhere.id)
            .unwrap()
            .agent_codes,
        vec!["another-agent"]
    );
    f.service
        .set_agent_knowledge_libraries(
            USER,
            &f.agent,
            &[b.id.clone(), elsewhere.id.clone()],
        )
        .unwrap();
    assert!(
        f.service
            .detail(USER, &elsewhere.id)
            .unwrap()
            .agent_codes
            .contains(&f.agent)
    );
    for invalid in [&foreign.id, &"missing".to_string()] {
        assert!(matches!(
            f.service
                .set_agent_knowledge_libraries(USER, &f.agent, &[invalid.clone()]),
            Err(KnowledgeBaseError::NotFound)
        ));
        assert!(
            f.service
                .detail(USER, &b.id)
                .unwrap()
                .agent_codes
                .contains(&f.agent)
        );
    }
    assert!(matches!(
        f.service
            .set_agent_knowledge_libraries(USER, "unknown-agent", &[]),
        Err(KnowledgeBaseError::Invalid(_))
    ));
    conn.execute_batch("CREATE TRIGGER fail_grant_revision BEFORE UPDATE OF acl_revision ON knowledge_bases BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
    assert!(
        f.service
            .set_agent_knowledge_libraries(USER, &f.agent, &[])
            .is_err()
    );
    assert!(
        f.service
            .detail(USER, &b.id)
            .unwrap()
            .agent_codes
            .contains(&f.agent)
    );
    conn.execute_batch("DROP TRIGGER fail_grant_revision")
        .unwrap();
    f.service
        .set_agent_knowledge_libraries(USER, &f.agent, &[])
        .unwrap();
    assert_eq!(
        f.service
            .detail(USER, &b.id)
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
        .create(USER, "Library", "Documents")
        .unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let path = format!("/api/v1/knowledge-bases/agent-grants/{}", f.agent);
    let query = path.clone();
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
        Some(json!({"libraryIds":[library.id]})),
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
        Some(json!({"agentCodes":[]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = common::authenticated_request(&app, Method::GET, &query, None).await;
    assert_eq!(
        common::response_json(response).await["data"][0]["granted"],
        false
    );
    for (locale, message) in [
        ("zh-CN", "知识库不存在"),
        ("en-US", "Knowledge base not found"),
    ] {
        let request = Request::builder()
            .method(Method::PUT)
            .uri(&path)
            .header("authorization", "Bearer profile-test-token")
            .header("accept-language", locale)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"libraryIds":["missing"]}).to_string(),
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


#[tokio::test]
async fn platform_agent_knowledge_configuration_precedes_space_authorization() {
    use tron_backend::application::agent::CreateWorkspaceInternalAgentCommand;
    let f = Fixture::new();
    let agent = f
        .state
        .agent_management
        .create_platform_internal_agent(CreateWorkspaceInternalAgentCommand {
            avatar_id: None,
            loop_code: None,
            agent_code: "platform-reader".into(),
            name: "Reader".into(),
            role_prompt: "Read documents".into(),
            group_name: None,
        })
        .unwrap();
    let library = f.service.create(USER, "Global library", "").unwrap();
    let foreign = f.service.create("OTHER-USER", "Private library", "").unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let path = format!("/api/v1/knowledge-bases/agent-grants/{}", agent.agent_code);
    let query = path.clone();
    let response = common::authenticated_request(&app, Method::GET, &query, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = common::response_json(response).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["id"], library.id);
    assert_eq!(body["data"][0]["granted"], false);
    let response = common::authenticated_request(&app, Method::PUT, &path,
        Some(json!({"libraryIds":[library.id]}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(f.state.agent_management.agent_workspace_grants(&agent.agent_code).unwrap().is_empty());
    let response = common::authenticated_request(&app, Method::GET, &query, None).await;
    assert_eq!(common::response_json(response).await["data"][0]["granted"], true);
    let response = common::authenticated_request(&app, Method::PUT, &path,
        Some(json!({"libraryIds":[foreign.id]}))).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(f.service.library_agent_grants(USER, &library.id).unwrap(), vec![agent.agent_code]);
    let response = common::authenticated_request(&app, Method::GET,
        "/api/v1/knowledge-bases/agent-grants/unknown-agent", None).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}


#[tokio::test]
async fn legacy_resources_indexes_and_tasks_migrate_without_bypassing_authorization() {
    use tron_backend::adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository;
    let f = Fixture::new();
    let library = f.service.create(USER, "Legacy indexed", "").unwrap();
    let source = b"orchid original evidence survives migration";
    let document = f.service.upload(USER, &library.id, "evidence.md", source).unwrap();
    f.service.index(USER, &library.id, &document.id).await.unwrap();
    let before = f.service.detail(USER, &library.id).unwrap().documents[0].clone();
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    conn.execute_batch("ALTER TABLE knowledge_bases ADD COLUMN workspace_code TEXT NOT NULL DEFAULT 'TRON_CORE'; UPDATE knowledge_bases SET payload_json=json_set(payload_json,'$.workspaceCode',workspace_code); CREATE TABLE knowledge_base_scoped_agent_grants(knowledge_base_id TEXT,workspace_code TEXT,agent_code TEXT,PRIMARY KEY(knowledge_base_id,workspace_code,agent_code)); CREATE TABLE knowledge_base_workspace_grants(knowledge_base_id TEXT,workspace_code TEXT);").unwrap();
    for table in ["resource_files", "resource_code_registry", "knowledge_base_resource_privacy", "knowledge_base_derived_resources"] {
        conn.execute(&format!("UPDATE {table} SET workspace_code=?1 WHERE workspace_code='PLATFORM'"), [WORKSPACE]).unwrap();
    }
    let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE ?1 AND sql NOT LIKE 'CREATE VIRTUAL TABLE%'").unwrap().query_map([format!("rag_{}_units%", library.id)], |r| r.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
    for table in tables {
        let scope: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name='scope_code')", [&table], |r| r.get(0)).unwrap();
        if scope { conn.execute(&format!("UPDATE \"{table}\" SET scope_code=?1"), [WORKSPACE]).unwrap(); }
    }
    conn.execute("UPDATE knowledge_base_ingestion_tasks SET payload_json=json_set(payload_json,'$.workspaceCode',?1)", [WORKSPACE]).unwrap();
    conn.execute("INSERT INTO knowledge_base_scoped_agent_grants VALUES(?1,?2,?3)", rusqlite::params![library.id, WORKSPACE, f.agent]).unwrap();
    let _repository = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    let columns: u32 = conn.query_row("SELECT count(*) FROM pragma_table_info('knowledge_bases') WHERE name IN ('workspace_code','resource_namespace')", [], |r| r.get(0)).unwrap();
    assert_eq!(columns, 0);
    let namespace: String = conn.query_row("SELECT workspace_code FROM resource_files WHERE resource_code=?1", [&document.resource_code], |r| r.get(0)).unwrap();
    assert_eq!(namespace, "PLATFORM");
    let legacy_tasks: u32 = conn.query_row("SELECT count(*) FROM knowledge_base_ingestion_tasks WHERE json_extract(payload_json,'$.workspaceCode') <> 'PLATFORM'", [], |r| r.get(0)).unwrap();
    assert_eq!(legacy_tasks, 0);
    let chunks = f.service.published_chunks(USER, &library.id, &document.id, 1, 100).unwrap();
    assert!(chunks.total > 0);
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let response = common::authenticated_request(&app, Method::GET, &format!("/api/v1/knowledge-bases/{}", library.id), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = common::response_json(response).await;
    assert!(body["data"].get("workspaceCode").is_none());
    assert!(body["data"].get("resourceNamespace").is_none());
    assert_eq!(body["data"]["documents"][0]["activeSourceCode"], json!(before.active_source_code));
    assert_eq!(body["data"]["documents"][0]["indexRevision"], before.index_revision);
    let response = common::authenticated_request(&app, Method::GET,
        &format!("/api/v1/knowledge-bases/{}/resources/{}/binary", library.id, document.resource_code), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(&bytes[..], source);
    let response = common::authenticated_request(&app, Method::GET,
        &format!("/api/v1/knowledge-bases/{}/resources/missing/binary", library.id), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(!f.service.search_for_agent(USER, "COMPLIANCE", &f.agent, "orchid", 1000).await.unwrap().hits.is_empty());
    assert!(f.service.detail("OTHER-USER", &library.id).is_err());
    f.service.assign_library_agents(USER, &library.id, &[]).unwrap();
    assert!(f.service.search_for_agent(USER, "COMPLIANCE", &f.agent, "orchid", 1000).await.unwrap().hits.is_empty());
    SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    assert!(f.service.library_agent_grants(USER, &library.id).unwrap().is_empty());
}

#[test]
fn platform_resource_collision_rolls_back_ownership_grants_and_resource_migration() {
    use tron_backend::adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository;
    let f = Fixture::new();
    let library = f.service.create(USER, "Collision", "").unwrap();
    let document = f.service.upload(USER, &library.id, "note.md", b"original").unwrap();
    let conn = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    conn.execute_batch("ALTER TABLE knowledge_bases ADD COLUMN workspace_code TEXT NOT NULL DEFAULT 'TRON_CORE'; UPDATE knowledge_bases SET payload_json=json_set(payload_json,'$.workspaceCode',workspace_code); CREATE TABLE knowledge_base_scoped_agent_grants(knowledge_base_id TEXT,workspace_code TEXT,agent_code TEXT,PRIMARY KEY(knowledge_base_id,workspace_code,agent_code));").unwrap();
    for table in ["resource_files", "resource_code_registry", "knowledge_base_resource_privacy"] {
        conn.execute(&format!("UPDATE {table} SET workspace_code=?1 WHERE resource_code=?2"), rusqlite::params![WORKSPACE, document.resource_code]).unwrap();
    }
    conn.execute("INSERT INTO resource_files SELECT id+1,'PLATFORM',resource_code,kind,name,description,file_name,file_extension,byte_size,storage_locator,allow_inheritance,revision,created_at,updated_at FROM resource_files WHERE resource_code=?1", [&document.resource_code]).unwrap();
    conn.execute("INSERT INTO knowledge_base_scoped_agent_grants VALUES(?1,?2,?3)", rusqlite::params![library.id, WORKSPACE, f.agent]).unwrap();
    assert!(SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).is_err());
    let old: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('knowledge_bases') WHERE name='workspace_code')", [], |r| r.get(0)).unwrap();
    assert!(old);
    let owner: String = conn.query_row("SELECT workspace_code FROM knowledge_base_resource_privacy WHERE resource_code=?1", [&document.resource_code], |r| r.get(0)).unwrap();
    assert_eq!(owner, WORKSPACE);
    let grants: u32 = conn.query_row("SELECT count(*) FROM knowledge_base_grants WHERE knowledge_base_id=?1", [&library.id], |r| r.get(0)).unwrap();
    assert_eq!(grants, 0);
    // Removing the conflict lets the exact same migration resume atomically.
    conn.execute("DELETE FROM resource_files WHERE workspace_code='PLATFORM' AND resource_code=?1", [&document.resource_code]).unwrap();
    let repository = SqliteKnowledgeBaseRepository::connect(&f.state.config.database_url).unwrap();
    assert_eq!(repository.grants(&library.id).unwrap(), vec![f.agent]);
}

#[test]
#[ignore = "Run only against a disposable SQLite backup via TRON_KB_MIGRATION_DB"]
fn migrate_existing_database_backup_preserves_documents_resources_and_indexes() {
    use tron_backend::adapters::outbound::persistence::sqlite::SqliteKnowledgeBaseRepository;
    let path = std::env::var("TRON_KB_MIGRATION_DB").expect("disposable backup required");
    let conn = rusqlite::Connection::open(&path).unwrap();
    let documents: Vec<(String,String)> = conn.prepare("SELECT id,payload_json FROM knowledge_base_documents ORDER BY id").unwrap().query_map([], |r| Ok((r.get(0)?,r.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
    let resources: Vec<(i64,String,i64)> = conn.prepare("SELECT id,storage_locator,revision FROM resource_files ORDER BY id").unwrap().query_map([], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap().collect::<Result<_,_>>().unwrap();
    let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'rag_kb_%' AND sql NOT LIKE 'CREATE VIRTUAL TABLE%'").unwrap().query_map([], |r| r.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
    let counts: Vec<i64> = tables.iter().map(|name| conn.query_row(&format!("SELECT count(*) FROM \"{}\"", name.replace('"',"\"\"")), [], |r| r.get(0)).unwrap()).collect();
    let repository = SqliteKnowledgeBaseRepository::connect(&path).unwrap();
    let after: Vec<(String,String)> = conn.prepare("SELECT id,payload_json FROM knowledge_base_documents ORDER BY id").unwrap().query_map([], |r| Ok((r.get(0)?,r.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(documents, after);
    let after: Vec<(i64,String,i64)> = conn.prepare("SELECT id,storage_locator,revision FROM resource_files ORDER BY id").unwrap().query_map([], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(resources, after);
    for (table,count) in tables.iter().zip(counts) {
        let after: i64 = conn.query_row(&format!("SELECT count(*) FROM \"{}\"", table.replace('"',"\"\"")), [], |r| r.get(0)).unwrap();
        assert_eq!(count, after, "{table}");
    }
    for table in ["knowledge_base_resource_privacy","knowledge_base_derived_resources"] {
        let stale: u32 = conn.query_row(&format!("SELECT count(*) FROM {table} WHERE workspace_code <> 'PLATFORM'"), [], |r| r.get(0)).unwrap();
        assert_eq!(stale, 0);
    }
    let old: u32 = conn.query_row("SELECT count(*) FROM pragma_table_info('knowledge_bases') WHERE name IN ('workspace_code','resource_namespace')", [], |r| r.get(0)).unwrap();
    assert_eq!(old, 0);
    drop(repository);
    SqliteKnowledgeBaseRepository::connect(&path).unwrap();
    println!("migration verified: {} documents, {} resource locators, {} index tables", documents.len(),resources.len(),tables.len());
}
