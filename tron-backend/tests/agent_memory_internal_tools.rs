mod common;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    body::Body,
    extract::OriginalUri,
    http::{Method, Request, StatusCode, header},
    routing::post,
};
use serde_json::{Map, Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::ServiceExt;

use common::{app, authenticated_bearer_token, request_with_headers, response_json};

const MEMORY_CODE: &str = "S:TRON_CORE:citation-preference:v1";
const USER_MEMORY_CODE: &str = "U:USR-AUTH-0001:user-citation-preference:v1";
const WORK_DOMAIN_MEMORY_CODE: &str = "D:main-domain:domain-citation-preference:v1";
const MEMORY_TEXT_INITIAL: &str = "Always cite sources in workspace answers.";
const MEMORY_TEXT_UPDATED: &str = "Prefer concise source citations in workspace answers.";

struct RecordingModelServer {
    base_url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    _task: JoinHandle<()>,
}

async fn recording_model_server() -> RecordingModelServer {
    scripted_model_server(Vec::new()).await
}

async fn scripted_model_server(responses: Vec<Value>) -> RecordingModelServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("agent model mock server should bind");
    let address = listener
        .local_addr()
        .expect("agent model mock server should expose address");
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    let app = Router::new()
        .route("/v1/chat/completions", post(scripted_chat_completion))
        .route("/v1/embeddings", post(mock_embedding))
        .route("/v1/v1/embeddings", post(mock_embedding))
        .route("/embeddings", post(mock_embedding))
        .route("/v1/rerank", post(mock_rerank))
        .route("/rerank", post(mock_rerank))
        .with_state((requests.clone(), responses));
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("agent model mock server should stay available");
    });
    RecordingModelServer {
        base_url: format!("http://{address}/v1"),
        requests,
        _task: task,
    }
}

async fn scripted_chat_completion(
    axum::extract::State((requests, responses)): axum::extract::State<(
        Arc<Mutex<Vec<Value>>>,
        Arc<Mutex<VecDeque<Value>>>,
    )>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    requests
        .lock()
        .expect("recorded model request mutex poisoned")
        .push(payload.clone());
    let response = responses
        .lock()
        .expect("scripted model response mutex poisoned")
        .pop_front()
        .unwrap_or_else(|| {
            let prompt = payload["messages"]
                .as_array()
                .and_then(|messages| messages.last())
                .and_then(|message| message["content"].as_str())
                .unwrap_or_default();
            final_response(format!("mock: {prompt}"))
        });
    Json(response)
}

async fn mock_embedding(
    uri: OriginalUri,
    Json(_payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let _ = uri;
    let mut embedding = vec![0.0; 4096];
    embedding[0] = 1.0;
    embedding[1] = 0.25;
    (
        StatusCode::OK,
        Json(json!({
            "data": [{
                "embedding": embedding
            }]
        })),
    )
}

async fn mock_rerank(Json(payload): Json<Value>) -> (StatusCode, Json<Value>) {
    let results = payload["documents"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, _)| {
            json!({
                "index": index,
                "relevance_score": 1.0 - ((index as f64) * 0.01)
            })
        })
        .collect::<Vec<_>>();
    (StatusCode::OK, Json(json!({ "results": results })))
}

#[tokio::test]
async fn memory_tool_catalog_is_visible_and_zh_cn_localized() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let domains_response = request_with_locale(
        &app,
        Method::GET,
        "/api/v1/tools/domains",
        None,
        &authorization,
        "zh-CN",
    )
    .await;
    let domains_status = domains_response.status();
    let domains_body = response_json(domains_response).await;
    assert_eq!(
        domains_status,
        StatusCode::OK,
        "unexpected domains response body: {domains_body}"
    );
    assert_eq!(domains_body["code"], json!("SUCCESS"));
    let memory_domain = find_record(&domains_body["data"], "domainCode", "memory")
        .expect("memory tool domain should be visible");
    assert!(contains_cjk(&memory_domain["name"]));
    assert!(contains_cjk(&memory_domain["description"]));
    assert!(
        memory_domain["toolCount"]
            .as_u64()
            .is_some_and(|count| count >= 3)
    );

    let tools_response = request_with_locale(
        &app,
        Method::GET,
        "/api/v1/tools?domainCode=memory",
        None,
        &authorization,
        "zh-CN",
    )
    .await;
    let tools_status = tools_response.status();
    let tools_body = response_json(tools_response).await;
    assert_eq!(
        tools_status,
        StatusCode::OK,
        "unexpected tools response body: {tools_body}"
    );
    assert_eq!(tools_body["code"], json!("SUCCESS"));
    for tool_code in ["memory.search", "memory.save", "memory.delete"] {
        let tool = find_record(&tools_body["data"], "toolCode", tool_code)
            .unwrap_or_else(|| panic!("{tool_code} should be visible"));
        assert_eq!(tool["domainCode"], json!("memory"));
        assert_eq!(tool["availabilityState"], json!("AVAILABLE"));
        assert!(contains_cjk(&tool["name"]));
        assert!(contains_cjk(&tool["description"]));
    }

    let save_response = request_with_locale(
        &app,
        Method::GET,
        "/api/v1/tools/memory.save",
        None,
        &authorization,
        "zh-CN",
    )
    .await;
    let save_status = save_response.status();
    let save_body = response_json(save_response).await;
    assert_eq!(
        save_status,
        StatusCode::OK,
        "unexpected memory.save detail response body: {save_body}"
    );
    let save_schema = &save_body["data"]["inputSchema"];
    assert!(save_schema["properties"].get("memoryCode").is_some());
    assert!(contains_cjk(
        &save_schema["properties"]["memoryCode"]["description"]
    ));
    assert_json_mentions(save_schema, "WORK_DOMAIN");
    assert_json_does_not_mention(save_schema, "U/A/S");
    assert_json_does_not_mention(save_schema, "U|A|S");
    assert_json_does_not_mention(save_schema, "A:");
    assert_json_does_not_mention(save_schema, "main-agent");
    assert!(save_schema["properties"].get("pinLevel").is_none());
    assert_json_does_not_mention(save_schema, "pinLevel");
    assert!(save_schema["properties"].get("sourceCode").is_none());
    assert!(save_schema["properties"].get("sourceVersion").is_none());
}

