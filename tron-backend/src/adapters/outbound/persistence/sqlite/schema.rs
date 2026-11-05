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
    } else if scope == "business" {
        refresh_empty_server_account_schema(&transaction, sql)?;
    }
    if scope == "business" {
        refresh_builtin_run_count_defaults(&transaction)?;
    }
    transaction.commit()
}

// Replace the old built-in defaults once. A later user-selected limit, including
// 100, must survive subsequent startups; custom agents and other limits stay put.
fn refresh_builtin_run_count_defaults(connection: &Connection) -> rusqlite::Result<()> {
    const MARKER: &str = "builtin-unlimited-run-counts";
    let initialized: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM tron_schema_initialized WHERE scope = ?1)",
        [MARKER],
        |row| row.get(0),
    )?;
    if initialized {
        return Ok(());
    }
    connection.execute_batch(
        "UPDATE platform_internal_agent_configs
         SET config_json = json_set(config_json,
             '$.maxSteps', CASE WHEN json_extract(config_json, '$.maxSteps') = 100
                 THEN 0 ELSE json_extract(config_json, '$.maxSteps') END,
             '$.maxToolCalls', CASE WHEN json_extract(config_json, '$.maxToolCalls') = 100
                 THEN 0 ELSE json_extract(config_json, '$.maxToolCalls') END)
         WHERE agent_code IN ('main-agent', 'explore-agent', 'prd-agent', 'main-dude', 'memory-dude')
           AND (json_extract(config_json, '$.maxSteps') = 100
             OR json_extract(config_json, '$.maxToolCalls') = 100);",
    )?;
    connection.execute(
        "INSERT INTO tron_schema_initialized (scope) VALUES (?1)",
        [MARKER],
    )?;
    Ok(())
}

// Development databases may have the pre-credential account table while the
// initial schema has already changed. Recreate only an empty obsolete table;
// never discard accounts or invent encrypted credentials for existing records.
fn refresh_empty_server_account_schema(connection: &Connection, sql: &str) -> rusqlite::Result<()> {
    let current: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('resource_server_accounts') WHERE name = 'encrypted_credentials')",
        [], |row| row.get(0),
    )?;
    if current {
        return Ok(());
    }
    let populated: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM resource_server_accounts)",
        [],
        |row| row.get(0),
    )?;
    if populated {
        tracing::error!(
            "server account schema is obsolete and contains accounts; refusing to discard credentials"
        );
        return Err(rusqlite::Error::InvalidQuery);
    }
    let start = sql
        .find("CREATE TABLE resource_server_accounts (")
        .expect("server account schema");
    let end = sql[start..]
        .find("CREATE TABLE resource_servers (")
        .expect("resource server schema")
        + start;
    connection.execute_batch("DROP TABLE IF EXISTS resource_server_account_bindings; DROP TABLE resource_server_accounts;")?;
    connection.execute_batch(&sql[start..end])?;
    tracing::info!("refreshed empty server account tables from the initial schema");
    Ok(())
}
