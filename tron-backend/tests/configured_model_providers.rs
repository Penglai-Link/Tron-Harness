mod common;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::{
    application::model::ModelService, config::AppConfig, domain::agent::AgentModelProtocol,
};

use common::{app_with_state_setup, authenticated_request, prepared_config, response_json};

const PROVIDERS: &str = "/api/v1/model-providers/configured";

struct Fixture {
    router: Router,
    models: ModelService,
    config: AppConfig,
    database_path: PathBuf,
    _directory: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("provider fixture directory");
        let database_path = directory.path().join("providers.sqlite3");
        let config = prepared_config(AppConfig {
            database_url: database_path.to_string_lossy().into_owned(),
            ..AppConfig::default()
        });
        let (router, models) = open(&config);
        Self {
            router,
            models,
            config,
            database_path,
            _directory: directory,
        }
    }

    fn database(&self) -> Connection {
        Connection::open(&self.database_path).expect("provider fixture database")
    }
}

fn open(config: &AppConfig) -> (Router, ModelService) {
    let mut models = None;
    let router = app_with_state_setup(config.clone(), |state| models = Some(state.models.clone()));
    (router, models.expect("model service"))
}

fn model(id: &str, name: &str) -> Value {
    json!({
        "modelCode": null,
        "modelId": id,
        "name": name,
        "contextWindowTokens": 8192,
        "maxOutputTokens": 256,
        "reasoning": false,
        "supportsDeveloperRole": false,
        "inputCapabilities": ["Text"],
        "enabled": true
    })
}

fn provider(id: &str, url: &str, key: Option<&str>) -> Value {
    json!({
        "providerId": id,
        "displayName": format!("Provider {id}"),
        "protocol": "OpenAiChatCompletions",
        "baseUrl": url,
        "apiKey": key,
        "requestHeaders": { "X-Test-Provider": id },
        "enabled": true,
        "models": [model("same-remote-model", "Shared display name")]
    })
}

fn update(provider: &Value, key: Option<&str>) -> Value {
    json!({
        "displayName": provider["displayName"],
        "protocol": provider["protocol"],
        "baseUrl": provider["baseUrl"],
        "apiKey": key,
        "requestHeaders": provider["requestHeaders"],
        "enabled": provider["enabled"],
        "models": provider["models"]
    })
}

fn code(provider: &Value, index: usize) -> &str {
    provider["models"][index]["modelCode"]
        .as_str()
        .expect("stable model code")
}

fn endpoint(provider_id: &str) -> String {
    format!("{PROVIDERS}/{provider_id}")
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
    expected: StatusCode,
) -> Value {
    let response = authenticated_request(router, method, path, body).await;
    let status = response.status();
    let payload = response_json(response).await;
    assert_eq!(status, expected, "{path}: {payload}");
    payload
}

async fn create(fixture: &Fixture, request: Value) -> Value {
    let response = call(
        &fixture.router,
        Method::POST,
        PROVIDERS,
        Some(request),
        StatusCode::OK,
    )
    .await;
    assert_eq!(response["code"], "0");
    response["data"].clone()
}

async fn get(router: &Router, id: &str) -> Value {
    let response = call(router, Method::GET, &endpoint(id), None, StatusCode::OK).await;
    assert_eq!(response["code"], "0");
    response["data"].clone()
}

fn assert_no_credentials(payload: &Value, secrets: &[&str]) {
    let serialized = payload.to_string();
    assert!(
        payload.get("apiKey").is_none(),
        "API key must be write-only"
    );
    assert!(
        !serialized.contains("encryptedApiKey"),
        "encrypted credentials are internal too"
    );
    for secret in secrets {
        assert!(
            !serialized.contains(secret),
            "a credential leaked into the response"
        );
    }
}

