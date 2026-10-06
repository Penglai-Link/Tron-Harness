//! Opt-in startup rehearsal against an online-backup copy only.
#[tokio::test]
#[ignore = "requires TRON_KB_REHEARSAL_DB pointing to an isolated backup copy"]
async fn copied_development_database_initializers_are_auditable() {
    use std::{path::PathBuf, sync::Arc};
    use tron_backend::{
        app::AppState,
        application::profile::ProfileRegistryService,
        config::{AppConfig, ResourceConfig},
        shared::id::FixedSnowflakeIdProvider,
    };
    let db = PathBuf::from(std::env::var("TRON_KB_REHEARSAL_DB").expect("isolated copy required"));
    assert!(db.starts_with("/Users/clouds3n/Documents/Codex/2026-10-03/task/knowledge-base-implementation-evidence") && db.parent().unwrap().file_name().unwrap().to_string_lossy().starts_with("rollout-"),"default/user DB prohibited");
    let root = db.parent().unwrap();
    let mut config =
        AppConfig::from_yaml_file(root.join("application-before.yaml")).expect("backup config");
    config.database_url = db.display().to_string();
    config.graph_storage_path = ":memory:".into();
    config.resource =
        ResourceConfig::with_default_root(root.join("rehearsal-resources").display().to_string());
    config.skills_registry_root = root.join("rehearsal-skills").display().to_string();
    config.project_doc_root = root.display().to_string();
    config.terminal_working_directory = root.display().to_string();
    let state = AppState::new_with_profiles(
        config,
        Arc::new(FixedSnowflakeIdProvider::new(790000000000000000)),
        ProfileRegistryService::new(":memory:").unwrap(),
    )
    .expect("copied DB startup");
    // Do not construct HTTP router: avoid background provider sync/heartbeat.
    state.agent_framework.close().await;
    println!(
        "isolated copied DB startup finished; no user originals, model calls or grants requested"
    );
}
