use super::*;
use std::collections::VecDeque;

use crate::{adapters::outbound::crypto::Sm4GcmVariableCipher, config::VariableCryptoConfig};
use axum::{Router, body::to_bytes, extract::State, http::StatusCode, response::IntoResponse};
use sdk::ModelTransport;

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
    model_queries: Vec<String>,
}

type SharedMock = Arc<Mutex<MockState>>;

#[tokio::test]
async fn tron_chat_only_uses_high_quality_personai_models() {
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
        })
        .unwrap(),
    );
    let service = ModelProviderService::connect(":memory:", cipher).unwrap();
    service.connection.call(|connection| {
        connection.execute_batch(
            "INSERT INTO provider_connections(provider_code,server_base_url,status,created_at,updated_at)
             VALUES('PersonAi','https://provider.invalid','Connected',1,1);
             INSERT INTO provider_catalog_models(provider_code,model_code,model_name,upstream_vendor_code,
                 context_window_tokens,quality_type,status,first_seen_at,updated_at) VALUES
             ('PersonAi','shared','Same model','vendor',128000,'High','Available',1,1),
             ('PersonAi','shared','Same model','vendor',128000,'Middle','Available',1,1),
             ('PersonAi','minimax-m3','MiniMax M3','minimax_plan',128000,'Middle','Available',1,1),
             ('PersonAi','low-model','Low model','vendor',NULL,'Low','Available',1,1);"
        ).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    let candidates = service.auto_chat_candidates(&[], None).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].0,
        "provider:PersonAi:model:High:vendor/shared"
    );
    assert_eq!(
        person_ai_runtime_quality(&candidates[0].2),
        Some(ProviderModelQuality::High)
    );
    let middle = "provider:PersonAi:model:Middle:vendor/shared".to_owned();
    assert!(
        service
            .auto_chat_candidates(&[middle.clone()], None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        service
            .auto_chat_candidates(&[middle, candidates[0].0.clone()], None)
            .unwrap()
            .len(),
        1
    );
    let options = service.list_text_select_options("").unwrap();
    assert!(
        options
            .iter()
            .all(|option| option.quality_type == Some(ProviderModelQuality::High))
    );
    assert_eq!(
        options
            .iter()
            .filter(|option| option.selection.kind == ModelSelectionKind::Fixed)
            .count(),
        1
    );
    for quality in [ProviderModelQuality::Middle, ProviderModelQuality::Low] {
        let code = if quality == ProviderModelQuality::Middle {
            "vendor/shared"
        } else {
            "vendor/low-model"
        };
        assert!(matches!(
            service.resolve_chat_selection(
                "",
                Some(&ModelSelection::person_ai_fixed(code, quality)),
                None,
                None
            ),
            Err(ModelProviderError::InvalidSelection(_))
        ));
    }
    let router = crate::application::model_router::LlmModelRouter::new(service.clone());
    let temporary = router.route_text("", None).unwrap();
    assert_eq!(
        temporary.model_code,
        "PERSON_AI:FIXED:Middle:minimax_plan/minimax-m3"
    );
    assert!(!temporary.thinking_enabled);
    assert!(temporary.reasoning_effort.is_none());
    assert_eq!(
        router
            .route_text("", Some("PERSON_AI:ROUTE:TextGenerationAuto"))
            .unwrap()
            .model_code,
        temporary.model_code
    );
    struct LastTextModel;
    impl fw::TextGenerationAutoRouter for LastTextModel {
        fn select(
            &self,
            _: &fw::AutoRouteContext<'_>,
            candidates: &[fw::AutoRouteCandidate],
        ) -> Result<fw::ModelRef, fw::AgentError> {
            Ok(candidates.last().unwrap().model.clone())
        }
    }
    let custom = crate::application::model_router::LlmModelRouter::new(
        service.clone().with_auto_routers(fw::AutoRouters {
            text_generation: Some(Arc::new(LastTextModel)),
            ..Default::default()
        }),
    );
    assert_eq!(
        custom.route_text("", None).unwrap().model_code,
        "PERSON_AI:FIXED:Low:vendor/low-model"
    );
    assert!(
        router
            .route_text("", Some("PERSON_AI:FIXED:Middle:vendor/shared"))
            .is_ok()
    );
    service.connection.call(|connection| {
        connection.execute("UPDATE provider_catalog_models SET status='Unavailable' WHERE quality_type='High'", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    assert!(service.auto_chat_candidates(&[], None).unwrap().is_empty());
    assert!(router.route_text("", None).is_ok());
    service.connection.call(|connection| {
        connection.execute("UPDATE provider_catalog_models SET status='Unavailable' WHERE quality_type='Middle'", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    assert_eq!(
        router.route_text("", None).unwrap().model_code,
        "PERSON_AI:FIXED:Low:vendor/low-model"
    );
    assert!(service.auto_chat_candidates(&[], None).unwrap().is_empty());
    service.connection.call(|connection| {
        connection.execute("UPDATE provider_catalog_models SET status=CASE quality_type WHEN 'High' THEN 'Available' ELSE 'Unavailable' END", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    assert!(!service.auto_chat_candidates(&[], None).unwrap().is_empty());
    assert!(router.route_text("", None).is_err());
}

#[test]
fn embedding_auto_route_stays_within_one_model_code() {
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
        })
        .unwrap(),
    );
    let service = ModelProviderService::connect(":memory:", cipher).unwrap();
    let selected = service.connection.call(
        |connection| {
            for (id, code, status) in [
                ("a", "bge-m3", "Available"),
                ("b", "bge-m3", "Available"),
                ("c", "other-model", "Available"),
                ("d", "bge-m3", "Unavailable"),
            ] {
                connection.execute(
                    "INSERT INTO provider_multimedia_catalog_models
                     (provider_code,model_id,model_code,model_name,media_type,status,first_seen_at,last_seen_at,updated_at)
                     VALUES('PersonAi',?1,?2,?2,6,?3,1,1,1)",
                    params![id, code, status],
                ).unwrap();
            }
            Ok::<_, ModelProviderError>((0..4)
                .map(|ordinal| select_embedding_instance(connection, "bge-m3", ordinal).unwrap().unwrap())
                .collect::<Vec<_>>())
        },
        || ModelProviderError::Storage,
    ).unwrap();
    assert_eq!(selected, ["a", "b", "a", "b"]);
}

#[tokio::test]
async fn embedding_transport_dispatches_instance_and_reports_logical_model() {
    struct Capture(Arc<Mutex<Value>>);
    #[async_trait]
    impl sdk::ModelTransport for Capture {
        async fn json(&self, _: &str, body: &Value) -> Result<Value, fw::ModelError> {
            *self.0.lock().unwrap() = body.clone();
            Ok(json!({"model":"physical-instance","data":[]}))
        }
        async fn events(
            &self,
            _: &str,
            _: &Value,
            _: sdk::EventSink<'_>,
        ) -> Result<(), fw::ModelError> {
            unreachable!()
        }
    }
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
        })
        .unwrap(),
    );
    let observed = Arc::new(Mutex::new(Value::Null));
    let transport = SnapshotTransport {
        inner: Arc::new(Capture(observed.clone())),
        providers: ModelProviderService::connect(":memory:", cipher).unwrap(),
        config: person_ai_agent_config(
            "PERSON_AI:EMBEDDING:bge-m3",
            "bge-m3",
            None,
            None,
            AgentModelProtocol::Embeddings,
        ),
        run: None,
        attempt: "fixture".into(),
        embedding_instance_id: Some("physical-instance".into()),
        embedding_logical_code: Some("bge-m3".into()),
    };
    let response = transport
        .json("embeddings", &json!({"model":"bge-m3","input":["text"]}))
        .await
        .unwrap();
    assert_eq!(observed.lock().unwrap()["model"], "physical-instance");
    assert_eq!(response["model"], "bge-m3");
}

async fn respond(
    State(state): State<SharedMock>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let query = request.uri().query().unwrap_or_default().to_owned();
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
    if path.ends_with("/chat/models") {
        state.model_queries.push(query);
    }
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
        service.connection.call(move |connection| {
        connection.execute("INSERT INTO provider_connections(provider_code,server_base_url,status,credentials_encrypted,person_ai_user_id,person_ai_username,last_error,created_at,updated_at) VALUES ('PersonAi',?1,'AuthRequired',?2,'previous-user','previous-name','previous authorization failure',1,1)",params![base_url,encrypted]).unwrap();
        connection.execute("INSERT INTO provider_authorization_sessions(session_id,provider_code,server_base_url,client_name,login_url,status,created_at,updated_at) VALUES ('fixture-session','PersonAi',?1,'Tron','https://unused.invalid','pending',1,1)",[base_url]).unwrap();
        Ok::<_,ModelProviderError>(())
        },||ModelProviderError::Storage).unwrap();
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
async fn upstream_model_401_keeps_personai_authorization_and_exposes_unavailable() {
    let (service, state, server) = fixture().await;
    let row = service
        .connection
        .call(
            |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
        .unwrap()
        .unwrap();
    let mut credentials = service
        .decrypt_credentials(row.credentials_encrypted.as_deref().unwrap())
        .unwrap();
    credentials.authorization_expires_at = Some(now_millis() + 30 * 24 * 60 * 60 * 1000);
    let encrypted = service.encrypt_credentials(&credentials).unwrap();
    service
        .connection
        .call(
            move |connection| {
                connection
                    .execute(
                        "UPDATE provider_connections SET status='Connected', last_error=NULL, credentials_encrypted=?1",
                        [encrypted],
                    )
                    .unwrap();
                Ok::<_, ModelProviderError>(())
            },
            || ModelProviderError::Storage,
        )
        .unwrap();
    state.lock().unwrap().steps = VecDeque::from([Step {
        method: "POST",
        path: "/chat/completions",
        status: StatusCode::UNAUTHORIZED,
        body: json!({"error":{"type":"gateway_error","message":
            "上游返回 HTTP 401: {\"error\":{\"code\":\"\",\"message\":\"The model does not exist or you do not have access to it.\",\"param\":\"model\",\"type\":\"invalid_request_error\"}}"}}),
    }]);
    let config = person_ai_agent_config(
        "PERSON_AI:FIXED:High:vendor/model",
        "vendor/model",
        None,
        None,
        AgentModelProtocol::ChatCompletions,
    );
    let transport = service.sdk_model_transport(&config).await.unwrap();
    let error = transport
        .json(
            "chat/completions",
            &json!({"model":"vendor/model","messages":[{"role":"user","content":"hello"}]}),
        )
        .await
        .unwrap_err();
    assert!(error.is_unavailable(), "{error}");
    service.sdk_final_model_failure(&error);
    let row = service
        .connection
        .call(
            |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
        .unwrap()
        .unwrap();
    assert_eq!(row.status, ProviderConnectionStatus::Connected);
    assert!(row.credentials_encrypted.is_some());
    assert!(row.last_error.is_none());
    let observed = state.lock().unwrap();
    assert_eq!(
        observed.observed,
        ["POST /person-ai-index/api/v1/chat/completions"]
    );
    assert!(observed.errors.is_empty(), "{:?}", observed.errors);
    server.abort();
}

#[tokio::test]
async fn rag_graph_auto_route_uses_chat_provider_transport_and_requires_connection() {
    use crate::{
        adapters::outbound::model::{
            HttpLlmRuntimeClient, HttpModelClient, HttpModelProviderClient,
            PersonAiModelProviderClient,
        },
        application::{llm::LlmRuntimeClient, model_router::LlmModelRouter},
    };
    let (service, state, server) = fixture().await;
    let router = LlmModelRouter::new(service.clone());
    assert!(
        router
            .route_text("", Some("PERSON_AI:ROUTE:ChatAuto"))
            .is_err()
    );
    let row = service
        .connection
        .call(
            |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
        .unwrap()
        .unwrap();
    let mut credentials = service
        .decrypt_credentials(row.credentials_encrypted.as_deref().unwrap())
        .unwrap();
    credentials.authorization_expires_at = Some(now_millis() + 30 * 24 * 60 * 60 * 1000);
    let encrypted = service.encrypt_credentials(&credentials).unwrap();
    service.connection.call(move |connection| {
        connection.execute("UPDATE provider_connections SET status='Connected', credentials_encrypted=?1 WHERE provider_code='PersonAi'", [encrypted]).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    let config = router
        .route_text("", Some("PERSON_AI:ROUTE:ChatAuto"))
        .unwrap();
    assert_eq!(config.model_code, "PERSON_AI:ROUTE:ChatAuto");
    assert!(config.model_name.is_empty());
    state.lock().unwrap().steps = VecDeque::from([Step {
        method: "POST",
        path: "/chat/completions",
        status: StatusCode::OK,
        body: json!({"choices":[{"message":{"role":"assistant","content":"{\"nodes\":[],\"edges\":[]}"},"finish_reason":"stop"}]}),
    }]);
    let models = HttpModelClient::new(Arc::new(PersonAiModelProviderClient::new(
        HttpModelProviderClient::new(
            eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
        ),
        service,
    )));
    let runtime = HttpLlmRuntimeClient::with_model_client(Arc::new(models));
    assert_eq!(
        runtime
            .generate(
                &config,
                "Extract entities and relations from the supplied text."
            )
            .await
            .unwrap(),
        "{\"nodes\":[],\"edges\":[]}"
    );
    {
        let state = state.lock().unwrap();
        assert!(state.steps.is_empty());
        assert!(state.errors.is_empty(), "{:?}", state.errors);
        assert_eq!(
            state.observed,
            ["POST /person-ai-index/api/v1/chat/completions"]
        );
    }
    server.abort();
}

#[tokio::test]
async fn personai_sync_preserves_vendor_and_tier_identity_through_fixed_selection() {
    let (service, state, server) = fixture().await;
    let row = service
        .connection
        .call(
            |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
        .unwrap()
        .unwrap();
    let mut credentials = service
        .decrypt_credentials(row.credentials_encrypted.as_deref().unwrap())
        .unwrap();
    credentials.authorization_expires_at = Some(now_millis() + 30 * 24 * 60 * 60 * 1000);
    let encrypted = service.encrypt_credentials(&credentials).unwrap();
    service.connection.call(move |connection| {
        connection.execute(
            "UPDATE provider_connections SET status='Connected', credentials_encrypted=?1 WHERE provider_code='PersonAi'",
            [encrypted],
        ).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    let high = json!([
        {"model_code":"deepseek-v4.1-flash","provider_code":"alibaba_coding_plan","provider_name":"百炼tokenplan 个人","protocol_types":["chat_completions","responses"]},
        {"model_code":"deepseek-v4.1-flash","provider_code":"qianfan_plan","provider_name":"百度千帆","protocol_types":["chat_completions"]},
        {"model_code":"glm-5.3-flash","provider_code":"qianfan_plan","provider_name":"百度千帆"},
        {"model_code":"glm-5.3-flash","provider_code":"glm_plan","provider_name":"GLM PLAN"},
        {"model_code":"qwen3.8-flash","provider_code":"qwen","provider_name":"百炼平台"},
        {"model_code":"qwen3.8-flash","provider_code":"alibaba_coding_plan","provider_name":"百炼tokenplan 个人"},
        {"model_code":"qwen3.8-flash","provider_code":"qwen","provider_name":"百炼平台"}
    ]);
    let steps = VecDeque::from([
        step("GET", "/chat/models", high),
        step(
            "GET",
            "/chat/models",
            json!([
                {"model_code":"deepseek-v4.1-flash","provider_code":"alibaba_coding_plan","provider_name":"百炼tokenplan 个人","protocol_types":["chat_completions"]}
            ]),
        ),
        step(
            "GET",
            "/chat/models",
            json!([
                {"model_code":"qwen3.6-flash","provider_code":"alibaba_coding_plan","provider_name":"百炼tokenplan 个人"}
            ]),
        ),
        step(
            "GET",
            "/multimedia/models",
            json!([
                {"model_id":"image-1","model_code":"agnes-image","media_type":0},
                {"model_id":"speech-1","model_code":"qwen-asr","media_type":2},
                {"model_id":"aigc-1","model_code":"aigc-detector","media_type":3},
                {"model_id":"embedding-1","model_code":"bge-m3","media_type":6},
                {"model_id":"rerank-1","model_code":"bge-reranker","media_type":7}
            ]),
        ),
    ]);
    state.lock().unwrap().steps = steps;
    let run = service.sync_now(ModelProviderCode::PersonAi).await.unwrap();
    assert_eq!(run.model_count, Some(13));
    let catalog = service
        .list_catalog_models(ModelProviderCode::PersonAi)
        .unwrap();
    assert_eq!(catalog.len(), 13);
    assert!(catalog.iter().any(|model| {
        model.model_code == "bge-m3"
            && model.catalog_model_type == ProviderCatalogModelType::Embedding
            && model.status == ProviderCatalogModelStatus::Available
    }));
    assert!(catalog.iter().any(|model| model.catalog_model_type
        == ProviderCatalogModelType::Rerank
        && model.model_id.as_deref() == Some("rerank-1")));
    let native = service
        .rerank_runtime_model_config("", Some("PERSON_AI:ROUTE:RerankAuto"))
        .unwrap()
        .unwrap();
    assert_eq!(native.model_code, "PERSON_AI:RERANK:rerank-1");
    assert_eq!(native.api_path.as_deref(), Some("/rerank"));
    assert_eq!(native.model_type, ModelType::Rerank);
    let fixed_native = service
        .rerank_runtime_model_config("", Some("PERSON_AI:RERANK:rerank-1"))
        .unwrap()
        .unwrap();
    assert_eq!(fixed_native.model_code, native.model_code);
    assert_eq!(
        catalog
            .iter()
            .filter(|model| model.quality_type == Some(ProviderModelQuality::High))
            .count(),
        6
    );
    service.connection.call(|connection| {
        connection.execute_batch(
            "INSERT INTO default_model_selections (
                scope_key, model_type, selection_kind, provider_code, model_code,
                quality_type, created_at, updated_at
             ) VALUES ('platform', 'TEXT', 'Fixed', 'PersonAi', 'alibaba_coding_plan/deepseek-v4.1-flash', 'High', 1, 1);"
        ).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    assert!(matches!(
        service.resolve_chat_selection(
            "",
            Some(&ModelSelection::person_ai_fixed(
                "qwen3.6-flash",
                ProviderModelQuality::Low
            )),
            None,
            None,
        ),
        Err(ModelProviderError::InvalidSelection(_))
    ));
    service.connection.call(|connection| {
        connection.execute("UPDATE default_model_selections SET model_code='deepseek-v4.1-flash' WHERE scope_key='platform'", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    let ambiguous_default_options = service.list_text_select_options("").unwrap();
    assert!(
        ambiguous_default_options
            .iter()
            .any(|option| option.enabled)
    );
    assert!(
        !ambiguous_default_options
            .iter()
            .any(|option| option.is_default)
    );
    service.connection.call(|connection| {
        connection.execute("UPDATE default_model_selections SET model_code='alibaba_coding_plan/deepseek-v4.1-flash' WHERE scope_key='platform'", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    let options = service.list_text_select_options("").unwrap();
    assert_eq!(options.iter().filter(|option| option.is_default).count(), 1);
    let fixed = options
        .iter()
        .filter(|option| option.selection.kind == ModelSelectionKind::Fixed)
        .collect::<Vec<_>>();
    assert_eq!(fixed.len(), 6);
    assert_eq!(
        fixed
            .iter()
            .map(|option| &option.option_code)
            .collect::<HashSet<_>>()
            .len(),
        6
    );
    for option in &fixed {
        let config = service
            .resolve_chat_selection("", Some(&option.selection), None, None)
            .unwrap()
            .unwrap();
        assert_eq!(
            config.model_name,
            *option.selection.model_code.as_ref().unwrap()
        );
        assert!(config.model_name.contains('/'));
        assert_eq!(person_ai_runtime_quality(&config), option.quality_type);
    }
    let alibaba = service
        .resolve_chat_selection(
            "",
            Some(&ModelSelection::person_ai_fixed(
                "alibaba_coding_plan/deepseek-v4.1-flash",
                ProviderModelQuality::High,
            )),
            None,
            None,
        )
        .unwrap()
        .unwrap();
    let qianfan = service
        .resolve_chat_selection(
            "",
            Some(&ModelSelection::person_ai_fixed(
                "qianfan_plan/deepseek-v4.1-flash",
                ProviderModelQuality::High,
            )),
            None,
            None,
        )
        .unwrap()
        .unwrap();
    assert_eq!(alibaba.protocol, AgentModelProtocol::Responses);
    assert_eq!(qianfan.protocol, AgentModelProtocol::ChatCompletions);
    // Bare codes are accepted only when vendor/tier resolution is unique.
    assert!(matches!(
        service.resolve_chat_selection(
            "",
            Some(&ModelSelection::person_ai_fixed(
                "deepseek-v4.1-flash",
                ProviderModelQuality::High,
            )),
            None,
            None,
        ),
        Err(ModelProviderError::InvalidSelection(_))
    ));
    assert!(matches!(
        service.resolve_chat_selection(
            "",
            Some(&ModelSelection::person_ai_fixed(
                "unknown/deepseek-v4.1-flash",
                ProviderModelQuality::High,
            )),
            None,
            None
        ),
        Err(ModelProviderError::CatalogModelUnavailable(_))
    ));
    {
        let state = state.lock().unwrap();
        assert!(state.steps.is_empty());
        assert!(state.errors.is_empty(), "{:?}", state.errors);
        assert_eq!(
            state.model_queries,
            [
                "quality_type=high",
                "quality_type=middle",
                "quality_type=low"
            ]
        );
    }
    // Retiring one vendor must not retire a same-named model on another vendor.
    state.lock().unwrap().steps = VecDeque::from([
        step(
            "GET",
            "/chat/models",
            json!([
                {"model_code":"deepseek-v4.1-flash","provider_code":"alibaba_coding_plan","provider_name":"百炼tokenplan 个人","protocol_types":["chat_completions","responses"]}
            ]),
        ),
        step("GET", "/chat/models", json!([])),
        step("GET", "/chat/models", json!([])),
        step("GET", "/multimedia/models", json!([])),
    ]);
    let run = service.sync_now(ModelProviderCode::PersonAi).await.unwrap();
    assert_eq!(run.model_count, Some(1));
    let last_good = service
        .list_catalog_models(ModelProviderCode::PersonAi)
        .unwrap();
    assert_eq!(last_good.len(), 13);
    assert!(last_good.iter().any(|model| {
        model.model_code == "bge-m3" && model.status == ProviderCatalogModelStatus::Unavailable
    }));
    assert_eq!(
        last_good
            .iter()
            .filter(|model| model.status == ProviderCatalogModelStatus::Available)
            .count(),
        1
    );
    assert!(
        service
            .resolve_chat_selection(
                "",
                Some(&ModelSelection::person_ai_fixed(
                    "alibaba_coding_plan/deepseek-v4.1-flash",
                    ProviderModelQuality::High,
                )),
                None,
                None
            )
            .unwrap()
            .is_some()
    );
    assert!(matches!(
        service.resolve_chat_selection(
            "",
            Some(&ModelSelection::person_ai_fixed(
                "qianfan_plan/deepseek-v4.1-flash",
                ProviderModelQuality::High,
            )),
            None,
            None
        ),
        Err(ModelProviderError::CatalogModelUnavailable(_))
    ));
    // A later remote failure must not overwrite the last successful snapshot.
    state.lock().unwrap().steps = VecDeque::from([Step {
        method: "GET",
        path: "/chat/models",
        status: StatusCode::INTERNAL_SERVER_ERROR,
        body: json!({"code":500,"message":"fixture failure"}),
    }]);
    assert!(service.sync_now(ModelProviderCode::PersonAi).await.is_err());
    assert_eq!(
        service
            .list_catalog_models(ModelProviderCode::PersonAi)
            .unwrap(),
        last_good
    );
    {
        let state = state.lock().unwrap();
        assert!(state.steps.is_empty());
        assert!(state.errors.is_empty(), "{:?}", state.errors);
    }
    server.abort();
}

#[tokio::test]
async fn authorization_verification_allows_reasoning_before_the_answer_but_requires_content() {
    struct ReasoningModel {
        answer: bool,
        calls: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl sdk::ModelTransport for ReasoningModel {
        async fn json(&self, path: &str, body: &Value) -> Result<Value, fw::ModelError> {
            self.calls.lock().unwrap().push(path.to_owned());
            if path == "embeddings" {
                return Ok(json!({"data":[{"index":0,"embedding":[0.25,0.75]}]}));
            }
            assert_eq!(path, "chat/completions");
            // A real automatic-route response needed 157 reasoning tokens
            // before returning OK; the former 8-token cap produced no answer.
            let has_answer = self.answer && body["max_tokens"].as_u64().unwrap() >= 162;
            Ok(json!({
                "choices":[{
                    "message":{
                        "role":"assistant",
                        "content":if has_answer { "OK" } else { "" },
                        "reasoning_content":"Reasoning preceding the final answer"
                    },
                    "finish_reason":if has_answer { "stop" } else { "length" }
                }],
                "usage":{"prompt_tokens":16,"completion_tokens":162,"total_tokens":178,
                    "completion_tokens_details":{"reasoning_tokens":157}}
            }))
        }

        async fn events(
            &self,
            _: &str,
            _: &Value,
            _: sdk::EventSink<'_>,
        ) -> Result<(), fw::ModelError> {
            unreachable!("authorization verification uses non-streaming requests")
        }
    }

    let (service, _, server) = fixture().await;
    for answer in [true, false] {
        let transport = Arc::new(ReasoningModel {
            answer,
            calls: Mutex::new(vec![]),
        });
        let result = service
            .verify_sdk_model_routes(Arc::new(sdk::FixedModelTransportFactory(transport.clone())))
            .await;
        if answer {
            result.unwrap();
            assert_eq!(
                *transport.calls.lock().unwrap(),
                ["chat/completions", "embeddings"]
            );
        } else {
            assert!(result.is_err());
            assert_eq!(*transport.calls.lock().unwrap(), ["chat/completions"]);
        }
    }
    server.abort();
}

#[tokio::test]
async fn completed_personai_authorization_verifies_distinct_model_routes_before_commit() {
    let (service, state, server) = fixture().await;
    state.lock().unwrap().steps = completed_steps(StatusCode::OK);
    let result = poll(&service).await.unwrap();
    assert!(result.text_verified && result.embedding_verified);
    let row = service
        .connection
        .call(
            move |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
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
        service
            .connection
            .call(
                move |connection| read_person_ai_profile_credentials(connection, "fixture-user"),
                || ModelProviderError::Storage
            )
            .unwrap()
            .is_some()
    );
    assert_eq!(
        service
            .connection
            .call(
                move |connection| catalog_model_count(connection, ModelProviderCode::PersonAi),
                || ModelProviderError::Storage
            )
            .unwrap(),
        3
    );
    assert_eq!(
        service
            .connection
            .call(
                move |connection| read_authorization_session(connection, "fixture-session"),
                || ModelProviderError::Storage
            )
            .unwrap()
            .status,
        "Completed"
    );
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
    let previous = service
        .connection
        .call(
            |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
        .unwrap()
        .unwrap();
    state.lock().unwrap().steps = completed_steps(StatusCode::NOT_FOUND);
    let error = poll(&service).await.unwrap_err();
    assert!(
        matches!(error,ModelProviderError::Remote(ref message) if message.contains("personai model error: Not found"))
    );
    {
        let row = service
            .connection
            .call(
                move |connection| read_connection(connection, ModelProviderCode::PersonAi),
                || ModelProviderError::Storage,
            )
            .unwrap()
            .unwrap();
        assert_eq!(row.status, ProviderConnectionStatus::AuthRequired);
        assert!(row.credentials_encrypted == previous.credentials_encrypted);
        assert_eq!(row.person_ai_user_id, previous.person_ai_user_id);
        assert_eq!(row.last_error, previous.last_error);
        assert_eq!(
            service
                .connection
                .call(
                    move |connection| read_authorization_session(connection, "fixture-session"),
                    || ModelProviderError::Storage
                )
                .unwrap()
                .status,
            "pending"
        );
        assert_eq!(
            service
                .connection
                .call(
                    move |connection| catalog_model_count(connection, ModelProviderCode::PersonAi),
                    || ModelProviderError::Storage
                )
                .unwrap(),
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
    let row = service
        .connection
        .call(
            move |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
        .unwrap()
        .unwrap();
    assert_eq!(row.status, ProviderConnectionStatus::AuthRequired);
    assert!(row.credentials_encrypted.is_none());
    assert_eq!(
        row.last_error.as_deref(),
        Some("PersonAI device was unbound; login again")
    );
    assert!(matches!(
        service.connection.call(
            move |connection| read_authorization_session(connection, "fixture-session"),
            || ModelProviderError::Storage
        ),
        Err(ModelProviderError::AuthorizationSessionNotFound)
    ));
    assert!(service.device_unbound());
    let state = state.lock().unwrap();
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.steps.is_empty());
    server.abort();
}

#[tokio::test]
async fn personai_tool_context_rejects_logged_out_and_stale_account_snapshots() {
    let (service, state, server) = fixture().await;
    let scope = service.sdk_model_authorization_scope().unwrap();
    assert!(service.personai_tool_context(&scope).await.is_err());
    let row = service
        .connection
        .call(
            |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
        .unwrap()
        .unwrap();
    let mut credentials = service
        .decrypt_credentials(row.credentials_encrypted.as_deref().unwrap())
        .unwrap();
    credentials.authorization_expires_at = Some(now_millis() + 30 * 24 * 60 * 60 * 1000);
    let encrypted = service.encrypt_credentials(&credentials).unwrap();
    service.connection.call(move |connection| {
        connection.execute("UPDATE provider_connections SET status='Connected', credentials_encrypted=?1 WHERE provider_code='PersonAi'", [encrypted]).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    let scope = service.sdk_model_authorization_scope().unwrap();
    let (_, _, context) = service.personai_tool_context(&scope).await.unwrap();
    let observed = context.subscribe();
    let first = context.current().unwrap();
    assert!(Arc::ptr_eq(&first, &context.current().unwrap()));
    service.connection.call(|connection| {
        connection.execute("UPDATE provider_connections SET person_ai_user_id='another-user' WHERE provider_code='PersonAi'", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    assert!(context.current().is_none());
    assert!(observed.borrow().is_none());
    assert!(service.personai_tool_context(&scope).await.is_err());
    assert!(state.lock().unwrap().steps.is_empty());
    server.abort();
}
use std::collections::HashSet;

#[tokio::test]
async fn query_amplification_stream_deltas_only_write_changed_lineage() {
    use sdk::ModelTransport;
    struct Stream;
    #[async_trait]
    impl sdk::ModelTransport for Stream {
        async fn json(&self, _: &str, _: &Value) -> Result<Value, fw::ModelError> {
            unreachable!()
        }
        async fn events(
            &self,
            _: &str,
            _: &Value,
            sink: sdk::EventSink<'_>,
        ) -> Result<(), fw::ModelError> {
            for _ in 0..1000 {
                sink(json!({"model":"actual-model", "choices":[{"delta":{"content":"x"}}]}))?;
            }
            sink(json!({"response":{"model":"actual-model","provider_code":"actual-vendor"}}))?;
            sink(json!({"type":"response.output_text.delta","delta":"x"}))?;
            // Metadata must survive a stream that fails after delivering partial output.
            Err(fw::ModelError::Fatal("fixture interrupted".into()))
        }
    }
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
        })
        .unwrap(),
    );
    let providers = ModelProviderService::connect(":memory:", cipher).unwrap();
    providers.connection.call(|c| c.execute_batch("CREATE TABLE snapshot_writes(n INTEGER); CREATE TRIGGER count_snapshot_insert AFTER INSERT ON provider_model_run_snapshots BEGIN INSERT INTO snapshot_writes VALUES(1); END; CREATE TRIGGER count_snapshot_update AFTER UPDATE ON provider_model_run_snapshots BEGIN INSERT INTO snapshot_writes VALUES(1); END;"), || rusqlite::Error::InvalidQuery).unwrap();
    let transport = SnapshotTransport {
        inner: Arc::new(Stream),
        providers: providers.clone(),
        config: person_ai_agent_config(
            "PERSON_AI:ROUTE:ChatAuto",
            "",
            None,
            None,
            AgentModelProtocol::ChatCompletions,
        ),
        run: Some("stream-run".into()),
        attempt: "fixture".into(),
        embedding_instance_id: None,
        embedding_logical_code: None,
    };
    let delivered = std::sync::atomic::AtomicUsize::new(0);
    assert!(
        transport
            .events(
                "/fixture",
                &json!({}),
                Arc::new(|_| {
                    delivered.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            )
            .await
            .is_err()
    );
    assert_eq!(delivered.load(Ordering::SeqCst), 1002);
    providers.connection.call(|c| {
        assert_eq!(c.query_row("SELECT COUNT(*) FROM snapshot_writes", [], |r| r.get::<_, u32>(0))?, 2);
        let row = c.query_row("SELECT resolved_model_code,resolved_upstream_vendor_code FROM provider_model_run_snapshots WHERE run_code='stream-run'", [], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?;
        assert_eq!(row, ("actual-model".into(),"actual-vendor".into()));
        Ok::<_,rusqlite::Error>(())
    }, || rusqlite::Error::InvalidQuery).unwrap();
}

#[tokio::test]
async fn rerank_transport_dispatches_native_model_id_and_preserves_scores_and_usage() {
    struct Capture(Arc<Mutex<Value>>);
    #[async_trait]
    impl sdk::ModelTransport for Capture {
        async fn json(&self, path: &str, body: &Value) -> Result<Value, fw::ModelError> {
            assert_eq!(path, "/rerank");
            *self.0.lock().unwrap() = body.clone();
            Ok(json!({"results":[{"index":1,"relevance_score":0.87}],"usage":{"total_tokens":13}}))
        }
        async fn events(
            &self,
            _: &str,
            _: &Value,
            _: sdk::EventSink<'_>,
        ) -> Result<(), fw::ModelError> {
            unreachable!()
        }
    }
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
        })
        .unwrap(),
    );
    let observed = Arc::new(Mutex::new(Value::Null));
    let mut config = person_ai_agent_config(
        "PERSON_AI:RERANK:rerank-1",
        "rerank-1",
        None,
        None,
        AgentModelProtocol::ChatCompletions,
    );
    config.api_path = super::super::auto_selection::person_ai_rerank_config("rerank-1").api_path;
    let transport = SnapshotTransport {
        inner: Arc::new(Capture(observed.clone())),
        providers: ModelProviderService::connect(":memory:", cipher).unwrap(),
        config: config.clone(),
        run: None,
        attempt: "fixture".into(),
        embedding_instance_id: None,
        embedding_logical_code: None,
    };
    let model = sdk::HttpRerankModel {
        config: (&config).into(),
        factory: Arc::new(sdk::FixedModelTransportFactory(Arc::new(transport))),
        chat_fallback: false,
    };
    let response = fw::RerankModel::rerank_with_usage(
        &model,
        "rerank-1",
        fw::RerankRequest {
            choice: fw::ModelChoice::model(fw::ModelRef::new("personai", "rerank-1")),
            query: "orchid".into(),
            documents: vec!["stone".into(), "flower".into()],
            top_n: Some(1),
        },
        None,
    )
    .await
    .unwrap();
    let body = observed.lock().unwrap();
    assert_eq!(body["model_id"], "rerank-1");
    assert!(body.get("model").is_none());
    assert!(body.get("messages").is_none());
    assert_eq!(body["top_n"], 1);
    assert_eq!(response.response.results[0].index, 1);
    assert_eq!(response.response.results[0].score, 0.87);
    assert_eq!(response.usage.unwrap().input_tokens, 13);
}

#[test]
fn embedding_auto_routes_use_platform_tags_and_preserve_one_declared_space() {
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
        })
        .unwrap(),
    );
    let service = ModelProviderService::connect(":memory:", cipher).unwrap();
    service.connection.call(|connection| {
        for (id, code, dimension, tags) in [
            ("text", "arbitrary-text", 1024, "[\"text_embedding\"]"),
            ("image", "arbitrary-image", 1024, "[\"multimodal_embedding\"]"),
            ("small", "small-image", 512, "[\"multimodal_embedding\"]"),
            ("unknown", "Qwen3-VL-Embedding-2B", 1024, "[]"),
        ] {
            connection.execute("INSERT INTO provider_multimedia_catalog_models(provider_code,model_id,model_code,model_name,media_type,status,first_seen_at,updated_at,dimension,tags_json) VALUES('PersonAi',?1,?2,?2,6,'Available',1,1,?3,?4)", params![id,code,dimension,tags]).unwrap();
        }
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    assert_eq!(
        service
            .auto_embedding_config("PLATFORM")
            .unwrap()
            .model_name,
        "arbitrary-text"
    );
    assert_eq!(
        service
            .auto_multimodal_embedding_config("PLATFORM")
            .unwrap()
            .model_name,
        "arbitrary-image"
    );
    let models = service
        .list_catalog_models(ModelProviderCode::PersonAi)
        .unwrap();
    let image = models
        .iter()
        .find(|model| model.model_code == "arbitrary-image")
        .unwrap();
    assert_eq!(image.tags, ["multimodal_embedding"]);
    assert_eq!(image.dimension, Some(1024));
    service.connection.call(|connection| {
        connection.execute("INSERT INTO provider_multimedia_catalog_models(provider_code,model_id,model_code,model_name,media_type,status,first_seen_at,updated_at,dimension,tags_json) VALUES('PersonAi','inconsistent','arbitrary-image','Image',6,'Available',1,1,1024,'[\"text_embedding\"]')", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    assert!(
        service
            .auto_multimodal_embedding_config("PLATFORM")
            .is_err()
    );
    assert!(
        service
            .auto_embedding_config("archived-or-missing")
            .is_err()
    );
}
