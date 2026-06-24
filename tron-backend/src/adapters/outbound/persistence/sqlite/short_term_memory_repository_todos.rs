use rusqlite::{Connection, TransactionBehavior};

use crate::domain::short_term_memory::{
    ShortTermMemoryError, ShortTermMemoryEvent, ShortTermMemoryTodo,
};

use super::{
    super::{
        short_term_memory_repository_append::assign_event_storage_fields,
        short_term_memory_rows::{insert_event, insert_todo, row_to_todo, storage_error},
    },
    repository_init::next_table_id,
};

pub(super) fn list_todos(
    connection: &Connection,
    session_code: &str,
) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, todo_code, session_code, workspace_code, position, content, status, priority, created_by_run_code, updated_by_run_code, created_at, updated_at
             FROM agent_session_todos
             WHERE session_code = ?1
             ORDER BY position ASC, id ASC",
        )
        .map_err(storage_error)?;
    statement
        .query_map([session_code], row_to_todo)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

pub(super) fn replace_todos(
    connection: &mut Connection,
    session_code: &str,
    workspace_code: &str,
    todos: &[ShortTermMemoryTodo],
) -> Result<(), ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    validate_todo_workspace(&tx, session_code, workspace_code, todos)?;
    tx.execute(
        "DELETE FROM agent_session_todos WHERE session_code = ?1",
        [session_code],
    )
    .map_err(storage_error)?;
    let mut todos = todos.to_vec();
    assign_todo_storage_ids(&tx, &mut todos)?;
    for todo in &todos {
        insert_todo(&tx, todo)?;
    }
    tx.commit().map_err(storage_error)
}

pub(super) fn replace_todos_with_event(
    connection: &mut Connection,
    session_code: &str,
    workspace_code: &str,
    todos: &[ShortTermMemoryTodo],
    event: &ShortTermMemoryEvent,
) -> Result<(), ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    validate_todo_workspace(&tx, session_code, workspace_code, todos)?;
    tx.execute(
        "DELETE FROM agent_session_todos WHERE session_code = ?1",
        [session_code],
    )
    .map_err(storage_error)?;
    let mut todos = todos.to_vec();
    assign_todo_storage_ids(&tx, &mut todos)?;
    for todo in &todos {
        insert_todo(&tx, todo)?;
    }
    let mut event = event.clone();
    assign_event_storage_fields(&tx, &mut event)?;
    insert_event(&tx, &event)?;
    tx.commit().map_err(storage_error)
}

fn validate_todo_workspace(
    connection: &Connection,
    session_code: &str,
    workspace_code: &str,
    todos: &[ShortTermMemoryTodo],
) -> Result<(), ShortTermMemoryError> {
    let stored_workspace = connection
        .query_row(
            "SELECT workspace_code FROM chat_sessions WHERE session_code = ?1",
            [session_code],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage_error)?;
    if stored_workspace != workspace_code {
        return Err(ShortTermMemoryError::StorageFailure);
    }
    for todo in todos {
        if todo.session_code != session_code || todo.workspace_code != workspace_code {
            return Err(ShortTermMemoryError::TodoInvalid);
        }
    }
    Ok(())
}

fn assign_todo_storage_ids(
    connection: &Connection,
    todos: &mut [ShortTermMemoryTodo],
) -> Result<(), ShortTermMemoryError> {
    let mut next_id = next_table_id(connection, "agent_session_todos")?;
    for todo in todos {
        if todo.id == 0 {
            todo.id = next_id;
            next_id += 1;
        }
    }
    Ok(())
}
