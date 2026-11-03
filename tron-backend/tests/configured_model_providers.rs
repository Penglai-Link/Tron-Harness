mod common;

use std::path::PathBuf;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::{
    application::model::ModelService, config::AppConfig, domain::agent::AgentModelProtocol,
};

use common::{app_with_state_setup, authenticated_request, prepared_config, response_json};

const PROVIDERS: &str = "/api/v1/model-providers/configured";

#[tokio::test]
async fn api_key_summary_is_persisted_retained_and_updated_without_revealing_the_key() {
    let fixture = Fixture::new();
    let key = "sk-1234567890abcdef-sdf-hsd";
    let created = create(
        &fixture,
        provider("summary", "https://models.example.test/v1", Some(key)),
    )
    .await;
    assert_eq!(created["apiKeySummary"], "sk-123***sdf-hsd");
    assert_no_credentials(&created, &[key]);
    let stored: String = fixture
        .database()
        .query_row(
            "SELECT configuration_json FROM configured_model_providers WHERE provider_id=?1",
            [id(&created)],
            |row| row.get(0),
        )
        .unwrap();
    let stored: Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored["apiKeySummary"], created["apiKeySummary"]);
    assert!(!stored.to_string().contains(key));
    let retained = call(
        &fixture.router,
        Method::PUT,
        &endpoint(id(&created)),
        Some(update(&created, None)),
        StatusCode::OK,
    )
    .await;
    assert_eq!(retained["data"]["apiKeySummary"], created["apiKeySummary"]);
    let replacement = "sk-9876543210abcdef-qwe-zxc";
    let changed = call(
        &fixture.router,
        Method::PUT,
        &endpoint(id(&created)),
        Some(update(&created, Some(replacement))),
        StatusCode::OK,
    )
    .await;
    assert_eq!(changed["data"]["apiKeySummary"], "sk-987***qwe-zxc");
    assert_no_credentials(&changed, &[key, replacement]);
    let (router, models) = open(&fixture.config);
    assert_eq!(
        get(&router, id(&created)).await["apiKeySummary"],
        "sk-987***qwe-zxc"
    );
    assert_eq!(
        models
            .text_agent_model_config("workspace", Some(code(&created, 0)))
            .unwrap()
            .unwrap()
            .api_key,
        replacement
    );
}

#[tokio::test]
async fn api_key_summary_masks_short_keys_and_handles_unicode_and_absent_keys() {
    let fixture = Fixture::new();
    for (key, summary) in [
        ("abc", "***"),
        ("abcd", "***"),
        ("abcdef", "a***f"),
        ("abcdefghi", "ab***hi"),
        ("密钥安全测试中文", "密***文"),
    ] {
        let created = create(
            &fixture,
            provider("short-summary", "https://models.example.test/v1", Some(key)),
        )
        .await;
        assert_eq!(created["apiKeySummary"], summary);
        assert_no_credentials(&created, &[key]);
    }
    let without_key = create(
        &fixture,
        provider("no-summary", "https://models.example.test/v1", None),
    )
    .await;
    assert_eq!(without_key["apiKeySummary"], Value::Null);
    assert_eq!(without_key["hasApiKey"], false);
}

#[tokio::test]
async fn identical_create_requests_receive_distinct_server_generated_identities() {
    let fixture = Fixture::new();
    let request = provider("same-supplier", "https://models.example.test/v1", None);
    assert!(request.get("providerId").is_none());
    let first = create(&fixture, request.clone()).await;
    let second = create(&fixture, request).await;
    assert!(id(&first).starts_with("provider-"));
    assert_ne!(id(&first), id(&second));
    assert_eq!(get(&fixture.router, id(&first)).await, first);
    assert_eq!(get(&fixture.router, id(&second)).await, second);
}

