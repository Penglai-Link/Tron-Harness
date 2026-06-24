use rusqlite::{Connection, params};

use crate::domain::short_term_memory::ShortTermMemoryError;

use super::super::short_term_memory_rows::storage_error;

pub(super) fn list_family_session_codes(
    connection: &Connection,
    family_code: &str,
    exclude_session_code: &str,
    limit: usize,
) -> Result<Vec<String>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT session_code
             FROM agent_session_family_members
             WHERE family_code = ?1
               AND session_code != ?2
               AND model_context_enabled = 1
             ORDER BY last_activity_at DESC, id DESC
             LIMIT ?3",
        )
        .map_err(storage_error)?;
    statement
        .query_map(
            params![family_code, exclude_session_code, limit as i64],
            |row| row.get(0),
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}
