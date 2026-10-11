//! Durable queue invariants, exercised against two independent SQLite handles.
use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteDudeRepository,
    application::dude::DudeRepository,
    domain::dude::{
        Dude, DudeEvent, DudeEventSource, DudeEventStatus, DudeKind, PlatformNotification,
    },
};

fn dude() -> Dude {
    Dude {
        dude_code: "queue-dude".into(),
        kind: DudeKind::Custom,
        enabled: true,
        heartbeat_minutes: 30,
        memory_auto_approve: false,
        is_main: false,
        created_at: 0,
        updated_at: 0,
    }
}

fn event(id: &str, key: &str, session: &str) -> DudeEvent {
    DudeEvent {
        event_id: id.into(),
        dude_code: "queue-dude".into(),
        source: DudeEventSource::TaskReminder,
        dedup_key: key.into(),
        session_code: Some(session.into()),
        task_code: None,
        workspace_code: None,
        payload: json!({"message":id}),
        status: DudeEventStatus::Queued,
        available_at: 100,
        lease_until: None,
        lease_token: None,
        attempts: 0,
        result: None,
        created_at: 0,
    }
}

fn repository(path: &std::path::Path) -> SqliteDudeRepository {
    SqliteDudeRepository::connect(path.to_str().unwrap()).expect("isolated Dude queue")
}

#[test]
fn persisted_event_deduplicates_and_expired_claim_is_fenced_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("queue.sqlite3");
    let mut first = repository(&path);
    first.save(&dude()).unwrap();
    let original = first
        .enqueue(&event("original", "logical-event", "work"))
        .unwrap();
    let duplicate = first
        .enqueue(&event("duplicate", "logical-event", "work"))
        .unwrap();
    assert_eq!(original.event_id, duplicate.event_id);
    assert_eq!(first.events("queue-dude").unwrap().len(), 1);
    assert!(
        first.claim(99, 1000).unwrap().is_none(),
        "future events must wait"
    );
    let claim = first.claim(100, 1000).unwrap().unwrap();
    drop(first);

    let mut restarted = repository(&path);
    assert!(
        restarted.claim(1099, 1000).unwrap().is_none(),
        "a live lease must not be stolen"
    );
    let recovered = restarted.claim(1101, 1000).unwrap().unwrap();
    assert_eq!(recovered.event_id, original.event_id);
    assert_eq!(recovered.attempts, 2);
    assert_ne!(claim.lease_token, recovered.lease_token);
    assert!(
        restarted
            .finish(
                &claim.event_id,
                claim.lease_token.as_deref().unwrap(),
                DudeEventStatus::Completed,
                "stale result",
                1102
            )
            .is_err(),
        "expired worker must not overwrite the recovered claim"
    );
    restarted
        .finish(
            &recovered.event_id,
            recovered.lease_token.as_deref().unwrap(),
            DudeEventStatus::Completed,
            "recovered result",
            1102,
        )
        .unwrap();
    assert!(restarted.claim(1103, 1000).unwrap().is_none());
    let persisted = restarted.events("queue-dude").unwrap();
    assert_eq!(persisted[0].status, DudeEventStatus::Completed);
    assert_eq!(persisted[0].result.as_deref(), Some("recovered result"));
}

#[test]
fn another_session_can_claim_while_foreground_session_remains_busy() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("parallel.sqlite3");
    let mut foreground = repository(&path);
    foreground.save(&dude()).unwrap();
    foreground
        .enqueue(&event("foreground", "foreground-1", "interactive"))
        .unwrap();
    foreground
        .enqueue(&event("queued-foreground", "foreground-2", "interactive"))
        .unwrap();
    foreground
        .enqueue(&event("background", "background-1", "task-work"))
        .unwrap();
    let active = foreground.claim(100, 1000).unwrap().unwrap();
    assert_eq!(active.event_id, "foreground");

    let mut background = repository(&path);
    let parallel = background.claim(100, 1000).unwrap().unwrap();
    assert_eq!(
        parallel.event_id, "background",
        "foreground traffic must not starve a different session"
    );
    assert!(
        background.claim(100, 1000).unwrap().is_none(),
        "same-session work stays serialized"
    );
    foreground
        .finish(
            &active.event_id,
            active.lease_token.as_deref().unwrap(),
            DudeEventStatus::Completed,
            "foreground done",
            101,
        )
        .unwrap();
    let next = background.claim(102, 1000).unwrap().unwrap();
    assert_eq!(next.event_id, "queued-foreground");
}

#[test]
fn notification_delivery_is_idempotent_and_read_does_not_resolve_action() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notifications.sqlite3");
    let mut first = repository(&path);
    let notice = PlatformNotification {
        id: "notice-original".into(),
        dedup_key: "candidate-pending-1".into(),
        dude_code: Some("queue-dude".into()),
        title: "Approve Skill".into(),
        body: "Review exact artifact".into(),
        category: "SkillApproval".into(),
        target: json!({"candidateCode":"candidate-1"}),
        read_at: None,
        resolved_at: None,
        archived_at: None,
        created_at: 100,
    };
    first.notify(&notice).unwrap();
    let mut duplicate = notice.clone();
    duplicate.id = "notice-retry".into();
    assert_eq!(first.notify(&duplicate).unwrap().id, notice.id);
    first
        .update_notification(&notice.id, true, false, 101)
        .unwrap();
    drop(first);

    let mut restarted = repository(&path);
    let persisted = restarted.notifications().unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].read_at, Some(101));
    assert_eq!(
        persisted[0].resolved_at, None,
        "reading a notice must not approve a Skill or answer a question"
    );
    assert_eq!(persisted[0].target, notice.target);
    restarted
        .update_notification(&notice.id, true, true, 102)
        .unwrap();
    assert_eq!(restarted.notifications().unwrap()[0].archived_at, Some(102));
}
