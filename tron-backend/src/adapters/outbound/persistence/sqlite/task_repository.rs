
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::{
    application::task::TaskRepository,
    domain::task::{
        Task, TaskCodeLink, TaskError, TaskFlow, TaskFlowStatus, TaskUserAssignment,
        task_priority_from_storage, task_priority_to_storage, task_source_type_from_storage,
        task_source_type_to_storage, task_type_from_storage, task_type_to_storage,
    },
    shared::{
        id::SnowflakeId,
        time::{
            option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
            timestamp_to_millis,
        },
    },
};

struct SqliteTaskRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteTaskRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteTaskRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, TaskError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
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
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
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

    fn read_task_flow(&self, flow_code: &str) -> Result<Option<TaskFlow>, TaskError> {
        let Some((flow_code, name)) = self
            .connection
            .query_row(
                "SELECT flow_code, name FROM task_flows WHERE flow_code = ?1",
                [flow_code],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?
        else {
            return Ok(None);
        };
        let mut statement = self
            .connection
            .prepare(
                "SELECT status_code, name, position, required FROM task_flow_statuses WHERE flow_code = ?1 ORDER BY position ASC",
            )
            .map_err(storage_error)?;
        let statuses = statement
            .query_map([&flow_code], |row| {
                Ok(TaskFlowStatus {
                    status_code: row.get(0)?,
                    name: row.get(1)?,
                    position: row.get::<_, i64>(2)? as usize,
                    required: row.get::<_, i64>(3)? != 0,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(Some(TaskFlow {
            flow_code,
            name,
            statuses,
        }))
    }

    fn replace_task_flow_statuses(&mut self, flow: &TaskFlow) -> Result<(), TaskError> {
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        tx.execute(
            "DELETE FROM task_flow_statuses WHERE flow_code = ?1",
            [&flow.flow_code],
        )
        .map_err(storage_error)?;
        for status in &flow.statuses {
            tx.execute(
                "INSERT INTO task_flow_statuses (flow_code, status_code, name, position, required) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    flow.flow_code,
                    status.status_code,
                    status.name,
                    status.position as i64,
                    if status.required { 1_i64 } else { 0_i64 },
                ],
            )
            .map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)
    }
}

impl TaskRepository for SqliteTaskRepositorySql<'_> {
    fn list_task_flows(&mut self) -> Result<Vec<TaskFlow>, TaskError> {
        let mut statement = self
            .connection
            .prepare("SELECT flow_code FROM task_flows ORDER BY flow_code ASC")
            .map_err(storage_error)?;
        let codes = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        codes
            .iter()
            .map(|flow_code| {
                self.read_task_flow(flow_code)?
                    .ok_or(TaskError::StorageFailure)
            })
            .collect()
    }

    fn get_task_flow(&mut self, flow_code: &str) -> Result<Option<TaskFlow>, TaskError> {
        self.read_task_flow(flow_code)
    }

    fn create_task_flow(&mut self, flow: &TaskFlow) -> Result<(), TaskError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        self.connection
            .execute(
                "INSERT INTO task_flows (flow_code, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                params![flow.flow_code, flow.name, timestamp],
            )
            .map_err(storage_error)?;
        self.replace_task_flow_statuses(flow)
    }

    fn update_task_flow(&mut self, flow: &TaskFlow) -> Result<(), TaskError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        self.connection
            .execute(
                "UPDATE task_flows SET name = ?2, updated_at = ?3 WHERE flow_code = ?1",
                params![flow.flow_code, flow.name, timestamp],
            )
            .map_err(storage_error)?;
        self.replace_task_flow_statuses(flow)
    }

    fn delete_task_flow(&mut self, flow_code: &str) -> Result<(), TaskError> {
        let assigned: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workspace_task_flows WHERE flow_code = ?1)",
                [flow_code],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if assigned {
            return Err(TaskError::FlowInUse);
        }
        self.connection
            .execute("DELETE FROM task_flows WHERE flow_code = ?1", [flow_code])
            .map_err(storage_error)?;
        Ok(())
    }

    fn workspace_task_flow_code(
        &mut self,
        workspace_code: &str,
    ) -> Result<Option<String>, TaskError> {
        self.connection
            .query_row(
                "SELECT flow_code FROM workspace_task_flows WHERE workspace_code = ?1",
                [workspace_code],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)
    }

    fn assign_workspace_task_flow(
        &mut self,
        workspace_code: &str,
        flow_code: &str,
    ) -> Result<(), TaskError> {
        self.connection
            .execute(
                "INSERT INTO workspace_task_flows (workspace_code, flow_code, updated_at) VALUES (?1, ?2, ?3) ON CONFLICT(workspace_code) DO UPDATE SET flow_code = excluded.flow_code, updated_at = excluded.updated_at",
                params![workspace_code, flow_code, chrono::Utc::now().timestamp_millis()],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn update_workspace_task_flow(
        &mut self,
        workspace_code: &str,
        flow_code: &str,
    ) -> Result<(), TaskError> {
        self.connection
            .execute(
                "UPDATE tasks SET flow_code = ?2, updated_at = ?3 WHERE workspace_code = ?1",
                params![
                    workspace_code,
                    flow_code,
                    chrono::Utc::now().timestamp_millis()
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_tasks(&mut self) -> Result<Vec<Task>, TaskError> {
        let mut statement = self.connection.prepare(
            "SELECT id, task_code, workspace_code, parent_task_code, title, description, execution_result, flow_code, status, previous_status, priority, task_type, source_type, source_url, agent_code, model_code, progress_value, created_at, updated_at, archived_at FROM tasks ORDER BY id ASC"
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
            "INSERT INTO tasks (id, task_code, workspace_code, parent_task_code, title, description, execution_result, flow_code, status, previous_status, priority, task_type, source_type, source_url, agent_code, model_code, progress_value, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![task.id.value(), task.task_code, task.workspace_code, task.parent_task_code, task.title, task.description, task.execution_result, task.flow_code, task.status, task.previous_status, task_priority_to_storage(task.priority), task_type_to_storage(task.task_type), task_source_type_to_storage(task.source_type), task.source_url, task.agent_code, task.model_code, i64::from(task.progress), timestamp_to_millis(&task.created_at), timestamp_to_millis(&task.updated_at), option_timestamp_to_millis(task.archived_at.as_deref())],
        ).map_err(storage_error)?;
        Ok(())
    }

    fn update_task(&mut self, task: &Task) -> Result<(), TaskError> {
        self.connection.execute(
            "UPDATE tasks SET parent_task_code = ?2, title = ?3, description = ?4, execution_result = ?5, flow_code = ?6, status = ?7, previous_status = ?8, priority = ?9, task_type = ?10, source_type = ?11, source_url = ?12, agent_code = ?13, model_code = ?14, progress_value = ?15, updated_at = ?16, archived_at = ?17 WHERE task_code = ?1",
            params![task.task_code, task.parent_task_code, task.title, task.description, task.execution_result, task.flow_code, task.status, task.previous_status, task_priority_to_storage(task.priority), task_type_to_storage(task.task_type), task_source_type_to_storage(task.source_type), task.source_url, task.agent_code, task.model_code, i64::from(task.progress), timestamp_to_millis(&task.updated_at), option_timestamp_to_millis(task.archived_at.as_deref())],
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
    let priority: String = row.get(10)?;
    let task_type: String = row.get(11)?;
    let source_type: String = row.get(12)?;
    let progress: i64 = row.get(16)?;
    Ok(Task {
        id: SnowflakeId::new(row.get(0)?),
        task_code: row.get(1)?,
        workspace_code: row.get(2)?,
        parent_task_code: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        execution_result: row.get(6)?,
        flow_code: row.get(7)?,
        status: row.get(8)?,
        previous_status: row.get(9)?,
        priority: task_priority_from_storage(&priority).map_err(task_mapping_error)?,
        task_type: task_type_from_storage(&task_type).map_err(task_mapping_error)?,
        source_type: task_source_type_from_storage(&source_type).map_err(task_mapping_error)?,
        source_url: row.get(13)?,
        agent_code: row.get(14)?,
        model_code: row.get(15)?,
        progress: progress.clamp(0, 100) as u8,
        created_at: timestamp_from_millis(row.get(17)?),
        updated_at: timestamp_from_millis(row.get(18)?),
        archived_at: option_timestamp_from_millis(row.get::<_, Option<i64>>(19)?),
    })
}

fn storage_error(error: rusqlite::Error) -> TaskError {
    eprintln!("task sqlite error: {error}");
    TaskError::StorageFailure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::DEFAULT_TASK_FLOW_CODE;

    #[test]
    fn new_database_seeds_default_flow_from_migration() {
        let mut repository =
            SqliteTaskRepository::connect(":memory:").expect("repository connects");
        let flow = repository
            .get_task_flow(DEFAULT_TASK_FLOW_CODE)
            .expect("flow query works")
            .expect("default flow exists");

        assert_eq!(flow.name, "默认流程");
        assert_eq!(
            flow.statuses
                .iter()
                .map(|status| (
                    status.status_code.as_str(),
                    status.name.as_str(),
                    status.position,
                    status.required,
                ))
                .collect::<Vec<_>>(),
            vec![
                ("NOT_STARTED", "未开始", 0, true),
                ("IN_PROGRESS", "进行中", 1, false),
                ("REOPEN", "重新打开", 2, false),
                ("CLOSED", "关闭", 3, true),
            ]
        );
    }
}

fn task_mapping_error(error: TaskError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

impl SqliteTaskRepository {
    pub fn connect(database_url: &str) -> Result<Self, TaskError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteTaskRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl TaskRepository for SqliteTaskRepository {
    fn list_task_flows(&mut self) -> Result<Vec<TaskFlow>, TaskError> {
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.list_task_flows(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn get_task_flow(&mut self, flow_code: &str) -> Result<Option<TaskFlow>, TaskError> {
        let flow_code = flow_code.to_owned();
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.get_task_flow(&flow_code),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn create_task_flow(&mut self, flow: &TaskFlow) -> Result<(), TaskError> {
        let flow = flow.to_owned();
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.create_task_flow(&flow),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_task_flow(&mut self, flow: &TaskFlow) -> Result<(), TaskError> {
        let flow = flow.to_owned();
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.update_task_flow(&flow),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_task_flow(&mut self, flow_code: &str) -> Result<(), TaskError> {
        let flow_code = flow_code.to_owned();
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.delete_task_flow(&flow_code),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn workspace_task_flow_code(
        &mut self,
        workspace_code: &str,
    ) -> Result<Option<String>, TaskError> {
        let workspace_code = workspace_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteTaskRepositorySql { connection }.workspace_task_flow_code(&workspace_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn assign_workspace_task_flow(
        &mut self,
        workspace_code: &str,
        flow_code: &str,
    ) -> Result<(), TaskError> {
        let workspace_code = workspace_code.to_owned();
        let flow_code = flow_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteTaskRepositorySql { connection }
                    .assign_workspace_task_flow(&workspace_code, &flow_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_workspace_task_flow(
        &mut self,
        workspace_code: &str,
        flow_code: &str,
    ) -> Result<(), TaskError> {
        let workspace_code = workspace_code.to_owned();
        let flow_code = flow_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteTaskRepositorySql { connection }
                    .update_workspace_task_flow(&workspace_code, &flow_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_tasks(&mut self) -> Result<Vec<Task>, TaskError> {
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.list_tasks(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn next_id(&mut self) -> Result<i64, TaskError> {
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.next_id(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn create_task(&mut self, task: &Task) -> Result<(), TaskError> {
        let task = task.to_owned();
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.create_task(&task),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_task(&mut self, task: &Task) -> Result<(), TaskError> {
        let task = task.to_owned();
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.update_task(&task),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_user_assignments(
        &mut self,
        _task_code: &str,
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        let _task_code = _task_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteTaskRepositorySql { connection }.list_user_assignments(&_task_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn replace_user_assignments(
        &mut self,
        _task_code: &str,
        _entries: &[TaskUserAssignment],
    ) -> Result<Vec<TaskUserAssignment>, TaskError> {
        let _task_code = _task_code.to_owned();
        let _entries = _entries.to_owned();
        self.connection.call(
            move |connection| {
                SqliteTaskRepositorySql { connection }
                    .replace_user_assignments(&_task_code, &_entries)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_agent_links(&mut self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError> {
        let task_code = task_code.to_owned();
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.list_agent_links(&task_code),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn replace_agent_links(
        &mut self,
        task_code: &str,
        entries: &[TaskCodeLink],
    ) -> Result<Vec<TaskCodeLink>, TaskError> {
        let task_code = task_code.to_owned();
        let entries = entries.to_owned();
        self.connection.call(
            move |connection| {
                SqliteTaskRepositorySql { connection }.replace_agent_links(&task_code, &entries)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_app_links(&mut self, task_code: &str) -> Result<Vec<TaskCodeLink>, TaskError> {
        let task_code = task_code.to_owned();
        self.connection.call(
            move |connection| SqliteTaskRepositorySql { connection }.list_app_links(&task_code),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn replace_app_links(
        &mut self,
        task_code: &str,
        entries: &[TaskCodeLink],
    ) -> Result<Vec<TaskCodeLink>, TaskError> {
        let task_code = task_code.to_owned();
        let entries = entries.to_owned();
        self.connection.call(
            move |connection| {
                SqliteTaskRepositorySql { connection }.replace_app_links(&task_code, &entries)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
