use std::{fs, path::Path};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    application::agent::AgentManagementRepository,
    domain::agent::{
        AgentManagementError, ManagedAgentSource, PlatformMainAgentConfig,
        SystemAgentRegistryEntry, WorkspaceSystemAgentPolicy,
    },
    shared::time::{
        option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
        timestamp_to_millis,
    },
};

use super::migrations::AGENT_MANAGEMENT_MIGRATION;

const MAIN_AGENT_SINGLETON_KEY: &str = "platform";

pub struct SqliteAgentManagementRepository {
    connection: Connection,
}

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
        Ok(repository)
    }
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
                "SELECT agent_code, name, description, command_name, source, enabled, available, last_scanned_at, last_seen_at, last_error
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
                    command_name: row.get(3)?,
                    source: match row.get::<_, String>(4)?.as_str() {
                        "SYSTEM" => ManagedAgentSource::System,
                        _ => ManagedAgentSource::System,
                    },
                    enabled: row.get::<_, i64>(5)? != 0,
                    available: row.get::<_, i64>(6)? != 0,
                    last_scanned_at: option_timestamp_from_millis(row.get(7)?),
                    last_seen_at: option_timestamp_from_millis(row.get(8)?),
                    last_error: row.get(9)?,
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
                    command_name,
                    source,
                    enabled,
                    available,
                    last_scanned_at,
                    last_seen_at,
                    last_error
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(agent_code) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
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
}

fn ensure_database_parent(database_url: &str) -> Result<(), AgentManagementError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| AgentManagementError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(_error: rusqlite::Error) -> AgentManagementError {
    AgentManagementError::StorageFailure
}
