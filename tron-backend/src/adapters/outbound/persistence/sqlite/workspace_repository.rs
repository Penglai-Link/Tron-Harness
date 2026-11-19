use std::{collections::BTreeSet, fs, path::Path};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    application::workspace::{WorkspacePurgeOutcome, WorkspaceReadRepository, WorkspaceRepository},
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
    fn reader(&self) -> std::sync::Arc<dyn WorkspaceReadRepository> {
        std::sync::Arc::new(SqliteWorkspaceReader {
            connection: self.connection.clone(),
        })
    }
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

    fn delete_subtree(
        &mut self,
        workspace_codes: &[String],
    ) -> Result<WorkspacePurgeOutcome, WorkspaceError> {
        let workspace_codes = workspace_codes.to_owned();
        self.connection.call(
            move |connection| delete_workspace_subtree(connection, &workspace_codes),
            || WorkspaceError::StorageFailure,
        )
    }
}

fn delete_workspace_subtree(
    connection: &mut Connection,
    workspace_codes: &[String],
) -> Result<WorkspacePurgeOutcome, WorkspaceError> {
    if workspace_codes.is_empty() {
        return Ok(WorkspacePurgeOutcome::default());
    }
    let scope =
        serde_json::to_string(workspace_codes).map_err(|_| WorkspaceError::StorageFailure)?;
    let tx =
        crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(
            connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(storage_error)?;
    // Every row owned by the subtree disappears in this transaction, so
    // self-references and NO ACTION constraints can be checked at commit time.
    tx.execute_batch("PRAGMA defer_foreign_keys = ON;")
        .map_err(storage_error)?;

    let storage_locators = {
        let mut statement = tx
            .prepare(
                "SELECT storage_locator FROM resource_files
                 WHERE workspace_code IN (SELECT value FROM json_each(?1))
                 UNION
                 SELECT storage_locator FROM resource_chat_session_attachments
                 WHERE workspace_code IN (SELECT value FROM json_each(?1))",
            )
            .map_err(storage_error)?;
        statement
            .query_map([&scope], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(storage_error)?
            .into_iter()
            .collect::<Vec<_>>()
    };

    // Task-owned relations first, then tasks.
    tx.execute(
        "DELETE FROM assistant_work_changes
         WHERE task_code IN (
             SELECT task_code FROM tasks WHERE workspace_code IN (SELECT value FROM json_each(?1))
         )
            OR json_extract(record_json, '$.sessionCode') IN (
             SELECT session_id FROM workspace_session_links WHERE workspace_code IN (SELECT value FROM json_each(?1))
         )",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM task_agent_links
         WHERE task_code IN (SELECT task_code FROM tasks WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM task_app_links
         WHERE task_code IN (SELECT task_code FROM tasks WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM tasks WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;

    // Projects own topics and app links; remove those before the apps.
    tx.execute(
        "DELETE FROM project_app_links
         WHERE project_code IN (SELECT project_code FROM projects WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM topic_memos
         WHERE project_code IN (SELECT project_code FROM projects WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM topic_references
         WHERE project_code IN (SELECT project_code FROM projects WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM projects WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;

    tx.execute(
        "DELETE FROM app_tags
         WHERE app_code IN (SELECT app_code FROM apps WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM project_app_links
         WHERE app_code IN (SELECT app_code FROM apps WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM task_app_links
         WHERE app_code IN (SELECT app_code FROM apps WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM apps WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;

    tx.execute(
        "DELETE FROM automation_execution_records WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM automations WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;

    // Business projections of workspace conversations. Message history and
    // artifacts are removed by the session purger before this transaction.
    tx.execute(
        "DELETE FROM chat_session_auto_model_routes WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM provider_model_run_snapshots WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM collaboration_thread_feedback
         WHERE session_id IN (SELECT session_id FROM collaboration_threads WHERE workspace_code IN (SELECT value FROM json_each(?1)))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM collaboration_threads WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM assistant_events
         WHERE EXISTS (
             SELECT 1 FROM assistant_sessions s
             WHERE s.session_code IN (
                 SELECT session_id FROM workspace_session_links
                 WHERE workspace_code IN (SELECT value FROM json_each(?1))
             )
               AND (assistant_events.session_code = s.session_code
                    OR assistant_events.event_id = s.correlation_key)
         )",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM assistant_sessions
         WHERE session_code IN (
             SELECT session_id FROM workspace_session_links
             WHERE workspace_code IN (SELECT value FROM json_each(?1))
         )",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM workspace_session_links WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;

    tx.execute(
        "DELETE FROM knowledge_base_derived_resources WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM knowledge_base_resource_privacy WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM resource_chat_session_attachments WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM resource_files WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM resource_code_registry WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM rag_diagnostics WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;

    tx.execute(
        "DELETE FROM default_model_selections
         WHERE workspace_code IN (SELECT value FROM json_each(?1))
            OR scope_key IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM connector_connections WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM directories WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;

    tx.execute(
        "DELETE FROM workspace_internal_agent_mcp_server_assignments WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM workspace_internal_agent_tool_assignments WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM workspace_internal_agent_policies WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM workspace_internal_agent_definitions WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM workspace_system_agent_policies WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM workspace_mcp_server_policies WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM workspace_task_flows WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM workspace_variables WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;
    tx.execute(
        "DELETE FROM agent_workspace_grants WHERE workspace_code IN (SELECT value FROM json_each(?1))",
        [&scope],
    )
    .map_err(storage_error)?;

    let affected_workspace_count = tx
        .execute(
            "DELETE FROM workspaces WHERE workspace_code IN (SELECT value FROM json_each(?1))",
            [&scope],
        )
        .map_err(storage_error)?;
    tx.commit().map_err(storage_error)?;
    Ok(WorkspacePurgeOutcome {
        affected_workspace_count,
        storage_locators,
    })
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

struct SqliteWorkspaceReader {
    connection: super::framework_storage::SharedDatabase,
}
impl WorkspaceReadRepository for SqliteWorkspaceReader {
    fn is_active(&self, workspace_code: &str) -> Result<bool, WorkspaceError> {
        let code = workspace_code.to_owned();
        self.connection.call_read(
            move |c| {
                let status = c
                    .query_row(
                        "SELECT status FROM workspaces WHERE workspace_code=?1",
                        [code],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(storage_error)?
                    .ok_or(WorkspaceError::NotFound)?;
                Ok(workspace_status_from_storage(&status)? == WorkspaceStatus::Active)
            },
            || WorkspaceError::StorageFailure,
        )
    }
}
