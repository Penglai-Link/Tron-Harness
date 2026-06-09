mod codecs;
mod connection;
#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::{
    application::agent::AgentManagementRepository,
    domain::agent::{
        AgentManagementError, ManagedAgentSource, PlatformMainAgentConfig,
        SystemAgentRegistryEntry, WorkspaceInternalAgentMcpServerAssignment,
        WorkspaceInternalAgentPolicy, WorkspaceInternalAgentToolAssignment,
        WorkspaceSystemAgentPolicy,
    },
    shared::time::{
        option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
        timestamp_to_millis,
    },
};

use self::codecs::{
    column_exists, ensure_database_parent, expanded_domain_persistence_mode_from_storage,
    expanded_domain_persistence_mode_to_storage, mcp_access_mode_from_storage,
    mcp_access_mode_to_storage, runtime_type_from_storage, runtime_type_to_storage, storage_error,
    tool_assignment_mode_from_storage, tool_assignment_mode_to_storage,
    tool_exposure_mode_from_storage, tool_exposure_mode_to_storage,
};

use super::migrations::{
    AGENT_MANAGEMENT_INTERNAL_AGENT_LIMITS_MIGRATION,
    AGENT_MANAGEMENT_INTERNAL_AGENT_MCP_MIGRATION, AGENT_MANAGEMENT_INTERNAL_AGENT_MIGRATION,
    AGENT_MANAGEMENT_INTERNAL_AGENT_PARALLEL_LIMIT_MIGRATION,
};
use super::migrations::{AGENT_MANAGEMENT_MIGRATION, AGENT_MANAGEMENT_RUNTIME_TYPE_MIGRATION};

const MAIN_AGENT_SINGLETON_KEY: &str = "platform";

pub struct SqliteAgentManagementRepository {
    connection: Connection,
}

impl AgentManagementRepository for SqliteAgentManagementRepository {
    fn get_main_agent_config(
        &mut self,
    ) -> Result<Option<PlatformMainAgentConfig>, AgentManagementError> {
        self.connection
            .query_row(
                "SELECT personality, instructions, created_at, updated_at
                 FROM platform_main_agent_configs
                 WHERE singleton_key = ?1",
                [MAIN_AGENT_SINGLETON_KEY],
                |row| {
                    Ok(PlatformMainAgentConfig {
                        personality: row.get(0)?,
                        instructions: row.get(1)?,
                        created_at: option_timestamp_from_millis(row.get(2)?),
                        updated_at: option_timestamp_from_millis(row.get(3)?),
                    })
                },
            )
            .optional()
            .map_err(storage_error)
    }

