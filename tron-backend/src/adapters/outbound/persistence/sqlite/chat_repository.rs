use std::{fs, path::Path};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::{
    application::chat::ChatRepository,
    domain::chat::{
        ChatError, ChatMessage, ChatMessageChannel, ChatMessageRole, ChatSession, ChatSessionStatus,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

use super::migrations::{CHAT_MANAGEMENT_MIGRATION, CHAT_MESSAGE_TOKEN_USAGE_MIGRATION};

pub struct SqliteChatRepository {
    connection: Connection,
}

impl SqliteChatRepository {
    pub fn connect(database_url: &str) -> Result<Self, ChatError> {
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
        let repository = Self { connection };
        repository
            .connection
            .execute_batch(CHAT_MANAGEMENT_MIGRATION)
            .map_err(storage_error)?;
        if !column_exists(&repository.connection, "chat_messages", "input_tokens")? {
            repository
                .connection
                .execute_batch(CHAT_MESSAGE_TOKEN_USAGE_MIGRATION)
                .map_err(storage_error)?;
        }
        Ok(repository)
    }
}

impl ChatRepository for SqliteChatRepository {
    fn next_session_id(&mut self) -> Result<i64, ChatError> {
        next_id(&self.connection, "chat_sessions")
    }

    fn next_message_id(&mut self) -> Result<i64, ChatError> {
        next_id(&self.connection, "chat_messages")
    }

    fn create_session(&mut self, session: &ChatSession) -> Result<(), ChatError> {
        self.connection
            .execute(
                "INSERT INTO chat_sessions (id, session_code, workspace_code, user_code, app_code, task_code, title, status, last_run_code, last_run_status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    session.id,
                    session.session_code,
                    session.workspace_code,
                    session.user_code,
                    session.app_code,
                    session.task_code,
                    session.title,
                    session_status_to_storage(session.status),
                    session.last_run_code,
                    session.last_run_status,
                    timestamp_to_millis(&session.created_at),
                    timestamp_to_millis(&session.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_session(&mut self, session_code: &str) -> Result<(), ChatError> {
        self.connection
            .execute(
                "DELETE FROM chat_messages WHERE session_code = ?1",
                [session_code],
            )
            .map_err(storage_error)?;
        let changed = self
            .connection
            .execute(
                "DELETE FROM chat_sessions WHERE session_code = ?1",
                [session_code],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(ChatError::SessionNotFound);
        }
        Ok(())
    }

    fn list_sessions(&mut self) -> Result<Vec<ChatSession>, ChatError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, session_code, workspace_code, user_code, app_code, task_code, title, status, last_run_code, last_run_status, created_at, updated_at
                 FROM chat_sessions
                 ORDER BY updated_at DESC, id DESC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], row_to_session)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn get_session(&mut self, session_code: &str) -> Result<ChatSession, ChatError> {
        self.connection
            .query_row(
                "SELECT id, session_code, workspace_code, user_code, app_code, task_code, title, status, last_run_code, last_run_status, created_at, updated_at
                 FROM chat_sessions
                 WHERE session_code = ?1",
                [session_code],
                row_to_session,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(ChatError::SessionNotFound)
    }

    fn update_session_status(
        &mut self,
        session_code: &str,
        status: ChatSessionStatus,
        updated_at: &str,
    ) -> Result<(), ChatError> {
        let changed = self
            .connection
            .execute(
                "UPDATE chat_sessions
                 SET status = ?2, updated_at = ?3
                 WHERE session_code = ?1",
                params![
                    session_code,
                    session_status_to_storage(status),
                    timestamp_to_millis(updated_at),
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(ChatError::SessionNotFound);
        }
        Ok(())
    }

    fn update_session_title(
        &mut self,
        session_code: &str,
        title: &str,
        updated_at: &str,
    ) -> Result<(), ChatError> {
        let changed = self
            .connection
            .execute(
                "UPDATE chat_sessions
                 SET title = ?2, updated_at = ?3
                 WHERE session_code = ?1",
                params![session_code, title, timestamp_to_millis(updated_at)],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(ChatError::SessionNotFound);
        }
        Ok(())
    }

    fn update_session_run(
        &mut self,
        session_code: &str,
        run_code: &str,
        run_status: &str,
        updated_at: &str,
    ) -> Result<(), ChatError> {
        let changed = self
            .connection
            .execute(
                "UPDATE chat_sessions
                 SET last_run_code = ?2, last_run_status = ?3, updated_at = ?4
                 WHERE session_code = ?1",
                params![
                    session_code,
                    run_code,
                    run_status,
                    timestamp_to_millis(updated_at),
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(ChatError::SessionNotFound);
        }
        Ok(())
    }

    fn create_message(&mut self, message: &ChatMessage) -> Result<(), ChatError> {
        self.connection
            .execute(
                "INSERT INTO chat_messages (id, message_code, session_code, run_code, role, channel, actor, content, input_tokens, output_tokens, total_tokens, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    message.id,
                    message.message_code,
                    message.session_code,
                    message.run_code,
                    message_role_to_storage(message.role),
                    message_channel_to_storage(message.channel),
                    message.actor,
                    message.content,
                    message.input_tokens,
                    message.output_tokens,
                    message.total_tokens,
                    timestamp_to_millis(&message.created_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_messages(&mut self, session_code: &str) -> Result<Vec<ChatMessage>, ChatError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, message_code, session_code, run_code, role, channel, actor, content, input_tokens, output_tokens, total_tokens, created_at
                 FROM chat_messages
                 WHERE session_code = ?1
                 ORDER BY created_at ASC, id ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([session_code], row_to_message)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }
}

fn row_to_session(row: &Row<'_>) -> rusqlite::Result<ChatSession> {
    let status: String = row.get(7)?;
    Ok(ChatSession {
        id: row.get(0)?,
        session_code: row.get(1)?,
        workspace_code: row.get(2)?,
        user_code: row.get(3)?,
        app_code: row.get(4)?,
        task_code: row.get(5)?,
        title: row.get(6)?,
        status: session_status_from_storage(&status).map_err(chat_mapping_error)?,
        last_run_code: row.get(8)?,
        last_run_status: row.get(9)?,
        created_at: timestamp_from_millis(row.get(10)?),
        updated_at: timestamp_from_millis(row.get(11)?),
    })
}

fn row_to_message(row: &Row<'_>) -> rusqlite::Result<ChatMessage> {
    let role: String = row.get(4)?;
    let channel: String = row.get(5)?;
    Ok(ChatMessage {
        id: row.get(0)?,
        message_code: row.get(1)?,
        session_code: row.get(2)?,
        run_code: row.get(3)?,
        role: message_role_from_storage(&role).map_err(chat_mapping_error)?,
        channel: message_channel_from_storage(&channel).map_err(chat_mapping_error)?,
        actor: row.get(6)?,
        content: row.get(7)?,
        input_tokens: row.get(8)?,
        output_tokens: row.get(9)?,
        total_tokens: row.get(10)?,
        created_at: timestamp_from_millis(row.get(11)?),
    })
}

fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, ChatError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|value| value == column_name))
}

fn next_id(connection: &Connection, table: &str) -> Result<i64, ChatError> {
    let max_id = connection
        .query_row(&format!("SELECT max(id) FROM {table}"), [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .map_err(storage_error)?
        .unwrap_or(735_200_000_000_000_000);
    Ok(max_id + 1)
}

fn session_status_to_storage(status: ChatSessionStatus) -> &'static str {
    match status {
        ChatSessionStatus::Active => "ACTIVE",
        ChatSessionStatus::Archived => "ARCHIVED",
    }
}

fn session_status_from_storage(value: &str) -> Result<ChatSessionStatus, ChatError> {
    match value {
        "ACTIVE" => Ok(ChatSessionStatus::Active),
        "ARCHIVED" => Ok(ChatSessionStatus::Archived),
        _ => Err(ChatError::StorageFailure),
    }
}

fn message_role_to_storage(role: ChatMessageRole) -> &'static str {
    match role {
        ChatMessageRole::User => "USER",
        ChatMessageRole::Assistant => "ASSISTANT",
    }
}

fn message_role_from_storage(value: &str) -> Result<ChatMessageRole, ChatError> {
    match value {
        "USER" => Ok(ChatMessageRole::User),
        "ASSISTANT" => Ok(ChatMessageRole::Assistant),
        _ => Err(ChatError::StorageFailure),
    }
}

fn message_channel_to_storage(channel: ChatMessageChannel) -> &'static str {
    match channel {
        ChatMessageChannel::MainAgent => "MAIN_AGENT",
        ChatMessageChannel::TaskAgent => "TASK_AGENT",
    }
}

fn message_channel_from_storage(value: &str) -> Result<ChatMessageChannel, ChatError> {
    match value {
        "MAIN_AGENT" => Ok(ChatMessageChannel::MainAgent),
        "TASK_AGENT" => Ok(ChatMessageChannel::TaskAgent),
        _ => Err(ChatError::StorageFailure),
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), ChatError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| ChatError::StorageFailure)?;
        }
    }
    Ok(())
}

fn storage_error(error: rusqlite::Error) -> ChatError {
    eprintln!("chat sqlite error: {error}");
    ChatError::StorageFailure
}

fn chat_mapping_error(error: ChatError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
