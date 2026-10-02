//! Waiting logical runs release only the finished physical SQLite execution lease.
use super::*;

fn pause_context(service: &ShortTermMemoryService, session: &str, run: &str) -> AgentRunContext {
    let mut context = base_context(session);
    context.run_code = run.into();
    let input = service
        .record_user_input(&context, "paused question input")
        .unwrap();
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.into(),
        input_code: input,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    context
}

#[test]
fn waiting_execution_releases_lease_atomically_and_is_not_backfilled_or_recovered() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("pause.sqlite3").display().to_string();
    let session = create_test_session(&database, "durable pause lease");
    let mut config = ShortTermMemoryConfig::default();
    config.event_log.mode = "lifecycleOnly".into();
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database).unwrap(),
        config,
    );
    let context = pause_context(&service, &session.session_code, "RUN-PAUSED");
    service.record_run_started(&context).unwrap();
    let mut observer = SqliteShortTermMemoryRepository::connect(&database).unwrap();
    assert!(observer.active_run_exists(&session.session_code).unwrap());
    service.record_run_awaiting_input(&context).unwrap();
    assert!(!observer.active_run_exists(&session.session_code).unwrap());
    drop(observer);
    drop(service);
    let mut restarted = SqliteShortTermMemoryRepository::connect(&database).unwrap();
    assert!(!restarted.active_run_exists(&session.session_code).unwrap());
    assert_eq!(restarted.recover_stale_runs(100).unwrap(), 0);
    let connection = rusqlite::Connection::open(&database).unwrap();
    let events = connection
        .prepare(
            "SELECT event_type FROM agent_transcript_events WHERE session_code=?1 ORDER BY seq",
        )
        .unwrap()
        .query_map([&session.session_code], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        events,
        vec!["agent.run.started", "agent.run.awaiting_input"]
    );
}

#[test]
fn late_waiting_event_does_not_release_a_new_execution_lease() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory
        .path()
        .join("late-pause.sqlite3")
        .display()
        .to_string();
    let session = create_test_session(&database, "late pause lease");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database).unwrap(),
        ShortTermMemoryConfig::default(),
    );
    let waiting = pause_context(&service, &session.session_code, "RUN-PAUSED");
    service.record_run_started(&waiting).unwrap();
    service.record_run_awaiting_input(&waiting).unwrap();
    let active = pause_context(&service, &session.session_code, "RUN-RESUMED");
    service.record_run_started(&active).unwrap();
    service.record_run_awaiting_input(&waiting).unwrap();
    let connection = rusqlite::Connection::open(&database).unwrap();
    let owner: String = connection
        .query_row(
            "SELECT run_code FROM agent_session_run_leases WHERE session_code=?1",
            [&session.session_code],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(owner, "RUN-RESUMED");
    let other = pause_context(&service, &session.session_code, "RUN-OTHER");
    assert_eq!(
        service.record_run_started(&other).unwrap_err(),
        ShortTermMemoryError::RunConflict
    );
    service.record_run_completed(&active).unwrap();
    service.record_run_started(&other).unwrap();
    service
        .record_run_failed(&other, "fixture cleanup")
        .unwrap();
}
