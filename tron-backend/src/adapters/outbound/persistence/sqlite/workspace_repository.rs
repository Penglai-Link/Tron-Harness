use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::workspace::WorkspaceRepository,
    domain::workspace::{
        Workspace, WorkspaceError, WorkspaceRoleAccess, WorkspaceStatus, WorkspaceUserAccess,
        workspace_status_from_storage, workspace_status_to_storage,
    },
    shared::id::SnowflakeId,
};

use super::migrations::{WORKSPACE_ACCESS_GRANTS_MIGRATION, WORKSPACES_MIGRATION};

const DEFAULT_TIMESTAMP: &str = "2026-05-16T00:00:00Z";

pub struct SqliteWorkspaceRepository {
    connection: Connection,
}

impl SqliteWorkspaceRepository {
    pub fn connect(database_url: &str) -> Result<Self, WorkspaceError> {
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
        repository.seed_if_empty()?;
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), WorkspaceError> {
        self.connection
            .execute_batch(WORKSPACES_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(WORKSPACE_ACCESS_GRANTS_MIGRATION)
            .map_err(storage_error)
    }

    fn seed_if_empty(&mut self) -> Result<(), WorkspaceError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM workspaces", [], |row| row.get(0))
            .map_err(storage_error)?;
        if count > 0 {
            return Ok(());
        }

        let workspaces = [
            Workspace {
                id: SnowflakeId::new(734_400_000_000_000_001),
                workspace_code: "ENTERPRISE".into(),
                name: "Enterprise".into(),
                description: Some("Global governance workspace".into()),
                parent_workspace_code: None,
                status: WorkspaceStatus::Active,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
                archived_at: None,
            },
            Workspace {
                id: SnowflakeId::new(734_400_000_000_000_002),
                workspace_code: "PLATFORM".into(),
                name: "Platform".into(),
                description: Some("Core platform runtime workspace".into()),
                parent_workspace_code: Some("ENTERPRISE".into()),
                status: WorkspaceStatus::Active,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
                archived_at: None,
            },
            Workspace {
                id: SnowflakeId::new(734_400_000_000_000_003),
                workspace_code: "TRON_CORE".into(),
                name: "Tron Core".into(),
                description: Some("Main Tron console workspace".into()),
                parent_workspace_code: Some("PLATFORM".into()),
                status: WorkspaceStatus::Active,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
                archived_at: None,
            },
            Workspace {
                id: SnowflakeId::new(734_400_000_000_000_004),
                workspace_code: "COMPLIANCE".into(),
                name: "Compliance".into(),
                description: Some("Compliance evidence workspace".into()),
                parent_workspace_code: None,
                status: WorkspaceStatus::Active,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
                archived_at: None,
            },
        ];

        for workspace in &workspaces {
            self.create_workspace(workspace)?;
        }

        self.replace_user_access(
            "TRON_CORE",
            &[WorkspaceUserAccess {
                user_code: "USR-AUTH-0001".into(),
                is_owner: true,
                granted_at: DEFAULT_TIMESTAMP.into(),
            }],
        )?;
        self.replace_role_access(
            "PLATFORM",
            &[WorkspaceRoleAccess {
                role_code: "WORKSPACE_ADMIN".into(),
                granted_at: DEFAULT_TIMESTAMP.into(),
            }],
        )?;

        Ok(())
    }
}

