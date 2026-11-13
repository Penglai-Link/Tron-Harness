use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    application::directory::DirectoryRepository,
    domain::directory::{Directory, DirectoryError},
};

use super::framework_storage::{SharedDatabase, business_transaction};

pub struct SqliteDirectoryRepository {
    connection: SharedDatabase,
}

impl SqliteDirectoryRepository {
    pub fn connect(database_url: &str) -> Result<Self, DirectoryError> {
        let connection =
            SharedDatabase::open(database_url).map_err(|_| DirectoryError::StorageFailure)?;
        connection.call(
            |connection| {
                super::schema::initialize_business(connection)
                    .map_err(|_| DirectoryError::StorageFailure)
            },
            || DirectoryError::StorageFailure,
        )?;
        Ok(Self { connection })
    }
}

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Directory> {
    Ok(Directory {
        directory_code: row.get(0)?,
        workspace_code: row.get(1)?,
        execution_host: row.get(2)?,
        canonical_path: row.get(3)?,
        display_name: row.get(4)?,
        archived: row.get::<_, i64>(5)? != 0,
    })
}

fn get(connection: &Connection, code: &str) -> Result<Directory, DirectoryError> {
    connection.query_row("SELECT directory_code, workspace_code, execution_host, canonical_path, display_name, archived FROM directories WHERE directory_code=?1 AND archived=0", [code], row)
        .optional().map_err(|_| DirectoryError::StorageFailure)?.ok_or(DirectoryError::NotFound)
}

impl DirectoryRepository for SqliteDirectoryRepository {
    fn list(&self, workspace_code: &str) -> Result<Vec<Directory>, DirectoryError> {
        let workspace_code = workspace_code.to_owned();
        self.connection.call_read(move |connection| {
            let mut statement = connection.prepare("SELECT directory_code, workspace_code, execution_host, canonical_path, display_name, archived FROM directories WHERE workspace_code=?1 AND archived=0").map_err(|_| DirectoryError::StorageFailure)?;
            statement.query_map([workspace_code], row).map_err(|_| DirectoryError::StorageFailure)?
                .collect::<Result<Vec<_>, _>>().map_err(|_| DirectoryError::StorageFailure)
        }, || DirectoryError::StorageFailure)
    }

    fn get(&self, directory_code: &str) -> Result<Directory, DirectoryError> {
        let code = directory_code.to_owned();
        self.connection.call_read(
            move |connection| get(connection, &code),
            || DirectoryError::StorageFailure,
        )
    }

    fn register(&self, directory: &Directory) -> Result<Directory, DirectoryError> {
        let directory = directory.clone();
        self.connection.call(move |connection| {
            let transaction = business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(|_| DirectoryError::StorageFailure)?;
            transaction.execute("INSERT INTO directories (directory_code,workspace_code,execution_host,canonical_path,display_name,archived) VALUES (?1,?2,?3,?4,?5,0) ON CONFLICT(workspace_code,execution_host,canonical_path) DO UPDATE SET archived=0", params![directory.directory_code, directory.workspace_code, directory.execution_host, directory.canonical_path, directory.display_name]).map_err(|_| DirectoryError::StorageFailure)?;
            let result = transaction.query_row("SELECT directory_code,workspace_code,execution_host,canonical_path,display_name,archived FROM directories WHERE workspace_code=?1 AND execution_host=?2 AND canonical_path=?3", params![directory.workspace_code, directory.execution_host, directory.canonical_path], row).map_err(|_| DirectoryError::StorageFailure)?;
            transaction.commit().map_err(|_| DirectoryError::StorageFailure)?;
            Ok(result)
        }, || DirectoryError::StorageFailure)
    }

    fn rename(&self, directory_code: &str, name: &str) -> Result<(), DirectoryError> {
        let code = directory_code.to_owned();
        let name = name.to_owned();
        self.connection.call(move |connection| {
            let changed = connection.execute("UPDATE directories SET display_name=?2 WHERE directory_code=?1 AND archived=0", params![code, name]).map_err(|_| DirectoryError::StorageFailure)?;
            if changed == 0 { return Err(DirectoryError::NotFound); }
            Ok(())
        }, || DirectoryError::StorageFailure)
    }
}
