use super::*;
use std::collections::VecDeque;

use crate::{adapters::outbound::crypto::Sm4GcmVariableCipher, config::VariableCryptoConfig};
use axum::{Router, body::to_bytes, extract::State, http::StatusCode, response::IntoResponse};

// Synthetic public key generated solely for this fixture; no private key is stored.
const PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAlKl+adEvKdKFqV+yOzZX\ncLN501KY3n07350ilbPO9h5y+vQII/TIg7ds++O50O/1Md5tAM2CxxtRzKLZR0tG\naqlZfIx8GtBFFi1UsB/aX2WnxSgwctFgwFAK7dMcc0J8/UUQ2jORuTOCestuXODV\nYHlP3HAtmZQ00a3LQHJFwfLNvWn4zgAbD3s6tagpcjtl6qxzRB3d42d1Zp80enVr\n1sAQL1yGNlF2y9f19HG1U+Bn+KfX/0cWymn8yDSixjPLWN3N/7FBpAD6NnNykYs2\n82pF0ITF5WxLoTfUxAMa1PRzLhvaC5czX6blziL3dziJh1ObQvt4Onp18ASWXypR\nVQIDAQAB\n-----END PUBLIC KEY-----";

struct Step {
    method: &'static str,
    path: &'static str,
    status: StatusCode,
    body: Value,
}

#[derive(Default)]
struct MockState {
    steps: VecDeque<Step>,
    observed: Vec<String>,
    errors: Vec<String>,
}

type SharedMock = Arc<Mutex<MockState>>;

async fn respond(
    State(state): State<SharedMock>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let product = request
        .headers()
        .get("x-product-code")
        .is_some_and(|value| value == "tron");
    let client = request
        .headers()
        .get("x-client-id")
        .is_some_and(|value| value == "fixture-client");
    let body = to_bytes(request.into_body(), 1024 * 1024).await.unwrap();
    let mut state = state.lock().unwrap();
    state.observed.push(format!("{method} {path}"));
    let Some(step) = state.steps.pop_front() else {
        state.errors.push("unexpected additional request".into());
        return (
            StatusCode::NOT_FOUND,
            axum::Json(json!({"message":"Not found"})),
        );
    };
    if method != step.method || path != format!("/person-ai-index/api/v1{}", step.path) {
        state
            .errors
            .push(format!("unexpected route: {method} {path}"));
        return (
            StatusCode::NOT_FOUND,
            axum::Json(json!({"message":"Not found"})),
        );
    }
    if method == "POST" {
        let envelope: Value = serde_json::from_slice(&body).unwrap();
        if !product
            || !client
            || envelope.get("messages").is_some()
            || envelope.get("input").is_some()
        {
            state
                .errors
                .push("model request lost product/client headers or encryption".into());
        }
    }
    (step.status, axum::Json(step.body))
}

fn step(method: &'static str, path: &'static str, data: Value) -> Step {
    Step {
        method,
        path,
        status: StatusCode::OK,
        body: json!({"code":200,"data":data}),
    }
}

fn completed_steps(chat_status: StatusCode) -> VecDeque<Step> {
    let mut steps = VecDeque::from([
        step(
            "GET",
            "/auth/client-sessions/fixture-session",
            json!({"session_id":"fixture-session","status":"completed","token":"new-fixture-token","refresh_token":"new-fixture-refresh","client_id":"fixture-client","public_key":PUBLIC_KEY,"expires_in":3600}),
        ),
        step(
            "GET",
            "/chat/models",
            json!([{"model_code":"fixture-chat","model_name":"Fixture Chat"}]),
        ),
        step(
            "GET",
            "/chat/models",
            json!([{"model_code":"fixture-chat","model_name":"Fixture Chat"}]),
        ),
        step(
            "GET",
            "/chat/models",
            json!([{"model_code":"fixture-chat","model_name":"Fixture Chat"}]),
        ),
        step("GET", "/multimedia/models", json!([])),
        Step {
            method: "POST",
            path: "/chat/completions",
            status: chat_status,
            body: if chat_status == StatusCode::OK {
                json!({"choices":[{"message":{"role":"assistant","content":"OK"},"finish_reason":"stop"}]})
            } else {
                json!({"message":"Not found (device_unbound / 设备已解绑)","error":{"type":"device_unbound"}})
            },
        },
    ]);
    if chat_status == StatusCode::OK {
        steps.extend([
            Step {
                method: "POST",
                path: "/embeddings",
                status: StatusCode::OK,
                body: json!({"data":[{"index":0,"embedding":[0.25,0.75]}]}),
            },
            step(
                "GET",
                "/auth/token/status",
                json!({"valid":true,"expired":false,"subject":"fixture-user"}),
            ),
            step(
                "GET",
                "/account/info",
                json!({"username":"fixture-user-name"}),
            ),
        ]);
    }
    steps
}

async fn fixture() -> (
    ModelProviderService,
    SharedMock,
    tokio::task::JoinHandle<()>,
) {
    let state: SharedMock = Default::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}/person-ai-index", listener.local_addr().unwrap());
    let router = Router::new().fallback(respond).with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
        })
        .unwrap(),
    );
    let service = ModelProviderService::connect(":memory:", cipher).unwrap();
    let previous = ProviderCredentials {
        token: "old-fixture-token".into(),
        refresh_token: "old-fixture-refresh".into(),
        client_id: "fixture-client".into(),
        public_key: PUBLIC_KEY.into(),
        server_base_url: base_url.clone(),
        authorization_expires_at: None,
    };
    let encrypted = service.encrypt_credentials(&previous).unwrap();
    {
        let connection = service.connection.lock().unwrap();
        connection.execute("INSERT INTO provider_connections(provider_code,server_base_url,status,credentials_encrypted,person_ai_user_id,person_ai_username,last_error,created_at,updated_at) VALUES ('PersonAi',?1,'AuthRequired',?2,'previous-user','previous-name','previous authorization failure',1,1)",params![base_url,encrypted]).unwrap();
        connection.execute("INSERT INTO provider_authorization_sessions(session_id,provider_code,server_base_url,client_name,login_url,status,created_at,updated_at) VALUES ('fixture-session','PersonAi',?1,'Tron','https://unused.invalid','pending',1,1)",[base_url]).unwrap();
    }
    (service, state, server)
}

