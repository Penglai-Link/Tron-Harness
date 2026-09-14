use std::{fs, path::Path};

use rusqlite::{Connection, OptionalExtension, params};

use super::migrations::PROJECT_CHAT_CONTEXT_MIGRATION;
use crate::{
    application::project::ProjectRepository,
    domain::project::{
        Project, ProjectAppLink, ProjectError, project_status_from_storage,
        project_status_to_storage,
    },
    shared::time::{option_timestamp_to_millis, timestamp_from_millis, timestamp_to_millis},
};

pub struct SqliteProjectRepository {
    connection: Connection,
}
impl SqliteProjectRepository {
    pub fn connect(database_url: &str) -> Result<Self, ProjectError> {
        if database_url != ":memory:" && !database_url.starts_with("file:") {
            if let Some(parent) = Path::new(database_url)
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                fs::create_dir_all(parent).map_err(|_| ProjectError::StorageFailure)?;
            }
        }
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
        .map_err(|_| ProjectError::StorageFailure)?;
        connection
            .execute_batch(PROJECT_CHAT_CONTEXT_MIGRATION)
            .map_err(|_| ProjectError::StorageFailure)?;
        Ok(Self { connection })
    }
}
impl ProjectRepository for SqliteProjectRepository {
    fn next_id(&mut self) -> Result<i64, ProjectError> {
        self.connection.query_row("SELECT max(id) FROM (SELECT id FROM projects UNION ALL SELECT id FROM project_app_links)", [], |row| row.get::<_, Option<i64>>(0)).map_err(|_| ProjectError::StorageFailure).map(|value| value.unwrap_or(736_000_000_000_000_000) + 1)
    }
    fn list_projects(&mut self, workspace_code: &str) -> Result<Vec<Project>, ProjectError> {
        let mut statement = self.connection.prepare("SELECT id, project_code, workspace_code, name, status, created_at, updated_at, archived_at FROM projects WHERE workspace_code = ?1 ORDER BY updated_at DESC, id DESC").map_err(|_| ProjectError::StorageFailure)?;
        statement
            .query_map([workspace_code], project_row)
            .map_err(|_| ProjectError::StorageFailure)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ProjectError::StorageFailure)
    }
    fn get_project(&mut self, project_code: &str) -> Result<Project, ProjectError> {
        self.connection.query_row("SELECT id, project_code, workspace_code, name, status, created_at, updated_at, archived_at FROM projects WHERE project_code = ?1", [project_code], project_row).optional().map_err(|_| ProjectError::StorageFailure)?.ok_or(ProjectError::NotFound)
    }
    fn list_links(&mut self, project_code: &str) -> Result<Vec<ProjectAppLink>, ProjectError> {
        let mut statement = self.connection.prepare("SELECT id, project_code, app_code, position, is_primary FROM project_app_links WHERE project_code = ?1 ORDER BY position ASC").map_err(|_| ProjectError::StorageFailure)?;
        statement
            .query_map([project_code], |row| {
                Ok(ProjectAppLink {
                    id: row.get(0)?,
                    project_code: row.get(1)?,
                    app_code: row.get(2)?,
                    position: row.get(3)?,
                    is_primary: row.get::<_, i64>(4)? == 1,
                })
            })
            .map_err(|_| ProjectError::StorageFailure)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ProjectError::StorageFailure)
    }
    fn save_project(
        &mut self,
        project: &Project,
        links: &[ProjectAppLink],
    ) -> Result<(), ProjectError> {
        let tx = self
            .connection
            .transaction()
            .map_err(|_| ProjectError::StorageFailure)?;
        tx.execute("INSERT INTO projects (id, project_code, workspace_code, name, status, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(project_code) DO UPDATE SET name = excluded.name, updated_at = excluded.updated_at", params![project.id, project.project_code, project.workspace_code, project.name, project_status_to_storage(project.status), timestamp_to_millis(&project.created_at), timestamp_to_millis(&project.updated_at), option_timestamp_to_millis(project.archived_at.as_deref())]).map_err(|_| ProjectError::StorageFailure)?;
        tx.execute(
            "DELETE FROM project_app_links WHERE project_code = ?1",
            [&project.project_code],
        )
        .map_err(|_| ProjectError::StorageFailure)?;
        for link in links {
            tx.execute("INSERT INTO project_app_links (id, project_code, app_code, position, is_primary) VALUES (?1, ?2, ?3, ?4, ?5)", params![link.id, link.project_code, link.app_code, link.position, if link.is_primary { 1 } else { 0 }]).map_err(|_| ProjectError::StorageFailure)?;
        }
        tx.commit().map_err(|_| ProjectError::StorageFailure)
    }
    fn archive_project(
        &mut self,
        project_code: &str,
        archived_at: &str,
    ) -> Result<(), ProjectError> {
        let changed = self.connection.execute("UPDATE projects SET status = 'ARCHIVED', archived_at = ?2, updated_at = ?2 WHERE project_code = ?1 AND status = 'ACTIVE'", params![project_code, timestamp_to_millis(archived_at)]).map_err(|_| ProjectError::StorageFailure)?;
        if changed == 0 {
            return Err(ProjectError::NotFound);
        }
        Ok(())
    }
}
fn project_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    let status: String = row.get(4)?;
    Ok(Project {
        id: row.get(0)?,
        project_code: row.get(1)?,
        workspace_code: row.get(2)?,
        name: row.get(3)?,
        status: project_status_from_storage(&status).map_err(|_| rusqlite::Error::InvalidQuery)?,
        created_at: timestamp_from_millis(row.get(5)?),
        updated_at: timestamp_from_millis(row.get(6)?),
        archived_at: row.get::<_, Option<i64>>(7)?.map(timestamp_from_millis),
    })
}
