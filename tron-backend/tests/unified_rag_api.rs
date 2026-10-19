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
    let response =
        common::authenticated_request(&app, Method::GET, "/api/v1/rag/tasks", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let result = common::response_json(response).await;
    let task = result["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["documentId"] == doc.id)
        .unwrap();
    assert_eq!(
        task["fileSizeBytes"],
        b"# Orchid\norchid authorized HTTP evidence".len()
    );
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
    rag.configure_corpus(
        "memory-card",
        RagSettings {
            vector: true,
            embedding_model_code: Some("PERSON_AI:ROUTE:EmbeddingAuto".into()),
            ..Default::default()
        },
    )
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

#[tokio::test]
async fn corpus_automatic_routes_are_saved_and_read_back() {
    let f = Fixture::new();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let settings = json!({"fulltext":true,"vector":true,"graph":true,"embeddingDimension":1024,
        "embeddingModelCode":"PERSON_AI:ROUTE:EmbeddingAuto", "graphModelCode":"PERSON_AI:ROUTE:ChatAuto", "rerankModelCode":"PERSON_AI:ROUTE:RerankAuto"});
    let response = common::authenticated_request(
        &app,
        Method::PUT,
        "/api/v1/rag/corpora/memory-card/settings",
        Some(settings.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response =
        common::authenticated_request(&app, Method::GET, "/api/v1/rag/corpora", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = common::response_json(response).await;
    let corpus = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|corpus| corpus["id"] == "memory-card")
        .unwrap();
    for key in ["embeddingModelCode", "graphModelCode", "rerankModelCode"] {
        assert_eq!(corpus["settings"][key], settings[key]);
    }
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn shared_model_picker_exposes_four_typed_automatic_routes() {
    let f = Fixture::new();
    rusqlite::Connection::open(&f.state.config.database_url).unwrap().execute_batch(
        "INSERT INTO provider_connections(provider_code,server_base_url,status,created_at,updated_at) VALUES('PersonAi','https://synthetic.invalid','Connected',1,1) ON CONFLICT(provider_code) DO UPDATE SET status='Connected';"
    ).unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let response = common::authenticated_request(
        &app,
        Method::GET,
        &format!("/api/v1/models/workspaces/{WORKSPACE}/select-options"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = common::response_json(response).await;
    let options = body["data"].as_array().unwrap();
    for (route, model_type, name) in [
        ("ChatAuto", "TEXT", "对话自动路由"),
        ("EmbeddingAuto", "EMBEDDING", "向量自动路由"),
        ("MultimodalEmbeddingAuto", "EMBEDDING", "多模态向量自动路由"),
        ("RerankAuto", "RERANK", "重排自动路由"),
    ] {
        let option = options
            .iter()
            .find(|option| option["modelCode"] == format!("PERSON_AI:ROUTE:{route}"))
            .unwrap();
        assert_eq!(option["modelType"], model_type);
        assert_eq!(option["displayName"], name);
        assert_eq!(option["modelSelection"]["routeCode"], route);
        assert_eq!(option["enabled"], true);
    }
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn builtins_and_new_library_persist_automatic_model_defaults() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Automatic defaults", "").unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let response =
        common::authenticated_request(&app, Method::GET, "/api/v1/rag/corpora", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = common::response_json(response).await;
    let connection = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    for id in ["memory-card", "skill", kb.id.as_str()] {
        let corpus = body["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|corpus| corpus["id"] == id)
            .unwrap();
        let saved: String = connection
            .query_row(
                "SELECT capabilities_json FROM rag_corpora WHERE corpus_id=?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
        if id == kb.id {
            assert_eq!(corpus["settings"]["graph"], true);
            assert_eq!(saved["settings"]["graph"], true);
        }
        for (field, route) in [
            (
                "embeddingModelCode",
                if id == kb.id {
                    "MultimodalEmbeddingAuto"
                } else {
                    "EmbeddingAuto"
                },
            ),
            ("graphModelCode", "ChatAuto"),
            ("rerankModelCode", "RerankAuto"),
        ] {
            assert_eq!(
                corpus["settings"][field],
                format!("PERSON_AI:ROUTE:{route}")
            );
            assert_eq!(saved["settings"][field], corpus["settings"][field]);
        }
    }
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn deleting_task_history_preserves_publication_and_latest_fences() {
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
    let request = |owner: &str, code: &str| RagIngestRequest {
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
        payload: json!({"title":code,"text":"orchid history evidence","domainFields":{"memoryStatus":"ACTIVE","reviewStatus":"APPROVED","injectable":true}}),
    };
    rag.ingest(request(USER, "history-source")).await.unwrap();
    let old = rag
        .record_task_views(USER)
        .unwrap()
        .into_iter()
        .find(|t| t.source_code == "history-source")
        .unwrap();
    rag.ingest(request(USER, "history-source")).await.unwrap();
    rag.ingest(request("FOREIGN", "private-history"))
        .await
        .unwrap();
    let repository = SqliteRagRepository::connect(&f.state.config.database_url).unwrap();
    let foreign = repository.record_tasks("FOREIGN").unwrap().remove(0);
    let latest = rag
        .record_task_views(USER)
        .unwrap()
        .into_iter()
        .find(|t| t.source_code == "history-source" && t.is_latest)
        .unwrap();
    assert_ne!(latest.id, old.id);
    let mut active = repository.record_task(USER, &latest.id).unwrap().unwrap();
    active.id = "active-history-task".into();
    active.source_code = "active-history".into();
    active.state = "QUEUED".into();
    repository.save_record_task(&active).unwrap();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    for (id, status) in [
        (&foreign.id, StatusCode::NOT_FOUND),
        (&active.id, StatusCode::CONFLICT),
    ] {
        assert_eq!(
            common::authenticated_request(
                &app,
                Method::DELETE,
                &format!("/api/v1/rag/tasks/{id}"),
                None
            )
            .await
            .status(),
            status
        );
    }
    for (locale, message) in [
        ("zh-CN", "请先取消运行中的任务，再删除记录"),
        ("en-US", "Cancel the active task before deleting its record"),
    ] {
        use tower::ServiceExt;
        let request = axum::http::Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/rag/tasks/{}", active.id))
            .header("authorization", "Bearer profile-test-token")
            .header("accept-language", locale)
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let error = common::response_json(response).await;
        assert_eq!(error["code"], "101033002");
        assert_eq!(error["message"], message);
        assert_eq!(error["data"], json!({}));
    }
    let before = repository.index_counts("memory-card").unwrap();
    for _ in 0..2 {
        assert_eq!(
            common::authenticated_request(
                &app,
                Method::DELETE,
                &format!("/api/v1/rag/tasks/{}", latest.id),
                None
            )
            .await
            .status(),
            StatusCode::OK
        );
    }
    let response =
        common::authenticated_request(&app, Method::GET, "/api/v1/rag/tasks", None).await;
    let data = common::response_json(response).await;
    let visible = data["data"].as_array().unwrap();
    assert!(!visible.iter().any(|t| t["id"] == latest.id));
    assert_eq!(
        visible.iter().find(|t| t["id"] == old.id).unwrap()["isLatest"],
        false
    );
    assert!(repository.record_task(USER, &latest.id).unwrap().is_some());
    assert!(
        repository
            .deleted_task_history("FOREIGN")
            .unwrap()
            .is_empty()
    );
    assert!(rag.prepare_retry_record_task(USER, &old.id, false).is_err());
    assert_eq!(
        repository.index_counts("memory-card").unwrap().chunk_count,
        before.chunk_count
    );
    let response = common::authenticated_request(
        &app,
        Method::POST,
        "/api/v1/rag/recall",
        Some(json!({"query":"orchid","corpusIds":["memory-card"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        !common::response_json(response).await["data"]["evidence"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // A knowledge-base task is execution state for the published document as well.
    let kb = f.service.create(USER, "History library", "").unwrap();
    f.state
        .rag
        .configure_corpus(
            &kb.id,
            RagSettings {
                vector: false,
                ..Default::default()
            },
        )
        .unwrap();
    let doc = f
        .service
        .upload(USER, &kb.id, "history.md", b"orchid published document")
        .unwrap();
    f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    let task = f
        .service
        .rag_task_views(USER)
        .unwrap()
        .into_iter()
        .find(|t| t.document_id.as_deref() == Some(&doc.id))
        .unwrap();
    assert_eq!(
        common::authenticated_request(
            &app,
            Method::DELETE,
            &format!("/api/v1/rag/tasks/{}", task.id),
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(f.service.detail(USER, &kb.id).unwrap().documents.len(), 1);
    assert!(
        !f.service
            .ingestion_tasks(USER, &kb.id, &doc.id)
            .unwrap()
            .is_empty()
    );
    let reopened = SqliteRagRepository::connect(&f.state.config.database_url).unwrap();
    assert!(
        reopened
            .deleted_task_history(USER)
            .unwrap()
            .contains(&task.id)
    );
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn task_summaries_keep_latest_sources_outside_recent_history_without_loading_vectors() {
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
    rag.ingest(RagIngestRequest {
        corpus_id: "memory-card".into(),
        workspace_code: WORKSPACE.into(),
        access_context: RagIngestAccessContext {
            user_code: USER.into(),
            workspace_code: WORKSPACE.into(),
            agent_code: f.agent.clone(),
        },
        target_scope: RagIngestScope {
            scope_type: "USER".into(),
            scope_code: USER.into(),
        },
        source: RagIngestSource {
            code: "quiet-source".into(),
            version: "v1".into(),
        },
        payload: json!({"title":"same title","text":"orchid summary"}),
    })
    .await
    .unwrap();
    let repository = SqliteRagRepository::connect(&f.state.config.database_url).unwrap();
    let mut task = repository.record_tasks(USER).unwrap().remove(0);
    // Many attempts of another source must not displace this source's latest record.
    for i in 0..205 {
        task.id = format!("summary-test-{i}");
        task.source_code = "busy-source".into();
        task.created_at = i;
        task.updated_at = 205 - i;
        task.checkpoint.as_mut().unwrap().vectors.vectors = vec![vec![1.0; 1024]];
        repository.save_record_task(&task).unwrap();
    }
    let views = rag.record_task_views(USER).unwrap();
    assert_eq!(views.iter().filter(|t| t.is_latest).count(), 2);
    assert!(
        views
            .iter()
            .any(|t| t.source_code == "quiet-source" && t.is_latest)
    );
    assert!(
        views
            .iter()
            .any(|t| t.id == "summary-test-204" && t.is_latest)
    );
    let summaries = repository.record_task_summaries(USER).unwrap();
    assert!(summaries.len() <= 201);
    assert!(summaries.iter().all(|t| t.payload.get("text").is_none()
        && t.checkpoint.as_ref().unwrap().vectors.vectors.is_empty()));
    assert_eq!(
        repository
            .record_task(USER, "summary-test-204")
            .unwrap()
            .unwrap()
            .checkpoint
            .unwrap()
            .vectors
            .vectors
            .len(),
        1
    );
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn knowledge_library_create_persists_selected_settings_before_first_ingestion() {
    let f = Fixture::new();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let settings = RagSettings {
        vector: false,
        graph: true,
        graph_model_code: Some("selected-chat".into()),
        rerank_model_code: None,
        ..Default::default()
    };
    let response=common::authenticated_request(&app,Method::POST,"/api/v1/knowledge-bases",Some(json!({"name":"Created with settings","description":"Direct setup","indexSettings":settings}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let created = common::response_json(response).await;
    assert_eq!(created["code"], "0");
    let id = created["data"]["id"].as_str().unwrap();
    assert_eq!(created["data"]["documentCount"], 0);
    assert_eq!(f.state.rag.settings_for_corpus(id).unwrap(), settings);
    let response =
        common::authenticated_request(&app, Method::GET, "/api/v1/rag/corpora", None).await;
    let listed = common::response_json(response).await;
    assert_eq!(
        listed["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap()["settings"],
        json!(settings)
    );
    let doc = f
        .service
        .upload(USER, id, "first.md", b"orchid first document")
        .unwrap();
    f.service.start_index(USER, id, &doc.id).unwrap();
    let task = f
        .service
        .ingestion_tasks(USER, id, &doc.id)
        .unwrap()
        .remove(0);
    assert_eq!(task.configuration["ragSettings"], json!(settings));
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn invalid_creation_index_settings_leave_no_library_or_corpus() {
    let f = Fixture::new();
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let before = f.service.platform_libraries(USER).unwrap().len();
    for invalid in [
        RagSettings {
            fulltext: false,
            vector: false,
            graph: false,
            ..Default::default()
        },
        RagSettings {
            vector: true,
            embedding_model_code: None,
            ..Default::default()
        },
        RagSettings {
            graph: true,
            graph_model_code: None,
            ..Default::default()
        },
        RagSettings {
            embedding_dimension: 0,
            ..Default::default()
        },
    ] {
        let response = common::authenticated_request(
            &app,
            Method::POST,
            "/api/v1/knowledge-bases",
            Some(json!({"name":"Invalid draft","indexSettings":invalid})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = common::response_json(response).await;
        assert_eq!(error["code"], "101027005");
        assert_eq!(error["data"], json!({}));
        assert_eq!(f.service.platform_libraries(USER).unwrap().len(), before);
    }
    f.state.agent_framework.close().await;
}

#[tokio::test]
async fn builtin_task_chunks_are_paged_private_and_exclude_obsolete_attempts() {
    let f = Fixture::new();
    let rag = &f.state.rag;
    for corpus in ["memory-card", "skill"] {
        rag.configure_corpus(
            corpus,
            RagSettings {
                vector: false,
                graph: false,
                ..Default::default()
            },
        )
        .unwrap();
    }
    let request = |owner: &str, corpus: &str, code: &str| RagIngestRequest {
        corpus_id: corpus.into(),
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
        payload: json!({"title":code,"text":"orchid chunk evidence","domainFields":if corpus == "skill" { json!({"skillName":code,"skillDescription":"fixture"}) } else { json!({"memoryStatus":"ACTIVE","reviewStatus":"APPROVED","injectable":true}) }}),
    };
    rag.ingest(request(USER, "memory-card", "memory-chunks"))
        .await
        .unwrap();
    let old = rag
        .record_task_views(USER)
        .unwrap()
        .into_iter()
        .find(|t| t.source_code == "memory-chunks")
        .unwrap();
    rag.ingest(request(USER, "memory-card", "memory-chunks"))
        .await
        .unwrap();
    rag.ingest(request("SYSTEM", "skill", "shared-skill-chunks"))
        .await
        .unwrap();
    rag.ingest(request("FOREIGN", "memory-card", "private-memory-chunks"))
        .await
        .unwrap();
    let repository = SqliteRagRepository::connect(&f.state.config.database_url).unwrap();
    let foreign = repository
        .record_task_summaries("FOREIGN")
        .unwrap()
        .remove(0);
    let tasks = rag.record_task_views(USER).unwrap();
    let latest = tasks
        .iter()
        .find(|t| t.source_code == "memory-chunks" && t.is_latest)
        .unwrap();
    let skill = tasks
        .iter()
        .find(|t| t.source_code == "shared-skill-chunks")
        .unwrap();
    let summary = repository
        .record_task_summary(USER, &latest.id)
        .unwrap()
        .unwrap();
    assert!(summary.checkpoint.is_none());
    assert!(summary.payload.get("text").is_none());
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    for task in [latest, skill] {
        let response = common::authenticated_request(
            &app,
            Method::GET,
            &format!("/api/v1/rag/tasks/{}/chunks?current=1&size=1", task.id),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = common::response_json(response).await;
        assert_eq!(body["code"], "0");
        assert_eq!(body["data"]["current"], 1);
        assert_eq!(body["data"]["size"], 1);
        assert_eq!(body["data"]["records"][0]["text"], "orchid chunk evidence");
        assert!(body["data"]["records"][0].get("embedding").is_none());
        let response = common::authenticated_request(
            &app,
            Method::GET,
            &format!("/api/v1/rag/tasks/{}/chunks?current=2&size=100", task.id),
            None,
        )
        .await;
        assert!(
            common::response_json(response).await["data"]["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    for (id, paging, status) in [
        (
            foreign.id.as_str(),
            "current=1&size=15",
            StatusCode::NOT_FOUND,
        ),
        (old.id.as_str(), "current=1&size=15", StatusCode::CONFLICT),
        (
            latest.id.as_str(),
            "current=0&size=15",
            StatusCode::BAD_REQUEST,
        ),
        (
            latest.id.as_str(),
            "current=1&size=101",
            StatusCode::BAD_REQUEST,
        ),
        (
            latest.id.as_str(),
            "current=invalid&size=15",
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = common::authenticated_request(
            &app,
            Method::GET,
            &format!("/api/v1/rag/tasks/{id}/chunks?{paging}"),
            None,
        )
        .await;
        assert_eq!(response.status(), status);
        assert_eq!(common::response_json(response).await["data"], json!({}));
    }
    f.state.agent_framework.close().await;
}