#[tokio::test]
async fn main_agent_default_all_platform_tools_injects_memory_tools() {
    let model_server = recording_model_server().await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(&model_server.base_url, None, json!({}))),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected default all-platform run response body: {body}"
    );

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 1);
    let tool_names = tool_names_in_request(&requests[0]);
    assert!(tool_names.contains(&"memory_search".to_string()));
    assert!(tool_names.contains(&"memory_save".to_string()));
    assert!(tool_names.contains(&"memory_delete".to_string()));
    assert!(tool_names.contains(&"automation_create".to_string()));
}

#[tokio::test]
async fn selected_tools_without_memory_selection_does_not_inject_memory_tools() {
    let model_server = recording_model_server().await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let patch_response = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({
            "toolAssignmentMode": "selected_tools",
            "toolExposureMode": "direct_list"
        })),
        None,
        Some(&authorization),
    )
    .await;
    let patch_status = patch_response.status();
    let patch_body = response_json(patch_response).await;
    assert_eq!(
        patch_status,
        StatusCode::OK,
        "unexpected selected-tools policy response body: {patch_body}"
    );

    let assign_response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/tool-assignments",
        Some(json!({ "toolCodes": [] })),
        None,
        Some(&authorization),
    )
    .await;
    let assign_status = assign_response.status();
    let assign_body = response_json(assign_response).await;
    assert_eq!(
        assign_status,
        StatusCode::OK,
        "unexpected selected-tools assignment response body: {assign_body}"
    );

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(&model_server.base_url, None, json!({}))),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected selected-tools run response body: {body}"
    );

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 1);
    let tool_names = tool_names_in_request(&requests[0]);
    assert!(!tool_names.contains(&"memory_search".to_string()));
    assert!(!tool_names.contains(&"memory_save".to_string()));
    assert!(!tool_names.contains(&"memory_delete".to_string()));
}

#[tokio::test]
async fn selected_tools_only_injects_selected_memory_tools() {
    let model_server = recording_model_server().await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let patch_response = request_with_headers(
        &app,
        Method::PATCH,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/policy",
        Some(json!({
            "toolAssignmentMode": "selected_tools",
            "toolExposureMode": "direct_list"
        })),
        None,
        Some(&authorization),
    )
    .await;
    let patch_status = patch_response.status();
    let patch_body = response_json(patch_response).await;
    assert_eq!(
        patch_status,
        StatusCode::OK,
        "unexpected selected-tools policy response body: {patch_body}"
    );

    let assign_response = request_with_headers(
        &app,
        Method::PUT,
        "/api/v1/agents/workspaces/TRON_CORE/internal/main-agent/tool-assignments",
        Some(json!({ "toolCodes": ["memory.search", "workspace.page"] })),
        None,
        Some(&authorization),
    )
    .await;
    let assign_status = assign_response.status();
    let assign_body = response_json(assign_response).await;
    assert_eq!(
        assign_status,
        StatusCode::OK,
        "unexpected memory selected-tools assignment response body: {assign_body}"
    );
    assert_eq!(
        assign_body["data"]["toolAssignmentMode"],
        json!("SELECTED_TOOLS")
    );
    assert_eq!(
        assign_body["data"]["selectedToolCodes"],
        json!(["memory.search", "workspace.page"])
    );

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(&model_server.base_url, None, json!({}))),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected selected memory run response body: {body}"
    );

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 1);
    let tool_names = tool_names_in_request(&requests[0]);
    assert!(tool_names.contains(&"memory_search".to_string()));
    assert!(tool_names.contains(&"workspace_page".to_string()));
    assert!(!tool_names.contains(&"memory_save".to_string()));
    assert!(!tool_names.contains(&"memory_delete".to_string()));
}

#[test]
fn memory_current_lookup_uses_standard_rag_search_boundary() {
    let memory_service = include_str!("../src/application/memory/service.rs");
    let memory_code = include_str!("../src/application/memory/code.rs");
    let rag_facade = include_str!("../src/application/rag/facade.rs");
    let rag_mod = include_str!("../src/application/rag.rs");

    for source in [memory_service, memory_code, rag_facade, rag_mod] {
        for forbidden in [
            ["load", "_", "memory", "_", "current"].concat(),
            ["Rag", "Memory", "Current"].concat(),
            ["memory", "-", "current"].concat(),
        ] {
            assert!(
                !source.contains(&forbidden),
                "memory current lookup must not use the removed RAG helper {forbidden}"
            );
        }
    }

    for forbidden in [
        ["FULLTEXT_SQLITE", "_LIKE_V1"].concat(),
        ["repository", ".", "search", "_", "route", "("].concat(),
    ] {
        assert!(
            !rag_facade.contains(&forbidden),
            "RAG facade must not keep removed direct route lookup {forbidden}"
        );
    }
    assert!(memory_service.contains("RagSearchRequest"));
    assert!(memory_service.contains("MEMORY_SEARCH_PROFILE_ID"));
    assert!(memory_service.contains("\"RESULT_LIST\""));
}

