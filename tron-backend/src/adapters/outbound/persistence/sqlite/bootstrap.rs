use rusqlite::Connection;

use super::{
    migrations::BOOTSTRAP_PLATFORM_ADMIN_MIGRATION, storage_mapping::ensure_database_parent,
};

pub fn bootstrap_platform_admin(database_url: &str) -> Result<(), rusqlite::Error> {
    if database_url == ":memory:" {
        return Ok(());
    }

    let _ = ensure_database_parent(database_url);
    let connection = if database_url.starts_with("file:") {
        Connection::open_with_flags(
            database_url,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
    } else {
        Connection::open(database_url)
    }?;

    connection.execute_batch(BOOTSTRAP_PLATFORM_ADMIN_MIGRATION)
}
