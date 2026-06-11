use std::{fs, path::Path};

use rusqlite::{Connection, ErrorCode, OptionalExtension, params};

use crate::{
    application::open_app::OpenAppRepository,
    domain::open_app::{
        OpenApiNonceRecord, OpenApp, OpenAppAuthState, OpenAppCapabilityGrant, OpenAppCredential,
        OpenAppCredentialStatus, OpenAppError, OpenAppUserGrant,
        open_app_credential_status_from_storage, open_app_credential_status_to_storage,
        open_app_status_from_storage, open_app_status_to_storage,
    },
    shared::{
        id::SnowflakeId,
        time::{
            option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
            timestamp_to_millis,
        },
    },
};

use super::migrations::OPEN_APPS_MIGRATION;

pub struct SqliteOpenAppRepository {
    connection: Connection,
}

impl SqliteOpenAppRepository {
    pub fn connect(database_url: &str) -> Result<Self, OpenAppError> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
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

        let mut repository = Self { connection };
        repository.apply_migrations()?;
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), OpenAppError> {
        self.connection
            .execute_batch(OPEN_APPS_MIGRATION)
            .map_err(storage_error)?;
        self.migrate_legacy_open_app_status_schema()
    }

    fn migrate_legacy_open_app_status_schema(&mut self) -> Result<(), OpenAppError> {
        let schema = self
            .connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table' AND name = 'platform_open_apps'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_error)?;
        let Some(schema) = schema else {
            return Ok(());
        };

        if schema.contains("'enabled'") && schema.contains("'archived'") {
            return Ok(());
        }

        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute("ALTER TABLE platform_open_apps RENAME TO platform_open_apps_legacy", [])
            .map_err(storage_error)?;
        tx.execute_batch(
            "CREATE TABLE platform_open_apps (
                id INTEGER PRIMARY KEY,
                open_app_code TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'enabled',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_rotated_at INTEGER,
                CHECK (status IN ('enabled', 'disabled', 'archived'))
            );",
        )
        .map_err(storage_error)?;
        tx.execute(
            "INSERT INTO platform_open_apps (id, open_app_code, name, description, status, created_at, updated_at, last_rotated_at)
             SELECT
                 id,
                 open_app_code,
                 name,
                 description,
                 CASE status
                     WHEN 'active' THEN 'enabled'
                     WHEN 'disabled' THEN 'disabled'
                     WHEN 'archived' THEN 'archived'
                     ELSE 'disabled'
                 END,
                 created_at,
                 updated_at,
                 last_rotated_at
             FROM platform_open_apps_legacy",
            [],
        )
        .map_err(storage_error)?;
        tx.execute("DROP TABLE platform_open_apps_legacy", [])
            .map_err(storage_error)?;
        tx.commit().map_err(storage_error)
    }

    fn map_open_app_row(&self, row: &rusqlite::Row<'_>) -> Result<OpenApp, rusqlite::Error> {
        let status = row.get::<_, String>(4)?;
        Ok(OpenApp {
            id: SnowflakeId::new(row.get(0)?),
            open_app_code: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            status: open_app_status_from_storage(status.as_str())
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
            created_at: timestamp_from_millis(row.get(5)?),
            updated_at: timestamp_from_millis(row.get(6)?),
            last_rotated_at: option_timestamp_from_millis(row.get(7)?),
        })
    }

    fn map_credential_row(
        &self,
        row: &rusqlite::Row<'_>,
    ) -> Result<OpenAppCredential, rusqlite::Error> {
        let status = row.get::<_, String>(3)?;
        Ok(OpenAppCredential {
            id: SnowflakeId::new(row.get(0)?),
            credential_code: row.get(1)?,
            open_app_code: row.get(2)?,
            status: open_app_credential_status_from_storage(status.as_str())
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
            secret_ciphertext: row.get(4)?,
            secret_fingerprint: row.get(5)?,
            created_at: timestamp_from_millis(row.get(6)?),
            revoked_at: option_timestamp_from_millis(row.get(7)?),
        })
    }

    fn load_auth_state_from_open_app(
        &self,
        open_app: OpenApp,
    ) -> Result<OpenAppAuthState, OpenAppError> {
        let active_credential = self
            .connection
            .query_row(
                "SELECT id, credential_code, open_app_code, status, secret_ciphertext, secret_fingerprint, created_at, revoked_at
                 FROM platform_open_app_credentials
                 WHERE open_app_code = ?1
                   AND status = 'active'
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![open_app.open_app_code.clone()],
                |row| self.map_credential_row(row),
            )
            .optional()
            .map_err(storage_error)?;

        let user_grants = self
            .connection
            .prepare(
                "SELECT id, grant_code, open_app_code, user_code, created_at
                 FROM platform_open_app_user_grants
                 WHERE open_app_code = ?1
                 ORDER BY user_code ASC",
            )
            .map_err(storage_error)?
            .query_map(params![open_app.open_app_code.clone()], |row| {
                Ok(OpenAppUserGrant {
                    id: SnowflakeId::new(row.get(0)?),
                    grant_code: row.get(1)?,
                    open_app_code: row.get(2)?,
                    user_code: row.get(3)?,
                    created_at: timestamp_from_millis(row.get(4)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        let capability_grants = self
            .connection
            .prepare(
                "SELECT id, grant_code, open_app_code, capability_code, operation_code, created_at
                 FROM platform_open_app_capability_grants
                 WHERE open_app_code = ?1
                 ORDER BY capability_code ASC, operation_code ASC",
            )
            .map_err(storage_error)?
            .query_map(params![open_app.open_app_code.clone()], |row| {
                Ok(OpenAppCapabilityGrant {
                    id: SnowflakeId::new(row.get(0)?),
                    grant_code: row.get(1)?,
                    open_app_code: row.get(2)?,
                    capability_code: row.get(3)?,
                    operation_code: row.get(4)?,
                    created_at: timestamp_from_millis(row.get(5)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        Ok(OpenAppAuthState {
            open_app,
            active_credential,
            user_grants,
            capability_grants,
        })
    }
}

impl OpenAppRepository for SqliteOpenAppRepository {
    fn next_id(&mut self) -> Result<i64, OpenAppError> {
        let max_id = self
            .connection
            .query_row(
                "SELECT max(id) FROM (
                    SELECT id FROM platform_open_apps
                    UNION ALL SELECT id FROM platform_open_app_credentials
                    UNION ALL SELECT id FROM platform_open_app_user_grants
                    UNION ALL SELECT id FROM platform_open_app_capability_grants
                    UNION ALL SELECT id FROM platform_open_api_nonces
                )",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(734_960_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_open_app(
        &mut self,
        open_app: &OpenApp,
        credential: &OpenAppCredential,
    ) -> Result<(), OpenAppError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "INSERT INTO platform_open_apps (id, open_app_code, name, description, status, created_at, updated_at, last_rotated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                open_app.id.value(),
                open_app.open_app_code,
                open_app.name,
                open_app.description,
                open_app_status_to_storage(open_app.status),
                timestamp_to_millis(&open_app.created_at),
                timestamp_to_millis(&open_app.updated_at),
                option_timestamp_to_millis(open_app.last_rotated_at.as_deref()),
            ],
        )
        .map_err(map_create_error)?;
        tx.execute(
            "INSERT INTO platform_open_app_credentials (id, credential_code, open_app_code, status, secret_ciphertext, secret_fingerprint, created_at, revoked_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                credential.id.value(),
                credential.credential_code,
                credential.open_app_code,
                open_app_credential_status_to_storage(credential.status),
                credential.secret_ciphertext,
                credential.secret_fingerprint,
                timestamp_to_millis(&credential.created_at),
                option_timestamp_to_millis(credential.revoked_at.as_deref()),
            ],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)
    }

    fn list_open_app_auth_states(&mut self) -> Result<Vec<OpenAppAuthState>, OpenAppError> {
        let open_apps = self
            .connection
            .prepare(
                "SELECT id, open_app_code, name, description, status, created_at, updated_at, last_rotated_at
                 FROM platform_open_apps
                 ORDER BY created_at DESC, name ASC",
            )
            .map_err(storage_error)?
            .query_map([], |row| self.map_open_app_row(row))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        open_apps
            .into_iter()
            .map(|open_app| self.load_auth_state_from_open_app(open_app))
            .collect()
    }

    fn find_open_app_by_open_app_code(
        &mut self,
        open_app_code: &str,
    ) -> Result<Option<OpenApp>, OpenAppError> {
        self.connection
            .query_row(
                "SELECT id, open_app_code, name, description, status, created_at, updated_at, last_rotated_at
                 FROM platform_open_apps
                 WHERE open_app_code = ?1",
                params![open_app_code],
                |row| self.map_open_app_row(row),
            )
            .optional()
            .map_err(storage_error)
    }

    fn update_open_app(&mut self, open_app: &OpenApp) -> Result<(), OpenAppError> {
        self.connection
            .execute(
                "UPDATE platform_open_apps
                 SET name = ?2,
                     description = ?3,
                     status = ?4,
                     updated_at = ?5,
                     last_rotated_at = ?6
                 WHERE open_app_code = ?1",
                params![
                    open_app.open_app_code,
                    open_app.name,
                    open_app.description,
                    open_app_status_to_storage(open_app.status),
                    timestamp_to_millis(&open_app.updated_at),
                    option_timestamp_to_millis(open_app.last_rotated_at.as_deref()),
                ],
            )
            .map_err(map_create_error)?;
        Ok(())
    }

    fn rotate_credential(
        &mut self,
        open_app_code: &str,
        revoked_at: &str,
        credential: &OpenAppCredential,
    ) -> Result<(), OpenAppError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "UPDATE platform_open_app_credentials
             SET status = 'revoked',
                 revoked_at = ?2
             WHERE open_app_code = ?1
               AND status = 'active'",
            params![open_app_code, timestamp_to_millis(revoked_at)],
        )
        .map_err(storage_error)?;
        tx.execute(
            "INSERT INTO platform_open_app_credentials (id, credential_code, open_app_code, status, secret_ciphertext, secret_fingerprint, created_at, revoked_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                credential.id.value(),
                credential.credential_code,
                credential.open_app_code,
                open_app_credential_status_to_storage(OpenAppCredentialStatus::Active),
                credential.secret_ciphertext,
                credential.secret_fingerprint,
                timestamp_to_millis(&credential.created_at),
                option_timestamp_to_millis(credential.revoked_at.as_deref()),
            ],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)
    }

    fn load_auth_state(
        &mut self,
        open_app_code: &str,
    ) -> Result<Option<OpenAppAuthState>, OpenAppError> {
        let Some(open_app) = self.find_open_app_by_open_app_code(open_app_code)? else {
            return Ok(None);
        };
        Ok(Some(self.load_auth_state_from_open_app(open_app)?))
    }

    fn insert_user_grant(&mut self, grant: &OpenAppUserGrant) -> Result<(), OpenAppError> {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO platform_open_app_user_grants (id, grant_code, open_app_code, user_code, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    grant.id.value(),
                    grant.grant_code,
                    grant.open_app_code,
                    grant.user_code,
                    timestamp_to_millis(&grant.created_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn insert_capability_grant(
        &mut self,
        grant: &OpenAppCapabilityGrant,
    ) -> Result<(), OpenAppError> {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO platform_open_app_capability_grants (id, grant_code, open_app_code, capability_code, operation_code, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    grant.id.value(),
                    grant.grant_code,
                    grant.open_app_code,
                    grant.capability_code,
                    grant.operation_code,
                    timestamp_to_millis(&grant.created_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn replace_user_grants(
        &mut self,
        open_app_code: &str,
        grants: &[OpenAppUserGrant],
    ) -> Result<(), OpenAppError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "DELETE FROM platform_open_app_user_grants WHERE open_app_code = ?1",
            params![open_app_code],
        )
        .map_err(storage_error)?;
        for grant in grants {
            tx.execute(
                "INSERT INTO platform_open_app_user_grants (id, grant_code, open_app_code, user_code, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    grant.id.value(),
                    grant.grant_code,
                    grant.open_app_code,
                    grant.user_code,
                    timestamp_to_millis(&grant.created_at),
                ],
            )
            .map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)
    }

    fn replace_capability_grants(
        &mut self,
        open_app_code: &str,
        grants: &[OpenAppCapabilityGrant],
    ) -> Result<(), OpenAppError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "DELETE FROM platform_open_app_capability_grants WHERE open_app_code = ?1",
            params![open_app_code],
        )
        .map_err(storage_error)?;
        for grant in grants {
            tx.execute(
                "INSERT INTO platform_open_app_capability_grants (id, grant_code, open_app_code, capability_code, operation_code, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    grant.id.value(),
                    grant.grant_code,
                    grant.open_app_code,
                    grant.capability_code,
                    grant.operation_code,
                    timestamp_to_millis(&grant.created_at),
                ],
            )
            .map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)
    }

    fn purge_expired_nonces(&mut self, now_epoch: i64) -> Result<(), OpenAppError> {
        self.connection
            .execute(
                "DELETE FROM platform_open_api_nonces WHERE expires_at < ?1",
                params![now_epoch],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn insert_nonce(&mut self, record: &OpenApiNonceRecord) -> Result<(), OpenAppError> {
        self.connection
            .execute(
                "INSERT INTO platform_open_api_nonces (id, nonce_code, open_app_code, nonce, signed_at, expires_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.id.value(),
                    record.nonce_code,
                    record.open_app_code,
                    record.nonce,
                    record.signed_at,
                    record.expires_at,
                    timestamp_to_millis(&record.created_at),
                ],
            )
            .map_err(map_nonce_error)?;
        Ok(())
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), OpenAppError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| OpenAppError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(_error: rusqlite::Error) -> OpenAppError {
    OpenAppError::StorageFailure
}

fn map_create_error(error: rusqlite::Error) -> OpenAppError {
    match error {
        rusqlite::Error::SqliteFailure(sqlite_error, _)
            if sqlite_error.code == ErrorCode::ConstraintViolation =>
        {
            OpenAppError::DuplicateName
        }
        other => storage_error(other),
    }
}

fn map_nonce_error(error: rusqlite::Error) -> OpenAppError {
    match error {
        rusqlite::Error::SqliteFailure(sqlite_error, _)
            if sqlite_error.code == ErrorCode::ConstraintViolation =>
        {
            OpenAppError::NonceReplayed
        }
        other => storage_error(other),
    }
}

#[cfg(test)]
mod tests;