#[test]
fn public_work_domain_memory_scope_keeps_agent_rag_wire_contract() {
    let memory_code = include_str!("../src/application/memory/code.rs");
    let memory_payload = include_str!("../src/application/memory/payload.rs");

    assert!(memory_code.contains("MAIN_WORK_DOMAIN_CODE"));
    assert!(memory_code.contains("\"main-domain\""));
    assert!(memory_code.contains("\"D\""));
    assert!(memory_code.contains("\"WORK_DOMAIN\""));
    assert!(
        memory_code.contains("Self::WorkDomain => \"AGENT\""),
        "WORK_DOMAIN public memory must continue to use AGENT as the internal RAG wire scope"
    );
    assert!(
        memory_payload.contains("\"AGENT\" => \"WORK_DOMAIN\""),
        "AGENT RAG hits must be translated back to WORK_DOMAIN for public memory output"
    );
}

#[tokio::test]
async fn unauthorized_memory_tool_call_is_rejected_without_aborting_run() {
    let model_server = scripted_model_server(vec![
        tool_call_response(
            "memory.delete",
            json!({
                "memoryCode": MEMORY_CODE
            }),
        ),
        final_response("recovered after unauthorized memory tool"),
    ])
    .await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    create_embedding_default(&app, &authorization, &model_server.base_url).await;

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(
            &model_server.base_url,
            Some(vec![effective_tool("memory.search")]),
            json!({ "maxSteps": 2, "maxToolCalls": 1 }),
        )),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected unauthorized response body: {body}"
    );
    assert_eq!(body["code"], json!("SUCCESS"));
    assert_eq!(
        body["data"]["finalOutput"],
        json!("recovered after unauthorized memory tool")
    );
    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 2);
    assert_json_mentions(&requests[1]["messages"], "memory_delete");
    assert_json_mentions(&requests[1]["messages"], "is not allowed");
}

#[tokio::test]
async fn invalid_memory_code_short_formats_are_rejected() {
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    for (case_name, input) in [
        (
            "missing scope prefix",
            json!({ "memoryCode": "TRON_CORE:citation-preference:v1" }),
        ),
        (
            "unknown scope prefix",
            json!({ "memoryCode": "X:TRON_CORE:citation-preference:v1" }),
        ),
        (
            "missing source version",
            json!({ "memoryCode": "S:TRON_CORE:citation-preference" }),
        ),
        (
            "blank scope code",
            json!({ "memoryCode": "S::citation-preference:v1" }),
        ),
        (
            "unsupported source version",
            json!({ "memoryCode": "S:TRON_CORE:citation-preference:v2" }),
        ),
        (
            "legacy public agent memory code",
            json!({ "memoryCode": "A:main-agent:agent-citation-preference:v1" }),
        ),
        (
            "legacy public agent scope",
            json!({
                "query": "legacy agent scope",
                "scope": "AGENT",
                "scopeCode": "main-agent"
            }),
        ),
    ] {
        let model_server =
            scripted_model_server(vec![tool_call_response("memory.search", input)]).await;

        let response = request_with_headers(
            &app,
            Method::POST,
            "/api/v1/agents/runs",
            Some(agent_run_payload(
                &model_server.base_url,
                Some(vec![effective_tool("memory.search")]),
                json!({ "maxSteps": 2, "maxToolCalls": 1 }),
            )),
            None,
            Some(&authorization),
        )
        .await;
        let status = response.status();
        let body = response_json(response).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "invalid memory short-format case `{case_name}` returned unexpected body: {body}"
        );
        assert_eq!(body["code"], json!("SUCCESS"));
        let final_output = body["data"]["finalOutput"].as_str().unwrap_or_default();
        assert!(
            final_output.contains("memoryCode") || final_output.contains("scope"),
            "invalid memory short-format case `{case_name}` should fail at validation, got: {final_output}"
        );
    }
}

