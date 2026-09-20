use std::{fs, path::Path};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::{
    application::task::TaskRepository,
    domain::task::{
        Task, TaskCodeLink, TaskError, TaskUserAssignment, task_priority_from_storage,
        task_priority_to_storage, task_source_type_from_storage, task_source_type_to_storage,
        task_status_from_storage, task_status_to_storage, task_type_from_storage,
        task_type_to_storage,
    },
    shared::{
        id::SnowflakeId,
        time::{
            option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
            timestamp_to_millis,
        },
    },
};

use super::migrations::{TASKS_MIGRATION, TASKS_SUPERPOWER_RESET_MIGRATION};

pub struct SqliteTaskRepository {
    connection: Connection,
}

impl SqliteTaskRepository {
    pub fn connect(database_url: &str) -> Result<Self, TaskError> {
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
        repository
            .connection
            .execute_batch(TASKS_MIGRATION)
            .map_err(storage_error)?;
        repository.reset_legacy_schema_if_needed()?;
        let added_execution_result =
            if !column_exists(&repository.connection, "tasks", "execution_result")? {
                repository
                    .connection
                    .execute(
                        "ALTER TABLE tasks ADD COLUMN execution_result TEXT NULL",
                        [],
                    )
                    .map_err(storage_error)?;
                true
            } else {
                false
            };
        if added_execution_result {
            repository.migrate_legacy_completed_task_descriptions()?;
        }
        if !column_exists(&repository.connection, "tasks", "agent_code")? {
            repository
                .connection
                .execute(
                    "ALTER TABLE tasks ADD COLUMN agent_code TEXT NOT NULL DEFAULT 'main-agent'",
                    [],
                )
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "tasks", "model_code")? {
            repository
                .connection
                .execute("ALTER TABLE tasks ADD COLUMN model_code TEXT NULL", [])
                .map_err(storage_error)?;
        }
        repository.migrate_priority_extra_high_constraint_if_needed()?;
        Ok(repository)
    }

