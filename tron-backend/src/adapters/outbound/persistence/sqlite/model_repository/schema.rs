use super::*;

pub(super) fn ensure_database_parent(database_url: &str) -> Result<(), ModelError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| ModelError::StorageFailure)?;
    }
    Ok(())
}

pub(super) fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, ModelError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|value| value == column_name))
}