#[tokio::test]
async fn memory_save_rejects_removed_pin_level_public_input() {
    let model_server = scripted_model_server(vec![tool_call_response(
        "memory.save",
        json!({
            "memoryCode": WORK_DOMAIN_MEMORY_CODE,
            "text": "Pinned memory input should be rejected.",
            "pinLevel": 9
        }),
    )])
    .await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(
            &model_server.base_url,
            Some(vec![effective_tool("memory.save")]),
            json!({ "maxSteps": 2, "maxToolCalls": 1 }),
        )),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected removed pinLevel input response body: {body}"
    );
    assert_eq!(body["code"], json!("SUCCESS"));
    let final_output = body["data"]["finalOutput"].as_str().unwrap_or_default();
    assert!(
        final_output.contains("pinLevel") || final_output.contains("unknown field"),
        "removed pinLevel input should fail at tool input parsing, got: {final_output}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_search_allows_empty_query_without_memory_code() {
    let model_server = scripted_model_server(vec![
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": WORK_DOMAIN_MEMORY_CODE,
                "title": "Empty query searchable memory",
                "text": "Empty query search should still reach accessible memories.",
                "memoryType": "FACT",
                "reviewStatus": "APPROVED"
            }),
        ),
        tool_call_response("memory.search", json!({})),
        final_response("empty query search complete"),
    ])
    .await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    create_embedding_default(&app, &authorization, &model_server.base_url).await;

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(
            &model_server.base_url,
            Some(vec![
                effective_tool("memory.search"),
                effective_tool("memory.save"),
            ]),
            json!({ "maxSteps": 4, "maxToolCalls": 2 }),
        )),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected empty query memory search response body: {body}"
    );
    assert_eq!(
        body["data"]["finalOutput"],
        json!("empty query search complete")
    );

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 3);
    let observations = memory_observations_in_request(
        requests
            .last()
            .expect("final model request should include empty query search observations"),
    );
    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.tool_name.as_str())
            .collect::<Vec<_>>(),
        vec!["memory_save", "memory_search"]
    );
    assert_json_mentions(&observations[1].output, WORK_DOMAIN_MEMORY_CODE);
    assert!(
        result_count(&observations[1].output).is_some_and(|count| count >= 1),
        "empty query search should execute and return accessible indexed memories"
    );
    assert!(
        observations[1].output.get("contextPack").is_none(),
        "SEARCH mode should not return a context pack"
    );
    assert_memory_search_observation_is_slim(&observations[1]);
    for observation in &observations {
        assert_no_raw_locator_or_sensitive_keys(&observation.output);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_search_observation_stays_under_event_bus_limit_with_governance_metadata() {
    let mut responses = Vec::new();
    for index in 1..=7 {
        responses.push(tool_call_response(
            "memory.save",
            json!({
                "memoryCode": format!("U:USR-AUTH-0001:user-like-preference-{index}:v1"),
                "title": format!("用户喜好偏好测试 {index}"),
                "text": format!(
                    "用户喜好 偏好 喜欢 第 {index} 条。这个记忆用于验证常规 search 返回给 agent 的内容保留正文，但不重复 preview，也不携带完整治理 metadata。"
                ),
                "memoryType": "USER_PREFERENCE",
                "reviewStatus": "APPROVED",
                "sourceRefs": [{
                    "type": "test",
                    "id": format!("memory-search-compact-source-{index}"),
                    "locator": "governance evidence locator that should not be returned in ordinary memory search observations"
                }],
                "semanticKey": format!("user:preference:compact:{index}"),
                "metadata": {
                    "language": "zh-CN",
                    "subject": format!("用户喜好 {index}"),
                    "purpose": "验证检索时只返回轻量 agent-facing metadata",
                    "typeSpecific": {
                        "evidenceSummary": "this verbose governance field should remain stored but must not be echoed in ordinary search output",
                        "application": "ordinary search result compaction"
                    }
                }
            }),
        ));
    }
    responses.push(tool_call_response(
        "memory.search",
        json!({
            "limit": 20,
            "query": "用户喜好 偏好 喜欢",
            "scope": "USER"
        }),
    ));
    responses.push(final_response("compact memory search complete"));

    let model_server = scripted_model_server(responses).await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    create_embedding_default(&app, &authorization, &model_server.base_url).await;
    create_rerank_default(&app, &authorization, &model_server.base_url).await;

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(
            &model_server.base_url,
            Some(vec![
                effective_tool("memory.search"),
                effective_tool("memory.save"),
            ]),
            json!({ "maxSteps": 12, "maxToolCalls": 8 }),
        )),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected compact memory search response body: {body}"
    );
    assert_eq!(
        body["data"]["finalOutput"],
        json!("compact memory search complete")
    );

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 9);
    let observations = memory_observations_in_request(
        requests
            .last()
            .expect("final model request should include compact search observations"),
    );
    let search_observation = observations
        .last()
        .expect("final memory observation should be the compact search");
    assert_eq!(search_observation.tool_name, "memory_search");
    assert_eq!(result_count(&search_observation.output), Some(7));
    assert_memory_search_observation_is_slim(search_observation);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_context_pack_merges_baseline_injectable_and_query_related_memories() {
    let baseline_code = "D:main-domain:baseline-context-rule:v1";
    let related_code = "D:main-domain:query-related-note:v1";
    let model_server = scripted_model_server(vec![
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": baseline_code,
                "title": "Baseline context rule",
                "text": "Default baseline context should be available before every agent answer.",
                "memoryType": "RULE",
                "reviewStatus": "APPROVED",
                "injectable": true,
                "metadata": {
                    "semanticKey": "memory-context-pack:baseline",
                    "language": "en-US"
                }
            }),
        ),
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": related_code,
                "title": "Quantum context note",
                "text": "Quantum context answers should include this query related non injectable memory.",
                "memoryType": "FACT",
                "reviewStatus": "APPROVED",
                "injectable": false,
                "metadata": {
                    "semanticKey": "memory-context-pack:related",
                    "language": "en-US"
                }
            }),
        ),
        tool_call_response(
            "memory.search",
            json!({
                "query": "quantum context",
                "mode": "CONTEXT_PACK",
                "limit": 5,
                "tokenBudget": 100
            }),
        ),
        final_response("context pack merged"),
    ])
    .await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    create_embedding_default(&app, &authorization, &model_server.base_url).await;
    create_rerank_default(&app, &authorization, &model_server.base_url).await;

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(
            &model_server.base_url,
            Some(vec![
                effective_tool("memory.search"),
                effective_tool("memory.save"),
            ]),
            json!({ "maxSteps": 6, "maxToolCalls": 3 }),
        )),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected context pack merge response body: {body}"
    );
    assert_eq!(body["data"]["finalOutput"], json!("context pack merged"));

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 4);
    let observations = memory_observations_in_request(
        requests
            .last()
            .expect("final model request should include context pack observations"),
    );
    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.tool_name.as_str())
            .collect::<Vec<_>>(),
        vec!["memory_save", "memory_save", "memory_search"]
    );
    let search_observation = &observations[2];
    assert_eq!(result_count(&search_observation.output), Some(2));
    assert_json_mentions(&search_observation.output, baseline_code);
    assert_json_mentions(&search_observation.output, related_code);
    assert_json_mentions(&search_observation.output, "contextPack");
    let related = search_observation.output["items"]
        .as_array()
        .expect("memory search should return item array")
        .iter()
        .find(|item| item["memoryCode"] == json!(related_code))
        .expect("related non-injectable memory should be returned");
    assert_eq!(related["injectable"], json!(false));
    let context_pack_codes = search_observation.output["contextPack"]["items"]
        .as_array()
        .expect("context pack should return item array")
        .iter()
        .filter_map(|item| item["memoryCode"].as_str())
        .collect::<Vec<_>>();
    assert!(context_pack_codes.contains(&baseline_code));
    assert!(context_pack_codes.contains(&related_code));
    assert_memory_search_observation_is_slim(search_observation);
    assert_context_pack_metadata_is_slim(&search_observation.output);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_code_upsert_loads_current_without_default_governance_filters() {
    let text = "Candidate memory should survive current lookup.";
    let model_server = scripted_model_server(vec![
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": MEMORY_CODE,
                "title": "Candidate citation preference",
                "text": text,
                "memoryType": "FACT",
                "memoryStatus": "CANDIDATE",
                "reviewStatus": "PENDING_REVIEW",
                "injectable": true,
                "sourceRefs": [{
                    "type": "test",
                    "id": "candidate-memory-source"
                }],
                "semanticKey": "candidate:citation-preference",
                "metadata": {
                    "stable": "original"
                }
            }),
        ),
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": MEMORY_CODE,
                "title": "Candidate citation preference updated",
                "memoryStatus": "CANDIDATE",
                "reviewStatus": "PENDING_REVIEW",
                "injectable": true,
                "metadata": {
                    "patched": "yes"
                }
            }),
        ),
        tool_call_response(
            "memory.search",
            json!({
                "memoryCode": MEMORY_CODE,
                "memoryStatus": "CANDIDATE",
                "reviewStatus": "PENDING_REVIEW"
            }),
        ),
        final_response("candidate memory updated"),
    ])
    .await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    create_embedding_default(&app, &authorization, &model_server.base_url).await;

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(
            &model_server.base_url,
            Some(vec![
                effective_tool("memory.search"),
                effective_tool("memory.save"),
            ]),
            json!({ "maxSteps": 6, "maxToolCalls": 4 }),
        )),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected candidate memory response body: {body}"
    );
    assert_eq!(
        body["data"]["finalOutput"],
        json!("candidate memory updated")
    );

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 4);
    let observations = memory_observations_in_request(
        requests
            .last()
            .expect("final model request should include candidate memory observations"),
    );
    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.tool_name.as_str())
            .collect::<Vec<_>>(),
        vec!["memory_save", "memory_save", "memory_search"]
    );
    assert_json_mentions_any(&observations[0].output, &["CREATED", "created"]);
    assert_json_mentions_any(&observations[1].output, &["UPDATED", "updated"]);
    assert_eq!(observations[0].output["injectable"], json!(false));
    assert_eq!(observations[1].output["injectable"], json!(false));
    assert_eq!(result_count(&observations[2].output), Some(1));
    assert_json_mentions(&observations[2].output, text);
    assert_json_mentions(&observations[2].output, "semanticKey");
    assert_json_does_not_mention(&observations[2].output, "sourceRefs");
    assert_json_does_not_mention(&observations[2].output, "patched");
    assert_memory_search_observation_is_slim(&observations[2]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_work_domain_and_space_memory_code_short_formats_are_accepted() {
    let model_server = scripted_model_server(vec![
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": USER_MEMORY_CODE,
                "title": "User citation preference",
                "text": "User memories prefer direct citations.",
                "memoryType": "RULE",
                "reviewStatus": "APPROVED"
            }),
        ),
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": WORK_DOMAIN_MEMORY_CODE,
                "title": "Work domain citation preference",
                "text": "Work domain memories prefer compact citations.",
                "memoryType": "RULE",
                "reviewStatus": "APPROVED"
            }),
        ),
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": MEMORY_CODE,
                "title": "Space citation preference",
                "text": MEMORY_TEXT_INITIAL,
                "memoryType": "RULE",
                "reviewStatus": "APPROVED"
            }),
        ),
        final_response("memory short formats accepted"),
    ])
    .await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    create_embedding_default(&app, &authorization, &model_server.base_url).await;

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(
            &model_server.base_url,
            Some(vec![effective_tool("memory.save")]),
            json!({ "maxSteps": 5, "maxToolCalls": 4 }),
        )),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected U/D/S memoryCode response body: {body}"
    );
    assert_eq!(
        body["data"]["finalOutput"],
        json!("memory short formats accepted")
    );

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 4);
    let observations = memory_observations_in_request(
        requests
            .last()
            .expect("final model request should include short-format observations"),
    );
    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.output["memoryCode"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!(USER_MEMORY_CODE),
            json!(WORK_DOMAIN_MEMORY_CODE),
            json!(MEMORY_CODE),
        ]
    );
    assert_memory_scope(&observations[1].output, "WORK_DOMAIN");
    assert_json_mentions(&observations[1].output, "D:main-domain");
    for observation in &observations {
        assert_json_does_not_mention(&observation.output, "pinLevel");
        assert_no_raw_locator_or_sensitive_keys(&observation.output);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_code_save_update_delete_search_observations_are_sanitized() {
    let model_server = scripted_model_server(vec![
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": WORK_DOMAIN_MEMORY_CODE,
                "title": "Citation preference",
                "text": MEMORY_TEXT_INITIAL,
                "memoryType": "RULE",
                "reviewStatus": "APPROVED"
            }),
        ),
        tool_call_response(
            "memory.save",
            json!({
                "memoryCode": WORK_DOMAIN_MEMORY_CODE,
                "title": "Citation preference",
                "text": MEMORY_TEXT_UPDATED,
                "memoryType": "RULE",
                "reviewStatus": "APPROVED"
            }),
        ),
        tool_call_response(
            "memory.search",
            json!({
                "memoryCode": WORK_DOMAIN_MEMORY_CODE,
                "query": "source citations",
                "limit": 5
            }),
        ),
        tool_call_response(
            "memory.search",
            json!({
                "memoryCode": WORK_DOMAIN_MEMORY_CODE,
                "mode": "CONTEXT_PACK"
            }),
        ),
        tool_call_response(
            "memory.delete",
            json!({
                "memoryCode": WORK_DOMAIN_MEMORY_CODE
            }),
        ),
        tool_call_response(
            "memory.search",
            json!({
                "memoryCode": WORK_DOMAIN_MEMORY_CODE,
                "query": "source citations",
                "limit": 5
            }),
        ),
        final_response("memory lifecycle complete"),
    ])
    .await;
    let app = app();
    let token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {token}");
    create_embedding_default(&app, &authorization, &model_server.base_url).await;

    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/agents/runs",
        Some(agent_run_payload(
            &model_server.base_url,
            Some(vec![
                effective_tool("memory.search"),
                effective_tool("memory.save"),
                effective_tool("memory.delete"),
            ]),
            json!({ "maxSteps": 9, "maxToolCalls": 6 }),
        )),
        None,
        Some(&authorization),
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected memory lifecycle response body: {body}"
    );
    assert_eq!(
        body["data"]["finalOutput"],
        json!("memory lifecycle complete")
    );

    let requests = recorded_requests(&model_server);
    assert_eq!(requests.len(), 7);
    let observations = memory_observations_in_request(
        requests
            .last()
            .expect("final model request should include prior observations"),
    );
    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.tool_name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "memory_save",
            "memory_save",
            "memory_search",
            "memory_search",
            "memory_delete",
            "memory_search"
        ]
    );

    for observation in &observations {
        let object = observation
            .output
            .as_object()
            .expect("memory tool observation should be a JSON object");
        assert_no_forbidden_top_level_fields(object);
        assert_no_raw_locator_or_sensitive_keys(&observation.output);
    }

    assert_eq!(
        observations[0].output["memoryCode"],
        json!(WORK_DOMAIN_MEMORY_CODE),
        "create observation should return the canonical memoryCode"
    );
    assert_memory_scope(&observations[0].output, "WORK_DOMAIN");
    assert_json_mentions_any(&observations[0].output, &["CREATED", "created"]);
    assert_eq!(
        observations[0].output["rag"]["fulltextStatus"],
        json!("INDEXED")
    );
    assert_eq!(
        observations[0].output["rag"]["embeddingStatus"],
        json!("INDEXED")
    );

    assert_eq!(
        observations[1].output["memoryCode"],
        json!(WORK_DOMAIN_MEMORY_CODE),
        "update observation should return the canonical memoryCode"
    );
    assert_memory_scope(&observations[1].output, "WORK_DOMAIN");
    assert_json_mentions_any(&observations[1].output, &["UPDATED", "updated"]);
    assert_eq!(
        observations[1].output["rag"]["fulltextStatus"],
        json!("INDEXED")
    );
    assert_eq!(
        observations[1].output["rag"]["embeddingStatus"],
        json!("INDEXED")
    );

    assert_json_mentions(&observations[2].output, WORK_DOMAIN_MEMORY_CODE);
    assert_json_mentions(&observations[2].output, "D:main-domain");
    assert_memory_scope(&observations[2].output, "WORK_DOMAIN");
    assert_json_mentions(&observations[2].output, MEMORY_TEXT_UPDATED);
    assert!(
        result_count(&observations[2].output).is_some_and(|count| count >= 1),
        "search after update should return the saved memory"
    );
    assert_memory_search_observation_is_slim(&observations[2]);

    assert_json_mentions(&observations[3].output, WORK_DOMAIN_MEMORY_CODE);
    assert_json_mentions(&observations[3].output, "D:main-domain");
    assert_json_mentions(&observations[3].output, "contextPack");
    assert_memory_scope(&observations[3].output, "WORK_DOMAIN");
    assert_context_pack_metadata_is_slim(&observations[3].output);

    assert_eq!(
        observations[4].output["memoryCode"],
        json!(WORK_DOMAIN_MEMORY_CODE),
        "delete observation should return the canonical memoryCode"
    );
    assert_memory_scope(&observations[4].output, "WORK_DOMAIN");
    assert_json_mentions_any(&observations[4].output, &["DELETED", "deleted"]);

    assert_eq!(
        result_count(&observations[5].output),
        Some(0),
        "search after delete should return no active records"
    );
    assert_json_does_not_mention(&observations[5].output, MEMORY_TEXT_UPDATED);
    assert_memory_search_observation_is_slim(&observations[5]);
}

