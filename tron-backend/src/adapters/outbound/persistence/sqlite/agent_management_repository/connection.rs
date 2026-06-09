use super::*;

impl SqliteAgentManagementRepository {
    pub fn connect(database_url: &str) -> Result<Self, AgentManagementError> {
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
