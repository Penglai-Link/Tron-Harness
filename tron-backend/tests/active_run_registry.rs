use std::sync::Arc;

use tron_backend::{
    adapters::outbound::persistence::SqliteChatRepository,
    application::{
        agent::{ActiveRunError, ActiveRunRegistry, AgentSteerInput},
        chat::{ChatService, CreateChatSessionCommand, SteerChatMessageCommand},
    },
    shared::id::FixedSnowflakeIdProvider,
};

fn steer(code: &str, content: &str) -> AgentSteerInput {
    AgentSteerInput {
        requested_skills: Vec::new(),
        steer_code: code.to_string(),
        content: content.to_string(),
        attachments: Vec::new(),
    }
}

#[tokio::test]
async fn active_run_snapshot_and_stop_signal_are_consistent() {
    let registry = ActiveRunRegistry::new();
    assert!(
        registry
            .snapshot("SESSION-STOP")
            .expect("idle snapshot should load")
            .is_none()
    );

    let guard = registry
        .register("SESSION-STOP", "RUN-STOP", "REACT", true)
        .expect("active run should register");
    let running = registry
        .snapshot("SESSION-STOP")
        .expect("running snapshot should load")
        .expect("running snapshot should exist");
    assert_eq!(running.run_code, "RUN-STOP");
    assert_eq!(running.runtime_adapter_key, "REACT");
    assert!(running.supports_steer);
    assert_eq!(running.state.code(), "RUNNING");

    let mut cancellation = guard.cancellation();
    let stopped = registry
        .request_stop("SESSION-STOP", "RUN-STOP")
        .expect("matching stop should be accepted");
    assert_eq!(stopped.state.code(), "FINISHING");
    tokio::time::timeout(std::time::Duration::from_secs(1), cancellation.cancelled())
        .await
        .expect("stop signal should be observable");
    let finishing = registry
        .snapshot("SESSION-STOP")
        .expect("finishing snapshot should load")
        .expect("finishing snapshot should exist");
    assert_eq!(finishing.state.code(), "FINISHING");
    assert_eq!(
        registry
            .request_stop("SESSION-STOP", "RUN-STOP")
            .expect_err("second stop should see finishing state"),
        ActiveRunError::RunFinishing
    );

    drop(guard);
    assert!(
        registry
            .snapshot("SESSION-STOP")
            .expect("dropped snapshot should load")
            .is_none()
    );
}

#[test]
fn active_run_stop_rejects_stale_expected_run_without_signalling() {
    let registry = ActiveRunRegistry::new();
    let _guard = registry
        .register("SESSION-MISMATCH", "RUN-ACTIVE", "CODEX_EXEC", false)
        .expect("active run should register");

    assert_eq!(
        registry
            .request_stop("SESSION-MISMATCH", "RUN-STALE")
            .expect_err("stale stop should be rejected"),
        ActiveRunError::RunMismatch {
            active_run_code: "RUN-ACTIVE".into(),
        }
    );
    assert_eq!(
        registry
            .snapshot("SESSION-MISMATCH")
            .expect("snapshot should load")
            .expect("snapshot should exist")
            .state
            .code(),
        "RUNNING"
    );
}