async fn request_with_locale(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    authorization_header: &str,
    locale: &str,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, authorization_header)
        .header(header::ACCEPT_LANGUAGE, locale);
    let request_body = if let Some(json_body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(json_body.to_string())
    } else {
        Body::empty()
    };

    app.clone()
        .oneshot(
            builder
                .body(request_body)
                .expect("request should be constructible"),
        )
        .await
        .expect("router should return a response")
}

async fn create_embedding_default(app: &Router, authorization_header: &str, base_url: &str) {
    let create_response = request_with_headers(
        app,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Memory Test Embedding",
            "vendor": "Test",
            "protocol": "OPENAI_COMPATIBLE_EMBEDDING",
            "modelType": "EMBEDDING",
            "modelCapabilities": ["TEXT_VECTOR"],
            "baseUrl": base_url,
            "apiKeyRef": "sk-test-embedding",
            "modelName": "memory-test-embedding",
            "enabled": true,
            "thinkingEnabled": false,
            "structuredOutputEnabled": false,
            "maxRetries": 0
        })),
        None,
        Some(authorization_header),
    )
    .await;
    let create_status = create_response.status();
    let create_body = response_json(create_response).await;
    assert_eq!(
        create_status,
        StatusCode::CREATED,
        "unexpected embedding model create response body: {create_body}"
    );
    let model_code = create_body["data"]["modelCode"]
        .as_str()
        .expect("created embedding model should expose modelCode");

    let default_response = request_with_headers(
        app,
        Method::POST,
        &format!("/api/v1/admin/platform/models/{model_code}/set-default"),
        None,
        None,
        Some(authorization_header),
    )
    .await;
    let default_status = default_response.status();
    let default_body = response_json(default_response).await;
    assert_eq!(
        default_status,
        StatusCode::OK,
        "unexpected embedding default response body: {default_body}"
    );
}

