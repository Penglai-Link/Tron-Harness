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

struct SqliteAppRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteAppRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteAppRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, AppError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl SqliteAppRepositorySql<'_> {
    fn read_apps(&mut self, codes: Option<&[String]>) -> Result<Vec<App>, AppError> {
        let selected = codes
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| AppError::StorageFailure)?;
        let predicate = if selected.is_some() {
            "app_code IN (SELECT value FROM json_each(?1))"
        } else {
            "?1 IS NULL"
        };
        let mut statement = self
            .connection
            .prepare(&format!(
                "SELECT id, app_code, workspace_code, app_name, description, local_directory, runtime_role, repository_url, default_branch, status, created_at, updated_at, archived_at
                 FROM apps WHERE {predicate}
                 ORDER BY app_name ASC, app_code ASC"),
            )
            .map_err(storage_error)?;

        let rows = statement
            .query_map([selected], |row| {
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
}

impl AppRepository for SqliteAppRepositorySql<'_> {
    fn delete_app(&mut self, app_code: &str) -> Result<(), AppError> {
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        let linked: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM project_app_links WHERE app_code=?1 UNION ALL SELECT 1 FROM task_app_links WHERE app_code=?1)", [app_code], |row| row.get(0)).map_err(storage_error)?;
        if linked {
            return Err(AppError::InUse);
        }
        tx.execute("DELETE FROM app_tags WHERE app_code=?1", [app_code])
            .map_err(storage_error)?;
        if tx
            .execute("DELETE FROM apps WHERE app_code=?1", [app_code])
            .map_err(storage_error)?
            == 0
        {
            return Err(AppError::NotFound);
        }
        tx.commit().map_err(storage_error)
    }

    fn list_apps(&mut self) -> Result<Vec<App>, AppError> {
        self.read_apps(None)
    }
    fn apps_by_codes(&mut self, codes: &[String]) -> Result<Vec<App>, AppError> {
        self.read_apps(Some(codes))
    }
    fn tags_for_apps(&mut self, codes: &[String]) -> Result<Vec<AppTag>, AppError> {
        let codes = serde_json::to_string(codes).map_err(|_| AppError::StorageFailure)?;
        let mut q = self.connection.prepare("SELECT app_code,tag_name,linked_at FROM app_tags WHERE app_code IN (SELECT value FROM json_each(?1)) ORDER BY app_code,tag_name").map_err(storage_error)?;
        q.query_map([codes], |row| {
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
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
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

fn storage_error(_error: rusqlite::Error) -> AppError {
    AppError::StorageFailure
}

impl SqliteAppRepository {
    pub fn connect(database_url: &str) -> Result<Self, AppError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteAppRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl AppRepository for SqliteAppRepository {
    fn delete_app(&mut self, app_code: &str) -> Result<(), AppError> {
        let app_code = app_code.to_owned();
        self.connection.call(
            move |connection| SqliteAppRepositorySql { connection }.delete_app(&app_code),
            || AppError::StorageFailure,
        )
    }

    fn list_apps(&mut self) -> Result<Vec<App>, AppError> {
        self.connection.call(
            move |connection| SqliteAppRepositorySql { connection }.list_apps(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_app_tags(&mut self) -> Result<Vec<AppTag>, AppError> {
        self.connection.call(
            move |connection| SqliteAppRepositorySql { connection }.list_app_tags(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn apps_by_codes(&mut self, codes: &[String]) -> Result<Vec<App>, AppError> {
        let codes = codes.to_vec();
        self.connection.call_read(
            move |connection| SqliteAppRepositorySql { connection }.apps_by_codes(&codes),
            || AppError::StorageFailure,
        )
    }
    fn tags_for_apps(&mut self, codes: &[String]) -> Result<Vec<AppTag>, AppError> {
        let codes = codes.to_vec();
        self.connection.call_read(
            move |connection| SqliteAppRepositorySql { connection }.tags_for_apps(&codes),
            || AppError::StorageFailure,
        )
    }
    fn next_id(&mut self) -> Result<i64, AppError> {
        self.connection.call(
            move |connection| SqliteAppRepositorySql { connection }.next_id(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn create_app(&mut self, app: &App) -> Result<(), AppError> {
        let app = app.to_owned();
        self.connection.call(
            move |connection| SqliteAppRepositorySql { connection }.create_app(&app),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_app(&mut self, app: &App) -> Result<(), AppError> {
        let app = app.to_owned();
        self.connection.call(
            move |connection| SqliteAppRepositorySql { connection }.update_app(&app),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn replace_app_tags(
        &mut self,
        app_code: &str,
        tags: &[AppTag],
    ) -> Result<Vec<AppTag>, AppError> {
        let app_code = app_code.to_owned();
        let tags = tags.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAppRepositorySql { connection }.replace_app_tags(&app_code, &tags)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