#[tokio::test]
async fn configured_provider_round_trip_encrypts_credentials_and_survives_reopening_storage() {
    let fixture = Fixture::new();
    let key = "fixture-secret-persisted";
    let created = create(
        &fixture,
        provider("persisted", "https://models.example.test/v1/", Some(key)),
    )
    .await;
    let model_code = code(&created, 0).to_owned();
    assert!(!model_code.is_empty());
    assert_eq!(created["hasApiKey"], true);
    assert_eq!(created["baseUrl"], "https://models.example.test/v1");
    assert!(created["createdAt"].as_i64().is_some_and(|value| value > 0));
    assert!(created["updatedAt"].as_i64().is_some_and(|value| value > 0));
    assert_no_credentials(&created, &[key]);
    assert_eq!(get(&fixture.router, "persisted").await, created);

    let list = call(
        &fixture.router,
        Method::GET,
        PROVIDERS,
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(list["data"], json!([created.clone()]));
    assert_no_credentials(&list, &[key]);
    let providers = call(
        &fixture.router,
        Method::GET,
        "/api/v1/model-providers",
        None,
        StatusCode::OK,
    )
    .await;
    let records = providers["data"].as_array().unwrap();
    assert!(
        records
            .iter()
            .any(|item| item["providerCode"] == "PersonAi")
    );
    assert!(
        records
            .iter()
            .any(|item| item["providerCode"] == "persisted" && item["source"] == "Custom")
    );
    assert_no_credentials(&providers, &[key]);

    let stored: String = fixture
        .database()
        .query_row(
            "SELECT configuration_json FROM configured_model_providers WHERE provider_id=?1",
            ["persisted"],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        !stored.contains(key),
        "plaintext credentials must not be persisted"
    );
    assert!(
        stored.contains("enc:v1:"),
        "provider credential must use the existing encrypted credential format"
    );
    let model_key: String = fixture
        .database()
        .query_row(
            "SELECT api_key_ref FROM models WHERE model_code=?1",
            [&model_code],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!model_key.contains(key));

    let resolved = fixture
        .models
        .text_agent_model_config("", Some(&model_code))
        .unwrap()
        .unwrap();
    assert_eq!(resolved.api_key, key);
    assert_eq!(resolved.base_url, "https://models.example.test/v1");
    assert_eq!(
        resolved
            .request_headers
            .get("X-Test-Provider")
            .map(String::as_str),
        Some("persisted")
    );
    let platform = call(
        &fixture.router,
        Method::GET,
        &format!("/api/v1/admin/platform/models/{model_code}"),
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(platform["data"]["providerId"], "persisted");
    assert_no_credentials(&platform, &[key]);

    let (reopened_router, reopened_models) = open(&fixture.config);
    assert_eq!(get(&reopened_router, "persisted").await, created);
    let reopened = reopened_models
        .text_agent_model_config("", Some(&model_code))
        .unwrap()
        .unwrap();
    assert_eq!(reopened.api_key, key);
    assert_eq!(reopened.base_url, resolved.base_url);
    assert_eq!(reopened.request_headers, resolved.request_headers);
    assert_eq!(reopened.model_code, model_code);
}

#[tokio::test]
async fn changing_one_connection_updates_runtime_without_affecting_another_provider_with_the_same_model_id()
 {
    let fixture = Fixture::new();
    let first = create(
        &fixture,
        provider(
            "first",
            "https://first.example.test/v1",
            Some("fixture-first-key"),
        ),
    )
    .await;
    let second = create(
        &fixture,
        provider(
            "second",
            "https://second.example.test/api",
            Some("fixture-second-key"),
        ),
    )
    .await;
    let first_code = code(&first, 0).to_owned();
    let second_code = code(&second, 0).to_owned();
    assert_ne!(
        first_code, second_code,
        "provider-local model IDs must have separate runtime identities"
    );
    assert_eq!(
        first["models"][0]["modelId"],
        second["models"][0]["modelId"]
    );

    let mut changed = update(&first, None);
    changed["baseUrl"] = json!("https://rotated.example.test/anthropic");
    changed["protocol"] = json!("AnthropicMessages");
    changed["requestHeaders"] = json!({"X-Test-Provider": "rotated"});
    changed["models"][0]["contextWindowTokens"] = json!(32768);
    changed["models"][0]["maxOutputTokens"] = json!(1024);
    let response = call(
        &fixture.router,
        Method::PUT,
        &endpoint("first"),
        Some(changed),
        StatusCode::OK,
    )
    .await;
    let updated = &response["data"];
    assert_eq!(code(updated, 0), first_code);
    assert!(updated["updatedAt"].as_i64().unwrap() > first["updatedAt"].as_i64().unwrap());
    let runtime = fixture
        .models
        .text_agent_model_config("", Some(&first_code))
        .unwrap()
        .unwrap();
    assert_eq!(runtime.base_url, "https://rotated.example.test/anthropic");
    assert_eq!(
        runtime.api_key, "fixture-first-key",
        "null means retain the saved credential"
    );
    assert_eq!(runtime.protocol, AgentModelProtocol::Anthropic);
    assert_eq!(runtime.context_window_tokens, Some(32768));
    assert_eq!(runtime.max_tokens, Some(1024));
    assert_eq!(
        runtime
            .request_headers
            .get("X-Test-Provider")
            .map(String::as_str),
        Some("rotated")
    );

    let rotated = call(
        &fixture.router,
        Method::PUT,
        &endpoint("first"),
        Some(update(updated, Some("fixture-rotated-key"))),
        StatusCode::OK,
    )
    .await;
    assert_no_credentials(&rotated, &["fixture-first-key", "fixture-rotated-key"]);
    let runtime = fixture
        .models
        .text_agent_model_config("", Some(&first_code))
        .unwrap()
        .unwrap();
    assert_eq!(runtime.api_key, "fixture-rotated-key");
    let other = fixture
        .models
        .text_agent_model_config("", Some(&second_code))
        .unwrap()
        .unwrap();
    assert_eq!(other.base_url, "https://second.example.test/api");
    assert_eq!(other.api_key, "fixture-second-key");
    assert_eq!(other.protocol, AgentModelProtocol::ChatCompletions);
    assert_eq!(get(&fixture.router, "second").await, second);
}

#[tokio::test]
async fn rejected_updates_leave_provider_models_and_credentials_unchanged() {
    let fixture = Fixture::new();
    let initial = create(
        &fixture,
        provider(
            "validated",
            "https://valid.example.test/v1",
            Some("fixture-valid-key"),
        ),
    )
    .await;
    let foreign = create(
        &fixture,
        provider(
            "foreign",
            "https://foreign.example.test/v1",
            Some("fixture-foreign-key"),
        ),
    )
    .await;
    let baseline = update(&initial, Some("fixture-rejected-new-key"));
    let mut cases = Vec::new();
    let mut duplicate = baseline.clone();
    duplicate["models"]
        .as_array_mut()
        .unwrap()
        .push(model("same-remote-model", "Duplicate"));
    cases.push((duplicate, "101032007"));
    let mut invalid_url = baseline.clone();
    invalid_url["baseUrl"] = json!("file:///tmp/provider");
    cases.push((invalid_url, "101032005"));
    let mut credential_url = baseline.clone();
    credential_url["baseUrl"] = json!("https://user:fixture-url-secret@example.test/v1");
    cases.push((credential_url, "101032005"));
    let mut auth_header = baseline.clone();
    auth_header["requestHeaders"] = json!({"Authorization": "Bearer fixture-header-secret"});
    cases.push((auth_header, "101032006"));
    let mut invalid_header = baseline.clone();
    invalid_header["requestHeaders"] = json!({"X-Test": "first\r\nX-Injected: second"});
    cases.push((invalid_header, "101032006"));
    let mut foreign_model = baseline.clone();
    foreign_model["models"][0]["modelCode"] = json!(code(&foreign, 0));
    cases.push((foreign_model, "101032008"));
    let mut invalid_tokens = baseline.clone();
    invalid_tokens["models"][0]["maxOutputTokens"] = json!(16384);
    cases.push((invalid_tokens, "101032007"));
    let mut unsupported_input = baseline.clone();
    unsupported_input["protocol"] = json!("GoogleGemini");
    unsupported_input["models"][0]["inputCapabilities"] = json!(["Text", "Audio"]);
    cases.push((unsupported_input, "101032007"));
    let mut empty = baseline.clone();
    empty["models"] = json!([]);
    cases.push((empty, "101032007"));

    for (request, expected_code) in cases {
        let response = call(
            &fixture.router,
            Method::PUT,
            &endpoint("validated"),
            Some(request),
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(response["code"], expected_code);
        assert_eq!(response["data"], json!({}));
        assert_no_credentials(
            &response,
            &[
                "fixture-rejected-new-key",
                "fixture-url-secret",
                "fixture-header-secret",
            ],
        );
        assert_eq!(
            get(&fixture.router, "validated").await,
            initial,
            "validation must finish before persistence"
        );
        let runtime = fixture
            .models
            .text_agent_model_config("", Some(code(&initial, 0)))
            .unwrap()
            .unwrap();
        assert_eq!(runtime.api_key, "fixture-valid-key");
        assert_eq!(runtime.base_url, "https://valid.example.test/v1");
    }
    let count: i64 = fixture
        .database()
        .query_row(
            "SELECT count(*) FROM configured_provider_models WHERE provider_id='validated'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(get(&fixture.router, "foreign").await, foreign);

    let duplicate = call(
        &fixture.router,
        Method::POST,
        PROVIDERS,
        Some(provider(
            "validated",
            "https://duplicate.example.test",
            Some("fixture-duplicate-secret"),
        )),
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(duplicate["code"], "101032002");
    assert_eq!(get(&fixture.router, "validated").await, initial);
}

fn assert_defaults_removed(database: &Connection, model_code: &str) {
    for table in [
        "platform_default_models",
        "workspace_default_models",
        "default_model_selections",
    ] {
        let count: i64 = database
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE model_code=?1"),
                [model_code],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 0,
            "{table} must not retain a disabled or removed model"
        );
    }
}

#[tokio::test]
async fn disabling_removing_models_and_deleting_the_provider_clear_selection_and_defaults() {
    let fixture = Fixture::new();
    let mut request = provider(
        "lifecycle",
        "https://lifecycle.example.test/v1",
        Some("fixture-lifecycle-key"),
    );
    request["models"]
        .as_array_mut()
        .unwrap()
        .push(model("fallback", "Fallback"));
    let created = create(&fixture, request).await;
    let primary = code(&created, 0).to_owned();
    let fallback = code(&created, 1).to_owned();
    fixture.models.set_platform_default(&primary).unwrap();
    fixture
        .models
        .set_workspace_default("TRON_CORE", &primary)
        .unwrap();
    assert!(
        fixture
            .models
            .text_agent_model_config("TRON_CORE", Some(&primary))
            .unwrap()
            .is_some()
    );

    let mut disabled = update(&created, None);
    disabled["enabled"] = json!(false);
    let disabled = call(
        &fixture.router,
        Method::PUT,
        &endpoint("lifecycle"),
        Some(disabled),
        StatusCode::OK,
    )
    .await;
    for workspace in ["", "TRON_CORE"] {
        assert!(
            fixture
                .models
                .text_agent_model_config(workspace, Some(&primary))
                .unwrap()
                .is_none()
        );
        assert!(
            fixture
                .models
                .list_workspace_select_options(workspace)
                .unwrap()
                .iter()
                .filter(|item| item.model_code == primary)
                .all(|item| !item.enabled)
        );
    }
    assert_defaults_removed(&fixture.database(), &primary);

    let mut enabled = update(&disabled["data"], None);
    enabled["enabled"] = json!(true);
    let enabled = call(
        &fixture.router,
        Method::PUT,
        &endpoint("lifecycle"),
        Some(enabled),
        StatusCode::OK,
    )
    .await;
    fixture.models.set_platform_default(&primary).unwrap();
    fixture
        .models
        .set_workspace_default("TRON_CORE", &primary)
        .unwrap();
    let mut removed = update(&enabled["data"], None);
    removed["models"].as_array_mut().unwrap().remove(0);
    let remaining = call(
        &fixture.router,
        Method::PUT,
        &endpoint("lifecycle"),
        Some(removed),
        StatusCode::OK,
    )
    .await;
    assert_eq!(code(&remaining["data"], 0), fallback);
    assert!(
        fixture
            .models
            .text_agent_model_config("", Some(&primary))
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .models
            .list_workspace_select_options("TRON_CORE")
            .unwrap()
            .iter()
            .all(|item| item.model_code != primary)
    );
    assert_defaults_removed(&fixture.database(), &primary);
    let removed_count: i64 = fixture
        .database()
        .query_row(
            "SELECT count(*) FROM models WHERE model_code=?1",
            [&primary],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(removed_count, 0);

    fixture.models.set_platform_default(&fallback).unwrap();
    let deleted = call(
        &fixture.router,
        Method::DELETE,
        &endpoint("lifecycle"),
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(deleted["code"], "0");
    assert_eq!(deleted["data"], json!({}));
    assert_defaults_removed(&fixture.database(), &fallback);
    for table in ["configured_model_providers", "configured_provider_models"] {
        let count: i64 = fixture
            .database()
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE provider_id='lifecycle'"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
    let model_count: i64 = fixture
        .database()
        .query_row(
            "SELECT count(*) FROM models WHERE model_code IN (?1,?2)",
            params![primary, fallback],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(model_count, 0);
    let missing = call(
        &fixture.router,
        Method::GET,
        &endpoint("lifecycle"),
        None,
        StatusCode::NOT_FOUND,
    )
    .await;
    assert_eq!(missing["code"], "101032001");
}

#[tokio::test]
async fn explicit_protocols_and_models_work_without_model_discovery_or_a_required_api_key() {
    let fixture = Fixture::new();
    for (id, protocol, expected) in [
        (
            "chat",
            "OpenAiChatCompletions",
            AgentModelProtocol::ChatCompletions,
        ),
        (
            "responses",
            "OpenAiResponses",
            AgentModelProtocol::Responses,
        ),
        (
            "anthropic",
            "AnthropicMessages",
            AgentModelProtocol::Anthropic,
        ),
        ("gemini", "GoogleGemini", AgentModelProtocol::Gemini),
    ] {
        // A closed loopback port makes any accidental discovery call fail.
        let mut request = provider(id, "http://127.0.0.1:1/custom-api", None);
        request["protocol"] = json!(protocol);
        let created = create(&fixture, request).await;
        assert_eq!(created["hasApiKey"], false);
        let runtime = fixture
            .models
            .text_agent_model_config("", Some(code(&created, 0)))
            .unwrap()
            .unwrap();
        assert_eq!(runtime.protocol, expected);
        assert_eq!(runtime.base_url, "http://127.0.0.1:1/custom-api");
        assert_eq!(runtime.model_name, "same-remote-model");
        assert!(runtime.api_key.is_empty());
        let candidates = fixture
            .models
            .auto_chat_candidates("", &[code(&created, 0).to_owned()], None)
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].protocol, expected);
        assert_eq!(
            candidates[0]
                .request_headers
                .get("X-Test-Provider")
                .map(String::as_str),
            Some(id)
        );
    }
}

#[derive(Clone)]
struct ProtocolServerState {
    anthropic: bool,
    requests: Arc<Mutex<Vec<(Uri, HeaderMap, Value)>>>,
}

struct ProtocolServer {
    url: String,
    requests: Arc<Mutex<Vec<(Uri, HeaderMap, Value)>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for ProtocolServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn protocol_request(
    State(state): State<ProtocolServerState>,
    uri: Uri,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    state.requests.lock().unwrap().push((uri, headers, body));
    if state.anthropic {
        Json(json!({
            "id": "msg-provider-test", "type": "message", "role": "assistant",
            "model": "same-remote-model", "content": [{"type":"text", "text":"OK"}],
            "stop_reason": "end_turn", "stop_sequence": null,
            "usage": {"input_tokens":5, "output_tokens":1}
        }))
    } else {
        Json(json!({
            "candidates": [{"content":{"parts":[{"text":"OK"}], "role":"model"}, "finishReason":"STOP", "index":0}],
            "usageMetadata": {"promptTokenCount":5, "candidatesTokenCount":1, "totalTokenCount":6},
            "modelVersion":"same-remote-model", "responseId":"response-provider-test"
        }))
    }
}

async fn protocol_server(anthropic: bool) -> ProtocolServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let router = Router::new()
        .fallback(protocol_request)
        .with_state(ProtocolServerState {
            anthropic,
            requests: requests.clone(),
        });
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    ProtocolServer {
        url: format!("http://{address}/fixture"),
        requests,
        task,
    }
}

#[tokio::test]
async fn platform_verification_uses_production_clients_for_anthropic_and_gemini() {
    let fixture = Fixture::new();
    for (id, protocol, anthropic) in [
        ("verify-anthropic", "AnthropicMessages", true),
        ("verify-gemini", "GoogleGemini", false),
    ] {
        let server = protocol_server(anthropic).await;
        let key = "fixture-production-client-key";
        let mut request = provider(id, &server.url, Some(key));
        request["protocol"] = json!(protocol);
        let created = create(&fixture, request).await;
        let verified = call(
            &fixture.router,
            Method::POST,
            &format!("/api/v1/admin/platform/models/{}/verify", code(&created, 0)),
            Some(json!({})),
            StatusCode::OK,
        )
        .await;
        assert_eq!(verified["data"]["lastVerifyStatus"], "SUCCEEDED");
        assert_no_credentials(&verified, &[key]);
        let requests = server.requests.lock().unwrap();
        assert_eq!(
            requests.len(),
            1,
            "verification should issue one model request, without discovery"
        );
        let (uri, headers, body) = &requests[0];
        assert_eq!(
            headers
                .get("x-test-provider")
                .and_then(|value| value.to_str().ok()),
            Some(id)
        );
        assert!(
            uri.path().starts_with("/fixture/"),
            "custom API base path must be preserved"
        );
        if anthropic {
            assert_eq!(uri.path(), "/fixture/messages");
            assert_eq!(
                headers
                    .get("x-api-key")
                    .and_then(|value| value.to_str().ok()),
                Some(key)
            );
            assert!(headers.contains_key("anthropic-version"));
            assert_eq!(body["model"], "same-remote-model");
            assert!(
                body["messages"]
                    .as_array()
                    .is_some_and(|messages| !messages.is_empty())
            );
            assert!(body["max_tokens"].as_u64().is_some_and(|limit| limit > 0));
        } else {
            assert!(
                uri.path()
                    .ends_with("/models/same-remote-model:generateContent")
            );
            assert!(
                headers
                    .get("x-goog-api-key")
                    .and_then(|value| value.to_str().ok())
                    == Some(key)
                    || uri.query().is_some_and(|query| query
                        .split('&')
                        .any(|part| part == format!("key={key}")))
            );
            assert!(
                body["contents"]
                    .as_array()
                    .is_some_and(|contents| !contents.is_empty())
            );
            assert!(
                body["generationConfig"]["maxOutputTokens"]
                    .as_u64()
                    .is_some_and(|limit| limit > 0)
            );
        }
    }
}

#[tokio::test]
async fn configured_provider_management_requires_the_active_profile() {
    let router = common::app_without_profile();
    for (method, body) in [
        (Method::GET, None),
        (
            Method::POST,
            Some(provider(
                "unauthorized",
                "https://provider.example.test/v1",
                Some("fixture-unused-key"),
            )),
        ),
    ] {
        let response = call(&router, method, PROVIDERS, body, StatusCode::CONFLICT).await;
        assert_eq!(response["code"], "PERSONAI_PROFILE_REQUIRED");
    }
}