async fn create_rerank_default(app: &Router, authorization_header: &str, base_url: &str) {
    let create_response = request_with_headers(
        app,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Memory Test Rerank",
            "vendor": "Test",
            "protocol": "RERANK",
            "modelType": "RERANK",
            "modelCapabilities": ["RERANK"],
            "baseUrl": base_url,
            "apiKeyRef": "sk-test-rerank",
            "modelName": "memory-test-rerank",
            "enabled": true,
            "thinkingEnabled": false,
            "structuredOutputEnabled": false,
            "maxRetries": 0
        })),
        None,
        Some(authorization_header),
    )
    .await;
    let create_status = create_response.status();
    let create_body = response_json(create_response).await;
    assert_eq!(
        create_status,
        StatusCode::CREATED,
        "unexpected rerank model create response body: {create_body}"
    );
    let model_code = create_body["data"]["modelCode"]
        .as_str()
        .expect("created rerank model should expose modelCode");

    let default_response = request_with_headers(
        app,
        Method::POST,
        &format!("/api/v1/admin/platform/models/{model_code}/set-default"),
        None,
        None,
        Some(authorization_header),
    )
    .await;
    let default_status = default_response.status();
    let default_body = response_json(default_response).await;
    assert_eq!(
        default_status,
        StatusCode::OK,
        "unexpected rerank default response body: {default_body}"
    );
}