    fn migrate_legacy_completed_task_descriptions(&mut self) -> Result<(), TaskError> {
        self.connection
            .execute(
                "UPDATE tasks SET execution_result = description, description = NULL WHERE execution_result IS NULL AND status = 'Completed' AND description IS NOT NULL AND trim(description) <> ''",
                [],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn reset_legacy_schema_if_needed(&mut self) -> Result<(), TaskError> {
        let create_sql: Option<String> = self
            .connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tasks'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        if create_sql
            .as_deref()
            .is_some_and(uses_legacy_task_schema_definition)
        {
            self.connection
                .execute_batch(TASKS_SUPERPOWER_RESET_MIGRATION)
                .map_err(storage_error)?;
        }
        Ok(())
    }

    fn migrate_priority_extra_high_constraint_if_needed(&mut self) -> Result<(), TaskError> {
        let create_sql: Option<String> = self
            .connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tasks'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        if create_sql
            .as_deref()
            .is_some_and(task_priority_constraint_excludes_extra_high)
        {
            self.rebuild_tasks_table_with_current_priority_constraint()?;
        }
        Ok(())
    }

    fn rebuild_tasks_table_with_current_priority_constraint(&mut self) -> Result<(), TaskError> {
        let foreign_keys_enabled: i64 = self
            .connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .map_err(storage_error)?;
        self.connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .map_err(storage_error)?;
        let result = self.rebuild_tasks_table_with_foreign_keys_disabled();
        if foreign_keys_enabled == 1 {
            self.connection
                .execute_batch("PRAGMA foreign_keys = ON;")
                .map_err(storage_error)?;
        }
        result
    }

    fn rebuild_tasks_table_with_foreign_keys_disabled(&mut self) -> Result<(), TaskError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute_batch(
            "
            CREATE TABLE tasks_rebuild (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL UNIQUE,
                workspace_code TEXT NOT NULL,
                parent_task_code TEXT NULL REFERENCES tasks(task_code),
                title TEXT NOT NULL,
                description TEXT NULL,
                execution_result TEXT NULL,
                status TEXT NOT NULL CHECK (status IN ('Open', 'Designing', 'Red', 'Green', 'Regressioning', 'Review', 'InProgress', 'Reopen', 'Completed', 'Archived')),
                previous_status TEXT NULL CHECK (previous_status IS NULL OR previous_status IN ('Open', 'Designing', 'Red', 'Green', 'Regressioning', 'Review', 'InProgress', 'Reopen', 'Completed')),
                priority TEXT NOT NULL CHECK (priority IN ('Low', 'Medium', 'High', 'Extra High')),
                task_type TEXT NOT NULL CHECK (task_type IN ('Feature', 'Bug')),
                source_type TEXT NOT NULL CHECK (source_type IN ('Internal', 'GitHub', 'GitLab')),
                source_url TEXT NULL,
                agent_code TEXT NOT NULL DEFAULT 'main-agent',
                model_code TEXT NULL,
                progress_value INTEGER NOT NULL DEFAULT 0 CHECK (progress_value >= 0 AND progress_value <= 100),
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                archived_at INTEGER NULL
            );
            INSERT INTO tasks_rebuild (
                id,
                task_code,
                workspace_code,
                parent_task_code,
                title,
                description,
                execution_result,
                status,
                previous_status,
                priority,
                task_type,
                source_type,
                source_url,
                agent_code,
                model_code,
                progress_value,
                created_at,
                updated_at,
                archived_at
            )
            SELECT
                id,
                task_code,
                workspace_code,
                parent_task_code,
                title,
                description,
                execution_result,
                status,
                previous_status,
                priority,
                task_type,
                source_type,
                source_url,
                agent_code,
                model_code,
                progress_value,
                created_at,
                updated_at,
                archived_at
            FROM tasks;
            DROP TABLE tasks;
            ALTER TABLE tasks_rebuild RENAME TO tasks;
            CREATE INDEX IF NOT EXISTS idx_tasks_workspace_status ON tasks(workspace_code, status);
            CREATE INDEX IF NOT EXISTS idx_tasks_parent_task_code ON tasks(parent_task_code);
            ",
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)
    }

    fn list_code_links(
        &mut self,
        table: &str,
        code_column: &str,
        task_code: &str,
    ) -> Result<Vec<TaskCodeLink>, TaskError> {
        let sql = format!(
            "SELECT {code_column}, linked_at FROM {table} WHERE task_code = ?1 ORDER BY {code_column} ASC"
        );
        let mut statement = self.connection.prepare(&sql).map_err(storage_error)?;
        statement
            .query_map([task_code], |row| {
                Ok(TaskCodeLink {
                    code: row.get(0)?,
                    linked_at: timestamp_from_millis(row.get(1)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn replace_code_links(
        &mut self,
        table: &str,
        code_column: &str,
        task_code: &str,
        entries: &[TaskCodeLink],
    ) -> Result<(), TaskError> {
        let mut next_id = self.next_id()?;
        let delete_sql = format!("DELETE FROM {table} WHERE task_code = ?1");
        let insert_sql = format!(
            "INSERT INTO {table} (id, task_code, {code_column}, linked_at) VALUES (?1, ?2, ?3, ?4)"
        );
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(&delete_sql, [task_code])
            .map_err(storage_error)?;
        for entry in entries {
            tx.execute(
                &insert_sql,
                params![
                    next_id,
                    task_code,
                    entry.code,
                    timestamp_to_millis(&entry.linked_at)
                ],
            )
            .map_err(storage_error)?;
            next_id += 1;
        }
        tx.commit().map_err(storage_error)
    }
}

impl TaskRepository for SqliteTaskRepository {
    fn list_tasks(&mut self) -> Result<Vec<Task>, TaskError> {
        let mut statement = self.connection.prepare(
            "SELECT id, task_code, workspace_code, parent_task_code, title, description, execution_result, status, previous_status, priority, task_type, source_type, source_url, agent_code, model_code, progress_value, created_at, updated_at, archived_at FROM tasks ORDER BY id ASC"
        ).map_err(storage_error)?;
        statement
            .query_map([], row_to_task)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id(&mut self) -> Result<i64, TaskError> {
        let max_id = self.connection.query_row(
            "SELECT max(id) FROM (SELECT id FROM tasks UNION ALL SELECT id FROM task_agent_links UNION ALL SELECT id FROM task_app_links)",
            [],
            |row| row.get::<_, Option<i64>>(0),
        ).map_err(storage_error)?.unwrap_or(734_500_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_task(&mut self, task: &Task) -> Result<(), TaskError> {
        self.connection.execute(
            "INSERT INTO tasks (id, task_code, workspace_code, parent_task_code, title, description, execution_result, status, previous_status, priority, task_type, source_type, source_url, agent_code, model_code, progress_value, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![task.id.value(), task.task_code, task.workspace_code, task.parent_task_code, task.title, task.description, task.execution_result, task_status_to_storage(task.status), task.previous_status.map(task_status_to_storage), task_priority_to_storage(task.priority), task_type_to_storage(task.task_type), task_source_type_to_storage(task.source_type), task.source_url, task.agent_code, task.model_code, i64::from(task.progress), timestamp_to_millis(&task.created_at), timestamp_to_millis(&task.updated_at), option_timestamp_to_millis(task.archived_at.as_deref())],
        ).map_err(storage_error)?;
        Ok(())
    }

    fn update_task(&mut self, task: &Task) -> Result<(), TaskError> {
        self.connection.execute(
            "UPDATE tasks SET parent_task_code = ?2, title = ?3, description = ?4, execution_result = ?5, status = ?6, previous_status = ?7, priority = ?8, task_type = ?9, source_type = ?10, source_url = ?11, agent_code = ?12, model_code = ?13, progress_value = ?14, updated_at = ?15, archived_at = ?16 WHERE task_code = ?1",
            params![task.task_code, task.parent_task_code, task.title, task.description, task.execution_result, task_status_to_storage(task.status), task.previous_status.map(task_status_to_storage), task_priority_to_storage(task.priority), task_type_to_storage(task.task_type), task_source_type_to_storage(task.source_type), task.source_url, task.agent_code, task.model_code, i64::from(task.progress), timestamp_to_millis(&task.updated_at), option_timestamp_to_millis(task.archived_at.as_deref())],
        ).map_err(storage_error)?;
        Ok(())
    }

    fn list_user_assignments(
        &mut self,
        _task_code: &str,
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        Ok(Vec::new())
    }

    fn replace_user_assignments(
        &mut self,
        _task_code: &str,
        _entries: &[TaskUserAssignment],
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        Ok(Vec::new())
    }

    fn list_agent_links(&mut self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError> {
        self.list_code_links("task_agent_links", "agent_code", task_code)
    }
    fn replace_agent_links(
        &mut self,
        task_code: &str,
        entries: &[TaskCodeLink],
    ) -> Result<Vec<TaskCodeLink>, TaskError> {
        self.replace_code_links("task_agent_links", "agent_code", task_code, entries)?;
        self.list_agent_links(task_code)
    }
    fn list_app_links(&mut self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError> {
        self.list_code_links("task_app_links", "app_code", task_code)
    }
    fn replace_app_links(
        &mut self,
        task_code: &str,
        entries: &[TaskCodeLink],
    ) -> Result<Vec<TaskCodeLink>, TaskError> {
        self.replace_code_links("task_app_links", "app_code", task_code, entries)?;
        self.list_app_links(task_code)
    }
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let status: String = row.get(7)?;
    let previous_status: Option<String> = row.get(8)?;
    let priority: String = row.get(9)?;
    let task_type: String = row.get(10)?;
    let source_type: String = row.get(11)?;
    let progress: i64 = row.get(15)?;
    Ok(Task {
        id: SnowflakeId::new(row.get(0)?),
        task_code: row.get(1)?,
        workspace_code: row.get(2)?,
        parent_task_code: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        execution_result: row.get(6)?,
        status: task_status_from_storage(&status).map_err(task_mapping_error)?,
        previous_status: previous_status
            .as_deref()
            .map(task_status_from_storage)
            .transpose()
            .map_err(task_mapping_error)?,
        priority: task_priority_from_storage(&priority).map_err(task_mapping_error)?,
        task_type: task_type_from_storage(&task_type).map_err(task_mapping_error)?,
        source_type: task_source_type_from_storage(&source_type).map_err(task_mapping_error)?,
        source_url: row.get(12)?,
        agent_code: row.get(13)?,
        model_code: row.get(14)?,
        progress: progress.clamp(0, 100) as u8,
        created_at: timestamp_from_millis(row.get(16)?),
        updated_at: timestamp_from_millis(row.get(17)?),
        archived_at: option_timestamp_from_millis(row.get::<_, Option<i64>>(18)?),
    })
}

fn ensure_database_parent(database_url: &str) -> Result<(), TaskError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| TaskError::StorageFailure)?;
        }
    }
    Ok(())
}
fn storage_error(error: rusqlite::Error) -> TaskError {
    eprintln!("task sqlite error: {error}");
    TaskError::StorageFailure
}

fn uses_legacy_task_schema_definition(create_sql: &str) -> bool {
    ["Backlog", "Todo", "Testing", "BugFix", "Refactor"]
        .iter()
        .any(|needle| create_sql.contains(needle))
}

fn task_priority_constraint_excludes_extra_high(create_sql: &str) -> bool {
    create_sql.contains("priority TEXT NOT NULL CHECK") && !create_sql.contains("Extra High")
}

fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, TaskError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|value| value == column_name))
}

fn task_mapping_error(error: TaskError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
