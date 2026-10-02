//! Product HTTP + native Framework + SQLite STM paused execution lifecycle.
use super::*;
use tron_backend::{
    app::{AppState, build_router_with_state},
    application::profile::ProfileRegistryService,
};

fn isolated_state(config: AppConfig, seed: i64) -> AppState {
    let profiles = ProfileRegistryService::new(":memory:").unwrap();
    profiles
        .register_or_update("USR-AUTH-0001", "profile.operator")
        .unwrap();
    AppState::new_with_profiles(
        config,
        Arc::new(FixedSnowflakeIdProvider::new(seed)),
        profiles,
    )
    .unwrap()
}

fn lease_count(database: &str, session: &str) -> i64 {
    Connection::open(database)
        .unwrap()
        .query_row(
            "SELECT count(*) FROM agent_session_run_leases WHERE session_code=?1",
            [session],
            |row| row.get(0),
        )
        .unwrap()
}

#[tokio::test]
async fn chat_question_resume_releases_physical_stm_lease_with_and_without_restart() {
    for restart in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let mut config = AppConfig::default();
        config.database_url = directory.path().join("chat.sqlite3").display().to_string();
        config = common::prepared_config(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
        let main_calls = Arc::new(AtomicUsize::new(0));
        let provider = Router::new().route("/v1/chat/completions", post({
            let requests = requests.clone(); let main_calls = main_calls.clone();
            move |Json(payload): Json<Value>| {
                let requests = requests.clone(); let main_calls = main_calls.clone();
                async move {
                    requests.lock().unwrap().push(payload.clone());
                    if payload["stream"] != true {
                        return valid_mock_completion(&payload, "Paused question session summary");
                    }
                    let first = main_calls.fetch_add(1, Ordering::SeqCst) == 0;
                    let delta = if first {
                        json!({"tool_calls":[{"index":0,"id":"lease-question-call","type":"function","function":{"name":"chat_ask_user","arguments":json!({"intro":"Choose the acceptance target","questions":[{"question":"Which target should I use?","options":[{"label":"Acceptance Local"},{"label":"Acceptance Remote"}]}]}).to_string()}}]})
                    } else { json!({"content":"framework-question-resumed"}) };
                    let chunk = json!({"id":"question-response","choices":[{"index":0,"delta":delta,"finish_reason":if first {"tool_calls"} else {"stop"}}],"usage":{"prompt_tokens":240,"completion_tokens":32,"total_tokens":272}});
                    let stream=async_stream::stream! {yield Ok::<Event,Infallible>(Event::default().data(chunk.to_string()));yield Ok::<Event,Infallible>(Event::default().data("[DONE]"));};
                    Sse::new(stream).into_response()
                }
            }
        }));
        let provider_task =
            tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
        let mut state = isolated_state(
            config.clone(),
            848_000_000_000_000_000 + i64::from(restart) * 100_000,
        );
        let mut app = build_router_with_state(state.clone()).unwrap();
        let token = authenticated_bearer_token(&app).await;
        let authorization = format!("Bearer {token}");
        let model =
            create_platform_text_model(&app, &authorization, &format!("http://{address}/v1")).await;
        let session = request_with_headers(
            &app,
            Method::POST,
            "/api/v1/chats/sessions",
            Some(json!({"workspaceCode":"TRON_CORE","title":"Pause and resume lease"})),
            None,
            Some(&authorization),
        )
        .await;
        let session = response_json(session).await;
        let session = session["data"]["sessionCode"].as_str().unwrap().to_owned();
        let waiting = request_with_headers(&app,Method::POST,&format!("/api/v1/chats/sessions/{session}/messages/stream"),
            Some(json!({"content":"acceptance-question","workspaceCode":"TRON_CORE","modelCode":model,"clientCapabilities":{"userInputRequests":true}})),None,Some(&authorization)).await;
        assert_eq!(waiting.status(), StatusCode::OK);
        let waiting = String::from_utf8(
            to_bytes(waiting.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(
            waiting.contains("AWAITING_USER_INPUT") || waiting.contains("interrupt"),
            "{waiting}"
        );
        assert!(!waiting.contains("event: RUN_ERROR"), "{waiting}");
        assert_eq!(
            lease_count(&config.database_url, &session),
            0,
            "native wait/drain must finish before the physical STM lease is released"
        );
        let active = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/chats/sessions/get-active-run?sessionCode={session}"),
            None,
            None,
            Some(&authorization),
        )
        .await;
        assert!(response_json(active).await["data"]["activeRun"].is_null());
        let records = request_with_headers(
            &app,
            Method::GET,
            &format!("/api/v1/chats/sessions/{session}/user-input-requests"),
            None,
            None,
            Some(&authorization),
        )
        .await;
        let records = response_json(records).await;
        let pending = &records["data"][0];
        assert_eq!(pending["status"], json!("Pending"));
        let request_code = pending["requestCode"].as_str().unwrap().to_owned();
        let question = pending["questions"][0]["questionCode"].clone();
        let option = pending["questions"][0]["options"][0]["optionCode"].clone();
        let connection = Connection::open(&config.database_url).unwrap();
        let events = connection.prepare("SELECT event_type FROM agent_transcript_events WHERE session_code=?1 AND event_type LIKE 'agent.run.%' ORDER BY seq").unwrap().query_map([&session],|row|row.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        assert_eq!(
            events,
            vec!["agent.run.started", "agent.run.awaiting_input"],
            "waiting must not be audited as completed or failed"
        );
        drop(connection);
        if restart {
            state.agent_framework.close().await;
            state.agent_framework.wait_shutdown_completion().await;
            drop(app);
            drop(state);
            state = isolated_state(config.clone(), 849_000_000_000_000_000);
            app = build_router_with_state(state.clone()).unwrap();
            assert_eq!(
                lease_count(&config.database_url, &session),
                0,
                "restart must not backfill a lease for the paused execution"
            );
            let reloaded = request_with_headers(
                &app,
                Method::GET,
                &format!("/api/v1/chats/sessions/{session}/user-input-requests"),
                None,
                None,
                Some(&authorization),
            )
            .await;
            assert_eq!(
                response_json(reloaded).await["data"][0],
                pending.clone(),
                "native card must survive restart without changing identity"
            );
        }
        let resumed = request_with_headers(&app,Method::POST,&format!("/api/v1/chats/sessions/{session}/user-input-requests/{request_code}/answers/stream"),
            Some(json!({"answers":[{"questionCode":question,"answerKind":"Option","optionCode":option}]})),None,Some(&authorization)).await;
        assert_eq!(resumed.status(), StatusCode::OK);
        let resumed = String::from_utf8(
            to_bytes(resumed.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(resumed.contains("framework-question-resumed"), "{resumed}");
        assert!(!resumed.contains("event: RUN_ERROR"), "{resumed}");
        assert_eq!(
            main_calls.load(Ordering::SeqCst),
            2,
            "resume must neither rerun the question tool nor dispatch duplicate main calls"
        );
        assert_eq!(lease_count(&config.database_url, &session), 0);
        {
            let requests = requests.lock().unwrap();
            let resumed = requests
                .iter()
                .filter(|request| request["stream"] == true)
                .nth(1)
                .unwrap();
            assert!(
                resumed["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|message| message["role"] == "tool"
                        && message["tool_call_id"] == "lease-question-call"),
                "original tool-call result pairing must survive resume"
            );
            assert!(resumed.to_string().contains("Acceptance Local"));
        }
        let connection = Connection::open(&config.database_url).unwrap();
        let recovered:i64=connection.query_row("SELECT count(*) FROM agent_transcript_events WHERE session_code=?1 AND event_type='agent.run.recovered'",[&session],|row|row.get(0)).unwrap();
        assert_eq!(
            recovered, 0,
            "an explicit waiting execution must never be recovered as an abandoned run"
        );
        drop(connection);
        state.agent_framework.close().await;
        state.agent_framework.wait_shutdown_completion().await;
        provider_task.abort();
        let _ = provider_task.await;
    }
}