impl WorkspaceRepository for SqliteWorkspaceRepository {
    fn list_workspaces(&mut self) -> Result<Vec<Workspace>, WorkspaceError> {
        let mut statement = self
            .connection
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
                    created_at: millis_to_timestamp(row.get(6)?),
                    updated_at: millis_to_timestamp(row.get(7)?),
                    archived_at: row.get::<_, Option<i64>>(8)?.map(millis_to_timestamp),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id(&mut self) -> Result<i64, WorkspaceError> {
        let max_id: i64 = self
            .connection
            .query_row(
                "SELECT max(id) FROM (
                    SELECT id FROM workspaces
                    UNION ALL SELECT id FROM workspace_user_access
                    UNION ALL SELECT id FROM workspace_role_access
                )",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(734_400_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_workspace(&mut self, workspace: &Workspace) -> Result<(), WorkspaceError> {
        self.connection
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
    }

    fn update_workspace(&mut self, workspace: &Workspace) -> Result<(), WorkspaceError> {
        self.connection
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
    }

    fn list_user_access(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT user_code, is_owner, created_at
                 FROM workspace_user_access
                 WHERE workspace_code = ?1
                 ORDER BY user_code ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([workspace_code], |row| {
                Ok(WorkspaceUserAccess {
                    user_code: row.get(0)?,
                    is_owner: row.get::<_, i64>(1)? == 1,
                    granted_at: millis_to_timestamp(row.get(2)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn replace_user_access(
        &mut self,
        workspace_code: &str,
        entries: &[WorkspaceUserAccess],
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
        let mut next_id = self.next_id()?;
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "DELETE FROM workspace_user_access WHERE workspace_code = ?1",
            [workspace_code],
        )
        .map_err(storage_error)?;
        for entry in entries {
            tx.execute(
                "INSERT INTO workspace_user_access (id, workspace_code, user_code, is_owner, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    next_id,
                    workspace_code,
                    entry.user_code,
                    i64::from(entry.is_owner),
                    timestamp_to_millis(&entry.granted_at),
                    timestamp_to_millis(&entry.granted_at),
                ],
            )
            .map_err(storage_error)?;
            next_id += 1;
        }
        tx.commit().map_err(storage_error)?;
        self.list_user_access(workspace_code)
    }

    fn list_role_access(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT role_code, created_at
                 FROM workspace_role_access
                 WHERE workspace_code = ?1
                 ORDER BY role_code ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([workspace_code], |row| {
                Ok(WorkspaceRoleAccess {
                    role_code: row.get(0)?,
                    granted_at: millis_to_timestamp(row.get(1)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn replace_role_access(
        &mut self,
        workspace_code: &str,
        entries: &[WorkspaceRoleAccess],
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
        let mut next_id = self.next_id()?;
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "DELETE FROM workspace_role_access WHERE workspace_code = ?1",
            [workspace_code],
        )
        .map_err(storage_error)?;
        for entry in entries {
            tx.execute(
                "INSERT INTO workspace_role_access (id, workspace_code, role_code, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    next_id,
                    workspace_code,
                    entry.role_code,
                    timestamp_to_millis(&entry.granted_at),
                    timestamp_to_millis(&entry.granted_at),
                ],
            )
            .map_err(storage_error)?;
            next_id += 1;
        }
        tx.commit().map_err(storage_error)?;
        self.list_role_access(workspace_code)
    }

    fn archive_subtree(
        &mut self,
        workspace_codes: &[String],
        archived_at: &str,
    ) -> Result<usize, WorkspaceError> {
        update_subtree_status(
            &mut self.connection,
            workspace_codes,
            WorkspaceStatus::Archived,
            Some(archived_at),
        )
    }

    fn restore_subtree(
        &mut self,
        workspace_codes: &[String],
        restored_at: &str,
    ) -> Result<usize, WorkspaceError> {
        update_subtree_status(
            &mut self.connection,
            workspace_codes,
            WorkspaceStatus::Active,
            Some(restored_at),
        )
    }
}

fn update_subtree_status(
    connection: &mut Connection,
    workspace_codes: &[String],
    status: WorkspaceStatus,
    timestamp: Option<&str>,
) -> Result<usize, WorkspaceError> {
    let tx = connection.transaction().map_err(storage_error)?;
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

fn timestamp_to_millis(value: &str) -> i64 {
    match value {
        "2026-05-11T08:00:00Z" => 1_778_457_600_000,
        "2026-05-11T08:30:00Z" => 1_778_459_400_000,
        _ => 1_778_889_600_000,
    }
}

fn millis_to_timestamp(value: i64) -> String {
    match value {
        1_778_457_600_000 => "2026-05-11T08:00:00Z".into(),
        1_778_459_400_000 => "2026-05-11T08:30:00Z".into(),
        _ => DEFAULT_TIMESTAMP.into(),
    }
}

fn storage_error(_error: rusqlite::Error) -> WorkspaceError {
    WorkspaceError::StorageFailure
}
