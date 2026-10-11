use super::*;
use crate::domain::agent::{AgentCollaborationOwner, AgentExecutionScope};
use std::sync::atomic::{AtomicBool, Ordering};

fn record(id: &str) -> ThreadRecord {
    ThreadRecord {
        summary: fw::ThreadSummary {
            thread_id: id.into(),
            title: Some("Delegated work".into()),
            status: fw::ThreadStatus::Failed,
            latest_run_id: Some(fw::RunId::new_v4()),
        },
        workspace_code: String::new(),
        user_code: Some("profile".into()),
        agent_code: "coder".into(),
        owner: AgentCollaborationOwner {
            kind: "DUDE".into(),
            code: "main-dude".into(),
            origin_session_code: Some("dude-work-session".into()),
            task_code: Some("task-1".into()),
        },
        initiator_agent_code: Some("main-dude".into()),
        origin_session_code: "dude-work-session".into(),
        execution_scope: AgentExecutionScope::default(),
        completion: Some((fw::ThreadStatus::Failed, Err("needs attention".into()))),
        closed: false,
        feedback_pending: true,
        revision: 0,
        run_admitted_at_ms: fw::lifecycle_time_ms(),
        execution_sequence: 0,
    }
}
struct Sink {
    fail: AtomicBool,
    events: std::sync::Mutex<Vec<ThreadFeedback>>,
}
#[async_trait::async_trait]
impl ThreadFeedbackSink for Sink {
    async fn publish(&self, feedback: ThreadFeedback) -> Result<(), String> {
        if self.fail.swap(false, Ordering::AcqRel) {
            return Err("transient outage".into());
        }
        self.events.lock().unwrap().push(feedback);
        Ok(())
    }
}
#[tokio::test]
async fn durable_thread_outbox_survives_host_and_parent_exit_and_retries_once() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("threads.sqlite3");
    let host = FrameworkHost::persistent(&path, None);
    let mut first = record("thread-1");
    host.persist_thread(&mut first, true).await.unwrap();
    // A second run must not replace undelivered feedback from the first run.
    first.execution_sequence += 1;
    host.persist_thread(&mut first, false).await.unwrap();
    drop(host);
    let host = FrameworkHost::persistent(&path, None);
    let sink = Arc::new(Sink {
        fail: AtomicBool::new(true),
        events: Default::default(),
    });
    host.set_thread_feedback_sink(sink.clone()).unwrap();
    assert!(host.flush_thread_feedback().await.is_err());
    assert_eq!(host.flush_thread_feedback().await.unwrap(), 2);
    assert_eq!(host.flush_thread_feedback().await.unwrap(), 0);
    let events = sink.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_ne!(events[0].event_key, events[1].event_key);
    assert!(events.iter().all(|event| event.owner.code == "main-dude"
        && event.origin_session_code == "dude-work-session"
        && event.owner.task_code.as_deref() == Some("task-1")));
}
#[tokio::test]
async fn stale_thread_writer_cannot_overwrite_latest_state_and_close_keeps_history_binding() {
    let host = FrameworkHost::temporary(None).unwrap();
    let mut first = record("thread-1");
    first.feedback_pending = false;
    host.persist_thread(&mut first, true).await.unwrap();
    let mut stale = first.clone();
    first.closed = true;
    host.persist_thread(&mut first, false).await.unwrap();
    stale.summary.title = Some("stale".into());
    assert!(host.persist_thread(&mut stale, false).await.is_err());
    let stored = host.thread_record("thread-1").await.unwrap().unwrap();
    assert!(stored.closed);
    assert_eq!(stored.agent_code, "coder");
    assert_eq!(host.dude_threads("other-dude").await.unwrap().len(), 0);
    assert_eq!(host.dude_threads("main-dude").await.unwrap().len(), 1);
    let json = serde_json::to_string(&stored).unwrap();
    assert!(!json.contains("apiKey"));
    assert!(!json.contains("variables"));
}

#[test]
fn thread_access_requires_root_owner_and_scope_but_allows_other_sessions_of_same_dude() {
    let record = record("thread-1");
    let mut caller = record.owner.clone();
    caller.origin_session_code = Some("another-work-session".into());
    assert!(record.authorized_owner("", Some("profile"), &caller));
    caller.code = "other-dude".into();
    assert!(!record.authorized_owner("", Some("profile"), &caller));
    caller = record.owner.clone();
    caller.kind = "USER".into();
    assert!(!record.authorized_owner("", Some("profile"), &caller));
    assert!(!record.authorized_owner("workspace", Some("profile"), &record.owner));
    assert!(!record.authorized_owner("", Some("other-profile"), &record.owner));
}
