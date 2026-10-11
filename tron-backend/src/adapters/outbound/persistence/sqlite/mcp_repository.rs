use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::{
    application::mcp::McpRepository,
    domain::mcp::{
        McpError, McpHealthStatus, McpServerAuthKind, McpServerTransportKind,
        PlatformMcpGatewayConfig, PlatformMcpServerRecord, WorkspaceMcpServerPolicy,
    },
    shared::time::{
        option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
        timestamp_to_millis,
    },
};

const MCP_GATEWAY_SINGLETON_KEY: &str = "platform";

struct SqliteMcpRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteMcpRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteMcpRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, McpError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl McpRepository for SqliteMcpRepositorySql<'_> {
    fn get_gateway_config(&mut self) -> Result<Option<PlatformMcpGatewayConfig>, McpError> {
        self.connection
            .query_row(
                "SELECT gateway_enabled, variables_endpoint_enabled, created_at, updated_at
                 FROM platform_mcp_gateway_configs
                 WHERE singleton_key = ?1",
                [MCP_GATEWAY_SINGLETON_KEY],
                |row| {
                    Ok(PlatformMcpGatewayConfig {
                        gateway_enabled: row.get::<_, i64>(0)? != 0,
                        variables_endpoint_enabled: row.get::<_, i64>(1)? != 0,
                        created_at: option_timestamp_from_millis(row.get(2)?),
                        updated_at: option_timestamp_from_millis(row.get(3)?),
                    })
                },
            )
            .optional()
            .map_err(storage_error)
    }

    fn upsert_gateway_config(&mut self, config: &PlatformMcpGatewayConfig) -> Result<(), McpError> {
        let created_at = config
            .created_at
            .as_deref()
            .map(timestamp_to_millis)
            .unwrap_or_else(|| {
                timestamp_to_millis(
                    config
                        .updated_at
                        .as_deref()
                        .unwrap_or("1970-01-01T00:00:00Z"),
                )
            });
        let updated_at = config
            .updated_at
            .as_deref()
            .map(timestamp_to_millis)
            .unwrap_or(created_at);
        self.connection
            .execute(
                "INSERT INTO platform_mcp_gateway_configs (
                    singleton_key,
                    gateway_enabled,
                    variables_endpoint_enabled,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(singleton_key) DO UPDATE SET
                    gateway_enabled = excluded.gateway_enabled,
                    variables_endpoint_enabled = excluded.variables_endpoint_enabled,
                    updated_at = excluded.updated_at",
                params![
                    MCP_GATEWAY_SINGLETON_KEY,
                    i64::from(config.gateway_enabled),
                    i64::from(config.variables_endpoint_enabled),
                    created_at,
                    updated_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_platform_servers(&mut self) -> Result<Vec<PlatformMcpServerRecord>, McpError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT server_code, name, description, transport_kind, url, auth_kind,
                        auth_payload_json, headers_json, headers_variable_refs_json,
                        command, args_json, env_variable_refs_json,
                        env_passthrough_refs_json, working_directory, enabled,
                        last_health_status, last_health_message, last_health_checked_at,
                        created_at, updated_at
                 FROM platform_mcp_servers
                 ORDER BY server_code ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], map_platform_server_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn get_platform_server(
        &mut self,
        server_code: &str,
    ) -> Result<Option<PlatformMcpServerRecord>, McpError> {
        self.connection
            .query_row(
                "SELECT server_code, name, description, transport_kind, url, auth_kind,
                        auth_payload_json, headers_json, headers_variable_refs_json,
                        command, args_json, env_variable_refs_json,
                        env_passthrough_refs_json, working_directory, enabled,
                        last_health_status, last_health_message, last_health_checked_at,
                        created_at, updated_at
                 FROM platform_mcp_servers
                 WHERE server_code = ?1",
                [server_code],
                map_platform_server_row,
            )
            .optional()
            .map_err(storage_error)
    }

    fn upsert_platform_server(&mut self, server: &PlatformMcpServerRecord) -> Result<(), McpError> {
        self.connection
            .execute(
                "INSERT INTO platform_mcp_servers (
                    server_code,
                    name,
                    description,
                    transport_kind,
                    url,
                    auth_kind,
                    auth_payload_json,
                    headers_json,
                    headers_variable_refs_json,
                    command,
                    args_json,
                    env_variable_refs_json,
                    env_passthrough_refs_json,
                    working_directory,
                    enabled,
                    last_health_status,
                    last_health_message,
                    last_health_checked_at,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
                 ON CONFLICT(server_code) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
                    transport_kind = excluded.transport_kind,
                    url = excluded.url,
                    auth_kind = excluded.auth_kind,
                    auth_payload_json = excluded.auth_payload_json,
                    headers_json = excluded.headers_json,
                    headers_variable_refs_json = excluded.headers_variable_refs_json,
                    command = excluded.command,
                    args_json = excluded.args_json,
                    env_variable_refs_json = excluded.env_variable_refs_json,
                    env_passthrough_refs_json = excluded.env_passthrough_refs_json,
                    working_directory = excluded.working_directory,
                    enabled = excluded.enabled,
                    last_health_status = excluded.last_health_status,
                    last_health_message = excluded.last_health_message,
                    last_health_checked_at = excluded.last_health_checked_at,
                    updated_at = excluded.updated_at",
                params![
                    server.server_code,
                    server.name,
                    server.description,
                    transport_kind_to_storage(server.transport_kind),
                    server.url,
                    auth_kind_to_storage(server.auth_kind),
                    option_json_to_storage(server.auth_payload.as_ref())?,
                    headers_to_storage(&server.headers)?,
                    headers_to_storage(&server.headers_variable_refs)?,
                    server.command,
                    json_vec_to_storage(&server.args)?,
                    headers_to_storage(&server.env_variable_refs)?,
                    json_vec_to_storage(&server.env_passthrough_refs)?,
                    server.working_directory,
                    i64::from(server.enabled),
                    health_status_to_storage(server.last_health_status),
                    server.last_health_message,
                    option_timestamp_to_millis(server.last_health_checked_at.as_deref()),
                    timestamp_to_millis(&server.created_at),
                    timestamp_to_millis(&server.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_platform_server(&mut self, server_code: &str) -> Result<bool, McpError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM platform_mcp_servers WHERE server_code = ?1",
                [server_code],
            )
            .map_err(storage_error)?;
        Ok(deleted_rows > 0)
    }

    fn list_workspace_server_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceMcpServerPolicy>, McpError> {
        let sql = if workspace_code.is_some() {
            "SELECT workspace_code, server_code, enabled, created_at, updated_at
             FROM workspace_mcp_server_policies
             WHERE workspace_code = ?1
             ORDER BY server_code ASC"
        } else {
            "SELECT workspace_code, server_code, enabled, created_at, updated_at
             FROM workspace_mcp_server_policies
             ORDER BY workspace_code ASC, server_code ASC"
        };
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        let mapper = |row: &rusqlite::Row<'_>| {
            Ok(WorkspaceMcpServerPolicy {
                workspace_code: row.get(0)?,
                server_code: row.get(1)?,
                enabled: row.get::<_, i64>(2)? != 0,
                created_at: timestamp_from_millis(row.get(3)?),
                updated_at: timestamp_from_millis(row.get(4)?),
            })
        };
        let rows = match workspace_code {
            Some(workspace_code) => statement.query_map([workspace_code], mapper),
            None => statement.query_map([], mapper),
        }
        .map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn upsert_workspace_server_policy(
        &mut self,
        policy: &WorkspaceMcpServerPolicy,
    ) -> Result<(), McpError> {
        self.connection
            .execute(
                "INSERT INTO workspace_mcp_server_policies (
                    workspace_code,
                    server_code,
                    enabled,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(workspace_code, server_code) DO UPDATE SET
                    enabled = excluded.enabled,
                    updated_at = excluded.updated_at",
                params![
                    policy.workspace_code,
                    policy.server_code,
                    i64::from(policy.enabled),
                    timestamp_to_millis(&policy.created_at),
                    timestamp_to_millis(&policy.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_workspace_server_policy(
        &mut self,
        workspace_code: &str,
        server_code: &str,
    ) -> Result<bool, McpError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM workspace_mcp_server_policies
                 WHERE workspace_code = ?1 AND server_code = ?2",
                params![workspace_code, server_code],
            )
            .map_err(storage_error)?;
        Ok(deleted_rows > 0)
    }
}

fn map_platform_server_row(
    row: &rusqlite::Row<'_>,
) -> Result<PlatformMcpServerRecord, rusqlite::Error> {
    Ok(PlatformMcpServerRecord {
        server_code: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        transport_kind: transport_kind_from_storage(&row.get::<_, String>(3)?),
        url: row.get(4)?,
        auth_kind: auth_kind_from_storage(&row.get::<_, String>(5)?),
        auth_payload: option_json_from_storage(row.get(6)?)?,
        headers: headers_from_storage(row.get(7)?)?,
        headers_variable_refs: headers_from_storage(row.get(8)?)?,
        command: row.get(9)?,
        args: json_vec_from_storage(row.get(10)?)?,
        env_variable_refs: headers_from_storage(row.get(11)?)?,
        env_passthrough_refs: json_vec_from_storage(row.get(12)?)?,
        working_directory: row.get(13)?,
        enabled: row.get::<_, i64>(14)? != 0,
        last_health_status: health_status_from_storage(&row.get::<_, String>(15)?),
        last_health_message: row.get(16)?,
        last_health_checked_at: option_timestamp_from_millis(row.get(17)?),
        created_at: timestamp_from_millis(row.get(18)?),
        updated_at: timestamp_from_millis(row.get(19)?),
    })
}

fn storage_error(_error: rusqlite::Error) -> McpError {
    McpError::StorageFailure
}

fn auth_kind_to_storage(auth_kind: McpServerAuthKind) -> &'static str {
    match auth_kind {
        McpServerAuthKind::None => "none",
        McpServerAuthKind::BearerStatic => "bearer_static",
    }
}

fn auth_kind_from_storage(value: &str) -> McpServerAuthKind {
    match value {
        "bearer_static" => McpServerAuthKind::BearerStatic,
        _ => McpServerAuthKind::None,
    }
}

fn transport_kind_to_storage(kind: McpServerTransportKind) -> &'static str {
    match kind {
        McpServerTransportKind::StreamableHttp => "streamable_http",
        McpServerTransportKind::Stdio => "stdio",
    }
}

fn transport_kind_from_storage(value: &str) -> McpServerTransportKind {
    match value {
        "stdio" => McpServerTransportKind::Stdio,
        _ => McpServerTransportKind::StreamableHttp,
    }
}

fn health_status_to_storage(status: McpHealthStatus) -> &'static str {
    match status {
        McpHealthStatus::NeverChecked => "never_checked",
        McpHealthStatus::Healthy => "healthy",
        McpHealthStatus::Unhealthy => "unhealthy",
    }
}

fn health_status_from_storage(value: &str) -> McpHealthStatus {
    match value {
        "healthy" => McpHealthStatus::Healthy,
        "unhealthy" => McpHealthStatus::Unhealthy,
        _ => McpHealthStatus::NeverChecked,
    }
}

fn option_json_to_storage(value: Option<&Value>) -> Result<Option<String>, McpError> {
    value
        .map(|payload| serde_json::to_string(payload).map_err(|_| McpError::StorageFailure))
        .transpose()
}

fn option_json_from_storage(value: Option<String>) -> Result<Option<Value>, rusqlite::Error> {
    value
        .map(|payload| {
            serde_json::from_str::<Value>(&payload).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    payload.len(),
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()
}

fn headers_to_storage(headers: &BTreeMap<String, String>) -> Result<Option<String>, McpError> {
    serde_json::to_string(headers)
        .map(Some)
        .map_err(|_| McpError::StorageFailure)
}

fn json_vec_to_storage(values: &[String]) -> Result<Option<String>, McpError> {
    serde_json::to_string(values)
        .map(Some)
        .map_err(|_| McpError::StorageFailure)
}

fn headers_from_storage(
    value: Option<String>,
) -> Result<BTreeMap<String, String>, rusqlite::Error> {
    match value {
        None => Ok(BTreeMap::new()),
        Some(payload) => {
            serde_json::from_str::<BTreeMap<String, String>>(&payload).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    payload.len(),
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        }
    }
}

fn json_vec_from_storage(value: Option<String>) -> Result<Vec<String>, rusqlite::Error> {
    match value {
        None => Ok(Vec::new()),
        Some(payload) => serde_json::from_str::<Vec<String>>(&payload).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                payload.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        }),
    }
}

impl SqliteMcpRepository {
    pub fn connect(database_url: &str) -> Result<Self, McpError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteMcpRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl McpRepository for SqliteMcpRepository {
    fn get_gateway_config(&mut self) -> Result<Option<PlatformMcpGatewayConfig>, McpError> {
        self.connection.call(
            move |connection| SqliteMcpRepositorySql { connection }.get_gateway_config(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_gateway_config(&mut self, config: &PlatformMcpGatewayConfig) -> Result<(), McpError> {
        let config = config.to_owned();
        self.connection.call(
            move |connection| SqliteMcpRepositorySql { connection }.upsert_gateway_config(&config),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_platform_servers(&mut self) -> Result<Vec<PlatformMcpServerRecord>, McpError> {
        self.connection.call(
            move |connection| SqliteMcpRepositorySql { connection }.list_platform_servers(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn get_platform_server(
        &mut self,
        server_code: &str,
    ) -> Result<Option<PlatformMcpServerRecord>, McpError> {
        let server_code = server_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteMcpRepositorySql { connection }.get_platform_server(&server_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_platform_server(&mut self, server: &PlatformMcpServerRecord) -> Result<(), McpError> {
        let server = server.to_owned();
        self.connection.call(
            move |connection| SqliteMcpRepositorySql { connection }.upsert_platform_server(&server),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_platform_server(&mut self, server_code: &str) -> Result<bool, McpError> {
        let server_code = server_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteMcpRepositorySql { connection }.delete_platform_server(&server_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_workspace_server_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceMcpServerPolicy>, McpError> {
        let workspace_code = workspace_code.map(ToOwned::to_owned);
        self.connection.call(
            move |connection| {
                SqliteMcpRepositorySql { connection }
                    .list_workspace_server_policies(workspace_code.as_deref())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_workspace_server_policy(
        &mut self,
        policy: &WorkspaceMcpServerPolicy,
    ) -> Result<(), McpError> {
        let policy = policy.to_owned();
        self.connection.call(
            move |connection| {
                SqliteMcpRepositorySql { connection }.upsert_workspace_server_policy(&policy)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_workspace_server_policy(
        &mut self,
        workspace_code: &str,
        server_code: &str,
    ) -> Result<bool, McpError> {
        let workspace_code = workspace_code.to_owned();
        let server_code = server_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteMcpRepositorySql { connection }
                    .delete_workspace_server_policy(&workspace_code, &server_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
