use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::workspace::WorkspaceRepository,
    domain::workspace::{
        Workspace, WorkspaceError, WorkspaceStatus, workspace_status_from_storage,
        workspace_status_to_storage,
    },
    shared::{
        id::SnowflakeId,
        time::{timestamp_from_millis, timestamp_to_millis},
    },
};

pub struct SqliteWorkspaceRepository {
    connection: super::framework_storage::SharedDatabase,
}

impl SqliteWorkspaceRepository {
    pub fn connect(database_url: &str) -> Result<Self, WorkspaceError> {
        ensure_database_parent(database_url)?;
        let connection =
            super::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl WorkspaceRepository for SqliteWorkspaceRepository {
    fn list_workspaces(&mut self) -> Result<Vec<Workspace>, WorkspaceError> {
        self.connection.call(move |connection| {

        let mut statement = connection
            .prepare(
                "SELECT id, workspace_code, name, description, parent_workspace_code, status, created_at, updated_at, archived_at
                 FROM workspaces
                 ORDER BY CASE WHEN parent_workspace_code IS NULL THEN 0 ELSE 1 END, name ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(Workspace {
                    id: SnowflakeId::new(row.get::<_, i64>(0)?),
                    workspace_code: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    parent_workspace_code: row.get(4)?,
                    status: workspace_status_from_storage(&row.get::<_, String>(5)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    created_at: timestamp_from_millis(row.get(6)?),
                    updated_at: timestamp_from_millis(row.get(7)?),
                    archived_at: row.get::<_, Option<i64>>(8)?.map(timestamp_from_millis),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)

        }, || WorkspaceError::StorageFailure)
    }

    fn next_id(&mut self) -> Result<i64, WorkspaceError> {
        self.connection.call(
            move |connection| {
                let max_id: i64 = connection
                    .query_row("SELECT max(id) FROM workspaces", [], |row| {
                        row.get::<_, Option<i64>>(0)
                    })
                    .map_err(storage_error)?
                    .unwrap_or(734_400_000_000_000_000);
                Ok(max_id + 1)
            },
            || WorkspaceError::StorageFailure,
        )
    }

    fn create_workspace(&mut self, workspace: &Workspace) -> Result<(), WorkspaceError> {
        let workspace = workspace.to_owned();
        self.connection.call(move |connection| {
            let workspace = &workspace;
        connection
            .execute(
                "INSERT INTO workspaces (id, workspace_code, name, description, parent_workspace_code, status, created_at, updated_at, archived_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    workspace.id.value(),
                    workspace.workspace_code,
                    workspace.name,
                    workspace.description,
                    workspace.parent_workspace_code,
                    workspace_status_to_storage(workspace.status),
                    timestamp_to_millis(&workspace.created_at),
                    timestamp_to_millis(&workspace.updated_at),
                    workspace.archived_at.as_deref().map(timestamp_to_millis),
                ],
            )
            .map_err(storage_error)?;
        Ok(())

        }, || WorkspaceError::StorageFailure)
    }

    fn update_workspace(&mut self, workspace: &Workspace) -> Result<(), WorkspaceError> {
        let workspace = workspace.to_owned();
        self.connection.call(
            move |connection| {
                let workspace = &workspace;
                connection
                    .execute(
                        "UPDATE workspaces
                 SET name = ?2,
                     description = ?3,
                     parent_workspace_code = ?4,
                     status = ?5,
                     updated_at = ?6,
                     archived_at = ?7
                 WHERE workspace_code = ?1",
                        params![
                            workspace.workspace_code,
                            workspace.name,
                            workspace.description,
                            workspace.parent_workspace_code,
                            workspace_status_to_storage(workspace.status),
                            timestamp_to_millis(&workspace.updated_at),
                            workspace.archived_at.as_deref().map(timestamp_to_millis),
                        ],
                    )
                    .map_err(storage_error)?;
                Ok(())
            },
            || WorkspaceError::StorageFailure,
        )
    }

    fn archive_subtree(
        &mut self,
        workspace_codes: &[String],
        archived_at: &str,
    ) -> Result<usize, WorkspaceError> {
        let workspace_codes = workspace_codes.to_owned();
        let archived_at = archived_at.to_owned();
        self.connection.call(
            move |connection| {
                let workspace_codes = &workspace_codes;
                let archived_at = &archived_at;
                update_subtree_status(
                    connection,
                    workspace_codes,
                    WorkspaceStatus::Archived,
                    Some(archived_at),
                )
            },
            || WorkspaceError::StorageFailure,
        )
    }

    fn restore_subtree(
        &mut self,
        workspace_codes: &[String],
        restored_at: &str,
    ) -> Result<usize, WorkspaceError> {
        let workspace_codes = workspace_codes.to_owned();
        let restored_at = restored_at.to_owned();
        self.connection.call(
            move |connection| {
                let workspace_codes = &workspace_codes;
                let restored_at = &restored_at;
                update_subtree_status(
                    connection,
                    workspace_codes,
                    WorkspaceStatus::Active,
                    Some(restored_at),
                )
            },
            || WorkspaceError::StorageFailure,
        )
    }
}

fn update_subtree_status(
    connection: &mut Connection,
    workspace_codes: &[String],
    status: WorkspaceStatus,
    timestamp: Option<&str>,
) -> Result<usize, WorkspaceError> {
    let tx =
        crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(
            &connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(storage_error)?;
    let mut affected = 0;
    for workspace_code in workspace_codes {
        affected += tx
            .execute(
                "UPDATE workspaces
                 SET status = ?2, updated_at = ?3, archived_at = ?4
                 WHERE workspace_code = ?1",
                params![
                    workspace_code,
                    workspace_status_to_storage(status),
                    timestamp.map(timestamp_to_millis).unwrap_or_default(),
                    match status {
                        WorkspaceStatus::Active => None,
                        WorkspaceStatus::Archived => timestamp.map(timestamp_to_millis),
                    },
                ],
            )
            .map_err(storage_error)?;
    }
    tx.commit().map_err(storage_error)?;
    Ok(affected)
}

fn ensure_database_parent(database_url: &str) -> Result<(), WorkspaceError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| WorkspaceError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(_error: rusqlite::Error) -> WorkspaceError {
    WorkspaceError::StorageFailure
}
