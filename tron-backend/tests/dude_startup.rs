//! First launch must not consume background events before account authorization.
mod common;

use std::{sync::Arc, time::Duration};
use tron_backend::{
    app::{AppState, build_router_with_state},
    application::{dude::DudeEventInput, profile::ProfileRegistryService},
    config::{AppConfig, ResourceConfig},
    domain::dude::{DudeEventSource, DudeEventStatus},
    shared::id::FixedSnowflakeIdProvider,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn first_launch_waits_for_profile_without_consuming_events_then_resumes() {
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
    let profiles = ProfileRegistryService::new(":memory:").unwrap();
    let state = AppState::new_with_profiles(
        config,
        Arc::new(FixedSnowflakeIdProvider::new(735_000_000_000_000_000)),
        profiles.clone(),
    )
    .unwrap();
    state.knowledge_bases.stop_ingestion_workers();
    // This test isolates the account gate; unfinished onboarding has its own gate.
    for dude in ["main-dude", "memory-dude"] {
        state.dudes.save_soul(dude, dude.into(), "Configured startup fixture.".into()).unwrap();
    }
    let mut pending = Vec::new();
    for (dude, source) in [
        ("main-dude", DudeEventSource::Heartbeat),
        ("memory-dude", DudeEventSource::Heartbeat),
        ("memory-dude", DudeEventSource::MemoryBatch),
        ("memory-dude", DudeEventSource::ThreadFeedback),
    ] {
        pending.push(
            state
                .dudes
                .publish_event(DudeEventInput {
                    dude_code: dude.into(),
                    source,
                    dedup_key: format!("first-launch:{dude}:{source:?}"),
                    session_code: None,
                    task_code: None,
                    workspace_code: None,
                    payload: serde_json::json!({}),
                    available_at: chrono::Utc::now().timestamp_millis(),
                })
                .unwrap(),
        );
    }
    let _app = build_router_with_state(state.clone()).unwrap();

    // Let the actual always-on dispatcher poll repeatedly before authorization.
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(profiles.active().unwrap().is_none());
    for event in &pending {
        let current = state
            .dudes
            .events(&event.dude_code)
            .unwrap()
            .into_iter()
            .find(|current| current.event_id == event.event_id)
            .unwrap();
        assert_eq!(current.status, DudeEventStatus::Queued);
        assert_eq!(current.attempts, 0);
        assert_eq!(current.session_code, event.session_code);
        assert!(current.lease_token.is_none());
        assert!(current.lease_until.is_none());
        assert!(current.result.is_none());
    }
    assert!(state.dudes.notifications().unwrap().is_empty());

    let profile = profiles
        .register_or_update("USR-FIRST-LAUNCH", "first.launch")
        .unwrap();
    profiles.activate(&profile.profile_id).unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let resumed = pending.iter().all(|event| {
                state
                    .dudes
                    .events(&event.dude_code)
                    .unwrap()
                    .iter()
                    .any(|current| {
                        current.event_id == event.event_id
                            && current.attempts > 0
                            && current.status != DudeEventStatus::Running
                            // Missing models and synthetic work inputs may be retryable.
                            && (current.status != DudeEventStatus::Queued || current.result.is_some())
                    })
            });
            if resumed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("queued work resumes after the first account is activated");
    assert!(state.agent_framework.close().await.is_success());
}
