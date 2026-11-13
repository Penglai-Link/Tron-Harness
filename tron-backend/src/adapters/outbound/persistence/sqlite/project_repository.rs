use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    application::project::{ProjectReadPage, ProjectReadRepository, ProjectRepository},
    domain::project::{
        Project, ProjectAppLink, ProjectError, ProjectView, project_status_from_storage,
        project_status_to_storage,
    },
    shared::time::{option_timestamp_to_millis, timestamp_from_millis, timestamp_to_millis},
};

struct SqliteProjectRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteProjectRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}
impl<'a> SqliteProjectRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, ProjectError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}
impl SqliteProjectRepositorySql<'_> {
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
    fn list_links_many(&mut self, codes: &[String]) -> Result<Vec<ProjectAppLink>, ProjectError> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let codes = serde_json::to_string(codes).map_err(|_| ProjectError::StorageFailure)?;
        let mut q = self.connection.prepare("SELECT id,project_code,app_code,position,is_primary FROM project_app_links WHERE project_code IN (SELECT value FROM json_each(?1)) ORDER BY project_code,position").map_err(storage_error)?;
        q.query_map([codes], |row| {
            Ok(ProjectAppLink {
                id: row.get(0)?,
                project_code: row.get(1)?,
                app_code: row.get(2)?,
                position: row.get(3)?,
                is_primary: row.get::<_, i64>(4)? == 1,
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
    }
    fn save_project(
        &mut self,
        project: &Project,
        links: &[ProjectAppLink],
    ) -> Result<(), ProjectError> {
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self
            .connection, rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| ProjectError::StorageFailure)?;
        if active_name_exists(
            &tx,
            &project.workspace_code,
            &project.name,
            &project.project_code,
        )
        .map_err(storage_error)?
        {
            return Err(ProjectError::DuplicateName);
        }
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

// The writer transaction serializes this check with saves and archive restores.
pub(super) fn active_name_exists(
    connection: &Connection,
    workspace_code: &str,
    name: &str,
    excluding_code: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE workspace_code=?1 AND name=?2 AND status='ACTIVE' AND project_code<>?3)",
        params![workspace_code, name, excluding_code],
        |row| row.get(0),
    )
}

fn storage_error(_: rusqlite::Error) -> ProjectError {
    ProjectError::StorageFailure
}

impl SqliteProjectRepository {
    pub fn connect(database_url: &str) -> Result<Self, ProjectError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteProjectRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl ProjectRepository for SqliteProjectRepository {
    fn reader(&self) -> std::sync::Arc<dyn ProjectReadRepository> {
        std::sync::Arc::new(SqliteProjectReader {
            connection: self.connection.clone(),
        })
    }
    fn next_id(&mut self) -> Result<i64, ProjectError> {
        self.connection.call(
            move |connection| SqliteProjectRepositorySql { connection }.next_id(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_projects(&mut self, workspace_code: &str) -> Result<Vec<Project>, ProjectError> {
        let workspace_code = workspace_code.to_owned();
        self.connection.call_read(
            move |connection| {
                SqliteProjectRepositorySql { connection }.list_projects(&workspace_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn get_project(&mut self, project_code: &str) -> Result<Project, ProjectError> {
        let project_code = project_code.to_owned();
        self.connection.call(
            move |connection| SqliteProjectRepositorySql { connection }.get_project(&project_code),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_links(&mut self, project_code: &str) -> Result<Vec<ProjectAppLink>, ProjectError> {
        let project_code = project_code.to_owned();
        self.connection.call(
            move |connection| SqliteProjectRepositorySql { connection }.list_links(&project_code),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_links_many(&mut self, codes: &[String]) -> Result<Vec<ProjectAppLink>, ProjectError> {
        let codes = codes.to_vec();
        self.connection.call_read(
            move |connection| SqliteProjectRepositorySql { connection }.list_links_many(&codes),
            || ProjectError::StorageFailure,
        )
    }
    fn save_project(
        &mut self,
        project: &Project,
        links: &[ProjectAppLink],
    ) -> Result<(), ProjectError> {
        let project = project.to_owned();
        let links = links.to_owned();
        self.connection.call(
            move |connection| {
                SqliteProjectRepositorySql { connection }.save_project(&project, &links)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn archive_project(
        &mut self,
        project_code: &str,
        archived_at: &str,
    ) -> Result<(), ProjectError> {
        let project_code = project_code.to_owned();
        let archived_at = archived_at.to_owned();
        self.connection.call(
            move |connection| {
                SqliteProjectRepositorySql { connection }
                    .archive_project(&project_code, &archived_at)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}

struct SqliteProjectReader {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}
impl ProjectReadRepository for SqliteProjectReader {
    fn page_active(
        &self,
        workspace_code: &str,
        offset: usize,
        limit: usize,
    ) -> Result<ProjectReadPage, ProjectError> {
        if !(1..=100).contains(&limit) {
            return Err(ProjectError::StorageFailure);
        }
        let workspace_code = workspace_code.to_owned();
        self.connection.call_read(move |c| {
            let tx=crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(c,rusqlite::TransactionBehavior::Deferred).map_err(storage_error)?;
            let total=tx.query_row("SELECT count(*) FROM projects WHERE workspace_code=?1 AND status='ACTIVE'",[&workspace_code],|r|r.get(0)).map_err(storage_error)?;
            // The helper only borrows the connection; keep this read transaction open through both statements.
            let records=read_active_projects(&tx,&workspace_code,Some((offset,limit)))?;
            tx.commit().map_err(storage_error)?;Ok(ProjectReadPage {total,records})
        },||ProjectError::StorageFailure)
    }
    fn list_active(&self, workspace_code: &str) -> Result<Vec<ProjectView>, ProjectError> {
        let workspace_code = workspace_code.to_owned();
        self.connection.call_read(
            move |connection| read_active_projects(connection, &workspace_code, None),
            || ProjectError::StorageFailure,
        )
    }
}

fn read_active_projects(
    connection: &Connection,
    workspace_code: &str,
    window: Option<(usize, usize)>,
) -> Result<Vec<ProjectView>, ProjectError> {
    // A single statement gives project rows and ordered links one snapshot,
    // without serializing requests behind the service's mutation mutex.
    let (scope, args) = if let Some((offset, limit)) = window {
        (
            "p.project_code IN (SELECT project_code FROM projects WHERE workspace_code=?1 AND status='ACTIVE' ORDER BY updated_at DESC,id DESC LIMIT ?2 OFFSET ?3)",
            vec![
                rusqlite::types::Value::Text(workspace_code.to_owned()),
                rusqlite::types::Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)),
                rusqlite::types::Value::Integer(i64::try_from(offset).unwrap_or(i64::MAX)),
            ],
        )
    } else {
        (
            "p.workspace_code=?1 AND p.status='ACTIVE'",
            vec![rusqlite::types::Value::Text(workspace_code.to_owned())],
        )
    };
    let mut statement=connection.prepare(&format!("SELECT p.project_code,p.workspace_code,p.name,p.status,p.created_at,p.updated_at,p.archived_at,l.app_code,l.is_primary FROM projects p LEFT JOIN project_app_links l ON l.project_code=p.project_code WHERE {scope} ORDER BY p.updated_at DESC,p.id DESC,l.position ASC")).map_err(storage_error)?;
    let mut rows = statement
        .query(rusqlite::params_from_iter(args))
        .map_err(storage_error)?;
    let mut projects = Vec::<ProjectView>::new();
    while let Some(row) = rows.next().map_err(storage_error)? {
        let code: String = row.get(0).map_err(storage_error)?;
        if projects.last().is_none_or(|p| p.project_code != code) {
            let status: String = row.get(3).map_err(storage_error)?;
            projects.push(ProjectView {
                project_code: code,
                workspace_code: row.get(1).map_err(storage_error)?,
                name: row.get(2).map_err(storage_error)?,
                status: project_status_from_storage(&status)?,
                created_at: timestamp_from_millis(row.get(4).map_err(storage_error)?),
                updated_at: timestamp_from_millis(row.get(5).map_err(storage_error)?),
                archived_at: row
                    .get::<_, Option<i64>>(6)
                    .map_err(storage_error)?
                    .map(timestamp_from_millis),
                app_codes: Vec::new(),
                primary_app_code: None,
            });
        }
        if let Some(app) = row.get::<_, Option<String>>(7).map_err(storage_error)? {
            let project = projects.last_mut().expect("project row inserted");
            if project.primary_app_code.is_none() && row.get::<_, bool>(8).map_err(storage_error)? {
                project.primary_app_code = Some(app.clone());
            }
            project.app_codes.push(app);
        }
    }
    Ok(projects)
}
