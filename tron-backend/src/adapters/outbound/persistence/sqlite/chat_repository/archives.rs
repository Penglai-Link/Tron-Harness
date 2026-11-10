use super::*;
use crate::application::chat::{ArchiveError, ArchivedContainer, ArchivedContainerKind};

fn archive_error(error: tokio_rusqlite::Error<ArchiveError>) -> ArchiveError {
    match error {
        tokio_rusqlite::Error::Error(error) => error,
        _ => ArchiveError::Storage,
    }
}

impl SqliteChatRepository {
    pub(super) fn read_archived_containers(
        &self,
        kind: ArchivedContainerKind,
        workspace: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<ArchivedContainer>, u64), ArchiveError> {
        let workspace = workspace.map(str::to_owned);
        blocking(async {
            self.framework().await.map_err(|_| ArchiveError::Storage)?;
            let db = self
                .host
                .shared_database()
                .await
                .map_err(|_| ArchiveError::Storage)?;
            db.call_read(move |connection| -> Result<_, ArchiveError> {
                let (table, code, name, path, archived) = match kind {
                    ArchivedContainerKind::Topic => ("projects", "project_code", "name", "NULL", "status='ARCHIVED'"),
                    ArchivedContainerKind::Project => ("directories", "directory_code", "display_name", "canonical_path", "archived=1"),
                };
                let tx = connection.unchecked_transaction().map_err(|_| ArchiveError::Storage)?;
                let filter = format!("{archived} AND (?1 IS NULL OR workspace_code=?1)");
                let total = tx.query_row(&format!("SELECT count(*) FROM {table} WHERE {filter}"), params![workspace], |row| row.get::<_, u64>(0)).map_err(|_| ArchiveError::Storage)?;
                let records = {
                    let query = format!("SELECT {code},workspace_code,{name},{path}, \
                        (SELECT count(*) FROM workspace_session_links l JOIN ef_sessions s ON s.id=l.session_id \
                         WHERE l.workspace_code=c.workspace_code AND json_extract(l.business_json,'$.{code}')=c.{code} \
                         AND s.status IN ('Archived','ARCHIVED')) \
                        FROM {table} c WHERE {filter} ORDER BY {name} COLLATE NOCASE,{code} LIMIT ?2 OFFSET ?3");
                    let mut statement = tx.prepare(&query).map_err(|_| ArchiveError::Storage)?;
                    statement.query_map(params![workspace, limit, offset], |row| Ok(ArchivedContainer {
                        code: row.get(0)?, workspace_code: row.get(1)?, name: row.get(2)?, path: row.get(3)?, chat_count: row.get(4)?,
                    })).map_err(|_| ArchiveError::Storage)?.collect::<Result<Vec<_>, _>>().map_err(|_| ArchiveError::Storage)?
                };
                tx.commit().map_err(|_| ArchiveError::Storage)?;
                Ok((records, total))
            }).await.map_err(archive_error)
        })
    }

    pub(super) fn restore_container_registration(
        &self,
        kind: ArchivedContainerKind,
        code: &str,
    ) -> Result<(), ArchiveError> {
        let code = code.to_owned();
        blocking(async {
            let db = self
                .host
                .shared_database()
                .await
                .map_err(|_| ArchiveError::Storage)?;
            db.call(move |connection| -> Result<(), ArchiveError> {
                let (table, key, archived, assignments) = match kind {
                    ArchivedContainerKind::Topic => ("projects", "project_code", "status='ARCHIVED'", "status='ACTIVE',archived_at=NULL,updated_at=CAST(strftime('%s','now') AS INTEGER)*1000"),
                    ArchivedContainerKind::Project => ("directories", "directory_code", "archived=1", "archived=0"),
                };
                let tx = super::super::framework_storage::business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(|_| ArchiveError::Storage)?;
                let workspace: String = tx.query_row(&format!("SELECT workspace_code FROM {table} WHERE {key}=?1 AND {archived}"), [&code], |row| row.get(0)).optional().map_err(|_| ArchiveError::Storage)?.ok_or(ArchiveError::NotFound)?;
                let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM workspaces WHERE workspace_code=?1 AND status='active')", [&workspace], |row| row.get(0)).map_err(|_| ArchiveError::Storage)?;
                if !active { return Err(ArchiveError::WorkspaceInactive); }
                if matches!(kind, ArchivedContainerKind::Topic) {
                    let name: String = tx.query_row("SELECT name FROM projects WHERE project_code=?1", [&code], |row| row.get(0)).map_err(|_| ArchiveError::Storage)?;
                    if super::super::project_repository::active_name_exists(&tx, &workspace, &name, &code).map_err(|_| ArchiveError::Storage)? {
                        return Err(ArchiveError::TopicNameConflict);
                    }
                }
                tx.execute(&format!("UPDATE {table} SET {assignments} WHERE {key}=?1"), [&code]).map_err(|_| ArchiveError::Storage)?;
                tx.commit().map_err(|_| ArchiveError::Storage)
            }).await.map_err(archive_error)
        })
    }
}