fn agent_run_payload(base_url: &str, tools: Option<Vec<Value>>, limits: Value) -> Value {
    let mut payload = json!({
        "prompt": "Use memory tools.",
        "workspaceCode": "TRON_CORE",
        "model": {
            "modelCode": "TEST_MODEL",
            "protocol": "CHAT_COMPLETIONS",
            "baseUrl": base_url,
            "apiKey": "sk-test-api",
            "modelName": "gpt-test"
        }
    });
    if let Some(tools) = tools {
        payload["tools"] = json!(tools);
    }
    if !limits.as_object().is_some_and(Map::is_empty) {
        payload["limits"] = limits;
    }
    payload
}

fn effective_tool(tool_name: &str) -> Value {
    let tool_name = tool_name.replace('.', "_");
    json!({
        "toolName": tool_name,
        "domainCode": "memory",
        "description": format!("{tool_name} test contract"),
        "inputSchema": {
            "type": "object",
            "properties": {
                "memoryCode": { "type": "string" }
            },
            "additionalProperties": true
        },
        "skillName": null,
        "requiredVariableKeys": []
    })
}

fn final_response(content: impl Into<String>) -> Value {
    json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": content.into()
            }
        }]
    })
}

fn tool_call_response(tool_name: &str, input: Value) -> Value {
    let tool_name = tool_name.replace('.', "_");
    json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "tool_calls": [{
                    "id": "call-memory",
                    "type": "function",
                    "function": {
                        "name": tool_name,
                        "arguments": input.to_string()
                    }
                }]
            }
        }]
    })
}

fn recorded_requests(server: &RecordingModelServer) -> Vec<Value> {
    server
        .requests
        .lock()
        .expect("recorded model request mutex poisoned")
        .clone()
}

fn find_record<'a>(rows: &'a Value, key: &str, value: &str) -> Option<&'a Value> {
    rows.as_array()?.iter().find(|row| row[key] == json!(value))
}

fn contains_cjk(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        text.chars()
            .any(|character| ('\u{4e00}'..='\u{9fff}').contains(&character))
    })
}

