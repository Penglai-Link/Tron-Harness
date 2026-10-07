use super::*;

impl<'a> SqliteAgentManagementRepositorySql<'a> {
    pub fn initialize(
        database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, AgentManagementError> {
        ensure_database_parent(database_url)?;

        let repository = Self { connection };
        repository
            .connection
            .execute_batch(AGENT_MANAGEMENT_MIGRATION)
            .map_err(storage_error)?;
        if !column_exists(
            &repository.connection,
            "system_agent_registries",
            "runtime_type",
        )? {
            repository
                .connection
                .execute_batch(AGENT_MANAGEMENT_RUNTIME_TYPE_MIGRATION)
                .map_err(storage_error)?;
        }
        repository
            .connection
            .execute_batch(AGENT_MANAGEMENT_INTERNAL_AGENT_MIGRATION)
            .map_err(storage_error)?;
        if !column_exists(
            &repository.connection,
            "workspace_internal_agent_policies",
            "max_steps",
        )? {
            repository
                .connection
                .execute_batch(AGENT_MANAGEMENT_INTERNAL_AGENT_LIMITS_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(
            &repository.connection,
            "workspace_internal_agent_policies",
            "max_parallel_tool_calls",
        )? {
            repository
                .connection
                .execute_batch(AGENT_MANAGEMENT_INTERNAL_AGENT_PARALLEL_LIMIT_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(
            &repository.connection,
            "workspace_internal_agent_policies",
            "mcp_access_mode",
        )? {
            repository
                .connection
                .execute_batch(AGENT_MANAGEMENT_INTERNAL_AGENT_MCP_MIGRATION)
                .map_err(storage_error)?;
        }
        Ok(repository)
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
