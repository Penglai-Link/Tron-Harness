use std::{fs, path::Path};

use rusqlite::{Error as SqliteError, OptionalExtension, params};

use crate::{
    application::resource::ResourceRepository,
    domain::resource::{
        ChatSessionAttachmentRecord, ResourceError, ResourceFileRecord,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

mod rows;
mod schema;

use rows::{bool_to_int, file_from_row};
use schema::{claim_resource_code, release_resource_code};

pub struct SqliteResourceRepository {
    connection: super::framework_storage::SharedDatabase,
}

impl SqliteResourceRepository {
    pub fn connect(database_url: &str) -> Result<Self, ResourceError> {
        ensure_database_parent(database_url)?;
        let connection =
            super::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        Ok(Self { connection })
    }

}
