use std::{fs, path::Path};

use rusqlite::{Connection, Row, params};

use crate::{
    application::task::TaskRepository,
    domain::task::{
        Task, TaskCodeLink, TaskError, TaskPriority, TaskSourceType, TaskStatus, TaskType,
        TaskUserAssignment, task_priority_from_storage, task_priority_to_storage,
        task_source_type_from_storage, task_source_type_to_storage, task_status_from_storage,
        task_status_to_storage, task_type_from_storage, task_type_to_storage,
    },
    shared::id::SnowflakeId,
};

use super::migrations::TASKS_MIGRATION;

const DEFAULT_TIMESTAMP: &str = "2026-05-16T00:00:00Z";
const DEFAULT_EPOCH_MS: i64 = 1_779_830_400_000;

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
        repository.seed_if_empty()?;
        Ok(repository)
    }

    fn seed_if_empty(&mut self) -> Result<(), TaskError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .map_err(storage_error)?;
        if count > 0 {
            return Ok(());
        }
        let seeds = [
            seed_task(
                734_500_000_000_000_001,
                "TSK-SEED-BACKLOG",
                TaskStatus::Backlog,
                None,
                TaskPriority::Medium,
                TaskType::Feature,
                TaskSourceType::Internal,
                Some(""),
                "Design task management v1",
                0,
                None,
            ),
            seed_task(
                734_500_000_000_000_002,
                "TSK-SEED-TESTING",
                TaskStatus::Testing,
                None,
                TaskPriority::High,
                TaskType::BugFix,
                TaskSourceType::GitHub,
                Some("https://github.com/clouds3n/tron/issues/42"),
                "Verify task smoke flow",
                65,
                None,
            ),
            seed_task(
                734_500_000_000_000_003,
                "TSK-SEED-ARCHIVED",
                TaskStatus::Archived,
                Some(TaskStatus::Todo),
                TaskPriority::Low,
                TaskType::Etc,
                TaskSourceType::Internal,
                Some(""),
                "Archived task example",
                15,
                Some(DEFAULT_TIMESTAMP),
            ),
        ];
        for task in &seeds {
            self.create_task(task)?;
        }
        self.replace_user_assignments(
            "TSK-SEED-BACKLOG",
            &[TaskUserAssignment {
                user_code: "USR-AUTH-0001".into(),
                assigned_at: DEFAULT_TIMESTAMP.into(),
            }],
        )?;
        self.replace_agent_links(
            "TSK-SEED-TESTING",
            &[TaskCodeLink {
                code: "agent-reviewer".into(),
                linked_at: DEFAULT_TIMESTAMP.into(),
            }],
        )?;
        self.replace_app_links(
            "TSK-SEED-TESTING",
            &[TaskCodeLink {
                code: "github".into(),
                linked_at: DEFAULT_TIMESTAMP.into(),
            }],
        )?;
        Ok(())
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
            "SELECT id, task_code, workspace_code, parent_task_code, title, description, status, previous_status, priority, task_type, source_type, source_url, progress_value, created_at, updated_at, archived_at FROM tasks ORDER BY id ASC"
        ).map_err(storage_error)?;
        statement
            .query_map([], row_to_task)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id(&mut self) -> Result<i64, TaskError> {
        let max_id = self.connection.query_row(
            "SELECT max(id) FROM (SELECT id FROM tasks UNION ALL SELECT id FROM task_user_assignments UNION ALL SELECT id FROM task_agent_links UNION ALL SELECT id FROM task_app_links)",
            [],
            |row| row.get::<_, Option<i64>>(0),
        ).map_err(storage_error)?.unwrap_or(734_500_000_000_000_000);
        Ok(max_id + 1)
    }

    fn create_task(&mut self, task: &Task) -> Result<(), TaskError> {
        self.connection.execute(
            "INSERT INTO tasks (id, task_code, workspace_code, parent_task_code, title, description, status, previous_status, priority, task_type, source_type, source_url, progress_value, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![task.id.value(), task.task_code, task.workspace_code, task.parent_task_code, task.title, task.description, task_status_to_storage(task.status), task.previous_status.map(task_status_to_storage), task_priority_to_storage(task.priority), task_type_to_storage(task.task_type), task_source_type_to_storage(task.source_type), task.source_url, i64::from(task.progress), timestamp_to_millis(&task.created_at), timestamp_to_millis(&task.updated_at), task.archived_at.as_deref().map(timestamp_to_millis)],
        ).map_err(storage_error)?;
        Ok(())
    }

    fn update_task(&mut self, task: &Task) -> Result<(), TaskError> {
        self.connection.execute(
            "UPDATE tasks SET parent_task_code = ?2, title = ?3, description = ?4, status = ?5, previous_status = ?6, priority = ?7, task_type = ?8, source_type = ?9, source_url = ?10, progress_value = ?11, updated_at = ?12, archived_at = ?13 WHERE task_code = ?1",
            params![task.task_code, task.parent_task_code, task.title, task.description, task_status_to_storage(task.status), task.previous_status.map(task_status_to_storage), task_priority_to_storage(task.priority), task_type_to_storage(task.task_type), task_source_type_to_storage(task.source_type), task.source_url, i64::from(task.progress), timestamp_to_millis(&task.updated_at), task.archived_at.as_deref().map(timestamp_to_millis)],
        ).map_err(storage_error)?;
        Ok(())
    }

    fn list_user_assignments(
        &mut self,
        task_code: &str,
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        let mut statement = self.connection.prepare("SELECT user_code, assigned_at FROM task_user_assignments WHERE task_code = ?1 ORDER BY user_code ASC").map_err(storage_error)?;
        statement
            .query_map([task_code], |row| {
                Ok(TaskUserAssignment {
                    user_code: row.get(0)?,
                    assigned_at: timestamp_from_millis(row.get(1)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn replace_user_assignments(
        &mut self,
        task_code: &str,
        entries: &[TaskUserAssignment],
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        let mut next_id = self.next_id()?;
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "DELETE FROM task_user_assignments WHERE task_code = ?1",
            [task_code],
        )
        .map_err(storage_error)?;
        for entry in entries {
            tx.execute("INSERT INTO task_user_assignments (id, task_code, user_code, assigned_at) VALUES (?1, ?2, ?3, ?4)", params![next_id, task_code, entry.user_code, timestamp_to_millis(&entry.assigned_at)]).map_err(storage_error)?;
            next_id += 1;
        }
        tx.commit().map_err(storage_error)?;
        self.list_user_assignments(task_code)
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

fn seed_task(
    id: i64,
    task_code: &str,
    status: TaskStatus,
    previous_status: Option<TaskStatus>,
    priority: TaskPriority,
    task_type: TaskType,
    source_type: TaskSourceType,
    source_url: Option<&str>,
    title: &str,
    progress: u8,
    archived_at: Option<&str>,
) -> Task {
    Task {
        id: SnowflakeId::new(id),
        task_code: task_code.into(),
        workspace_code: "TRON_CORE".into(),
        parent_task_code: None,
        title: title.into(),
        description: Some(format!("Seeded task: {title}")),
        status,
        previous_status,
        priority,
        task_type,
        source_type,
        source_url: source_url.map(str::to_owned),
        progress,
        created_at: DEFAULT_TIMESTAMP.into(),
        updated_at: DEFAULT_TIMESTAMP.into(),
        archived_at: archived_at.map(str::to_owned),
    }
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let status: String = row.get(6)?;
    let previous_status: Option<String> = row.get(7)?;
    let priority: String = row.get(8)?;
    let task_type: String = row.get(9)?;
    let source_type: String = row.get(10)?;
    let progress: i64 = row.get(12)?;
    Ok(Task {
        id: SnowflakeId::new(row.get(0)?),
        task_code: row.get(1)?,
        workspace_code: row.get(2)?,
        parent_task_code: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        status: task_status_from_storage(&status).map_err(task_mapping_error)?,
        previous_status: previous_status
            .as_deref()
            .map(task_status_from_storage)
            .transpose()
            .map_err(task_mapping_error)?,
        priority: task_priority_from_storage(&priority).map_err(task_mapping_error)?,
        task_type: task_type_from_storage(&task_type).map_err(task_mapping_error)?,
        source_type: task_source_type_from_storage(&source_type).map_err(task_mapping_error)?,
        source_url: row.get(11)?,
        progress: progress.clamp(0, 100) as u8,
        created_at: timestamp_from_millis(row.get(13)?),
        updated_at: timestamp_from_millis(row.get(14)?),
        archived_at: row.get::<_, Option<i64>>(15)?.map(timestamp_from_millis),
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
fn task_mapping_error(error: TaskError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
fn timestamp_to_millis(_value: &str) -> i64 {
    DEFAULT_EPOCH_MS
}
fn timestamp_from_millis(_value: i64) -> String {
    DEFAULT_TIMESTAMP.into()
}
