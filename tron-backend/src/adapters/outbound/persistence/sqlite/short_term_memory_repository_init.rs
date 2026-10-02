use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::domain::short_term_memory::ShortTermMemoryError;

use super::super::{
    migrations::{
        CHAT_MANAGEMENT_MIGRATION, CHAT_MESSAGE_TOKEN_USAGE_MIGRATION,
        CHAT_SESSION_AGENT_CODE_MIGRATION, CHAT_SESSION_LEGACY_SOURCE_MIGRATION,
        CHAT_SESSION_SYSTEM_AGENT_SESSION_ID_MIGRATION, CHAT_SESSION_VISIBILITY_MIGRATION,
        SHORT_TERM_MEMORY_TRANSCRIPT_MIGRATION,
    },
    short_term_memory_rows::{
        column_exists, ensure_database_parent, index_exists, storage_error, table_exists,
    },
};

#[path = "short_term_memory_repository_init_backfills_core.rs"]
mod repository_init_backfills_core;
#[path = "short_term_memory_repository_init_backfills_transcript.rs"]
mod repository_init_backfills_transcript;
#[path = "short_term_memory_repository_init_repair.rs"]
mod repository_init_repair;
#[path = "short_term_memory_repository_init_schema.rs"]
mod repository_init_schema;

use repository_init_backfills_core::STM_CORE_COLUMN_BACKFILLS;
use repository_init_backfills_transcript::STM_TRANSCRIPT_COLUMN_BACKFILLS;
use repository_init_repair::repair_transcript_backfilled_ownership;
use repository_init_schema::{
    STM_ALLOWED_VALUES, STM_JSON_COLUMNS, STM_ORPHAN_CHECKS, STM_REQUIRED_COLUMN_DEFINITIONS,
    STM_REQUIRED_COLUMNS, STM_REQUIRED_INDEX_DEFINITIONS, STM_REQUIRED_INDEXES, StmAllowedValues,
    StmJsonColumn, StmOrphanCheck, StmRequiredColumnDefinition, StmRequiredIndexDefinition,
};
pub(super) fn open_initialized_connection(
    database_url: &str,
) -> Result<Connection, ShortTermMemoryError> {
    ensure_database_parent(database_url)?;
    let mut connection = if database_url == ":memory:" || database_url.starts_with("file:") {
        Connection::open_with_flags(
            database_url,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
    } else {
        Connection::open(database_url)
    }
    .map_err(storage_error)?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    run_chat_schema_migrations(&tx)?;
    run_short_term_memory_schema_backfills(&tx)?;
    repair_short_term_memory_backfilled_data(&tx)?;
    migrate_family_snapshot_ownership_to_session(&tx)?;
    tx.execute_batch(SHORT_TERM_MEMORY_TRANSCRIPT_MIGRATION)
        .map_err(storage_error)?;
    repair_run_lease_backfilled_data(&tx)?;
    validate_short_term_memory_schema(&tx)?;
    tx.commit().map_err(storage_error)?;
    Ok(connection)
}

pub(super) fn next_table_id(
    connection: &Connection,
    table: &str,
) -> Result<i64, ShortTermMemoryError> {
    let max_id = connection
        .query_row(&format!("SELECT max(id) FROM {table}"), [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .map_err(storage_error)?
        .unwrap_or(735_900_000_000_000_000);
    Ok(max_id + 1)
}

fn run_chat_schema_migrations(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    connection
        .execute_batch(CHAT_MANAGEMENT_MIGRATION)
        .map_err(storage_error)?;
    if !column_exists(connection, "chat_sessions", "agent_code")? {
        connection
            .execute_batch(CHAT_SESSION_AGENT_CODE_MIGRATION)
            .map_err(storage_error)?;
    }
    if !column_exists(connection, "chat_sessions", "system_agent_session_id")? {
        connection
            .execute_batch(CHAT_SESSION_SYSTEM_AGENT_SESSION_ID_MIGRATION)
            .map_err(storage_error)?;
    }
    if !column_exists(connection, "chat_sessions", "source")? {
        connection
            .execute_batch(CHAT_SESSION_LEGACY_SOURCE_MIGRATION)
            .map_err(storage_error)?;
    }
    if !column_exists(connection, "chat_sessions", "visibility")? {
        connection
            .execute_batch(CHAT_SESSION_VISIBILITY_MIGRATION)
            .map_err(storage_error)?;
    }
    if !column_exists(connection, "chat_messages", "input_tokens")? {
        connection
            .execute_batch(CHAT_MESSAGE_TOKEN_USAGE_MIGRATION)
            .map_err(storage_error)?;
    }
    Ok(())
}

fn run_short_term_memory_schema_backfills(
    connection: &Connection,
) -> Result<(), ShortTermMemoryError> {
    for backfills in [STM_CORE_COLUMN_BACKFILLS, STM_TRANSCRIPT_COLUMN_BACKFILLS] {
        for backfill in backfills {
            if table_exists(connection, backfill.table)?
                && !column_exists(connection, backfill.table, backfill.column)?
            {
                connection
                    .execute_batch(&format!(
                        "ALTER TABLE {} ADD COLUMN {};",
                        backfill.table, backfill.ddl
                    ))
                    .map_err(storage_error)?;
            }
        }
    }
    Ok(())
}

fn migrate_family_snapshot_ownership_to_session(
    connection: &Connection,
) -> Result<(), ShortTermMemoryError> {
    const TABLE: &str = "agent_session_family_snapshots";
    const LEGACY_TABLE: &str = "agent_session_family_snapshots_legacy_family_owned";
    if !table_exists(connection, TABLE)? {
        return Ok(());
    }
    let family_code_not_null = connection
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('agent_session_family_snapshots') WHERE name = 'family_code'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(storage_error)?
        .unwrap_or(0)
        != 0;
    if !family_code_not_null {
        return Ok(());
    }
    if table_exists(connection, LEGACY_TABLE)? {
        return Err(ShortTermMemoryError::StorageFailure);
    }
    connection
        .execute_batch(
            "ALTER TABLE agent_session_family_snapshots
                 RENAME TO agent_session_family_snapshots_legacy_family_owned;

             CREATE TABLE agent_session_family_snapshots (
                 id INTEGER PRIMARY KEY,
                 snapshot_code TEXT NOT NULL UNIQUE,
                 family_code TEXT,
                 session_code TEXT NOT NULL,
                 run_code TEXT,
                 snapshot_kind TEXT NOT NULL,
                 status TEXT NOT NULL,
                 source_start_seq INTEGER,
                 source_end_seq INTEGER,
                 source_checkpoint_message_code TEXT,
                 summary_markdown TEXT,
                 token_estimate INTEGER,
                 error_code TEXT,
                 error_message TEXT,
                 metadata_json TEXT,
                 created_at INTEGER NOT NULL,
                 completed_at INTEGER,
                 expires_at INTEGER
             );

             INSERT INTO agent_session_family_snapshots (
                 id, snapshot_code, family_code, session_code, run_code,
                 snapshot_kind, status, source_start_seq, source_end_seq,
                 source_checkpoint_message_code, summary_markdown, token_estimate,
                 error_code, error_message, metadata_json, created_at,
                 completed_at, expires_at
             )
             SELECT id, snapshot_code, family_code, session_code, run_code,
                    snapshot_kind, status, source_start_seq, source_end_seq,
                    source_checkpoint_message_code, summary_markdown, token_estimate,
                    error_code, error_message, metadata_json, created_at,
                    completed_at, expires_at
             FROM agent_session_family_snapshots_legacy_family_owned;

             DROP TABLE agent_session_family_snapshots_legacy_family_owned;",
        )
        .map_err(storage_error)
}

fn repair_short_term_memory_backfilled_data(
    connection: &Connection,
) -> Result<(), ShortTermMemoryError> {
    repair_transcript_backfilled_ownership(connection)?;
    backfill_context_epoch_revisions(connection)?;
    backfill_goal_event_sequences(connection)?;
    Ok(())
}

fn repair_run_lease_backfilled_data(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    backfill_active_run_leases(connection)?;
    Ok(())
}

fn backfill_context_epoch_revisions(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    if !table_exists(connection, "agent_transcript_context_epochs")?
        || !column_exists(connection, "agent_transcript_context_epochs", "revision")?
    {
        return Ok(());
    }
    connection
        .execute_batch(
            "WITH ordered_epochs AS (
                 SELECT id,
                        ROW_NUMBER() OVER (
                            PARTITION BY session_code
                            ORDER BY created_at ASC, id ASC
                        ) AS repaired_revision
                 FROM agent_transcript_context_epochs
             )
             UPDATE agent_transcript_context_epochs
             SET revision = (
                 SELECT repaired_revision
                 FROM ordered_epochs
                 WHERE ordered_epochs.id = agent_transcript_context_epochs.id
             )
             WHERE id IN (SELECT id FROM ordered_epochs);",
        )
        .map_err(storage_error)?;
    Ok(())
}

fn backfill_goal_event_sequences(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    if !table_exists(connection, "agent_session_goal_events")?
        || !column_exists(connection, "agent_session_goal_events", "event_seq")?
    {
        return Ok(());
    }
    let has_only_backfilled_defaults = connection
        .query_row(
            "SELECT count(1) > 0
                    AND sum(CASE WHEN event_seq <> 0 THEN 1 ELSE 0 END) = 0
             FROM agent_session_goal_events",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(storage_error)?;
    if !has_only_backfilled_defaults {
        return Ok(());
    }
    connection
        .execute_batch(
            "WITH ordered_events AS (
                 SELECT id,
                        ROW_NUMBER() OVER (
                            PARTITION BY goal_code
                            ORDER BY created_at ASC, id ASC
                        ) AS repaired_seq
                 FROM agent_session_goal_events
             )
             UPDATE agent_session_goal_events
             SET event_seq = (
                 SELECT repaired_seq
                 FROM ordered_events
                 WHERE ordered_events.id = agent_session_goal_events.id
             )
             WHERE id IN (SELECT id FROM ordered_events);",
        )
        .map_err(storage_error)?;
    Ok(())
}

fn backfill_active_run_leases(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    if !table_exists(connection, "agent_session_run_leases")? {
        return Ok(());
    }
    connection
        .execute_batch(
            "WITH next_base AS (
                 SELECT COALESCE(MAX(id), 735910000000000000) AS base_id
                 FROM agent_session_run_leases
             ),
             open_runs AS (
                 SELECT
                     ROW_NUMBER() OVER (ORDER BY started.created_at ASC, started.id ASC) AS row_number,
                     started.session_code,
                     started.workspace_code,
                     started.run_code,
                     started.input_code,
                     started.created_at
                 FROM agent_transcript_events started
                 WHERE started.event_type = 'agent.run.started'
                   AND started.run_code IS NOT NULL
                   AND NOT EXISTS (
                       SELECT 1
                       FROM agent_transcript_events ended
                       WHERE ended.session_code = started.session_code
                         AND ended.run_code = started.run_code
                         AND ended.event_type IN ('agent.run.completed', 'agent.run.awaiting_input', 'agent.run.failed', 'agent.run.recovered')
                   )
                   AND NOT EXISTS (
                       SELECT 1
                       FROM agent_session_run_leases lease
                       WHERE lease.session_code = started.session_code
                          OR lease.run_code = started.run_code
                   )
             )
             INSERT INTO agent_session_run_leases
             (id, session_code, workspace_code, run_code, input_code, started_at, updated_at)
             SELECT
                 next_base.base_id + open_runs.row_number,
                 open_runs.session_code,
                 open_runs.workspace_code,
                 open_runs.run_code,
                 open_runs.input_code,
                 open_runs.created_at,
                 open_runs.created_at
             FROM open_runs, next_base;",
        )
        .map_err(storage_error)?;
    Ok(())
}

fn validate_short_term_memory_schema(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    for (table, columns) in STM_REQUIRED_COLUMNS {
        if !table_exists(connection, table)? {
            return Err(ShortTermMemoryError::StorageFailure);
        }
        for column in *columns {
            if !column_exists(connection, table, column)? {
                return Err(ShortTermMemoryError::StorageFailure);
            }
        }
    }
    for index in STM_REQUIRED_INDEXES {
        if !index_exists(connection, index)? {
            return Err(ShortTermMemoryError::StorageFailure);
        }
    }
    for column in STM_REQUIRED_COLUMN_DEFINITIONS {
        validate_column_definition(connection, column)?;
    }
    for index in STM_REQUIRED_INDEX_DEFINITIONS {
        validate_index_definition(connection, index)?;
    }
    for json_column in STM_JSON_COLUMNS {
        validate_json_column(connection, json_column)?;
    }
    for allowed_values in STM_ALLOWED_VALUES {
        validate_allowed_values(connection, allowed_values)?;
    }
    for orphan_check in STM_ORPHAN_CHECKS {
        validate_no_orphans(connection, orphan_check)?;
    }
    Ok(())
}

fn validate_column_definition(
    connection: &Connection,
    expected: &StmRequiredColumnDefinition,
) -> Result<(), ShortTermMemoryError> {
    let pragma = format!("PRAGMA table_info({})", expected.table);
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let column = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? != 0,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?
        .into_iter()
        .find(|(name, _, _, _)| name == expected.column)
        .ok_or(ShortTermMemoryError::StorageFailure)?;
    if !column.1.eq_ignore_ascii_case(expected.data_type) || column.2 != expected.not_null {
        return Err(ShortTermMemoryError::StorageFailure);
    }
    let actual_default = normalize_default(column.3.as_deref());
    let default_allowed = expected
        .allowed_default_values
        .iter()
        .any(|value| normalize_default(*value) == actual_default);
    if !default_allowed {
        return Err(ShortTermMemoryError::StorageFailure);
    }
    Ok(())
}

fn validate_index_definition(
    connection: &Connection,
    expected: &StmRequiredIndexDefinition,
) -> Result<(), ShortTermMemoryError> {
    let sql = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [expected.name],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(storage_error)?
        .flatten()
        .ok_or(ShortTermMemoryError::StorageFailure)?;
    let normalized_sql = normalize_sql(&sql);
    if expected.unique && !normalized_sql.starts_with("create unique index") {
        return Err(ShortTermMemoryError::StorageFailure);
    }
    for fragment in expected.fragments {
        if !normalized_sql.contains(&normalize_sql(fragment)) {
            return Err(ShortTermMemoryError::StorageFailure);
        }
    }
    Ok(())
}

fn validate_json_column(
    connection: &Connection,
    column: &StmJsonColumn,
) -> Result<(), ShortTermMemoryError> {
    let sql = format!(
        "SELECT {column_name} FROM {table_name} WHERE {column_name} IS NOT NULL",
        column_name = column.column,
        table_name = column.table,
    );
    let mut statement = connection.prepare(&sql).map_err(storage_error)?;
    let values = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    for value in values {
        serde_json::from_str::<serde_json::Value>(&value)
            .map_err(|_| ShortTermMemoryError::StorageFailure)?;
    }
    Ok(())
}

fn validate_allowed_values(
    connection: &Connection,
    allowed: &StmAllowedValues,
) -> Result<(), ShortTermMemoryError> {
    let sql = format!(
        "SELECT DISTINCT {column_name} FROM {table_name} WHERE {column_name} IS NOT NULL",
        column_name = allowed.column,
        table_name = allowed.table,
    );
    let mut statement = connection.prepare(&sql).map_err(storage_error)?;
    let values = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    for value in values {
        if !allowed.values.contains(&value.as_str()) {
            return Err(ShortTermMemoryError::StorageFailure);
        }
    }
    Ok(())
}

fn validate_no_orphans(
    connection: &Connection,
    check: &StmOrphanCheck,
) -> Result<(), ShortTermMemoryError> {
    let orphan_count = connection
        .query_row(check.sql, [], |row| row.get::<_, i64>(0))
        .map_err(storage_error)?;
    if orphan_count > 0 {
        return Err(ShortTermMemoryError::StorageFailure);
    }
    Ok(())
}

fn normalize_sql(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn normalize_default(value: Option<&str>) -> Option<String> {
    value.map(|item| {
        item.trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim_matches('\'')
            .trim_matches('"')
            .to_ascii_lowercase()
    })
}