fn tool_names_in_request(request: &Value) -> Vec<String> {
    request["tools"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| tool["function"]["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

struct ParsedObservation {
    tool_name: String,
    output: Value,
}

fn memory_observations_in_request(request: &Value) -> Vec<ParsedObservation> {
    let mut call_names = std::collections::HashMap::<String, String>::new();
    let mut observations = Vec::new();
    for message in request["messages"].as_array().into_iter().flatten() {
        if let Some(tool_calls) = message["tool_calls"].as_array() {
            for tool_call in tool_calls {
                if let (Some(call_id), Some(tool_name)) = (
                    tool_call["id"].as_str(),
                    tool_call["function"]["name"].as_str(),
                ) {
                    call_names.insert(call_id.to_string(), tool_name.to_string());
                }
            }
        }
        if let Some(observation) = parse_memory_observation_message(message, &call_names) {
            observations.push(observation);
        }
    }
    observations
}

fn parse_memory_observation_message(
    message: &Value,
    call_names: &std::collections::HashMap<String, String>,
) -> Option<ParsedObservation> {
    if message["role"] == json!("tool") {
        let call_id = message["tool_call_id"].as_str()?;
        let tool_name = call_names.get(call_id)?;
        if !tool_name.starts_with("memory_") {
            return None;
        }
        let output = message["content"].as_str()?;
        let output = serde_json::from_str(output).unwrap_or_else(|error| {
            panic!("{tool_name} observation should be valid JSON: {error}")
        });
        return Some(ParsedObservation {
            tool_name: tool_name.to_string(),
            output,
        });
    }
    message["content"]
        .as_str()
        .and_then(parse_memory_observation)
}

fn parse_memory_observation(content: &str) -> Option<ParsedObservation> {
    let (tool_line, output) = content.split_once("\nresult:\n")?;
    let tool_name = tool_line.strip_prefix("tool:")?;
    if !tool_name.starts_with("memory_") {
        return None;
    }
    let output = serde_json::from_str(output)
        .unwrap_or_else(|error| panic!("{tool_name} observation should be valid JSON: {error}"));
    Some(ParsedObservation {
        tool_name: tool_name.to_string(),
        output,
    })
}

fn assert_no_forbidden_top_level_fields(object: &Map<String, Value>) {
    for key in ["sourceCode", "sourceVersion"] {
        assert!(
            !object.contains_key(key),
            "memory observation must not expose top-level {key}"
        );
    }
}

fn assert_no_raw_locator_or_sensitive_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let normalized = key.to_ascii_lowercase().replace('_', "");
                assert!(
                    normalized != "pinlevel",
                    "memory observation leaked removed memory field {key}"
                );
                assert!(
                    !matches!(
                        normalized.as_str(),
                        "sourcecode"
                            | "sourceversion"
                            | "sourcetype"
                            | "sourcecontenthash"
                            | "scopekey"
                            | "scopetype"
                            | "textunitcode"
                            | "textunitid"
                            | "corpusid"
                            | "profileid"
                            | "routeid"
                            | "routeids"
                            | "storagebinding"
                            | "tablename"
                            | "table"
                            | "contenthash"
                    ),
                    "memory observation leaked raw RAG locator/storage key {key}"
                );
                assert!(
                    (normalized == "embeddingstatus" || !normalized.contains("embedding"))
                        && !normalized.contains("provider")
                        && !matches!(
                            normalized.as_str(),
                            "apikey"
                                | "authorization"
                                | "accesstoken"
                                | "refreshtoken"
                                | "authtoken"
                                | "authheader"
                                | "authvalue"
                                | "secret"
                                | "password"
                        ),
                    "memory observation leaked sensitive key {key}"
                );
                assert_no_raw_locator_or_sensitive_keys(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_raw_locator_or_sensitive_keys(item);
            }
        }
        _ => {}
    }
}

fn result_count(value: &Value) -> Option<usize> {
    for key in ["items", "records", "hits", "results", "memories"] {
        if let Some(items) = value.get(key).and_then(Value::as_array) {
            return Some(items.len());
        }
    }
    value
        .get("data")
        .and_then(|data| {
            ["items", "records", "hits", "results", "memories"]
                .into_iter()
                .find_map(|key| data.get(key).and_then(Value::as_array))
        })
        .map(Vec::len)
}

fn assert_memory_search_observation_is_slim(observation: &ParsedObservation) {
    let serialized = serde_json::to_string(&observation.output)
        .expect("memory observation output should serialize");
    assert!(
        serialized.len() < 8192,
        "memory search observation should stay below event bus attr limit, actual={} body={serialized}",
        serialized.len()
    );
    assert_json_does_not_mention(&observation.output, "\"preview\"");
    assert_json_does_not_mention(&observation.output, ":null");
    assert_json_does_not_mention(&observation.output, "sourceRefs");
    assert_json_does_not_mention(&observation.output, "typeSpecific");
    assert_json_does_not_mention(&observation.output, "returnFields");
    assert_search_items_have_core_fields(&observation.output);
    assert_search_item_metadata_is_slim(&observation.output);
}

fn assert_search_items_have_core_fields(value: &Value) {
    for item in value["items"].as_array().into_iter().flatten() {
        for key in ["memoryCode", "scope", "scopeCode", "title", "text"] {
            assert!(
                item.get(key).is_some(),
                "memory search item should keep core field {key}: {item}"
            );
        }
    }
}

fn assert_search_item_metadata_is_slim(value: &Value) {
    for item in value["items"].as_array().into_iter().flatten() {
        if let Some(metadata) = item.get("metadata") {
            assert_allowed_metadata_keys(metadata);
        }
    }
}

fn assert_context_pack_metadata_is_slim(value: &Value) {
    for item in value["contextPack"]["items"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let Some(metadata) = item.get("metadata") {
            assert_allowed_metadata_keys(metadata);
        }
    }
    assert_json_does_not_mention(value, "sourceRefs");
    assert_json_does_not_mention(value, "typeSpecific");
}

fn assert_allowed_metadata_keys(metadata: &Value) {
    let object = metadata
        .as_object()
        .expect("memory metadata should be a JSON object");
    for key in object.keys() {
        assert!(
            matches!(
                key.as_str(),
                "semanticKey" | "language" | "subject" | "purpose"
            ),
            "memory metadata should only expose lightweight agent-facing fields, got {key}: {metadata}"
        );
    }
}

fn assert_json_mentions(value: &Value, expected: &str) {
    let serialized = value.to_string();
    assert!(
        serialized.contains(expected),
        "expected JSON to mention {expected}: {serialized}"
    );
}

fn assert_memory_scope(value: &Value, expected_scope: &str) {
    let expected = format!("\"scope\":\"{expected_scope}\"");
    let serialized = value.to_string();
    assert!(
        serialized.contains(&expected),
        "expected memory JSON to expose scope {expected_scope}: {serialized}"
    );
}

fn assert_json_mentions_any(value: &Value, expected: &[&str]) {
    let serialized = value.to_string();
    assert!(
        expected.iter().any(|needle| serialized.contains(needle)),
        "expected JSON to mention one of {:?}: {serialized}",
        expected
    );
}

fn assert_json_does_not_mention(value: &Value, unexpected: &str) {
    let serialized = value.to_string();
    assert!(
        !serialized.contains(unexpected),
        "expected JSON not to mention {unexpected}: {serialized}"
    );
}
