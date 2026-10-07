use rusqlite::{Connection, OptionalExtension, Row, params};
use std::{fs, path::Path};

use crate::{
    application::chat::ChatRepository,
    domain::chat::{
        ChatError, ChatMessage, ChatMessageChannel, ChatMessageRole, ChatSession,
        ChatSessionSourceType, ChatSessionStatus, ChatUserInputAnswer, ChatUserInputRequest,
        ChatUserInputRequestStatus,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

use super::chat_message_attachments::{attachments_from_storage, attachments_to_storage};
use super::migrations::{
    CHAT_MANAGEMENT_MIGRATION, CHAT_MESSAGE_ATTACHMENTS_MIGRATION,
    CHAT_ARCHIVE_CLEANUP_INDEXES_MIGRATION, CHAT_MESSAGE_TOKEN_USAGE_MIGRATION,
    CHAT_SESSION_AGENT_CODE_MIGRATION,
    CHAT_SESSION_LEGACY_SOURCE_MIGRATION, CHAT_SESSION_SOURCE_MIGRATION,
    CHAT_SESSION_SYSTEM_AGENT_SESSION_ID_MIGRATION, CHAT_SESSION_VISIBILITY_MIGRATION,
    CHAT_SESSION_PROJECT_CODE_MIGRATION,
    CHAT_USER_INPUT_REQUESTS_MIGRATION, CONVERSATION_RUNTIME_SESSIONS_MIGRATION,
};

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
        if !column_exists(&repository.connection, "chat_sessions", "agent_code")? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_AGENT_CODE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(
            &repository.connection,
            "chat_sessions",
            "system_agent_session_id",
        )? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_SYSTEM_AGENT_SESSION_ID_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_sessions", "source")? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_LEGACY_SOURCE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_sessions", "visibility")? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_VISIBILITY_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_sessions", "project_code")? {
            repository.connection.execute_batch(CHAT_SESSION_PROJECT_CODE_MIGRATION).map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_messages", "input_tokens")? {
            repository
                .connection
                .execute_batch(CHAT_MESSAGE_TOKEN_USAGE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_messages", "attachments_json")? {
            repository
                .connection
                .execute_batch(CHAT_MESSAGE_ATTACHMENTS_MIGRATION)
                .map_err(storage_error)?;
        }
        if !table_exists(&repository.connection, "conversation_runtime_sessions")? {
            repository
                .connection
                .execute_batch(CONVERSATION_RUNTIME_SESSIONS_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_sessions", "source_type")? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_SOURCE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !table_exists(&repository.connection, "chat_user_input_requests")? {
            repository
                .connection
                .execute_batch(CHAT_USER_INPUT_REQUESTS_MIGRATION)
                .map_err(storage_error)?;
        }
        repository
            .connection
            .execute_batch(CHAT_ARCHIVE_CLEANUP_INDEXES_MIGRATION)
            .map_err(storage_error)?;
        Ok(repository)
    }
}
