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
