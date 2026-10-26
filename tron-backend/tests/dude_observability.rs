//! Actual always-on workers trace idle memory work and pre-model failures.
mod common;
use std::{collections::HashSet, time::Duration};
use tron_backend::{
    application::dude::DudeEventInput,
    config::{AppConfig, ResourceConfig},
    domain::dude::{DudeEventSource, DudeEventStatus},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn independent_wakeups_and_preparation_failures_persist_correlated_traces() {
    let directory = tempfile::tempdir().unwrap();
    let config = common::prepared_config(AppConfig {
        database_url: directory.path().join("dude.sqlite3").display().to_string(),
        graph_storage_path: directory.path().join("dude.grafeo").display().to_string(),
        skills_registry_root: directory.path().join("skills").display().to_string(),
        resource: ResourceConfig::with_default_root(
            directory.path().join("resources").display().to_string(),
        ),
        ..Default::default()
    });
    let mut captured = None;
    let _app = common::app_with_state_setup(config, |state| captured = Some(state.clone()));
    let state = captured.unwrap();
    state.knowledge_bases.stop_ingestion_workers();
    let mut expected = Vec::new();
    for (dude, source) in [
        ("memory-dude", DudeEventSource::Heartbeat),
        ("memory-dude", DudeEventSource::MemoryBatch),
        ("memory-dude", DudeEventSource::ThreadFeedback),
        // No model is configured: this must still produce an error trace.
        ("main-dude", DudeEventSource::Heartbeat),
    ] {
        let event = state
            .dudes
            .publish_event(DudeEventInput {
                dude_code: dude.into(),
                source,
                dedup_key: format!("observability:{dude}:{source:?}"),
                session_code: None,
                task_code: None,
                workspace_code: None,
                payload: serde_json::json!({}),
                available_at: chrono::Utc::now().timestamp_millis(),
            })
            .unwrap();
        expected.push(event);
    }
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if expected.iter().all(|event| {
                state
                    .dudes
                    .events(&event.dude_code)
                    .unwrap()
                    .iter()
                    .any(|current| {
                        current.event_id == event.event_id
                            && matches!(
                                current.status,
                                DudeEventStatus::Completed | DudeEventStatus::Failed
                            )
                    })
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("all wakeups reach a durable outcome");
    assert!(state.agent_framework.close().await.is_success());
    let db = rusqlite::Connection::open(
        std::path::Path::new(&state.config.database_url).with_extension("telemetry.sqlite3"),
    )
    .unwrap();
    let spans = db
        .prepare("SELECT data FROM ef_telemetry_spans")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|row| {
            serde_json::from_str::<eliteforge_agent_core::TelemetrySpan>(&row.unwrap()).unwrap()
        })
        .collect::<Vec<_>>();
    let mut trace_ids = HashSet::new();
    for event in &expected {
        let roots = spans
            .iter()
            .filter(|span| {
                span.name == "tron.dude.execute"
                    && span.attributes.get("tron.dude.event.id") == Some(&event.event_id)
            })
            .collect::<Vec<_>>();
        assert!(
            !roots.is_empty(),
            "each occurrence has a root, including before model admission"
        );
        for root in roots {
            assert!(
                root.parent_span_id.is_none(),
                "workers never reuse another wakeup's parent"
            );
            assert!(
                trace_ids.insert(root.trace_id.clone()),
                "attempts own independent traces"
            );
            assert_eq!(root.attributes["tron.dude.code"], event.dude_code);
            assert_eq!(
                root.attributes["tron.dude.event.source"],
                format!("{:?}", event.source)
            );
            assert_eq!(root.attributes["tron.workspace.code"], "");
            if event.dude_code == "main-dude" {
                assert_eq!(root.status_code, 2);
                assert_eq!(root.attributes["agent.operation.outcome"], "error");
            } else {
                assert_eq!(root.attributes["agent.operation.outcome"], if root.status_code == 2 { "error" } else { "completed" });
            }
        }
    }
}
