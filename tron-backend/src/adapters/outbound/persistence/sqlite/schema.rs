//! Tron 全新数据库初始化。两个数据库共用 SQL 定义，各自只执行所属区段。
use rusqlite::{Connection, TransactionBehavior, params};

use super::framework_storage::business_transaction;

const INITIAL_SCHEMA: &str = include_str!("../../../../../migrations/0001_init.sql");
const PROFILE_SECTION: &str = "-- tron:profiles\n";

pub(crate) fn initialize_business(connection: &Connection) -> rusqlite::Result<()> {
    let (business, _) = INITIAL_SCHEMA
        .split_once(PROFILE_SECTION)
        .expect("身份登记库 SQL 区段");
    initialize(connection, "business", business)
}

pub(crate) fn initialize_profiles(connection: &Connection) -> rusqlite::Result<()> {
    let (_, profiles) = INITIAL_SCHEMA
        .split_once(PROFILE_SECTION)
        .expect("身份登记库 SQL 区段");
    initialize(connection, "profiles", profiles)
}

fn initialize(connection: &Connection, scope: &str, sql: &str) -> rusqlite::Result<()> {
    let transaction = business_transaction(connection, TransactionBehavior::Immediate)?;
    let has_marker: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'tron_schema_initialized')",
        [], |row| row.get(0),
    )?;
    let initialized = has_marker
        && transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM tron_schema_initialized WHERE scope = ?1)",
            params![scope],
            |row| row.get::<_, bool>(0),
        )?;
    if !initialized {
        transaction.execute_batch(sql)?;
    }
    transaction.commit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(connection: &Connection, name: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [name],
                |row| row.get(0),
            )
            .unwrap()
    }

    #[test]
    fn fresh_business_schema_contains_current_columns_constraints_and_catalogs() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        initialize_business(&connection).unwrap();
        for (table, column) in [
            ("tasks", "flow_code"),
            ("models", "default_reasoning_effort"),
            ("workspace_internal_agent_definitions", "group_name"),
            ("workspace_internal_agent_definitions", "avatar_id"),
            ("resource_server_accounts", "server_workspace_code"),
            ("provider_catalog_models", "quality_type"),
            ("navigation_menu_tree_items", "child_display"),
        ] {
            assert!(
                connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name=?2)",
                        params![table, column],
                        |row| row.get::<_, bool>(0),
                    )
                    .unwrap(),
                "missing {table}.{column}"
            );
        }
        let foreign_key_errors = connection
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query_map([], |_| Ok(()))
            .unwrap()
            .count();
        assert_eq!(foreign_key_errors, 0);
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM task_flows WHERE flow_code='DEFAULT'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM rag_corpora", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
        for table in [
            "navigation_domain_menu_items",
            "navigation_migration_markers",
            "resource_management_migrations",
            "resource_management_migration_issues",
            "rag_schema_version",
        ] {
            assert!(!table_exists(&connection, table), "obsolete table {table}");
        }
    }

    #[test]
    fn agent_loop_references_require_a_persisted_design() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_business(&connection).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM agent_loop_designs", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        connection
            .execute_batch(
                "INSERT INTO workspace_internal_agent_definitions
                 (workspace_code, agent_code, created_at, updated_at)
                 VALUES ('WS', 'WORKSPACE', 0, 0);
             INSERT INTO platform_internal_agent_configs VALUES ('PLATFORM', '{}');",
            )
            .unwrap();
        for loop_code in [
            "builtin.react-loop",
            "builtin.review-workflow",
            "builtin.analysis-dag",
            "missing-loop",
        ] {
            let config = serde_json::json!({ "loopCode": loop_code }).to_string();
            for (sql, value) in [
                (
                    "INSERT INTO workspace_internal_agent_definitions
                    (workspace_code, agent_code, created_at, updated_at, loop_code)
                    VALUES ('WS', 'INVALID', 0, 0, ?1)",
                    loop_code,
                ),
                (
                    "UPDATE workspace_internal_agent_definitions SET loop_code=?1
                    WHERE agent_code='WORKSPACE'",
                    loop_code,
                ),
                (
                    "INSERT INTO platform_internal_agent_configs VALUES ('INVALID', ?1)",
                    config.as_str(),
                ),
                (
                    "UPDATE platform_internal_agent_configs SET config_json=?1
                    WHERE agent_code='PLATFORM'",
                    config.as_str(),
                ),
            ] {
                let error = connection.execute(sql, [value]).unwrap_err();
                assert!(error.to_string().contains("unknown agent loop"), "{error}");
            }
        }
        connection
            .execute_batch(
                "INSERT INTO agent_loop_designs VALUES ('custom-loop', 1, '{}');
             UPDATE workspace_internal_agent_definitions SET loop_code='custom-loop';
             UPDATE platform_internal_agent_configs
                 SET config_json='{\"loopCode\":\"custom-loop\"}';",
            )
            .unwrap();
    }

    #[test]
    fn profile_registry_and_business_databases_initialize_separately() {
        let business = Connection::open_in_memory().unwrap();
        let profiles = Connection::open_in_memory().unwrap();
        initialize_business(&business).unwrap();
        initialize_profiles(&profiles).unwrap();
        assert!(table_exists(&business, "tasks"));
        assert!(!table_exists(&business, "profiles"));
        assert!(table_exists(&profiles, "profiles"));
        assert!(table_exists(&profiles, "profile_avatar_cache"));
        assert!(!table_exists(&profiles, "tasks"));
        initialize_profiles(&profiles).unwrap();
    }

    #[test]
    fn reopening_preserves_user_changes_to_builtin_catalogs() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_business(&connection).unwrap();
        connection
            .execute("UPDATE navigation_menus SET enabled=0", [])
            .unwrap();
        connection
            .execute("UPDATE task_flow_statuses SET name='Custom status'", [])
            .unwrap();
        initialize_business(&connection).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM navigation_menus WHERE enabled<>0",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM task_flow_statuses WHERE name<>'Custom status'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn initialization_failure_rolls_back_partial_schema() {
        let connection = Connection::open_in_memory().unwrap();
        assert!(
            initialize(
                &connection,
                "business",
                "CREATE TABLE probe(id INTEGER); INSERT INTO missing_table VALUES (1);"
            )
            .is_err()
        );
        assert!(!table_exists(&connection, "probe"));
        initialize_business(&connection).unwrap();
    }

    #[test]
    fn initialization_joins_an_existing_transaction_without_committing_it() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("BEGIN").unwrap();
        initialize_business(&connection).unwrap();
        assert!(!connection.is_autocommit());
        connection.execute_batch("ROLLBACK").unwrap();
        assert!(!table_exists(&connection, "tasks"));
        assert!(!table_exists(&connection, "tron_schema_initialized"));
    }

    #[test]
    fn concurrent_connections_initialize_the_database_once() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("business.sqlite3");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        std::thread::scope(|scope| {
            let mut workers = Vec::new();
            for _ in 0..2 {
                let path = &path;
                let barrier = barrier.clone();
                workers.push(scope.spawn(move || {
                    let connection = Connection::open(path).unwrap();
                    connection
                        .busy_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                    barrier.wait();
                    initialize_business(&connection).unwrap();
                }));
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        let connection = Connection::open(path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM tron_schema_initialized", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            1
        );
    }
}
