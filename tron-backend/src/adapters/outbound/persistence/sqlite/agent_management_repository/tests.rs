use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

use super::SqliteAgentManagementRepository;

#[test]
fn connect_backfills_runtime_type_column_for_legacy_schema() {
    let database_path = temp_database_path("agent-management-legacy-runtime-type");
    {
        let connection =
            Connection::open(&database_path).expect("legacy sqlite database should open");
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS system_agent_registries (
                    agent_code TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT NOT NULL,
                    command_name TEXT NOT NULL,
                    source TEXT NOT NULL,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    available INTEGER NOT NULL DEFAULT 0,
                    last_scanned_at INTEGER,
                    last_seen_at INTEGER,
                    last_error TEXT
                );",
            )
            .expect("legacy schema should be created");
    }

    let _repository = SqliteAgentManagementRepository::connect(
        database_path
            .to_str()
            .expect("temporary database path should be UTF-8"),
    )
    .expect("repository should migrate legacy schema");

    let verification_connection =
        Connection::open(&database_path).expect("verification sqlite database should open");
    let mut statement = verification_connection
        .prepare("PRAGMA table_info(system_agent_registries)")
        .expect("table info query should prepare");
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("table info query should execute")
        .collect::<Result<Vec<_>, _>>()
        .expect("table info rows should decode");
    assert!(
        columns.iter().any(|column| column == "runtime_type"),
        "runtime_type column should be added for legacy schemas"
    );

    let _ = fs::remove_file(database_path);
}

#[test]
fn connect_backfills_internal_agent_mcp_storage_for_legacy_schema() {
    let database_path = temp_database_path("agent-management-legacy-internal-agent-mcp");
    {
        let connection =
            Connection::open(&database_path).expect("legacy sqlite database should open");
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS workspace_internal_agent_policies (
                    workspace_code TEXT NOT NULL,
                    agent_code TEXT NOT NULL,
                    tool_assignment_mode TEXT NOT NULL,
                    tool_exposure_mode TEXT NOT NULL,
                    expanded_domain_persistence_mode TEXT NOT NULL,
                    max_steps INTEGER NOT NULL DEFAULT 100,
                    max_tool_calls INTEGER NOT NULL DEFAULT 100,
                    max_parallel_tool_calls INTEGER NOT NULL DEFAULT 4,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    PRIMARY KEY (workspace_code, agent_code)
                );",
            )
            .expect("legacy schema should be created");
    }

    let _repository = SqliteAgentManagementRepository::connect(
        database_path
            .to_str()
            .expect("temporary database path should be UTF-8"),
    )
    .expect("repository should migrate legacy internal-agent schema");

    let verification_connection =
        Connection::open(&database_path).expect("verification sqlite database should open");
    let mut policy_statement = verification_connection
        .prepare("PRAGMA table_info(workspace_internal_agent_policies)")
        .expect("policy table info query should prepare");
    let policy_columns = policy_statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("policy table info query should execute")
        .collect::<Result<Vec<_>, _>>()
        .expect("policy table info rows should decode");
    assert!(
        policy_columns
            .iter()
            .any(|column| column == "mcp_access_mode"),
        "mcp_access_mode column should be added for legacy schemas"
    );

    let mut assignment_statement = verification_connection
        .prepare("PRAGMA table_info(workspace_internal_agent_mcp_server_assignments)")
        .expect("assignment table info query should prepare");
    let assignment_columns = assignment_statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("assignment table info query should execute")
        .collect::<Result<Vec<_>, _>>()
        .expect("assignment table info rows should decode");
    assert_eq!(
        assignment_columns,
        vec![
            "workspace_code".to_string(),
            "agent_code".to_string(),
            "server_code".to_string(),
            "created_at".to_string(),
            "updated_at".to_string(),
        ]
    );

    let _ = fs::remove_file(database_path);
}

fn temp_database_path(prefix: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current system time should be after unix epoch")
        .as_millis();
    std::env::temp_dir().join(format!("{prefix}-{}-{millis}.sqlite3", std::process::id()))
}

#[test]
fn tool_permissions_persist_and_failed_full_replace_rolls_back() {
    use crate::application::agent::AgentManagementRepository;
    use tron_agent_harness::{ToolPermission, ToolPermissionSetting, ToolRisk};
    let path = temp_database_path("tool-permissions-transaction");
    let mut repository = SqliteAgentManagementRepository::connect(path.to_str().unwrap()).unwrap();
    let original = ToolPermissionSetting {
        tool_code: "filesystem.write_text".into(),
        permission: ToolPermission::Ask,
        confirm_level: ToolRisk::Medium,
    };
    repository
        .replace_tool_permissions(std::slice::from_ref(&original))
        .unwrap();
    let mut replacement = original.clone();
    replacement.permission = ToolPermission::No;
    assert!(
        repository
            .replace_tool_permissions(&[replacement.clone(), replacement])
            .is_err()
    );
    assert_eq!(
        repository.list_tool_permissions().unwrap(),
        vec![original.clone()]
    );
    drop(repository);
    let mut restored = SqliteAgentManagementRepository::connect(path.to_str().unwrap()).unwrap();
    assert_eq!(restored.list_tool_permissions().unwrap(), vec![original]);
    let _ = fs::remove_file(path);
}