#[tokio::test]
async fn same_vendor_connections_keep_names_credentials_and_models_independent() {
    let fixture = Fixture::new();
    let url = "https://same-vendor.example.test/v1";
    let mut personal = provider("vendor-personal", url, Some("personal-key"));
    personal["displayName"] = json!("My personal account");
    let mut work = provider("vendor-work", url, Some("work-key"));
    work["displayName"] = json!("My work account");
    let personal = create(&fixture, personal).await;
    let work = create(&fixture, work).await;
    assert_ne!(code(&personal, 0), code(&work, 0));
    for (record, key) in [(&personal, "personal-key"), (&work, "work-key")] {
        let model = fixture
            .models
            .text_agent_model_config("workspace", Some(code(record, 0)))
            .unwrap()
            .unwrap();
        assert_eq!(model.base_url, url);
        assert_eq!(model.api_key, key);
    }
    let mut changed = update(&personal, Some("rotated-personal-key"));
    changed["displayName"] = json!("Renamed personal account");
    call(
        &fixture.router,
        Method::PUT,
        &endpoint(id(&personal)),
        Some(changed),
        StatusCode::OK,
    )
    .await;
    let (router, models) = open(&fixture.config);
    assert_eq!(get(&router, id(&work)).await, work);
    assert_eq!(
        models
            .text_agent_model_config("workspace", Some(code(&work, 0)))
            .unwrap()
            .unwrap()
            .api_key,
        "work-key"
    );
    call(
        &router,
        Method::DELETE,
        &endpoint(id(&personal)),
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(get(&router, id(&work)).await, work);
}

#[tokio::test]
async fn configured_media_model_types_survive_reopening_and_stay_out_of_chat_candidates() {
    let fixture = Fixture::new();
    let types = [
        "Image",
        "Video",
        "SpeechSynthesis",
        "SpeechRecognition",
        "RealtimeSpeech",
    ];
    let mut body = provider("media-types", "https://models.example.test/v1", None);
    body["models"] = Value::Array(
        types
            .iter()
            .map(|kind| {
                let mut entry = model(kind, kind);
                entry["modelType"] = json!(kind);
                entry["contextWindowTokens"] = Value::Null;
                entry["maxOutputTokens"] = Value::Null;
                entry["inputCapabilities"] = if *kind == "SpeechRecognition" {
                    json!(["Audio"])
                } else {
                    json!(["Text", "Image"])
                };
                entry
            })
            .collect(),
    );
    let created = create(&fixture, body).await;
    for (index, kind) in types.iter().enumerate() {
        assert_eq!(created["models"][index]["modelType"], *kind);
    }
    let (router, models) = open(&fixture.config);
    assert_eq!(get(&router, id(&created)).await, created);
    assert!(
        models
            .auto_chat_candidates("workspace", &[], None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(models.list_select_options().unwrap().len(), types.len());
}

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
        "modelType": "Text",
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

fn id(provider: &Value) -> &str {
    provider["providerId"]
        .as_str()
        .expect("server-generated provider ID")
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
    assert_eq!(get(&fixture.router, id(&created)).await, created);

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
            .any(|item| item["providerCode"] == created["providerId"] && item["source"] == "Custom")
    );
    assert_no_credentials(&providers, &[key]);

    let stored: String = fixture
        .database()
        .query_row(
            "SELECT configuration_json FROM configured_model_providers WHERE provider_id=?1",
            [id(&created)],
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
    let (reopened_router, reopened_models) = open(&fixture.config);
    assert_eq!(get(&reopened_router, id(&created)).await, created);
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
        &endpoint(id(&first)),
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
        &endpoint(id(&first)),
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
    assert_eq!(get(&fixture.router, id(&second)).await, second);
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
            &endpoint(id(&initial)),
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
            get(&fixture.router, id(&initial)).await,
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
            "SELECT count(*) FROM configured_provider_models WHERE provider_id=?1",
            [id(&initial)],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(get(&fixture.router, id(&foreign)).await, foreign);

    let mut client_identity = provider(
        "validated",
        "https://duplicate.example.test",
        Some("fixture-duplicate-secret"),
    );
    client_identity["providerId"] = json!(id(&initial));
    call(
        &fixture.router,
        Method::POST,
        PROVIDERS,
        Some(client_identity),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(get(&fixture.router, id(&initial)).await, initial);
}

fn set_defaults(database: &Connection, code: &str) {
    for (scope, workspace) in [("platform", None), ("TRON_CORE", Some("TRON_CORE"))] {
        database.execute("INSERT INTO default_model_selections(scope_key,workspace_code,model_type,selection_kind,model_code,created_at,updated_at) VALUES(?1,?2,'TEXT','Fixed',?3,1,1) ON CONFLICT(scope_key,model_type) DO UPDATE SET model_code=excluded.model_code", params![scope,workspace,code]).unwrap();
    }
}

fn assert_defaults_removed(database: &Connection, model_code: &str) {
    for table in ["default_model_selections"] {
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
    set_defaults(&fixture.database(), &primary);
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
        &endpoint(id(&created)),
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
                .list_select_options()
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
        &endpoint(id(&created)),
        Some(enabled),
        StatusCode::OK,
    )
    .await;
    set_defaults(&fixture.database(), &primary);
    let mut removed = update(&enabled["data"], None);
    removed["models"].as_array_mut().unwrap().remove(0);
    let remaining = call(
        &fixture.router,
        Method::PUT,
        &endpoint(id(&created)),
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
            .list_select_options()
            .unwrap()
            .iter()
            .all(|item| item.model_code != primary)
    );
    assert_defaults_removed(&fixture.database(), &primary);
    let removed_count: i64 = fixture
        .database()
        .query_row(
            "SELECT count(*) FROM configured_provider_models WHERE model_code=?1",
            [&primary],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(removed_count, 0);

    set_defaults(&fixture.database(), &fallback);
    let deleted = call(
        &fixture.router,
        Method::DELETE,
        &endpoint(id(&created)),
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
                &format!("SELECT count(*) FROM {table} WHERE provider_id=?1"),
                [id(&created)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
    let model_count: i64 = fixture
        .database()
        .query_row(
            "SELECT count(*) FROM configured_provider_models WHERE model_code IN (?1,?2)",
            params![primary, fallback],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(model_count, 0);
    let missing = call(
        &fixture.router,
        Method::GET,
        &endpoint(id(&created)),
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

#[tokio::test]
async fn automatic_sources_default_to_all_providers_and_control_real_runtime_resolution() {
    use tron_backend::domain::model::AutoRouterKind;
    let fixture = Fixture::new();
    let first = create(
        &fixture,
        provider(
            "first",
            "https://first.example.test/v1",
            Some("first-secret"),
        ),
    )
    .await;
    let second = create(
        &fixture,
        provider(
            "second",
            "https://second.example.test/v1",
            Some("second-secret"),
        ),
    )
    .await;
    let all = fixture
        .models
        .automatic_route_models(AutoRouterKind::TextGeneration)
        .unwrap();
    assert_eq!(all.len(), 2);
    assert_ne!(code(&first, 0), code(&second, 0));
    // Same remote model, independent connection and credential identity.
    call(
        &fixture.router,
        Method::PUT,
        "/api/v1/automatic-routers/TextGeneration",
        Some(json!({"mode":"SelectedProviders","providerIds":[id(&second)],"modelCodes":[]})),
        StatusCode::OK,
    )
    .await;
    let selected = fixture
        .models
        .route_automatic_model("workspace", AutoRouterKind::TextGeneration)
        .unwrap();
    assert_eq!(selected.model_code, code(&second, 0));
    assert_eq!(selected.api_key, "second-secret");
    assert_eq!(selected.base_url, "https://second.example.test/v1");
    assert_eq!(
        fixture
            .models
            .automatic_route_models(AutoRouterKind::Chat)
            .unwrap()
            .len(),
        2
    );
    let (router, reopened) = open(&fixture.config);
    assert_eq!(
        reopened
            .route_automatic_model("workspace", AutoRouterKind::TextGeneration)
            .unwrap()
            .model_code,
        code(&second, 0)
    );
    let list = call(
        &router,
        Method::GET,
        "/api/v1/automatic-routers",
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(list["data"].as_array().unwrap().len(), 4);
    assert_no_credentials(&list, &["first-secret", "second-secret"]);
    call(
        &fixture.router,
        Method::PUT,
        "/api/v1/automatic-routers/TextGeneration",
        Some(json!({"mode":"AllProviders","providerIds":[],"modelCodes":[]})),
        StatusCode::OK,
    )
    .await;
    let third = create(
        &fixture,
        provider("third", "https://third.example.test/v1", None),
    )
    .await;
    assert_eq!(
        fixture
            .models
            .automatic_route_models(AutoRouterKind::TextGeneration)
            .unwrap()
            .len(),
        3
    );
    call(
        &fixture.router,
        Method::PUT,
        "/api/v1/automatic-routers/Chat",
        Some(json!({"mode":"SelectedModels","providerIds":[],"modelCodes":[code(&third,0)]})),
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        fixture.models.automatic_chat_model_codes().unwrap(),
        [code(&third, 0).to_owned()].into_iter().collect()
    );
    assert_eq!(
        fixture
            .models
            .automatic_route_models(AutoRouterKind::TextGeneration)
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn empty_and_invalid_sources_never_widen_runtime_candidates() {
    use tron_backend::domain::model::AutoRouterKind;
    let fixture = Fixture::new();
    let created = create(
        &fixture,
        provider("empty", "https://models.example.test/v1", None),
    )
    .await;
    for body in [
        json!({"mode":"SelectedModels","providerIds":[],"modelCodes":["invented-model"]}),
        json!({"mode":"SelectedProviders","providerIds":["invented-provider"],"modelCodes":[]}),
        json!({"mode":"AllProviders","providerIds":[id(&created)],"modelCodes":[]}),
    ] {
        call(
            &fixture.router,
            Method::PUT,
            "/api/v1/automatic-routers/Chat",
            Some(body),
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    call(
        &fixture.router,
        Method::PUT,
        "/api/v1/automatic-routers/TextGeneration",
        Some(json!({"mode":"SelectedModels","providerIds":[],"modelCodes":[]})),
        StatusCode::OK,
    )
    .await;
    assert!(
        fixture
            .models
            .route_automatic_model("workspace", AutoRouterKind::TextGeneration)
            .is_err()
    );
    assert_eq!(
        fixture
            .models
            .automatic_route_models(AutoRouterKind::Chat)
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn text_and_multimodal_vector_and_rerank_types_remain_distinct_through_storage_and_runtime() {
    use tron_backend::domain::model::{AutoRouterKind, ModelType};
    let fixture = Fixture::new();
    let mut request = provider("modalities", "https://models.example.test/v1", None);
    request["models"] = json!([]);
    for kind in [
        "TextEmbedding",
        "MultimodalEmbedding",
        "TextRerank",
        "MultimodalRerank",
    ] {
        let mut m = model(kind, kind);
        m["modelType"] = json!(kind);
        if kind.starts_with("Multimodal") {
            m["inputCapabilities"] = json!(["Text", "Image"]);
        }
        request["models"].as_array_mut().unwrap().push(m);
    }
    let created = create(&fixture, request).await;
    for (route, expected) in [
        (
            AutoRouterKind::Embedding,
            [ModelType::TextEmbedding, ModelType::MultimodalEmbedding],
        ),
        (
            AutoRouterKind::Rerank,
            [ModelType::TextRerank, ModelType::MultimodalRerank],
        ),
    ] {
        let candidates = fixture.models.automatic_route_models(route).unwrap();
        assert_eq!(candidates.len(), 2);
        for kind in expected {
            assert!(candidates.iter().any(|m| m.model_type == kind));
        }
        let (_, reopened) = open(&fixture.config);
        assert_eq!(reopened.automatic_route_models(route).unwrap().len(), 2);
    }
    let mut invalid = update(&created, None);
    invalid["models"][0]["inputCapabilities"] = json!(["Text", "Image"]);
    call(
        &fixture.router,
        Method::PUT,
        &endpoint(id(&created)),
        Some(invalid),
        StatusCode::BAD_REQUEST,
    )
    .await;
}

#[tokio::test]
async fn auxiliary_text_requests_prefer_text_models_without_overriding_explicit_choices() {
    use eliteforge_agent_core::{Modality, ModelChoice};
    use tron_backend::domain::model::{AutoRouteSourceMode, AutoRouteSources, AutoRouterKind};
    let fixture = Fixture::new();
    let mut request = provider("modalities", "https://models.example.test/v1", None);
    request["models"] = json!([]);
    for (kind, logical) in [
        ("MultimodalEmbedding", "a-multimodal-embedding"),
        ("TextEmbedding", "z-text-embedding"),
        ("MultimodalRerank", "a-multimodal-rerank"),
        ("TextRerank", "z-text-rerank"),
    ] {
        let mut m = model(logical, logical);
        m["modelType"] = json!(kind);
        m["logicalModelCode"] = json!(logical);
        if kind.ends_with("Embedding") {
            m["dimension"] = json!(1024);
        }
        if kind.starts_with("Multimodal") {
            m["inputCapabilities"] = json!(["Text", "Image"]);
        }
        request["models"].as_array_mut().unwrap().push(m);
    }
    let created = create(&fixture, request).await;
    for (kind, suffix, dimensions, multi_index) in [
        (AutoRouterKind::Embedding, "embedding", Some(1024), 0),
        (AutoRouterKind::Rerank, "rerank", None, 2),
    ] {
        let text = format!("z-text-{suffix}");
        let multi = format!("a-multimodal-{suffix}");
        assert_eq!(
            fixture.models.automatic_route_models(kind).unwrap()[0].model_code,
            multi,
            "the catalog order must reproduce the original regression"
        );
        let route = |choice: ModelChoice| {
            fixture
                .models
                .route_auxiliary_model("workspace", kind, &choice, dimensions)
                .unwrap()
        };
        assert_eq!(route(ModelChoice::default()).model_code, text);
        assert_eq!(
            route(ModelChoice {
                required_modalities: vec![Modality::Text],
                ..Default::default()
            })
            .model_code,
            text
        );
        assert_eq!(
            route(ModelChoice {
                required_modalities: vec![Modality::Text, Modality::Image],
                ..Default::default()
            })
            .model_code,
            multi
        );
        assert_eq!(
            route(ModelChoice {
                model_code: Some(multi.clone()),
                required_modalities: vec![Modality::Text],
                ..Default::default()
            })
            .model_code,
            multi
        );
        assert_eq!(
            route(ModelChoice {
                model_code: Some(code(&created, multi_index).into()),
                ..Default::default()
            })
            .model_code,
            code(&created, multi_index)
        );
        fixture
            .models
            .save_automatic_route_sources(
                kind,
                AutoRouteSources {
                    mode: AutoRouteSourceMode::SelectedModels,
                    provider_ids: vec![],
                    model_codes: vec![multi.clone()],
                },
            )
            .unwrap();
        // A text request can still use multimodal when that is the configured pool.
        assert_eq!(route(ModelChoice::default()).model_code, multi);
    }
}

#[tokio::test]
async fn unified_routes_group_explicit_logical_models_and_keep_wire_ids_and_spaces_separate() {
    use eliteforge_agent_core::{Modality, ModelChoice};
    use tron_backend::domain::model::AutoRouterKind;
    let fixture = Fixture::new();
    let mut created = vec![];
    for (name, wire) in [("first", "vendor-alias-a"), ("second", "vendor-alias-b")] {
        let mut request = provider(name, "https://models.example.test/v1", None);
        let mut m = model(wire, "Shared embedding");
        m["modelType"] = json!("MultimodalEmbedding");
        m["dimension"] = json!(1024);
        m["inputCapabilities"] = json!(["Text", "Image"]);
        m["logicalModelCode"] = json!("shared-embedding-v1");
        request["models"] = json!([m]);
        created.push(create(&fixture, request).await);
    }
    let routes = call(
        &fixture.router,
        Method::GET,
        "/api/v1/automatic-routers",
        None,
        StatusCode::OK,
    )
    .await;
    assert_eq!(routes["data"].as_array().unwrap().len(), 4);
    let embedding = routes["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|route| route["kind"] == "Embedding")
        .unwrap();
    assert_eq!(embedding["candidates"].as_array().unwrap().len(), 2);
    assert!(
        embedding["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["modelCode"] == "shared-embedding-v1"
                && m["bindingId"].as_str().unwrap().starts_with("MODEL-"))
    );
    let mut choice = ModelChoice {
        model_code: Some("shared-embedding-v1".into()),
        required_modalities: vec![Modality::Text],
        ..Default::default()
    };
    let pool = fixture
        .models
        .route_auxiliary_model("workspace", AutoRouterKind::Embedding, &choice, Some(1024))
        .unwrap();
    assert_eq!(pool.route.as_ref().unwrap().candidates.len(), 2);
    assert_eq!(pool.model_code, "shared-embedding-v1");
    let wire_ids = pool
        .route
        .as_ref()
        .unwrap()
        .candidates
        .iter()
        .map(|m| m.config.model_name.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        wire_ids,
        std::collections::HashSet::from(["vendor-alias-a", "vendor-alias-b"])
    );
    let fixed = fixture
        .models
        .route_auxiliary_model(
            "workspace",
            AutoRouterKind::Embedding,
            &ModelChoice {
                model_code: Some(code(&created[0], 0).into()),
                ..Default::default()
            },
            Some(1024),
        )
        .unwrap();
    assert_eq!(fixed.model_code, code(&created[0], 0));
    assert!(!fixed.route.as_ref().unwrap().shared_identity);
    assert_eq!(fixed.route.as_ref().unwrap().candidates.len(), 1);
    choice.provider_id = Some(id(&created[0]).into());
    assert_eq!(
        fixture
            .models
            .route_auxiliary_model("workspace", AutoRouterKind::Embedding, &choice, Some(1024))
            .unwrap()
            .route
            .unwrap()
            .candidates
            .len(),
        1
    );
    choice.required_modalities = vec![Modality::Audio];
    assert!(
        fixture
            .models
            .route_auxiliary_model("workspace", AutoRouterKind::Embedding, &choice, Some(1024))
            .is_err()
    );
    choice.required_modalities = vec![Modality::Image];
    assert!(
        fixture
            .models
            .route_auxiliary_model("workspace", AutoRouterKind::Embedding, &choice, Some(768))
            .is_err()
    );
    // A pre-existing binding selection cannot silently widen to every provider
    // merely because the operator later adds a shared logical identity.
    fixture
        .models
        .save_automatic_route_sources(
            AutoRouterKind::Embedding,
            tron_backend::domain::model::AutoRouteSources {
                mode: tron_backend::domain::model::AutoRouteSourceMode::SelectedModels,
                provider_ids: vec![],
                model_codes: vec![code(&created[0], 0).into()],
            },
        )
        .unwrap();
    choice.provider_id = None;
    choice.required_modalities = vec![Modality::Text];
    assert_eq!(
        fixture
            .models
            .route_auxiliary_model("workspace", AutoRouterKind::Embedding, &choice, Some(1024))
            .unwrap()
            .route
            .unwrap()
            .candidates
            .len(),
        1
    );
    fixture
        .models
        .save_automatic_route_sources(AutoRouterKind::Embedding, Default::default())
        .unwrap();
    // Index identity survives provider removal and different upstream aliases.
    let llm = tron_backend::application::llm::LlmRuntimeService::new(
        std::sync::Arc::new(fixture.models.clone()),
        std::sync::Arc::new(
            tron_backend::adapters::outbound::model::HttpLlmRuntimeClient::new(
                eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
            ),
        ),
    );
    assert_eq!(
        llm.resolve_embedding_index_model_code("workspace", Some("AUTO:Embedding"))
            .unwrap(),
        "shared-embedding-v1"
    );
    let fingerprint = llm
        .embedding_space_fingerprint("workspace", Some("shared-embedding-v1"), 1024)
        .unwrap();
    let mut disabled = update(&created[0], None);
    disabled["enabled"] = json!(false);
    call(
        &fixture.router,
        Method::PUT,
        &endpoint(id(&created[0])),
        Some(disabled),
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        llm.embedding_space_fingerprint("workspace", Some("shared-embedding-v1"), 1024)
            .unwrap(),
        fingerprint
    );
    assert!(
        llm.embedding_space_fingerprint("workspace", Some("shared-embedding-v1"), 768)
            .is_err()
    );
    let mut incompatible = update(&created[1], None);
    incompatible["models"][0]["dimension"] = json!(768);
    call(
        &fixture.router,
        Method::PUT,
        &endpoint(id(&created[1])),
        Some(incompatible),
        StatusCode::BAD_REQUEST,
    )
    .await;
}
