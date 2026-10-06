mod common;

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::State,
    http::{Method, StatusCode},
    routing::post,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::crypto::Sm4GcmVariableCipher,
    application::variable::VariableCipher,
    config::{AppConfig, VariableCryptoConfig},
};

use common::{app_with_config, authenticated_bearer_token, request_with_headers, response_json};

const VARIABLE_KEY: &str = "RUNTIME_FIXTURE_SECRET";
const SECRET: &str = "runtime-variable-fixture-do-not-send-to-model";

struct Fixture {
    app: Router,
    authorization: String,
    session_code: String,
    model_code: String,
    requests: Arc<Mutex<Vec<Value>>>,
    database: std::path::PathBuf,
    _directory: tempfile::TempDir,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn model_reply(
    State(requests): State<Arc<Mutex<Vec<Value>>>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    requests.lock().unwrap().push(payload);
    Json(
        json!({"choices":[{"message":{"role":"assistant","content":"native fixture reply"},"finish_reason":"stop"}]}),
    )
}

async fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("chat-runtime-variables.sqlite");
    let app = app_with_config(AppConfig {
        database_url: database.to_string_lossy().into_owned(),
        ..Default::default()
    });
    let authorization = format!("Bearer {}", authenticated_bearer_token(&app).await);
    let requests: Arc<Mutex<Vec<Value>>> = Default::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
    let model = Router::new()
        .route("/v1/chat/completions", post(model_reply))
        .with_state(requests.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, model).await.unwrap();
    });
    let response = request_with_headers(&app,Method::POST,"/api/v1/admin/platform/models",Some(json!({
        "displayName":"Runtime variable fixture","vendor":"OpenAI","protocol":"OPENAI_COMPATIBLE","modelType":"TEXT","modelCapabilities":["TEXT_GENERATION"],"baseUrl":base_url,"apiPath":"/chat/completions","apiKeyRef":"fixture-model-key","modelName":"fixture-model","enabled":true,"thinkingEnabled":false,"structuredOutputEnabled":false,"maxTokens":128,"maxRetries":0
    })),None,Some(&authorization)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let model_code = response_json(response).await["data"]["modelCode"]
        .as_str()
        .unwrap()
        .to_owned();
    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(json!({"key":VARIABLE_KEY,"value":SECRET,"isSensitive":true,"valueMode":"REPLACE"})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = response_json(response).await;
    assert!(!created.to_string().contains(SECRET));
    let response = request_with_headers(
        &app,
        Method::POST,
        "/api/v1/chats/sessions",
        Some(json!({"workspaceCode":"TRON_CORE","title":"Runtime variable fixture"})),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let session_code = response_json(response).await["data"]["sessionCode"]
        .as_str()
        .unwrap()
        .to_owned();
    Fixture {
        app,
        authorization,
        session_code,
        model_code,
        requests,
        database,
        _directory: directory,
        server,
    }
}

async fn send(fixture: &Fixture) -> axum::response::Response {
    request_with_headers(&fixture.app,Method::POST,&format!("/api/v1/chats/sessions/{}/messages",fixture.session_code),Some(json!({"content":"hello native fixture","workspaceCode":"TRON_CORE","modelCode":fixture.model_code})),None,Some(&fixture.authorization)).await
}

fn stored_variable(database: &std::path::Path) -> Value {
    Connection::open(database).unwrap().query_row("SELECT variable_value,value_nonce,value_tag,value_algorithm,value_key_version,is_sensitive,updated_at FROM platform_variables WHERE variable_key=?1",[VARIABLE_KEY],|row| Ok(json!({
        "ciphertext":row.get::<_,String>(0)?,"nonce":row.get::<_,String>(1)?,"tag":row.get::<_,String>(2)?,"algorithm":row.get::<_,String>(3)?,"version":row.get::<_,String>(4)?,"sensitive":row.get::<_,i64>(5)?,"updated":row.get::<_,i64>(6)?
    }))).unwrap()
}

#[tokio::test]
async fn native_chat_resolves_sensitive_variables_without_sending_values_to_model() {
    let fixture = fixture().await;
    let before = stored_variable(&fixture.database);
    let response = send(&fixture).await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = response_json(response).await;
    assert_eq!(response["data"]["run"]["native"]["status"], "Completed");
    assert_eq!(
        response["data"]["run"]["finalOutput"],
        "native fixture reply"
    );
    assert!(!response.to_string().contains(SECRET));
    let requests = fixture.requests.lock().unwrap();
    assert!(!requests.is_empty(), "native model must actually run");
    assert!(
        requests
            .iter()
            .all(|request| !request.to_string().contains(SECRET)),
        "sensitive variable values must not enter model requests"
    );
    assert!(
        stored_variable(&fixture.database) == before,
        "running a chat must not rewrite stored variables"
    );
}

#[tokio::test]
async fn native_chat_reports_cipher_mismatch_without_model_dispatch_or_variable_mutation() {
    let fixture = fixture().await;
    // A valid synthetic SM4 envelope from a different key reproduces an existing
    // ciphertext/key mismatch without altering any real profile or secret.
    let other_cipher = Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
        key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
    })
    .unwrap();
    let encrypted = other_cipher
        .encrypt(SECRET, format!("platform:{VARIABLE_KEY}").as_bytes())
        .unwrap();
    Connection::open(&fixture.database).unwrap().execute("UPDATE platform_variables SET variable_value=?2,value_nonce=?3,value_tag=?4 WHERE variable_key=?1",params![VARIABLE_KEY,encrypted.ciphertext,encrypted.nonce,encrypted.tag]).unwrap();
    let before = stored_variable(&fixture.database);
    let response = send(&fixture).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let response = response_json(response).await;
    assert_eq!(response["code"], "VARIABLE_RUNTIME_DECRYPT_FAILED");
    assert_eq!(
        response["message"],
        "runtime variables could not be resolved: sensitive value could not be decrypted"
    );
    assert!(!response.to_string().contains(SECRET));
    assert!(!response.to_string().contains(VARIABLE_KEY));
    assert!(
        fixture.requests.lock().unwrap().is_empty(),
        "failed variable resolution must precede every model request"
    );
    assert!(
        stored_variable(&fixture.database) == before,
        "failed resolution must leave ciphertext and metadata intact"
    );
}
