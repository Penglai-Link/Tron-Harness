use rusqlite::{OptionalExtension, params};

use crate::{
    application::connector::ConnectionRepository,
    domain::connector::{ConnectionError, StoredConnection},
};

use super::framework_storage::SharedDatabase;

pub struct SqliteConnectionRepository {
    connection: SharedDatabase,
}

impl SqliteConnectionRepository {
    pub fn connect(database_url: &str) -> Result<Self, ConnectionError> {
        let connection =
            SharedDatabase::open(database_url).map_err(|_| ConnectionError::StorageFailure)?;
        connection.call(
            |db| {
                super::schema::initialize_business(db).map_err(|_| ConnectionError::StorageFailure)
            },
            || ConnectionError::StorageFailure,
        )?;
        Ok(Self { connection })
    }
}

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredConnection> {
    Ok(StoredConnection {
        connection_code: row.get(0)?,
        workspace_code: row.get(1)?,
        owner_user_code: row.get(2)?,
        provider_id: row.get(3)?,
        name: row.get(4)?,
        endpoint: row.get(5)?,
        encrypted_credential: row.get(6)?,
    })
}

impl ConnectionRepository for SqliteConnectionRepository {
    fn list(&self, workspace_code: &str, user_code: &str) -> Result<Vec<StoredConnection>, ConnectionError> {
        let workspace_code = workspace_code.to_owned();
        let user_code = user_code.to_owned();
        self.connection.call_read(move |db| {
            let mut statement = db.prepare("SELECT connection_code,workspace_code,owner_user_code,provider_id,name,endpoint,encrypted_credential FROM connector_connections WHERE workspace_code=?1 AND owner_user_code=?2 ORDER BY name,connection_code").map_err(|_| ConnectionError::StorageFailure)?;
            statement.query_map(params![workspace_code, user_code], row).map_err(|_| ConnectionError::StorageFailure)?.collect::<Result<_, _>>().map_err(|_| ConnectionError::StorageFailure)
        }, || ConnectionError::StorageFailure)
    }

    fn get(&self, code: &str) -> Result<StoredConnection, ConnectionError> {
        let code = code.to_owned();
        self.connection.call_read(move |db| {
            db.query_row("SELECT connection_code,workspace_code,owner_user_code,provider_id,name,endpoint,encrypted_credential FROM connector_connections WHERE connection_code=?1", [code], row).optional().map_err(|_| ConnectionError::StorageFailure)?.ok_or(ConnectionError::NotFound)
        }, || ConnectionError::StorageFailure)
    }

    fn save(&self, connection: &StoredConnection) -> Result<(), ConnectionError> {
        let connection = connection.clone();
        self.connection.call(move |db| {
            db.execute("INSERT INTO connector_connections (connection_code,workspace_code,owner_user_code,provider_id,name,endpoint,encrypted_credential) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(connection_code) DO UPDATE SET name=excluded.name,endpoint=excluded.endpoint,encrypted_credential=excluded.encrypted_credential", params![connection.connection_code,connection.workspace_code,connection.owner_user_code,connection.provider_id,connection.name,connection.endpoint,connection.encrypted_credential]).map_err(|error| match error {
                rusqlite::Error::SqliteFailure(ref error, _) if error.code == rusqlite::ErrorCode::ConstraintViolation => ConnectionError::DuplicateName,
                _ => ConnectionError::StorageFailure,
            })?;
            Ok(())
        }, || ConnectionError::StorageFailure)
    }

    fn delete(&self, code: &str) -> Result<(), ConnectionError> {
        let code = code.to_owned();
        self.connection.call(
            move |db| {
                let changed = db
                    .execute(
                        "DELETE FROM connector_connections WHERE connection_code=?1",
                        [code],
                    )
                    .map_err(|_| ConnectionError::StorageFailure)?;
                if changed == 0 {
                    return Err(ConnectionError::NotFound);
                }
                Ok(())
            },
            || ConnectionError::StorageFailure,
        )
    }
}
