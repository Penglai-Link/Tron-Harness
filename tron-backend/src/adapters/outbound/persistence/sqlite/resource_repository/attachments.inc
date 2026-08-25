fn chat_session_attachment_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<ChatSessionAttachmentRecord, SqliteError> {
    Ok(ChatSessionAttachmentRecord {
        workspace_code: row.get(0)?,
        session_code: row.get(1)?,
        attachment_code: row.get(2)?,
        file_name: row.get(3)?,
        content_type: row.get(4)?,
        byte_size: row.get::<_, i64>(5)? as u64,
        storage_locator: row.get(6)?,
        created_at: timestamp_from_millis(row.get(7)?),
    })
}

fn ensure_database_parent(database_url: &str) -> Result<(), ResourceError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| ResourceError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(error: SqliteError) -> ResourceError {
    match error {
        SqliteError::SqliteFailure(code, _)
            if code.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            ResourceError::Conflict
        }
        SqliteError::QueryReturnedNoRows => ResourceError::NotFound,
        _ => ResourceError::StorageFailure,
    }
}
