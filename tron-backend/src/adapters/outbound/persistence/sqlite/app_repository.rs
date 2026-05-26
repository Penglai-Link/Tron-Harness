use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::app::AppRepository,
    domain::app::{
        App, AppError, AppTag, app_runtime_role_from_storage, app_runtime_role_to_storage,
        app_status_from_storage, app_status_to_storage,
    },
    shared::{
        id::SnowflakeId,
        time::{
            option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
            timestamp_to_millis,
        },
    },
};

use super::migrations::APPS_MIGRATION;

pub struct SqliteAppRepository {
    connection: Connection,
}

impl SqliteAppRepository {
    pub fn connect(database_url: &str) -> Result<Self, AppError> {
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

    fn apply_migrations(&mut self) -> Result<(), AppError> {
        self.connection
            .execute_batch(APPS_MIGRATION)
            .map_err(storage_error)?;
        self.ensure_current_schema()
    }

    fn ensure_current_schema(&mut self) -> Result<(), AppError> {
        let columns = table_columns(&self.connection, "apps")?;
        if !columns.iter().any(|column| column == "local_directory") {
            self.connection
                .execute(
                    "ALTER TABLE apps ADD COLUMN local_directory TEXT NOT NULL DEFAULT ''",
                    [],
                )
                .map_err(storage_error)?;
        }
        if columns.iter().any(|column| column == "creation_mode") {
            rebuild_apps_schema_without_creation_mode(&self.connection)?;
        }
        self.connection
            .execute_batch(
                "UPDATE apps
                 SET local_directory = CASE app_code
                     WHEN 'APP-TRON-CONSOLE' THEN '/workspace/tron/tron-frontend'
                     WHEN 'APP-TRON-API' THEN '/workspace/tron/tron-backend'
                     WHEN 'APP-COMPLIANCE-API' THEN '/workspace/apps/compliance-api'
                     WHEN 'APP-TRON-LEGACY' THEN '/workspace/tron/legacy-console'
                     ELSE '/workspace/apps/' || lower(replace(replace(app_code, 'APP-', ''), '_', '-'))
                 END
                 WHERE trim(local_directory) = '';",
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

impl AppRepository for SqliteAppRepository {
    fn list_apps(&mut self) -> Result<Vec<App>, AppError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, app_code, workspace_code, app_name, description, local_directory, runtime_role, repository_url, default_branch, status, created_at, updated_at, archived_at
                 FROM apps
                 ORDER BY app_name ASC, app_code ASC",
            )
            .map_err(storage_error)?;

        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, Option<i64>>(12)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        rows.into_iter()
            .map(
                |(
                    id,
                    app_code,
                    workspace_code,
                    name,
                    description,
                    local_directory,
                    runtime_role,
                    repository_url,
                    default_branch,
                    status,
                    created_at,
                    updated_at,
                    archived_at,
                )| {
                    Ok(App {
                        id: SnowflakeId::new(id),
                        app_code,
                        workspace_code,
                        name,
                        description,
                        local_directory,
                        runtime_role: app_runtime_role_from_storage(runtime_role.as_str())?,
                        repository_url,
                        default_branch,
                        status: app_status_from_storage(status.as_str())?,
                        created_at: timestamp_from_millis(created_at),
                        updated_at: timestamp_from_millis(updated_at),
                        archived_at: option_timestamp_from_millis(archived_at),
                    })
                },
            )
            .collect()
    }

    fn list_app_tags(&mut self) -> Result<Vec<AppTag>, AppError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT app_code, tag_name, linked_at
                 FROM app_tags
                 ORDER BY app_code ASC, tag_name ASC",
            )
            .map_err(storage_error)?;

        statement
            .query_map([], |row| {
                Ok(AppTag {
                    app_code: row.get(0)?,
                    tag: row.get(1)?,
                    linked_at: timestamp_from_millis(row.get(2)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id(&mut self) -> Result<i64, AppError> {
        let max_id: i64 = self
            .connection
            .query_row(
                "SELECT max(id) FROM (SELECT id FROM apps UNION ALL SELECT id FROM app_tags)",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(734_900_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_app(&mut self, app: &App) -> Result<(), AppError> {
        self.connection
            .execute(
                "INSERT INTO apps (id, app_code, workspace_code, app_name, description, local_directory, runtime_role, repository_url, default_branch, status, created_at, updated_at, archived_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    app.id.value(),
                    app.app_code,
                    app.workspace_code,
                    app.name,
                    app.description,
                    app.local_directory,
                    app_runtime_role_to_storage(app.runtime_role),
                    app.repository_url,
                    app.default_branch,
                    app_status_to_storage(app.status),
                    timestamp_to_millis(&app.created_at),
                    timestamp_to_millis(&app.updated_at),
                    option_timestamp_to_millis(app.archived_at.as_deref()),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn update_app(&mut self, app: &App) -> Result<(), AppError> {
        let updated_rows = self
            .connection
            .execute(
                "UPDATE apps
                 SET app_name = ?2,
                     description = ?3,
                     local_directory = ?4,
                     runtime_role = ?5,
                     repository_url = ?6,
                     default_branch = ?7,
                     status = ?8,
                     updated_at = ?9,
                     archived_at = ?10
                 WHERE app_code = ?1",
                params![
                    app.app_code,
                    app.name,
                    app.description,
                    app.local_directory,
                    app_runtime_role_to_storage(app.runtime_role),
                    app.repository_url,
                    app.default_branch,
                    app_status_to_storage(app.status),
                    timestamp_to_millis(&app.updated_at),
                    option_timestamp_to_millis(app.archived_at.as_deref()),
                ],
            )
            .map_err(storage_error)?;
        if updated_rows == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    fn replace_app_tags(
        &mut self,
        app_code: &str,
        tags: &[AppTag],
    ) -> Result<Vec<AppTag>, AppError> {
        let mut next_id = self.next_id()?;
        let transaction = self.connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                "DELETE FROM app_tags WHERE app_code = ?1",
                params![app_code],
            )
            .map_err(storage_error)?;
        for tag in tags {
            transaction
                .execute(
                    "INSERT INTO app_tags (id, app_code, tag_name, linked_at) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        next_id,
                        tag.app_code,
                        tag.tag,
                        timestamp_to_millis(&tag.linked_at),
                    ],
                )
                .map_err(storage_error)?;
            next_id += 1;
        }
        transaction.commit().map_err(storage_error)?;
        self.list_app_tags().map(|records| {
            records
                .into_iter()
                .filter(|record| record.app_code == app_code)
                .collect()
        })
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), AppError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| AppError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(_error: rusqlite::Error) -> AppError {
    AppError::StorageFailure
}

fn rebuild_apps_schema_without_creation_mode(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            "PRAGMA foreign_keys=OFF;
             BEGIN TRANSACTION;

             ALTER TABLE app_tags RENAME TO app_tags_legacy;
             ALTER TABLE apps RENAME TO apps_legacy;

             CREATE TABLE apps (
                 id INTEGER PRIMARY KEY,
                 app_code TEXT NOT NULL UNIQUE,
                 workspace_code TEXT NOT NULL,
                 app_name TEXT NOT NULL,
                 description TEXT NULL,
                 local_directory TEXT NOT NULL,
                 runtime_role TEXT NOT NULL CHECK (runtime_role IN ('frontend', 'backend')),
                 repository_url TEXT NULL,
                 default_branch TEXT NULL,
                 status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL,
                 archived_at INTEGER NULL
             );

             CREATE TABLE app_tags (
                 id INTEGER PRIMARY KEY,
                 app_code TEXT NOT NULL REFERENCES apps(app_code),
                 tag_name TEXT NOT NULL,
                 linked_at INTEGER NOT NULL,
                 UNIQUE (app_code, tag_name)
             );

             INSERT INTO apps (
                 id,
                 app_code,
                 workspace_code,
                 app_name,
                 description,
                 local_directory,
                 runtime_role,
                 repository_url,
                 default_branch,
                 status,
                 created_at,
                 updated_at,
                 archived_at
             )
             SELECT
                 id,
                 app_code,
                 workspace_code,
                 app_name,
                 description,
                 local_directory,
                 runtime_role,
                 repository_url,
                 default_branch,
                 status,
                 created_at,
                 updated_at,
                 archived_at
             FROM apps_legacy;

             INSERT INTO app_tags (id, app_code, tag_name, linked_at)
             SELECT id, app_code, tag_name, linked_at
             FROM app_tags_legacy;

             DROP TABLE app_tags_legacy;
             DROP TABLE apps_legacy;

             CREATE INDEX IF NOT EXISTS idx_apps_workspace_status ON apps(workspace_code, status);
             CREATE INDEX IF NOT EXISTS idx_apps_workspace_name ON apps(workspace_code, app_name);
             CREATE INDEX IF NOT EXISTS idx_app_tags_app_code ON app_tags(app_code);

             COMMIT;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(storage_error)?;

    Ok(())
}

fn table_columns(connection: &Connection, table_name: &str) -> Result<Vec<String>, AppError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(storage_error)?;
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}
