use super::*;

// Synthetic replacement key; its private key is not retained.
const ROTATED_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAnqMZC2RgMMSKgIfN3ZKn\nQWcL4xowSq2MREuOOz7f9+iF+WIOey7Jcm+0ttSaCkvu5ldtpOwn130VAYFY5LC0\nVj5UbezEp0DHY76Tobq5dDnq9K/9+0XuwaJRTavrI6V20+BQLfo5g0NOmQNoCG9A\nt8Z248oJOO21uBPlY0fX1hz5p7QPZr31LV/nNIJLBpmmXbNNQAwamCC2etCHjVc8\nT5NdrM0SCVVc5OeDtW7YpHgUEr5jf3N40mjfhdaYC+C84dB1Be209OLY0KyKDh8w\n2DwaoAeiMFuW+iIKpMRdRwx01rADThU61lJDjd5tdmAg+yMU8gFFcGdreMmmR3Gz\n1QIDAQAB\n-----END PUBLIC KEY-----";

async fn key_fixture() -> (
    ModelProviderService,
    SharedMock,
    tokio::task::JoinHandle<()>,
) {
    let (service, state, server) = fixture().await;
    let row = read_row(&service);
    let mut credentials = service
        .decrypt_credentials(row.credentials_encrypted.as_deref().unwrap())
        .unwrap();
    credentials.authorization_expires_at = Some(now_millis() + 30 * 24 * 60 * 60 * 1000);
    let encrypted = service.encrypt_credentials(&credentials).unwrap();
    service.connection.call(move |connection| {
        connection.execute("UPDATE provider_connections SET status='Connected',last_error=NULL,credentials_encrypted=?1", [&encrypted]).unwrap();
        connection.execute("INSERT INTO person_ai_profile_credentials(person_ai_user_id,person_ai_username,server_base_url,credentials_encrypted,status,authorized_at,updated_at) SELECT person_ai_user_id,person_ai_username,server_base_url,credentials_encrypted,status,1,1 FROM provider_connections", []).unwrap();
        connection.execute("UPDATE provider_authorization_sessions SET status='Completed'", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    (service, state, server)
}

fn read_row(service: &ModelProviderService) -> ProviderConnectionRow {
    service
        .connection
        .call(
            |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )
        .unwrap()
        .unwrap()
}

fn rotated_session(client: &str) -> Step {
    step(
        "GET",
        "/auth/client-sessions/fixture-session",
        json!({
            "session_id":"fixture-session","status":"completed",
            "token":"obsolete-session-token","refresh_token":"obsolete-session-refresh",
            "client_id":client,"public_key":ROTATED_KEY,"expires_in":3600,
        }),
    )
}

fn failure(service: &ModelProviderService) -> fw::ModelError {
    fw::ModelError::Provider(Box::new(fw::ProviderModelError {
        provider: "personai".into(),
        category: fw::ProviderErrorCategory::Authentication,
        authorization_attempt: Some(service.sdk_credential_attempt("old-fixture-token")),
        http_status: Some(400),
        business_code: None,
        error_type: Some("decrypt_error".into()),
        model: None,
        message: "Decrypt failed".into(),
    }))
}

#[tokio::test]
async fn encryption_key_recovery_is_single_flight_preserves_tokens_and_fences_old_failures() {
    let (service, state, server) = key_fixture().await;
    let previous = read_row(&service);
    let before = service
        .decrypt_credentials(previous.credentials_encrypted.as_deref().unwrap())
        .unwrap();
    let rejected = failure(&service);
    state.lock().unwrap().steps = VecDeque::from([rotated_session("fixture-client")]);
    let (first, second) = tokio::join!(
        service.sdk_recover_model_authentication(&rejected),
        service.sdk_recover_model_authentication(&rejected),
    );
    assert!(first.unwrap() && second.unwrap());
    let row = read_row(&service);
    let after = service
        .decrypt_credentials(row.credentials_encrypted.as_deref().unwrap())
        .unwrap();
    assert_eq!(after.public_key, ROTATED_KEY);
    assert_eq!(after.token, before.token);
    assert_eq!(after.refresh_token, before.refresh_token);
    assert_eq!(
        after.authorization_expires_at,
        before.authorization_expires_at
    );
    assert_eq!(
        service.sdk_identity_scope(&row),
        service.sdk_identity_scope(&previous)
    );
    assert_ne!(
        service.sdk_attempt_for_row(&row, &after.token),
        service.sdk_attempt_for_row(&previous, &before.token)
    );
    service.sdk_final_model_failure(&rejected);
    assert_eq!(
        read_row(&service).status,
        ProviderConnectionStatus::Connected
    );
    let persisted: String = service
        .connection
        .call(
            |connection| {
                connection.query_row(
                    "SELECT credentials_encrypted FROM person_ai_profile_credentials",
                    [],
                    |row| row.get(0),
                )
            },
            || rusqlite::Error::InvalidQuery,
        )
        .unwrap();
    assert_eq!(
        service.decrypt_credentials(&persisted).unwrap().public_key,
        ROTATED_KEY
    );

    state.lock().unwrap().steps.push_back(Step {
        method: "POST",
        path: "/chat/completions",
        status: StatusCode::OK,
        body: json!({"choices":[{"message":{"content":"OK"},"finish_reason":"stop"}]}),
    });
    let config = person_ai_agent_config(
        "PERSON_AI:FIXED:High:vendor/model",
        "vendor/model",
        None,
        None,
        AgentModelProtocol::ChatCompletions,
    );
    let response = service
        .sdk_model_transport(&config)
        .await
        .unwrap()
        .json(
            "chat/completions",
            &json!({"model":"vendor/model","messages":[]}),
        )
        .await
        .unwrap();
    assert_eq!(response["choices"][0]["message"]["content"], "OK");
    let state = state.lock().unwrap();
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.steps.is_empty());
    assert_eq!(
        state.observed.len(),
        2,
        "one key lookup and one model request, no token refresh"
    );
    server.abort();
}

#[tokio::test]
async fn encryption_key_recovery_rejects_another_device_without_adopting_session_tokens() {
    let (service, state, server) = key_fixture().await;
    let before = read_row(&service).credentials_encrypted;
    state.lock().unwrap().steps = VecDeque::from([rotated_session("another-client")]);
    let error = service
        .sdk_recover_model_authentication(&failure(&service))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("authorize PersonAI again"));
    assert_eq!(read_row(&service).credentials_encrypted, before);
    service.sdk_final_model_failure(&error);
    let row = read_row(&service);
    assert_eq!(row.status, ProviderConnectionStatus::AuthRequired);
    assert_eq!(
        row.last_error.as_deref(),
        Some("PersonAI encryption key changed; authorize PersonAI again")
    );
    assert_eq!(row.credentials_encrypted, before);
    assert!(matches!(
        service
            .active_credentials(ModelProviderCode::PersonAi)
            .await,
        Err(ModelProviderError::AuthenticationRequired)
    ));
    assert_eq!(
        read_row(&service).status,
        ProviderConnectionStatus::AuthRequired
    );
    assert_eq!(
        state.lock().unwrap().observed.len(),
        1,
        "key rejection must not trigger a token refresh that resets the status"
    );
    assert!(state.lock().unwrap().errors.is_empty());
    server.abort();
}

#[tokio::test]
async fn encryption_key_lookup_outage_preserves_authorization_and_credentials() {
    let (service, state, server) = key_fixture().await;
    let before = read_row(&service).credentials_encrypted;
    state.lock().unwrap().steps = VecDeque::from([Step {
        method: "GET",
        path: "/auth/client-sessions/fixture-session",
        status: StatusCode::SERVICE_UNAVAILABLE,
        body: json!({"code":503,"message":"temporary outage"}),
    }]);
    let error = service
        .sdk_recover_model_authentication(&failure(&service))
        .await
        .unwrap_err();
    service.sdk_final_model_failure(&error);
    assert_eq!(read_row(&service).credentials_encrypted, before);
    assert_eq!(
        read_row(&service).status,
        ProviderConnectionStatus::Connected
    );
    server.abort();
}

#[tokio::test]
async fn encryption_key_recovery_never_crosses_an_account_change() {
    let (service, state, server) = key_fixture().await;
    let rejected = failure(&service);
    service
        .connection
        .call(
            |connection| {
                connection
                    .execute(
                        "UPDATE provider_connections SET person_ai_user_id='another-user'",
                        [],
                    )
                    .unwrap();
                Ok::<_, ModelProviderError>(())
            },
            || ModelProviderError::Storage,
        )
        .unwrap();
    let before = read_row(&service).credentials_encrypted;
    assert!(
        service
            .sdk_recover_model_authentication(&rejected)
            .await
            .is_err()
    );
    assert_eq!(read_row(&service).credentials_encrypted, before);
    assert!(state.lock().unwrap().observed.is_empty());
    server.abort();
}

#[tokio::test]
async fn encryption_key_recovery_repairs_the_previously_rejected_active_profile() {
    let (service, state, server) = key_fixture().await;
    service.connection.call(|connection| {
        connection.execute("UPDATE person_ai_profile_credentials SET status='AuthRequired',last_error='PersonAI rejected access credentials; reauthorization required'", []).unwrap();
        connection.execute("UPDATE provider_connections SET status='AuthRequired',last_error='PersonAI rejected access credentials; reauthorization required'", []).unwrap();
        Ok::<_, ModelProviderError>(())
    }, || ModelProviderError::Storage).unwrap();
    state.lock().unwrap().steps = VecDeque::from([rotated_session("fixture-client")]);
    let credentials = service
        .active_credentials(ModelProviderCode::PersonAi)
        .await
        .unwrap();
    assert_eq!(credentials.public_key, ROTATED_KEY);
    assert_eq!(credentials.token, "old-fixture-token");
    assert_eq!(
        read_row(&service).status,
        ProviderConnectionStatus::Connected
    );
    assert!(state.lock().unwrap().steps.is_empty());
    server.abort();
}

#[tokio::test]
async fn provider_reauthorization_preserves_the_profile_and_rejects_another_account() {
    use crate::{
        app::AppState, application::profile::ProfileRegistryService, config::AppConfig,
        shared::id::FixedSnowflakeIdProvider,
    };
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    for same_account in [true, false] {
        let (providers, mock, server) = key_fixture().await;
        providers.connection.call(|connection| {
            connection.execute("INSERT INTO default_model_selections(scope_key,model_type,selection_kind,provider_code,model_code,quality_type,created_at,updated_at) VALUES('platform','TEXT','Fixed','PersonAi','fixture-chat','High',1,1)", []).unwrap();
            Ok::<_, ModelProviderError>(())
        }, || ModelProviderError::Storage).unwrap();
        let profiles = ProfileRegistryService::new(":memory:").unwrap();
        profiles
            .register_or_update("previous-user", "previous-name")
            .unwrap();
        let before_profile = profiles.bootstrap_status().unwrap();
        let before_credentials = read_row(&providers).credentials_encrypted;
        let mut config = AppConfig::default();
        config.variable_crypto = VariableCryptoConfig::test_default();
        config.system_agents.codex_command.clear();
        config.system_agents.opencode_command.clear();
        let mut state = AppState::new_with_profiles(
            config,
            Arc::new(FixedSnowflakeIdProvider::new(735_000_000_000_000_000)),
            profiles.clone(),
        )
        .unwrap();
        state.model_providers = providers.clone();
        let mut steps = completed_steps(StatusCode::OK);
        let mut account = steps.pop_back().unwrap();
        let mut token = steps.pop_back().unwrap();
        token.body["data"]["subject"] = json!(if same_account {
            "previous-user"
        } else {
            "another-user"
        });
        account.body["data"]["username"] = json!("remote-name-must-not-replace-profile");
        let mut session = steps.pop_front().unwrap();
        session.body["data"]["expires_in"] = json!(30 * 24 * 60 * 60);
        if !same_account {
            steps.clear();
        }
        steps.push_front(account);
        steps.push_front(token);
        steps.push_front(session);
        if same_account {
            steps.push_back(step("POST", "/account/devices/heartbeat", json!({})));
        }
        mock.lock().unwrap().steps = steps;
        // Exercise the real HTTP routes without the full app's background
        // heartbeat consuming the mock authorization responses.
        let router = crate::adapters::inbound::http::admin::model_providers::profile_routes()
            .with_state(state);
        let response = router.oneshot(Request::post(
            "/api/v1/model-providers/PersonAi/reauthorization-sessions/fixture-session/poll",
        ).header("content-type", "application/json").header("accept-language", "zh-CN")
            .body(Body::from("{}")).unwrap()).await.unwrap();
        let status = response.status();
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        let diagnostic = {
            let mock = mock.lock().unwrap();
            format!("observed={:?}; errors={:?}", mock.observed, mock.errors)
        };
        assert_eq!(
            status,
            if same_account {
                StatusCode::OK
            } else {
                StatusCode::CONFLICT
            },
            "{body}; {diagnostic}"
        );
        if same_account {
            assert_eq!(
                body["data"]["profileId"],
                before_profile.active_profile_id.unwrap()
            );
            assert_ne!(
                read_row(&providers).credentials_encrypted,
                before_credentials
            );
        } else {
            assert_eq!(body["code"], "MODEL_PROVIDER_AUTH_ACCOUNT_MISMATCH");
            assert!(body["message"].as_str().unwrap().contains("当前 Profile"));
            assert_eq!(
                read_row(&providers).credentials_encrypted,
                before_credentials
            );
        }
        assert_eq!(
            profiles.bootstrap_status().unwrap().active_profile,
            before_profile.active_profile
        );
        let default_model: String = providers.connection.call(|connection| {
            connection.query_row("SELECT model_code FROM default_model_selections WHERE scope_key='platform' AND model_type='TEXT'", [], |row| row.get(0))
        }, || rusqlite::Error::InvalidQuery).unwrap();
        assert_eq!(default_model, "fixture-chat");
        let observed = mock.lock().unwrap();
        assert!(observed.steps.is_empty());
        assert!(observed.errors.is_empty(), "{:?}", observed.errors);
        server.abort();
    }
}
