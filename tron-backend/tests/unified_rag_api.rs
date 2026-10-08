mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use axum::http::{Method, StatusCode};
use fixture::*;
use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagRepository, application::rag::*,
};

struct SearchControl;
impl tron_backend::domain::agent::ToolExecutionControl for SearchControl {
    fn check_active(&self) -> Result<(), tron_backend::domain::agent::ToolError> {
        Ok(())
    }
    fn remaining(&self) -> std::time::Duration {
        std::time::Duration::from_secs(10)
    }
    fn cancelled(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(std::future::pending())
    }
}

#[tokio::test]
async fn management_recall_needs_no_agent_and_cannot_read_foreign_or_unpublished_sources() {
    let f = Fixture::new();
    let kb = f
        .service
        .create(USER, "Owner recall", "no Agent grants")
        .unwrap();
    let foreign = f.service.create("FOREIGN", "Private library", "").unwrap();
    for library in [&kb, &foreign] {
        f.state
            .rag
            .configure_corpus(
                &library.id,
                RagSettings {
                    vector: false,
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let doc = f
        .service
        .upload(
            USER,
            &kb.id,
            "orchid.md",
            b"# Orchid\norchid owned published evidence",
        )
        .unwrap();
    f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    let private = f
        .service
        .upload(
            "FOREIGN",
            &foreign.id,
            "private.md",
            b"orchid confidential evidence",
        )
        .unwrap();
    f.service
        .index("FOREIGN", &foreign.id, &private.id)
        .await
        .unwrap();
    f.service
        .upload(USER, &kb.id, "pending.md", b"orchid unpublished evidence")
        .unwrap();
    assert!(
        f.service
            .library_agent_grants(USER, &kb.id)
            .unwrap()
            .is_empty()
    );
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let input = json!({"query":"orchid","corpusIds":[kb.id],"limit":8});
    let response = common::authenticated_request(
        &app,
        Method::POST,
        "/api/v1/rag/recall",
        Some(input.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = common::response_json(response).await;
    let evidence = data["data"]["evidence"].as_array().unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0]["source"]["documentId"], doc.id);
    assert_eq!(evidence[0]["channels"], json!(["fulltext"]));
    let response = common::authenticated_request(
        &app,
        Method::POST,
        "/api/v1/rag/recall",
        Some(json!({"query":"orchid","corpusIds":[foreign.id]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    for forged in [
        json!({"query":"orchid","agentCode":"main-agent"}),
        json!({"query":"orchid","userCode":"FOREIGN"}),
        json!({"query":"orchid","filters":{"sourceCode":private.active_source_code}}),
    ] {
        assert!(
            common::authenticated_request(&app, Method::POST, "/api/v1/rag/recall", Some(forged))
                .await
                .status()
                .is_client_error()
        );
    }
    // User-owned built-in records remain isolated even when they share a space.
    f.state
        .rag
        .configure_corpus(
            "memory-card",
            RagSettings {
                vector: false,
                ..Default::default()
            },
        )
        .unwrap();
    for (code, owner) in [("owner-memory", USER), ("foreign-memory", "FOREIGN")] {
        f.state.rag.ingest(RagIngestRequest {
            corpus_id: "memory-card".into(), workspace_code: WORKSPACE.into(),
            access_context: RagIngestAccessContext { user_code: owner.into(), workspace_code: WORKSPACE.into(), agent_code: f.agent.clone() },
            target_scope: RagIngestScope { scope_type: "SPACE".into(), scope_code: WORKSPACE.into() },
            source: RagIngestSource { code: code.into(), version: "v1".into() },
            payload: json!({"text":"orchid approved memory", "domainFields":{"memoryStatus":"ACTIVE","reviewStatus":"APPROVED","injectable":true}}),
        }).await.unwrap();
    }
    let response = common::authenticated_request(
        &app,
        Method::POST,
        "/api/v1/rag/recall",
        Some(json!({"query":"orchid","corpusIds":["memory-card"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let data = common::response_json(response).await;
    let evidence = data["data"]["evidence"].as_array().unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0]["sourceCode"], "owner-memory");
    // Management access does not bypass the existing Agent-facing search policy.
    assert_eq!(common::authenticated_request(&app, Method::POST, "/api/v1/rag/search", Some(json!({"query":"orchid","workspaceCode":WORKSPACE,"agentCode":f.agent,"corpusIds":[kb.id]}))).await.status(), StatusCode::FORBIDDEN);
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn selected_record_rebuild_keeps_other_sources_and_rejects_foreign_or_obsolete_tasks() {
    let f = Fixture::new();
    let rag = f.state.rag.clone();
    rag.configure_corpus(
        "memory-card",
        RagSettings {
            vector: false,
            ..Default::default()
        },
    )
    .unwrap();
    for (code, owner) in [
        ("selected", USER),
        ("untouched", USER),
        ("private", "FOREIGN"),
    ] {
        rag.ingest(RagIngestRequest {
            corpus_id: "memory-card".into(),
            workspace_code: WORKSPACE.into(),
            access_context: RagIngestAccessContext {
                user_code: owner.into(),
                workspace_code: WORKSPACE.into(),
                agent_code: f.agent.clone(),
            },
            target_scope: RagIngestScope {
                scope_type: "USER".into(),
                scope_code: owner.into(),
            },
            source: RagIngestSource {
                code: code.into(),
                version: "v1".into(),
            },
            payload: json!({"text":"selected record rebuild evidence","title":code}),
        })
        .await
        .unwrap();
    }
    let original = rag
        .record_task_views(USER)
        .unwrap()
        .into_iter()
        .find(|task| task.source_code == "selected")
        .unwrap();
    let foreign = rag
        .record_task_views("FOREIGN")
        .unwrap()
        .into_iter()
        .find(|task| task.source_code == "private")
        .unwrap();
    rag.configure_corpus(
        "memory-card",
        RagSettings {
            vector: false,
            embedding_dimension: 64,
            ..Default::default()
        },
    )
    .unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let response = common::authenticated_request(
        &app,
        Method::POST,
        &format!("/api/v1/rag/tasks/{}/rebuild", original.id),
        Some(json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let task_id = common::response_json(response).await["data"]["taskId"]
        .as_str()
        .unwrap()
        .to_owned();
    let repository = SqliteRagRepository::connect(&f.state.config.database_url).unwrap();
    for _ in 0..100 {
        let task = repository.record_task(USER, &task_id).unwrap().unwrap();
        if task.state == "COMPLETED" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let rebuilt = repository.record_task(USER, &task_id).unwrap().unwrap();
    assert_eq!(rebuilt.state, "COMPLETED");
    assert_eq!(rebuilt.checkpoint.unwrap().settings.embedding_dimension, 64);
    let tasks = rag.record_task_views(USER).unwrap();
    assert_eq!(
        tasks
            .iter()
            .filter(|task| task.source_code == "selected")
            .count(),
        2
    );
    assert_eq!(
        tasks
            .iter()
            .filter(|task| task.source_code == "untouched")
            .count(),
        1
    );
    for id in [&foreign.id, &original.id] {
        assert!(
            common::authenticated_request(
                &app,
                Method::POST,
                &format!("/api/v1/rag/tasks/{id}/rebuild"),
                Some(json!({}))
            )
            .await
            .status()
            .is_client_error()
        );
    }
    let queued = rag.prepare_retry_record_task(USER, &task_id, true).unwrap();
    assert!(
        common::authenticated_request(
            &app,
            Method::POST,
            &format!("/api/v1/rag/tasks/{}/rebuild", queued.id),
            Some(json!({}))
        )
        .await
        .status()
        .is_client_error()
    );
    assert_eq!(
        rag.record_task_views(USER)
            .unwrap()
            .iter()
            .filter(|task| task.source_code == "untouched")
            .count(),
        1
    );
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn platform_rag_contract_lists_automatic_corpora_and_validates_test_identity() {
    let f = Fixture::new();
    let kb = f
        .service
        .create(USER, "Unified HTTP", "same library and corpus")
        .unwrap();
    let repo = SqliteRagRepository::connect(&f.state.config.database_url).unwrap();
    // The test exercises the real route and real fulltext index without a model service.
    let rag = RagService::new(
        std::sync::Arc::new(std::sync::Mutex::new(repo)),
        DefaultRagRuntime::new(f.state.llm_runtime.clone(), f.state.llm_router.clone()),
    );
    rag.configure_corpus(
        &kb.id,
        RagSettings {
            vector: false,
            ..Default::default()
        },
    )
    .unwrap();
    let doc = f
        .service
        .upload(
            USER,
            &kb.id,
            "proof.md",
            b"# Orchid\norchid authorized HTTP evidence",
        )
        .unwrap();
    assert_eq!(
        f.service.index(USER, &kb.id, &doc.id).await.unwrap().status,
        "READY"
    );
    f.service
        .set_grants(USER, &kb.id, vec![f.agent.clone()])
        .unwrap();
    let mut state = f.state.clone();
    state.knowledge_bases = f.service.clone();
    let app = tron_backend::app::build_router_with_state(state).unwrap();
    let response =
        common::authenticated_request(&app, Method::GET, "/api/v1/rag/corpora", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let result = common::response_json(response).await;
    let rows = result["data"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().any(|c| c["id"] == kb.id
        && c["knowledgeBaseId"] == kb.id
        && c["counts"]["chunkCount"] == 1));
    let input = json!({"workspaceCode":WORKSPACE,"agentCode":f.agent,"query":"orchid","corpusIds":[kb.id],"limit":8});
    let response = common::authenticated_request(
        &app,
        Method::POST,
        "/api/v1/rag/search",
        Some(input.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result = common::response_json(response).await;
    assert_eq!(
        result["data"]["evidence"][0]["source"]["documentId"],
        doc.id
    );
    assert_eq!(
        result["data"]["evidence"][0]["channels"],
        json!(["fulltext"])
    );
    let tool = tron_backend::application::agent::rag_search_tool(rag.clone(), f.service.clone());
    let context: tron_backend::domain::agent::AgentRunContext = serde_json::from_value(json!({
        "agentCode":f.agent,"conversationCode":null,"runCode":"rag-tool-test","workspaceCode":WORKSPACE,"userCode":USER,"locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"fixture-chat","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture"},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":2,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[],"observations":[],"startedAt":"fixture"
    })).unwrap();
    let call = tron_backend::domain::agent::ToolRequest {
        tool_name: "rag_search".into(),
        invocation_id: None,
        input: json!({"query":"orchid","corpusIds":[kb.id]}),
    };
    let output = tool
        .exec_async(call.clone(), &context, &SearchControl)
        .unwrap()
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_str(&output.observation.output).unwrap();
    assert_eq!(result["evidence"][0]["source"]["documentId"], doc.id);
    let mut forged = input.clone();
    forged["agentCode"] = json!("unassigned-agent");
    assert_eq!(
        common::authenticated_request(&app, Method::POST, "/api/v1/rag/search", Some(forged))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    f.service.set_grants(USER, &kb.id, vec![]).unwrap();
    assert!(
        tool.exec_async(call, &context, &SearchControl)
            .unwrap()
            .await
            .is_err()
    );
    assert_eq!(
        common::authenticated_request(&app, Method::POST, "/api/v1/rag/search", Some(input))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let invalid = json!({"fulltext":false,"vector":false,"graph":false});
    assert_eq!(
        common::authenticated_request(
            &app,
            Method::PUT,
            &format!("/api/v1/rag/corpora/{}/settings", kb.id),
            Some(invalid)
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        common::authenticated_request(
            &app,
            Method::POST,
            "/api/v1/rag/tasks",
            Some(json!({"corpusId":kb.id,"documentId":"missing-document"}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    for path in [
        "/api/v1/rag/ingest-profiles",
        "/api/v1/rag/search-profiles",
        "/api/v1/rag/strategies",
    ] {
        assert_eq!(
            common::authenticated_request(&app, Method::GET, path, None)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn memory_search_enforces_source_scope_governance_expiry_and_latest_retry() {
    let f = Fixture::new();
    let rag = RagService::new(
        std::sync::Arc::new(std::sync::Mutex::new(
            SqliteRagRepository::connect(&f.state.config.database_url).unwrap(),
        )),
        DefaultRagRuntime::new(f.state.llm_runtime.clone(), f.state.llm_router.clone()),
    );
    rag.configure_corpus(
        "memory-card",
        RagSettings {
            vector: false,
            ..Default::default()
        },
    )
    .unwrap();
    for (code, user, scope, expiry, review) in [
        ("visible", USER, WORKSPACE, None, "APPROVED"),
        ("other-user", "FOREIGN", WORKSPACE, None, "APPROVED"),
        ("other-space", USER, "OTHER-SPACE", None, "APPROVED"),
        ("expired", USER, WORKSPACE, Some(1i64), "APPROVED"),
        ("rejected", USER, WORKSPACE, None, "REJECTED"),
    ] {
        let scope_type = if code == "other-user" {
            "USER"
        } else {
            "SPACE"
        };
        rag.ingest(RagIngestRequest {
            corpus_id: "memory-card".into(), workspace_code: scope.into(),
            access_context: RagIngestAccessContext { user_code: user.into(),workspace_code: scope.into(),agent_code:f.agent.clone() },
            target_scope: RagIngestScope { scope_type: scope_type.into(), scope_code: if scope_type=="USER" {user.into()} else {scope.into()} },
            source: RagIngestSource { code:code.into(),version:"v1".into() },
            payload: json!({"text":"orchid memory evidence","title":code,"domainFields":{"memoryType":"FACT","memoryStatus":"ACTIVE","reviewStatus":review,"injectable":true,"expiresAt":expiry}}),
        }).await.unwrap();
    }
    let identity = RagIdentity {
        user_code: USER.into(),
        workspace_code: WORKSPACE.into(),
        agent_code: f.agent.clone(),
    };
    let search = |filters| RagQueryInput {
        query: "orchid".into(),
        corpus_ids: vec!["memory-card".into()],
        filters,
        limit: 8,
    };
    let hits = rag
        .search(&f.service, &identity, search(json!({})), None)
        .await
        .unwrap();
    assert_eq!(hits.evidence.len(), 1, "{:?}", hits.warnings);
    assert_eq!(hits.evidence[0].source_code, "visible");
    assert!(
        rag.search(
            &f.service,
            &identity,
            search(json!({"memoryStatus":"DISABLED"})),
            None
        )
        .await
        .unwrap()
        .evidence
        .is_empty()
    );
    let tasks = rag.record_task_views(USER).unwrap();
    let original = tasks.iter().find(|t| t.source_code == "visible").unwrap();
    let next = rag
        .prepare_retry_record_task(USER, &original.id, true)
        .unwrap();
    assert_eq!(next.state, "QUEUED");
    rag.execute_record_task(next).await.unwrap();
    // An old task cannot restore stale content after a newer source publication.
    assert!(
        rag.prepare_retry_record_task(USER, &original.id, false)
            .is_err()
    );
    let rows = rag.corpus_summaries(&Default::default()).unwrap();
    let memory = rows.iter().find(|r| r.id == "memory-card").unwrap();
    assert_eq!(memory.counts.rebuild_source_count, 0);
    rag.configure_corpus("memory-card", RagSettings::default())
        .unwrap();
    assert!(
        rag.corpus_summaries(&Default::default())
            .unwrap()
            .iter()
            .find(|r| r.id == "memory-card")
            .unwrap()
            .counts
            .rebuild_source_count
            > 0
    );
    f.state.agent_framework.close().await;
}
