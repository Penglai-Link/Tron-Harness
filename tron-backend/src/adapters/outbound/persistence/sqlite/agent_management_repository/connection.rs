use super::*;

impl<'a> SqliteAgentManagementRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, AgentManagementError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl SqliteAgentManagementRepository {
    pub fn connect(database_url: &str) -> Result<Self, AgentManagementError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql::initialize(&database_url, connection)
                    .map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
