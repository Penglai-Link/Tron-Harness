use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    application::user::UserRepository,
    domain::{
        auth::{AuthError, AuthMethod, OAuth2IdentityLink, RefreshSession},
        user::{
            ListUsersQuery, PagedUsers, User, UserError, UserSourceType, UserStatistics,
            UserStatus, normalize_code_segment,
        },
    },
    shared::id::SnowflakeId,
};

mod helpers;
mod repository_impl;
#[cfg(test)]
mod tests;
use super::{
    migrations::{
        ASSIGNMENTS_MIGRATION, AUTH_OAUTH_IDENTITIES_MIGRATION,
        AUTH_REFRESH_SESSIONS_LDAP_MIGRATION, AUTH_REFRESH_SESSIONS_MIGRATION,
        PASSWORD_RESET_MIGRATION, USERS_MIGRATION,
    },
    storage_mapping::{
        StoredUserRow, bool_to_int, ensure_database_parent, phone_from_parts, storage_error,
        timestamp_from_millis, timestamp_to_millis, user_source_type_from_provider_code,
        user_status_from_storage, user_status_to_storage,
    },
};

pub struct SqliteUserRepository {
    connection: Connection,
}

impl SqliteUserRepository {
    pub fn connect(database_url: &str) -> Result<Self, UserError> {
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
        repository.apply_migrations()?;
        Ok(repository)
    }
}