#[test]
fn active_run_registry_routes_fifo_and_rejects_after_finishing_or_drop() {
    let registry = ActiveRunRegistry::new();
    let guard = registry
        .register("SESSION-1", "RUN-1", "REACT", true)
        .expect("active run should register");
    let control = registry
        .control("SESSION-1", "RUN-1")
        .expect("active run control should resolve");

    for (code, content) in [("MSG-1", "first"), ("MSG-2", "second")] {
        registry
            .accept_steer("SESSION-1", "RUN-1", || {
                Ok::<_, ()>((code.to_string(), steer(code, content)))
            })
            .expect("steer should route")
            .expect("steer persistence callback should succeed");
    }

    let pending = control
        .take_pending()
        .expect("pending steers should be readable");
    assert_eq!(
        pending
            .iter()
            .map(|input| input.content.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    assert!(
        control
            .take_pending_or_begin_finishing()
            .expect("finishing boundary should be available")
            .is_empty()
    );
    assert_eq!(
        registry
            .accept_steer("SESSION-1", "RUN-1", || {
                Ok::<_, ()>(((), steer("MSG-3", "too late")))
            })
            .expect_err("finishing run must reject steer"),
        ActiveRunError::RunFinishing
    );

    drop(guard);
    assert!(matches!(
        registry.control("SESSION-1", "RUN-1"),
        Err(ActiveRunError::NoActiveRun)
    ));
}

#[test]
fn active_run_registry_distinguishes_mismatch_unsupported_and_failed_persistence() {
    let registry = ActiveRunRegistry::new();
    let _guard = registry
        .register("SESSION-2", "RUN-ACTIVE", "CODEX_EXEC", false)
        .expect("unsupported active runtime should still register");

    assert_eq!(
        registry
            .accept_steer("SESSION-2", "RUN-STALE", || {
                Ok::<_, ()>(((), steer("MSG-1", "stale")))
            })
            .expect_err("stale expected run should be rejected"),
        ActiveRunError::RunMismatch {
            active_run_code: "RUN-ACTIVE".into(),
        }
    );
    assert_eq!(
        registry
            .accept_steer("SESSION-2", "RUN-ACTIVE", || {
                Ok::<_, ()>(((), steer("MSG-2", "unsupported")))
            })
            .expect_err("unsupported runtime should reject steer"),
        ActiveRunError::RuntimeUnsupported {
            runtime_adapter_key: "CODEX_EXEC".into(),
        }
    );

    let supported_registry = ActiveRunRegistry::new();
    let _supported_guard = supported_registry
        .register("SESSION-3", "RUN-3", "REACT", true)
        .expect("react run should register");
    let control = supported_registry
        .control("SESSION-3", "RUN-3")
        .expect("react control should resolve");
    let persistence = supported_registry
        .accept_steer("SESSION-3", "RUN-3", || {
            Err::<((), AgentSteerInput), _>("storage failed")
        })
        .expect("active run routing should succeed");
    assert_eq!(persistence, Err("storage failed"));
    assert!(
        control
            .take_pending()
            .expect("pending queue should be readable")
            .is_empty(),
        "failed persistence must not enqueue steer"
    );
}

#[tokio::test]
async fn reserved_run_rejects_steer_until_activation_and_observes_pre_start_stop() {
    let registry = ActiveRunRegistry::new();
    let reservation = registry
        .reserve("SESSION-STARTING", "RUN-STARTING", "REACT", true)
        .expect("run admission should reserve the session");

    let starting = registry
        .snapshot("SESSION-STARTING")
        .expect("starting snapshot should load")
        .expect("starting snapshot should exist");
    assert_eq!(starting.state.code(), "RUNNING");
    assert!(
        !starting.supports_steer,
        "starting admission must not advertise steer"
    );
    assert_eq!(
        registry
            .accept_steer("SESSION-STARTING", "RUN-STARTING", || {
                Ok::<_, ()>(((), steer("MSG-EARLY", "too early")))
            })
            .expect_err("starting admission must reject steer"),
        ActiveRunError::RunStarting
    );

    let mut cancellation = reservation.cancellation();
    registry
        .request_stop("SESSION-STARTING", "RUN-STARTING")
        .expect("pre-start stop should be accepted");
    assert_eq!(
        registry
            .activate_reservation(
                &reservation,
                "SESSION-STARTING",
                "RUN-STARTING",
                "REACT",
                true,
            )
            .expect_err("stopped admission must not activate"),
        ActiveRunError::RunFinishing
    );
    tokio::time::timeout(std::time::Duration::from_secs(1), cancellation.cancelled())
        .await
        .expect("pre-start stop signal should remain observable");
}

#[test]
fn chat_service_persists_server_message_before_enqueuing_steer() {
    let active_runs = ActiveRunRegistry::new();
    let repository =
        SqliteChatRepository::connect(":memory:").expect("chat repository should initialize");
    let chats = ChatService::with_id_provider(
        repository,
        Arc::new(FixedSnowflakeIdProvider::new(735_800_000_000_000_001)),
    )
    .with_active_runs(active_runs.clone());
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("Steer persistence".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USR-1".into()),
        )
        .expect("chat session should be created");
    let _guard = active_runs
        .register(&session.session_code, "RUN-1", "REACT", true)
        .expect("active run should register");
    let control = active_runs
        .control(&session.session_code, "RUN-1")
        .expect("active control should resolve");

    let result = chats
        .steer_message(SteerChatMessageCommand {
            requested_skills: Vec::new(),
            session_code: Some(session.session_code.clone()),
            expected_run_code: Some("RUN-1".into()),
            content: Some("persist and steer".into()),
            attachments: Vec::new(),
        })
        .expect("steer should be accepted");

    assert_eq!(result.session_code, session.session_code);
    assert_eq!(result.run_code, "RUN-1");
    assert_eq!(result.message.run_code.as_deref(), Some("RUN-1"));
    assert_eq!(result.message.content, "persist and steer");
    let persisted = chats
        .list_messages(&session.session_code)
        .expect("chat messages should be readable");
    assert_eq!(persisted, vec![result.message.clone()]);
    let queued = control
        .take_pending()
        .expect("active queue should be readable");
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].steer_code, result.message.message_code);
    assert_eq!(queued[0].content, "persist and steer");
}
