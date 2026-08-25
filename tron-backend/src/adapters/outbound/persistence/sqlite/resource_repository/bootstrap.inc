use std::{fs, path::Path};

use rusqlite::{Connection, Error as SqliteError, OptionalExtension, params};

use crate::{
    application::resource::ResourceRepository,
    domain::resource::{
        ChatSessionAttachmentRecord, DataSourceConnectionRecord, ResourceError, ResourceFileRecord,
        ResourceKind, ServerAccountRecord, ServerRecord,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

use super::migrations::{
    CHAT_SESSION_ATTACHMENTS_RESOURCE_MIGRATION, RESOURCE_MANAGEMENT_SIMPLIFY_MIGRATION,
};

mod legacy;
mod rows;
mod schema;

use legacy::migrate_legacy_active_resources;
use rows::{
    authentication_type_to_storage, bool_to_int, data_source_from_row, file_from_row,
    insert_server_account_record, insert_server_record, server_account_from_row, server_from_row,
    value_to_storage,
};
use schema::{
    backfill_resource_code_registry, claim_resource_code, ensure_revision_columns,
    ensure_server_account_origin_column, ensure_server_account_parent_guards,
    release_resource_code,
};

const RESOURCE_MANAGEMENT_SIMPLIFY_VERSION: &str = "20260818_0001";
const CHAT_SESSION_ATTACHMENTS_RESOURCE_VERSION: &str = "20260820_0001";

pub struct SqliteResourceRepository {
    connection: Connection,
}

impl SqliteResourceRepository {
    pub fn connect(database_url: &str) -> Result<Self, ResourceError> {
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
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(storage_error)?;
        let mut repository = Self { connection };
        repository.apply_migrations()?;
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), ResourceError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS resource_management_migrations (
                    version TEXT PRIMARY KEY,
                    applied_at INTEGER NOT NULL
                );",
            )
            .map_err(storage_error)?;
        let applied = transaction
            .query_row(
                "SELECT 1 FROM resource_management_migrations WHERE version = ?1",
                [RESOURCE_MANAGEMENT_SIMPLIFY_VERSION],
                |_| Ok(()),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !applied {
            transaction
                .execute_batch(RESOURCE_MANAGEMENT_SIMPLIFY_MIGRATION)
                .map_err(storage_error)?;
        }
        ensure_revision_columns(&transaction)?;
        ensure_server_account_origin_column(&transaction)?;
        if !applied {
            migrate_legacy_active_resources(&transaction)?;
            backfill_resource_code_registry(&transaction)?;
            transaction
                .execute(
                    "INSERT INTO resource_management_migrations (version, applied_at)
                     VALUES (?1, strftime('%s', 'now') * 1000)",
                    [RESOURCE_MANAGEMENT_SIMPLIFY_VERSION],
                )
                .map_err(storage_error)?;
        }
        let session_attachments_applied = transaction
            .query_row(
                "SELECT 1 FROM resource_management_migrations WHERE version = ?1",
                [CHAT_SESSION_ATTACHMENTS_RESOURCE_VERSION],
                |_| Ok(()),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !session_attachments_applied {
            transaction
                .execute_batch(CHAT_SESSION_ATTACHMENTS_RESOURCE_MIGRATION)
                .map_err(storage_error)?;
            transaction
                .execute(
                    "INSERT INTO resource_management_migrations (version, applied_at)
                     VALUES (?1, strftime('%s', 'now') * 1000)",
                    [CHAT_SESSION_ATTACHMENTS_RESOURCE_VERSION],
                )
                .map_err(storage_error)?;
        }
        ensure_server_account_parent_guards(&transaction)?;
        transaction.commit().map_err(storage_error)
    }
}
