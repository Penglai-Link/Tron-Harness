use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::Router;
use tron_backend::{
    adapters::outbound::persistence::{
        SqliteAppRepository, SqliteVariableRepository, SqliteWorkspaceRepository,
    },
    app::{AppState, build_router_with_state},
    application::{
        app::AppRepository, variable::VariableRepository, workspace::WorkspaceRepository,
    },
    config::{AppConfig, VariableCryptoConfig},
    domain::{
        app::{App, AppRuntimeRole, AppStatus, AppTag},
        variable::Variable,
        workspace::{Workspace, WorkspaceStatus},
    },
    shared::id::{IdGenerationError, SnowflakeId, SnowflakeIdProvider},
};


struct FixedSnowflakeIdProvider(std::sync::atomic::AtomicI64);

impl FixedSnowflakeIdProvider {
    fn new(seed: i64) -> Self {
        Self(std::sync::atomic::AtomicI64::new(seed))
    }
}

impl SnowflakeIdProvider for FixedSnowflakeIdProvider {
    fn next_id(&self) -> Result<SnowflakeId, IdGenerationError> {
        Ok(SnowflakeId::new(self.0.fetch_add(1, Ordering::Relaxed)))
    }
}

static TEST_DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static TEST_ID_SEQUENCE: AtomicU64 = AtomicU64::new(734_900_000_000_000_000);
const FIXTURE_TIMESTAMP: &str = "2026-05-16T00:00:00Z";

pub fn app_with_mut_state_setup(config: AppConfig, setup: impl FnOnce(&mut AppState)) -> Router {
    let profiles = tron_backend::application::profile::ProfileRegistryService::new(":memory:")
        .expect("isolated profiles");
    profiles
        .register_or_update("USR-AUTH-0001", "profile.operator")
        .expect("active test profile");
    let mut state = AppState::new_with_profiles(
        config,
        Arc::new(FixedSnowflakeIdProvider::new(next_test_id_seed())),
        profiles,
    )
    .expect("test state");
    setup(&mut state);
    build_router_with_state(state).expect("test router should build")
}

pub fn prepared_config(mut config: AppConfig) -> AppConfig {
    if config.database_url == ":memory:" {
        config.database_url = temp_database_path("integration").display().to_string();
    }
    if config.variable_crypto.key_base64.trim().is_empty() {
        config.variable_crypto = VariableCryptoConfig::test_default();
    }
    let defaults = AppConfig::default().system_agents;
    if config.system_agents.codex_command == defaults.codex_command {
        config.system_agents.codex_command.clear();
    }
    if config.system_agents.opencode_command == defaults.opencode_command {
        config.system_agents.opencode_command.clear();
    }
    if config.system_agents.dsh_command == defaults.dsh_command {
        config.system_agents.dsh_command.clear();
    }
    seed_default_fixture_data(&config.database_url);
    config
}

fn next_test_id_seed() -> i64 {
    TEST_ID_SEQUENCE.fetch_add(10_000, Ordering::Relaxed) as i64
}

fn seed_default_fixture_data(database_url: &str) {
    seed_workspaces(database_url);
    seed_apps(database_url);
    seed_variables(database_url);
}

fn seed_workspaces(database_url: &str) {
    let mut repository =
        SqliteWorkspaceRepository::connect(database_url).expect("workspace repository");
    if repository
        .list_workspaces()
        .expect("workspace list")
        .is_empty()
    {
        for workspace in [
            ("ENTERPRISE", "Enterprise", None),
            ("PLATFORM", "Platform", Some("ENTERPRISE")),
            ("TRON_CORE", "Tron Core", Some("PLATFORM")),
            ("COMPLIANCE", "Compliance", None),
        ] {
            repository
                .create_workspace(&Workspace {
                    id: SnowflakeId::new(next_test_id_seed()),
                    workspace_code: workspace.0.into(),
                    name: workspace.1.into(),
                    description: Some(format!("{} workspace", workspace.1)),
                    parent_workspace_code: workspace.2.map(str::to_owned),
                    status: WorkspaceStatus::Active,
                    created_at: FIXTURE_TIMESTAMP.into(),
                    updated_at: FIXTURE_TIMESTAMP.into(),
                    archived_at: None,
                })
                .expect("workspace fixture");
        }
    }
}

fn seed_apps(database_url: &str) {
    let mut repository = SqliteAppRepository::connect(database_url).expect("app repository");
    if repository.list_apps().expect("app list").is_empty() {
        let app = App {
            id: SnowflakeId::new(next_test_id_seed()),
            app_code: "APP-TRON-CONSOLE".into(),
            workspace_code: "TRON_CORE".into(),
            name: "Tron Console".into(),
            description: Some("Test console application".into()),
            local_directory: "/workspace/tron".into(),
            runtime_role: AppRuntimeRole::Frontend,
            repository_url: None,
            default_branch: Some("master".into()),
            status: AppStatus::Active,
            created_at: FIXTURE_TIMESTAMP.into(),
            updated_at: FIXTURE_TIMESTAMP.into(),
            archived_at: None,
        };
        repository.create_app(&app).expect("app fixture");
        repository
            .replace_app_tags(
                &app.app_code,
                &[AppTag {
                    app_code: app.app_code.clone(),
                    tag: "Test".into(),
                    linked_at: FIXTURE_TIMESTAMP.into(),
                }],
            )
            .expect("app tags fixture");
    }
}

fn seed_variables(database_url: &str) {
    let mut repository =
        SqliteVariableRepository::connect(database_url).expect("variable repository");
    if repository
        .list_variables()
        .expect("variable list")
        .is_empty()
    {
        // Explicit test data; production startup must remain free of demo secrets.
        for (workspace, key, value) in [
            ("ENTERPRISE", "DEFAULT_REGION", "us-east-1"),
            ("PLATFORM", "ROUTER_TIMEOUT_MS", "800"),
            ("PLATFORM", "OPENAI_API_KEY", "fixture-platform-key"),
            ("TRON_CORE", "ROUTER_TIMEOUT_MS", "400"),
            ("TRON_CORE", "OPENAI_API_KEY", "fixture-workspace-key"),
        ] {
            repository
                .create_variable(&Variable {
                    id: SnowflakeId::new(next_test_id_seed()),
                    workspace_code: workspace.into(),
                    key: key.into(),
                    value: value.into(),
                    description: Some("Integration fixture".into()),
                    is_sensitive: false,
                    value_nonce: None,
                    value_tag: None,
                    value_algorithm: None,
                    value_key_version: None,
                    created_at: FIXTURE_TIMESTAMP.into(),
                    updated_at: FIXTURE_TIMESTAMP.into(),
                })
                .expect("variable fixture");
        }
    }
}

pub fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEST_DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "tron-tests-{test_name}-{}-{nanos}-{sequence}.sqlite3",
        std::process::id()
    ))
}

