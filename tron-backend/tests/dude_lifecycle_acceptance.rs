//! Durable lifecycle transitions, with two independent handles and a controlled clock.
use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteDudeRepository,
    application::dude::DudeRepository,
    domain::dude::{Dude, DudeEvent, DudeEventSource, DudeEventStatus, DudeKind},
};

fn open(path: &std::path::Path) -> SqliteDudeRepository {
    SqliteDudeRepository::connect(path.to_str().unwrap()).unwrap()
}
fn actor(enabled: bool) -> Dude {
    Dude {
        dude_code: "lifecycle".into(),
        kind: DudeKind::Custom,
        enabled,
        heartbeat_minutes: 30,
        memory_auto_approve: false,
        is_main: false,
        created_at: 0,
        updated_at: 0,
    }
}
fn event(id: &str, source: DudeEventSource, session: &str) -> DudeEvent {
    DudeEvent {
        event_id: id.into(),
        dude_code: "lifecycle".into(),
        source,
        dedup_key: id.into(),
        session_code: Some(session.into()),
        task_code: None,
        workspace_code: None,
        payload: json!({}),
        status: DudeEventStatus::Queued,
        available_at: 100,
        lease_until: None,
        lease_token: None,
        attempts: 0,
        result: None,
        created_at: 0,
    }
}
fn finish(
    repo: &mut SqliteDudeRepository,
    event: &DudeEvent,
    status: DudeEventStatus,
    now: i64,
    _notify: bool,
) {
    repo.finish(
        &event.event_id,
        event.lease_token.as_deref().unwrap(),
        status,
        "result",
        now,
    )
    .unwrap();
}

#[test]
fn lease_renewal_and_backoff_survive_reopen_and_fence_old_workers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lease.sqlite3");
    let mut first = open(&path);
    first.save(&actor(true)).unwrap();
    first
        .enqueue(&event(
            "work",
            DudeEventSource::TaskReminder,
            "work-session",
        ))
        .unwrap();
    let original = first.claim(100, 1000).unwrap().unwrap();
    first.renew(&original, 1000, 2000).unwrap();
    drop(first);
    let mut restarted = open(&path);
    assert!(
        restarted.claim(1101, 1000).unwrap().is_none(),
        "renewal extends the durable lease beyond its original expiry"
    );
    let recovered = restarted.claim(3001, 1000).unwrap().unwrap();
    assert_eq!(recovered.attempts, 2);
    assert!(restarted.renew(&original, 3002, 1000).is_err());
    assert!(restarted.retry(&original, 3002, 500, "stale").is_err());
    restarted
        .retry(&recovered, 3002, 500, "transient provider failure")
        .unwrap();
    drop(restarted);
    let mut restarted = open(&path);
    assert!(restarted.claim(3501, 1000).unwrap().is_none());
    let retry = restarted.claim(3502, 1000).unwrap().unwrap();
    assert_eq!(retry.attempts, 3);
    assert_eq!(retry.result.as_deref(), Some("transient provider failure"));
    assert_ne!(retry.lease_token, recovered.lease_token);
    finish(
        &mut restarted,
        &retry,
        DudeEventStatus::Completed,
        3503,
        false,
    );
    assert!(restarted.claim(4503, 1000).unwrap().is_none());
}

#[test]
fn awaiting_input_blocks_automatic_resume_but_allows_human_and_parallel_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.sqlite3");
    let mut repo = open(&path);
    repo.save(&actor(true)).unwrap();
    repo.enqueue(&event("question", DudeEventSource::TaskReminder, "waiting"))
        .unwrap();
    let question = repo.claim(100, 1000).unwrap().unwrap();
    finish(
        &mut repo,
        &question,
        DudeEventStatus::AwaitingInput,
        101,
        true,
    );
    repo.enqueue(&event("automatic", DudeEventSource::Heartbeat, "waiting"))
        .unwrap();
    repo.enqueue(&event("human", DudeEventSource::UserMessage, "waiting"))
        .unwrap();
    repo.enqueue(&event(
        "parallel",
        DudeEventSource::TaskReminder,
        "other-session",
    ))
    .unwrap();
    drop(repo);
    let mut repo = open(&path);
    let human = repo.claim(102, 1000).unwrap().unwrap();
    assert_eq!(
        human.event_id, "human",
        "automatic events must not bypass pending human input"
    );
    let parallel = repo.claim(102, 1000).unwrap().unwrap();
    assert_eq!(
        parallel.event_id, "parallel",
        "another session still runs while human input resumes"
    );
    assert!(repo.claim(102, 1000).unwrap().is_none());
    finish(&mut repo, &human, DudeEventStatus::Completed, 103, false);
    assert!(
        repo.claim(104, 1000).unwrap().is_none(),
        "a successful human run alone does not silently discard the pending question"
    );
    repo.resolve_awaiting_input("waiting", "human answered", 105)
        .unwrap();
    let automatic = repo.claim(106, 1000).unwrap().unwrap();
    assert_eq!(automatic.event_id, "automatic");
    assert!(repo.notifications().unwrap().is_empty());
}

#[test]
fn quiet_success_and_stale_reminder_finish_without_notifications() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quiet.sqlite3");
    let mut repo = open(&path);
    repo.save(&actor(true)).unwrap();
    for (id, source) in [
        ("quiet-heartbeat", DudeEventSource::Heartbeat),
        ("stale-reminder", DudeEventSource::TaskReminder),
    ] {
        repo.enqueue(&event(id, source, id)).unwrap();
        let claimed = repo.claim(100, 1000).unwrap().unwrap();
        finish(&mut repo, &claimed, DudeEventStatus::Completed, 101, false);
    }
    drop(repo);
    let mut repo = open(&path);
    assert!(repo.notifications().unwrap().is_empty());
    repo.enqueue(&event(
        "failed-heartbeat",
        DudeEventSource::Heartbeat,
        "failed",
    ))
    .unwrap();
    let failed = repo.claim(102, 1000).unwrap().unwrap();
    finish(&mut repo, &failed, DudeEventStatus::Failed, 103, true);
    assert!(repo.notifications().unwrap().is_empty());
}

#[test]
fn paused_dude_still_accepts_direct_user_messages_without_starting_background_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pause.sqlite3");
    let mut repo = open(&path);
    repo.save(&actor(false)).unwrap();
    repo.enqueue(&event("background", DudeEventSource::TaskReminder, "task"))
        .unwrap();
    repo.enqueue(&event(
        "direct",
        DudeEventSource::UserMessage,
        "conversation",
    ))
    .unwrap();
    let direct = repo.claim(100, 1000).unwrap().unwrap();
    assert_eq!(direct.event_id, "direct");
    finish(&mut repo, &direct, DudeEventStatus::Completed, 101, false);
    assert!(repo.claim(102, 1000).unwrap().is_none());
    repo.save(&actor(true)).unwrap();
    assert_eq!(
        repo.claim(103, 1000).unwrap().unwrap().event_id,
        "background"
    );
}