    fn upsert_main_agent_config(
        &mut self,
        config: &PlatformMainAgentConfig,
    ) -> Result<(), AgentManagementError> {
        let created_at = config
            .created_at
            .as_deref()
            .map(timestamp_to_millis)
            .unwrap_or_else(|| timestamp_to_millis(config.updated_at.as_deref().unwrap_or("")));
        let updated_at = config
            .updated_at
            .as_deref()
            .map(timestamp_to_millis)
            .unwrap_or(created_at);
        self.connection
            .execute(
                "INSERT INTO platform_main_agent_configs (
                    singleton_key,
                    personality,
                    instructions,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(singleton_key) DO UPDATE SET
                    personality = excluded.personality,
                    instructions = excluded.instructions,
                    updated_at = excluded.updated_at",
                params![
                    MAIN_AGENT_SINGLETON_KEY,
                    config.personality,
                    config.instructions,
                    created_at,
                    updated_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_system_agents(
        &mut self,
    ) -> Result<Vec<SystemAgentRegistryEntry>, AgentManagementError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT agent_code, name, description, runtime_type, command_name, source, enabled, available, last_scanned_at, last_seen_at, last_error
                 FROM system_agent_registries
                 ORDER BY agent_code ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(SystemAgentRegistryEntry {
                    agent_code: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    runtime_type: runtime_type_from_storage(&row.get::<_, String>(3)?),
                    command_name: row.get(4)?,
                    source: match row.get::<_, String>(5)?.as_str() {
                        "SYSTEM" => ManagedAgentSource::System,
                        _ => ManagedAgentSource::System,
                    },
                    enabled: row.get::<_, i64>(6)? != 0,
                    available: row.get::<_, i64>(7)? != 0,
                    last_scanned_at: option_timestamp_from_millis(row.get(8)?),
                    last_seen_at: option_timestamp_from_millis(row.get(9)?),
                    last_error: row.get(10)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn upsert_system_agent(
        &mut self,
        agent: &SystemAgentRegistryEntry,
    ) -> Result<(), AgentManagementError> {
        self.connection
            .execute(
                "INSERT INTO system_agent_registries (
                    agent_code,
                    name,
                    description,
                    runtime_type,
                    command_name,
                    source,
                    enabled,
                    available,
                    last_scanned_at,
                    last_seen_at,
                    last_error
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(agent_code) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
                    runtime_type = excluded.runtime_type,
                    command_name = excluded.command_name,
                    source = excluded.source,
                    enabled = excluded.enabled,
                    available = excluded.available,
                    last_scanned_at = excluded.last_scanned_at,
                    last_seen_at = excluded.last_seen_at,
                    last_error = excluded.last_error",
                params![
                    agent.agent_code,
                    agent.name,
                    agent.description,
                    runtime_type_to_storage(agent.runtime_type),
                    agent.command_name,
                    "SYSTEM",
                    i64::from(agent.enabled),
                    i64::from(agent.available),
                    option_timestamp_to_millis(agent.last_scanned_at.as_deref()),
                    option_timestamp_to_millis(agent.last_seen_at.as_deref()),
                    agent.last_error,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_workspace_system_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceSystemAgentPolicy>, AgentManagementError> {
        let sql = if workspace_code.is_some() {
            "SELECT workspace_code, agent_code, enabled, created_at, updated_at
             FROM workspace_system_agent_policies
             WHERE workspace_code = ?1
             ORDER BY agent_code ASC"
        } else {
            "SELECT workspace_code, agent_code, enabled, created_at, updated_at
             FROM workspace_system_agent_policies
             ORDER BY workspace_code ASC, agent_code ASC"
        };
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        let mapper = |row: &rusqlite::Row<'_>| {
            Ok(WorkspaceSystemAgentPolicy {
                workspace_code: row.get(0)?,
                agent_code: row.get(1)?,
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

    fn upsert_workspace_system_policy(
        &mut self,
        policy: &WorkspaceSystemAgentPolicy,
    ) -> Result<(), AgentManagementError> {
        self.connection
            .execute(
                "INSERT INTO workspace_system_agent_policies (
                    workspace_code,
                    agent_code,
                    enabled,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(workspace_code, agent_code) DO UPDATE SET
                    enabled = excluded.enabled,
                    updated_at = excluded.updated_at",
                params![
                    policy.workspace_code,
                    policy.agent_code,
                    i64::from(policy.enabled),
                    timestamp_to_millis(&policy.created_at),
                    timestamp_to_millis(&policy.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_workspace_system_policy(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<bool, AgentManagementError> {
        let deleted = self
            .connection
            .execute(
                "DELETE FROM workspace_system_agent_policies
                 WHERE workspace_code = ?1 AND agent_code = ?2",
                params![workspace_code, agent_code],
            )
            .map_err(storage_error)?;
        Ok(deleted > 0)
    }

    fn list_workspace_internal_agent_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentPolicy>, AgentManagementError> {
        let sql = match workspace_code {
            Some(_workspace_code) => {
                "SELECT workspace_code, agent_code, tool_assignment_mode, mcp_access_mode, tool_exposure_mode, expanded_domain_persistence_mode, max_steps, max_tool_calls, max_parallel_tool_calls, created_at, updated_at
                 FROM workspace_internal_agent_policies
                 WHERE workspace_code = ?1
                 ORDER BY agent_code ASC"
            }
            None => {
                "SELECT workspace_code, agent_code, tool_assignment_mode, mcp_access_mode, tool_exposure_mode, expanded_domain_persistence_mode, max_steps, max_tool_calls, max_parallel_tool_calls, created_at, updated_at
                 FROM workspace_internal_agent_policies
                 ORDER BY workspace_code ASC, agent_code ASC"
            }
        };
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(WorkspaceInternalAgentPolicy {
                workspace_code: row.get(0)?,
                agent_code: row.get(1)?,
                tool_assignment_mode: tool_assignment_mode_from_storage(row.get::<_, String>(2)?),
                mcp_access_mode: mcp_access_mode_from_storage(row.get::<_, String>(3)?),
                tool_exposure_mode: tool_exposure_mode_from_storage(row.get::<_, String>(4)?),
                expanded_domain_persistence_mode: expanded_domain_persistence_mode_from_storage(
                    row.get::<_, String>(5)?,
                ),
                max_steps: row.get(6)?,
                max_tool_calls: row.get(7)?,
                max_parallel_tool_calls: row.get(8)?,
                created_at: timestamp_from_millis(row.get(9)?),
                updated_at: timestamp_from_millis(row.get(10)?),
            })
        };
        let rows = if let Some(workspace_code) = workspace_code {
            statement.query_map(params![workspace_code], map_row)
        } else {
            statement.query_map(params![], map_row)
        };
        rows.map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn upsert_workspace_internal_agent_policy(
        &mut self,
        policy: &WorkspaceInternalAgentPolicy,
    ) -> Result<(), AgentManagementError> {
        self.connection
            .execute(
                "INSERT INTO workspace_internal_agent_policies (
                    workspace_code,
                    agent_code,
                    tool_assignment_mode,
                    mcp_access_mode,
                    tool_exposure_mode,
                    expanded_domain_persistence_mode,
                    max_steps,
                    max_tool_calls,
                    max_parallel_tool_calls,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(workspace_code, agent_code) DO UPDATE SET
                    tool_assignment_mode = excluded.tool_assignment_mode,
                    mcp_access_mode = excluded.mcp_access_mode,
                    tool_exposure_mode = excluded.tool_exposure_mode,
                    expanded_domain_persistence_mode = excluded.expanded_domain_persistence_mode,
                    max_steps = excluded.max_steps,
                    max_tool_calls = excluded.max_tool_calls,
                    max_parallel_tool_calls = excluded.max_parallel_tool_calls,
                    updated_at = excluded.updated_at",
                params![
                    policy.workspace_code,
                    policy.agent_code,
                    tool_assignment_mode_to_storage(policy.tool_assignment_mode),
                    mcp_access_mode_to_storage(policy.mcp_access_mode),
                    tool_exposure_mode_to_storage(policy.tool_exposure_mode),
                    expanded_domain_persistence_mode_to_storage(
                        policy.expanded_domain_persistence_mode
                    ),
                    policy.max_steps,
                    policy.max_tool_calls,
                    policy.max_parallel_tool_calls,
                    timestamp_to_millis(&policy.created_at),
                    timestamp_to_millis(&policy.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_workspace_internal_agent_policy(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<bool, AgentManagementError> {
        let deleted = self
            .connection
            .execute(
                "DELETE FROM workspace_internal_agent_policies
                 WHERE workspace_code = ?1 AND agent_code = ?2",
                params![workspace_code, agent_code],
            )
            .map_err(storage_error)?;
        Ok(deleted > 0)
    }

    fn list_workspace_internal_agent_tool_assignments(
        &mut self,
        workspace_code: Option<&str>,
        agent_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentToolAssignment>, AgentManagementError> {
        let (sql, arguments): (&str, Option<Vec<&str>>) = match (workspace_code, agent_code) {
            (Some(workspace_code), Some(agent_code)) => (
                "SELECT workspace_code, agent_code, tool_code, created_at, updated_at
                 FROM workspace_internal_agent_tool_assignments
                WHERE workspace_code = ?1 AND agent_code = ?2
                ORDER BY tool_code ASC",
                Some(vec![workspace_code, agent_code]),
            ),
            (Some(workspace_code), None) => (
                "SELECT workspace_code, agent_code, tool_code, created_at, updated_at
                 FROM workspace_internal_agent_tool_assignments
                WHERE workspace_code = ?1
                ORDER BY agent_code ASC, tool_code ASC",
                Some(vec![workspace_code]),
            ),
            (None, Some(agent_code)) => (
                "SELECT workspace_code, agent_code, tool_code, created_at, updated_at
                 FROM workspace_internal_agent_tool_assignments
                WHERE agent_code = ?1
                ORDER BY workspace_code ASC, agent_code ASC, tool_code ASC",
                Some(vec![agent_code]),
            ),
            (None, None) => (
                "SELECT workspace_code, agent_code, tool_code, created_at, updated_at
                 FROM workspace_internal_agent_tool_assignments
                ORDER BY workspace_code ASC, agent_code ASC, tool_code ASC",
                None,
            ),
        };
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(WorkspaceInternalAgentToolAssignment {
                workspace_code: row.get(0)?,
                agent_code: row.get(1)?,
                tool_code: row.get(2)?,
                created_at: timestamp_from_millis(row.get(3)?),
                updated_at: timestamp_from_millis(row.get(4)?),
            })
        };
        let rows = if let Some(args) = arguments {
            statement.query_map(params_from_iter(args), map_row)
        } else {
            statement.query_map(params![], map_row)
        };
        let rows = rows.map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn upsert_workspace_internal_agent_tool_assignment(
        &mut self,
        assignment: &WorkspaceInternalAgentToolAssignment,
    ) -> Result<(), AgentManagementError> {
        self.connection
            .execute(
                "INSERT INTO workspace_internal_agent_tool_assignments (
                    workspace_code,
                    agent_code,
                    tool_code,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(workspace_code, agent_code, tool_code) DO UPDATE SET
                    updated_at = excluded.updated_at",
                params![
                    assignment.workspace_code,
                    assignment.agent_code,
                    assignment.tool_code,
                    timestamp_to_millis(&assignment.created_at),
                    timestamp_to_millis(&assignment.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_workspace_internal_agent_tool_assignments(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<(), AgentManagementError> {
        self.connection
            .execute(
                "DELETE FROM workspace_internal_agent_tool_assignments
                 WHERE workspace_code = ?1 AND agent_code = ?2",
                params![workspace_code, agent_code],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_workspace_internal_agent_mcp_server_assignments(
        &mut self,
        workspace_code: Option<&str>,
        agent_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentMcpServerAssignment>, AgentManagementError> {
        let (sql, arguments): (&str, Option<Vec<&str>>) = match (workspace_code, agent_code) {
            (Some(workspace_code), Some(agent_code)) => (
                "SELECT workspace_code, agent_code, server_code, created_at, updated_at
                 FROM workspace_internal_agent_mcp_server_assignments
                 WHERE workspace_code = ?1 AND agent_code = ?2
                 ORDER BY server_code ASC",
                Some(vec![workspace_code, agent_code]),
            ),
            (Some(workspace_code), None) => (
                "SELECT workspace_code, agent_code, server_code, created_at, updated_at
                 FROM workspace_internal_agent_mcp_server_assignments
                 WHERE workspace_code = ?1
                 ORDER BY agent_code ASC, server_code ASC",
                Some(vec![workspace_code]),
            ),
            (None, Some(agent_code)) => (
                "SELECT workspace_code, agent_code, server_code, created_at, updated_at
                 FROM workspace_internal_agent_mcp_server_assignments
                 WHERE agent_code = ?1
                 ORDER BY workspace_code ASC, agent_code ASC, server_code ASC",
                Some(vec![agent_code]),
            ),
            (None, None) => (
                "SELECT workspace_code, agent_code, server_code, created_at, updated_at
                 FROM workspace_internal_agent_mcp_server_assignments
                 ORDER BY workspace_code ASC, agent_code ASC, server_code ASC",
                None,
            ),
        };
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(WorkspaceInternalAgentMcpServerAssignment {
                workspace_code: row.get(0)?,
                agent_code: row.get(1)?,
                server_code: row.get(2)?,
                created_at: timestamp_from_millis(row.get(3)?),
                updated_at: timestamp_from_millis(row.get(4)?),
            })
        };
        let rows = if let Some(args) = arguments {
            statement.query_map(params_from_iter(args), map_row)
        } else {
            statement.query_map(params![], map_row)
        };
        rows.map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn upsert_workspace_internal_agent_mcp_server_assignment(
        &mut self,
        assignment: &WorkspaceInternalAgentMcpServerAssignment,
    ) -> Result<(), AgentManagementError> {
        self.connection
            .execute(
                "INSERT INTO workspace_internal_agent_mcp_server_assignments (
                    workspace_code,
                    agent_code,
                    server_code,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(workspace_code, agent_code, server_code) DO UPDATE SET
                    updated_at = excluded.updated_at",
                params![
                    assignment.workspace_code,
                    assignment.agent_code,
                    assignment.server_code,
                    timestamp_to_millis(&assignment.created_at),
                    timestamp_to_millis(&assignment.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_workspace_internal_agent_mcp_server_assignments(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<(), AgentManagementError> {
        self.connection
            .execute(
                "DELETE FROM workspace_internal_agent_mcp_server_assignments
                 WHERE workspace_code = ?1 AND agent_code = ?2",
                params![workspace_code, agent_code],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}
