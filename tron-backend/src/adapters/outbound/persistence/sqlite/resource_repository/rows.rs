use rusqlite::Error as SqliteError;

use crate::{
    domain::resource::{
        ResourceError, ResourceFileRecord, ResourceKind,
    },
    shared::time::{timestamp_from_millis},
};

pub(super) fn file_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceFileRecord> {
    let kind_text: String = row.get(2)?;
    Ok(ResourceFileRecord {
        workspace_code: row.get(0)?,
        resource_code: row.get(1)?,
        kind: ResourceKind::parse(&kind_text)
            .ok_or(ResourceError::StorageFailure)
            .map_err(resource_to_sqlite_error)?,
        name: row.get(3)?,
        description: row.get(4)?,
        file_name: row.get(5)?,
        file_extension: row.get(6)?,
        byte_size: row.get::<_, i64>(7)? as u64,
        storage_locator: row.get(8)?,
        allow_inheritance: row.get::<_, i64>(9)? != 0,
        revision: row.get::<_, i64>(10)? as u64,
        created_at: timestamp_from_millis(row.get(11)?),
        updated_at: timestamp_from_millis(row.get(12)?),
    })
}

pub(super) fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn resource_to_sqlite_error(_error: ResourceError) -> SqliteError {
    SqliteError::InvalidQuery
}