async fn poll(
    service: &ModelProviderService,
) -> Result<CompleteProviderAuthorizationResult, ModelProviderError> {
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        service.poll_authorization_session(CompleteProviderAuthorizationCommand {
            session_id: "fixture-session".into(),
        }),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn completed_personai_authorization_verifies_distinct_model_routes_before_commit() {
    let (service, state, server) = fixture().await;
    state.lock().unwrap().steps = completed_steps(StatusCode::OK);
    let result = poll(&service).await.unwrap();
    assert!(result.text_verified && result.embedding_verified);
    let connection = service.connection.lock().unwrap();
    let row = read_connection(&connection, ModelProviderCode::PersonAi)
        .unwrap()
        .unwrap();
    assert_eq!(row.status, ProviderConnectionStatus::Connected);
    assert_eq!(row.person_ai_user_id.as_deref(), Some("fixture-user"));
    assert!(row.last_error.is_none());
    let committed = service
        .decrypt_credentials(row.credentials_encrypted.as_deref().unwrap())
        .unwrap();
    assert!(committed.token == "new-fixture-token");
    assert!(
        read_person_ai_profile_credentials(&connection, "fixture-user")
            .unwrap()
            .is_some()
    );
    assert_eq!(
        catalog_model_count(&connection, ModelProviderCode::PersonAi).unwrap(),
        1
    );
    assert_eq!(
        read_authorization_session(&connection, "fixture-session")
            .unwrap()
            .status,
        "Completed"
    );
    drop(connection);
    let state = state.lock().unwrap();
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.steps.is_empty());
    assert_eq!(
        state
            .observed
            .iter()
            .filter(|path| path.ends_with("/chat/completions"))
            .count(),
        1
    );
    assert_eq!(
        state
            .observed
            .iter()
            .filter(|path| path.ends_with("/embeddings"))
            .count(),
        1
    );
    server.abort();
}

#[tokio::test]
async fn completed_personai_authorization_404_preserves_old_credentials_and_can_resume_same_session()
 {
    let (service, state, server) = fixture().await;
    let previous = read_connection(
        &service.connection.lock().unwrap(),
        ModelProviderCode::PersonAi,
    )
    .unwrap()
    .unwrap();
    state.lock().unwrap().steps = completed_steps(StatusCode::NOT_FOUND);
    let error = poll(&service).await.unwrap_err();
    assert!(
        matches!(error,ModelProviderError::Remote(ref message) if message.contains("personai model error: Not found"))
    );
    {
        let connection = service.connection.lock().unwrap();
        let row = read_connection(&connection, ModelProviderCode::PersonAi)
            .unwrap()
            .unwrap();
        assert_eq!(row.status, ProviderConnectionStatus::AuthRequired);
        assert!(row.credentials_encrypted == previous.credentials_encrypted);
        assert_eq!(row.person_ai_user_id, previous.person_ai_user_id);
        assert_eq!(row.last_error, previous.last_error);
        assert_eq!(
            read_authorization_session(&connection, "fixture-session")
                .unwrap()
                .status,
            "pending"
        );
        assert_eq!(
            catalog_model_count(&connection, ModelProviderCode::PersonAi).unwrap(),
            0
        );
    }
    assert!(!service.device_unbound());
    {
        let mut state = state.lock().unwrap();
        assert!(state.errors.is_empty(), "{:?}", state.errors);
        assert!(state.steps.is_empty());
        state.steps = completed_steps(StatusCode::OK);
    }
    let resumed = poll(&service).await.unwrap();
    assert!(resumed.text_verified && resumed.embedding_verified);
    assert_eq!(
        service
            .get_connection(ModelProviderCode::PersonAi)
            .unwrap()
            .status,
        ProviderConnectionStatus::Connected
    );
    let state = state.lock().unwrap();
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.steps.is_empty());
    assert_eq!(
        state
            .observed
            .iter()
            .filter(|path| path.ends_with("/auth/client-sessions/fixture-session"))
            .count(),
        2
    );
    server.abort();
}

#[tokio::test]
async fn completed_personai_authorization_401_device_unbound_still_clears_credentials() {
    let (service, state, server) = fixture().await;
    state.lock().unwrap().steps = completed_steps(StatusCode::UNAUTHORIZED);
    assert_eq!(
        poll(&service).await.unwrap_err(),
        ModelProviderError::DeviceUnbound
    );
    let connection = service.connection.lock().unwrap();
    let row = read_connection(&connection, ModelProviderCode::PersonAi)
        .unwrap()
        .unwrap();
    assert_eq!(row.status, ProviderConnectionStatus::AuthRequired);
    assert!(row.credentials_encrypted.is_none());
    assert_eq!(
        row.last_error.as_deref(),
        Some("PersonAI device was unbound; login again")
    );
    assert!(matches!(
        read_authorization_session(&connection, "fixture-session"),
        Err(ModelProviderError::AuthorizationSessionNotFound)
    ));
    assert!(service.device_unbound());
    let state = state.lock().unwrap();
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.steps.is_empty());
    server.abort();
}
