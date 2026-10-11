use rusqlite::{Connection, params};

use crate::{
    application::model::ModelRepository,
    domain::model::{
        Model, ModelCapability, ModelError, ModelProtocol, ModelScopeKind, ModelSourceKind,
        ModelType, ModelVerifyStatus, PlatformDefaultModelAssignment,
        WorkspaceDefaultModelAssignment, default_model_capabilities_for_type,
    },
    domain::reasoning::ReasoningEffort,
    shared::{
        id::SnowflakeId,
        time::{
            option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
            timestamp_to_millis,
        },
    },
};

mod repository_impl;
mod storage;

use storage::storage_error;

const MODEL_ID_SEED: i64 = 734_600_000_000_000_000;

struct SqliteModelRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteModelRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteModelRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, ModelError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl SqliteModelRepository {
    pub fn connect(database_url: &str) -> Result<Self, ModelError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteModelRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
