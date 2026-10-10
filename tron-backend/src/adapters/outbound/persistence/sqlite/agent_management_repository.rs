mod codecs;
mod connection;

use rusqlite::{Connection, params, params_from_iter};

use crate::{
    application::agent::AgentManagementRepository,
    domain::agent::{
        AgentManagementError, ManagedAgentSource, PlatformInternalAgentConfig,
        SystemAgentRegistryEntry, WorkspaceInternalAgentDefinition,
        WorkspaceInternalAgentMcpServerAssignment, WorkspaceInternalAgentPolicy,
        WorkspaceInternalAgentToolAssignment, WorkspaceSystemAgentPolicy,
    },
    shared::time::{
        option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
        timestamp_to_millis,
    },
};

use self::codecs::{
    expanded_domain_persistence_mode_from_storage,
    expanded_domain_persistence_mode_to_storage, mcp_access_mode_from_storage,
    mcp_access_mode_to_storage, runtime_type_from_storage, runtime_type_to_storage, storage_error,
    tool_assignment_mode_from_storage, tool_assignment_mode_to_storage,
    tool_exposure_mode_from_storage, tool_exposure_mode_to_storage,
};

struct SqliteAgentManagementRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteAgentManagementRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

fn ensure_internal_agent_code_available(
    connection: &Connection,
    workspace_code: Option<&str>,
    agent_code: &str,
) -> Result<(), AgentManagementError> {
    // Global creation must not turn an existing local definition into an override.
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM platform_internal_agent_configs WHERE agent_code = ?1)
         OR EXISTS(SELECT 1 FROM workspace_internal_agent_definitions
                   WHERE agent_code = ?1 AND (?2 IS NULL OR workspace_code = ?2))
         OR EXISTS(SELECT 1 FROM system_agent_registries WHERE agent_code = ?1)",
            params![agent_code, workspace_code],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    if exists {
        Err(AgentManagementError::InternalAgentCodeConflict)
    } else {
        Ok(())
    }
}

impl AgentManagementRepository for SqliteAgentManagementRepositorySql<'_> {
    fn list_internal_agent_groups(
        &mut self,
        scope: &str,
    ) -> Result<Vec<String>, AgentManagementError> {
        self.connection
            .prepare("SELECT name FROM internal_agent_groups WHERE scope_code = ?1 ORDER BY name")
            .map_err(storage_error)?
            .query_map([scope], |row| row.get(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }
    fn save_internal_agent_group(
        &mut self,
        scope: &str,
        name: &str,
    ) -> Result<(), AgentManagementError> {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO internal_agent_groups(scope_code, name) VALUES (?1, ?2)",
                params![scope, name],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_tool_permissions(
        &mut self,
    ) -> Result<Vec<tron_harness_capability::ToolPermissionSetting>, AgentManagementError> {
        let mut statement = self
            .connection
            .prepare("SELECT config_json FROM platform_tool_permissions ORDER BY tool_code")
            .map_err(storage_error)?;
        statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(storage_error)?
            .map(|r| {
                serde_json::from_str(&r.map_err(storage_error)?)
                    .map_err(|_| AgentManagementError::StorageFailure)
            })
            .collect()
    }
    fn replace_tool_permissions(
        &mut self,
        settings: &[tron_harness_capability::ToolPermissionSetting],
    ) -> Result<(), AgentManagementError> {
        let tx = super::framework_storage::business_transaction(
            self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(storage_error)?;
        tx.execute("DELETE FROM platform_tool_permissions", [])
            .map_err(storage_error)?;
        for s in settings {
            let json =
                serde_json::to_string(s).map_err(|_| AgentManagementError::StorageFailure)?;
            tx.execute(
                "INSERT INTO platform_tool_permissions VALUES(?1,?2)",
                params![s.tool_code, json],
            )
            .map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)
    }
    fn list_discovered_tools(
        &mut self,
    ) -> Result<Vec<crate::domain::agent::ToolCatalogRecord>, AgentManagementError> {
        let mut statement = self
            .connection
            .prepare("SELECT config_json FROM platform_discovered_tools ORDER BY tool_code")
            .map_err(storage_error)?;
        statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(storage_error)?
            .map(|r| {
                serde_json::from_str(&r.map_err(storage_error)?)
                    .map_err(|_| AgentManagementError::StorageFailure)
            })
            .collect()
    }
    fn remember_discovered_tool(
        &mut self,
        tool: &crate::domain::agent::ToolCatalogRecord,
    ) -> Result<(), AgentManagementError> {
        let json = serde_json::to_string(tool).map_err(|_| AgentManagementError::StorageFailure)?;
        self.connection.execute("INSERT INTO platform_discovered_tools VALUES(?1,?2) ON CONFLICT(tool_code) DO UPDATE SET config_json=excluded.config_json WHERE config_json<>excluded.config_json",params![tool.tool_code,json]).map_err(storage_error)?;
        Ok(())
    }

    fn list_loop_designs(
        &mut self,
    ) -> Result<Vec<crate::domain::agent::loop_design::LoopDesign>, AgentManagementError> {
        let mut statement = self
            .connection
            .prepare("SELECT config_json FROM agent_loop_designs ORDER BY loop_code")
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?;
        rows.map(|row| {
            serde_json::from_str(&row.map_err(storage_error)?)
                .map_err(|_| AgentManagementError::StorageFailure)
        })
        .collect()
    }
    fn save_loop_design(
        &mut self,
        design: &crate::domain::agent::loop_design::LoopDesign,
        expected: u32,
    ) -> Result<(), AgentManagementError> {
        let json =
            serde_json::to_string(design).map_err(|_| AgentManagementError::StorageFailure)?;
        let count = if expected == 0 {
            self.connection.execute("INSERT OR IGNORE INTO agent_loop_designs(loop_code, version, config_json) VALUES(?1, ?2, ?3)", params![design.loop_code, design.version, json])
        } else {
            self.connection.execute("UPDATE agent_loop_designs SET version=?2, config_json=?3 WHERE loop_code=?1 AND version=?4", params![design.loop_code, design.version, json, expected])
        }.map_err(storage_error)?;
        if count == 0 {
            return Err(AgentManagementError::LoopConflict);
        }
        Ok(())
    }
    fn delete_loop_design(&mut self, code: &str) -> Result<(), AgentManagementError> {
        let transaction = super::framework_storage::business_transaction(
            self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(storage_error)?;
        let used: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM workspace_internal_agent_definitions WHERE loop_code=?1) OR EXISTS(SELECT 1 FROM platform_internal_agent_configs WHERE json_extract(config_json, '$.loopCode')=?1)", [code], |r| r.get(0)).map_err(storage_error)?;
        if used {
            return Err(AgentManagementError::LoopInUse);
        }
        if transaction
            .execute("DELETE FROM agent_loop_designs WHERE loop_code=?1", [code])
            .map_err(storage_error)?
            == 0
        {
            return Err(AgentManagementError::LoopNotFound);
        }
        transaction.commit().map_err(storage_error)
    }

    fn agent_workspace_grants(
        &mut self,
        agent: &str,
    ) -> Result<Option<Vec<String>>, AgentManagementError> {
        let explicit: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM agent_workspace_grant_policies WHERE agent_code=?1)",
                [agent],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if !explicit {
            return Ok(None);
        }
        let mut statement = self.connection.prepare("SELECT workspace_code FROM agent_workspace_grants WHERE agent_code=?1 ORDER BY workspace_code").map_err(storage_error)?;
        statement
            .query_map([agent], |row| row.get(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
            .map_err(storage_error)
    }
    fn replace_agent_workspace_grants(
        &mut self,
        agent: &str,
        workspaces: &[String],
    ) -> Result<(), AgentManagementError> {
        let tx = super::framework_storage::business_transaction(
            self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(storage_error)?;
        tx.execute(
            "INSERT OR IGNORE INTO agent_workspace_grant_policies VALUES(?1)",
            [agent],
        )
        .map_err(storage_error)?;
        tx.execute(
            "DELETE FROM agent_workspace_grants WHERE agent_code=?1",
            [agent],
        )
        .map_err(storage_error)?;
        for workspace in workspaces {
            tx.execute(
                "INSERT INTO agent_workspace_grants VALUES(?1,?2)",
                params![agent, workspace],
            )
            .map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)
    }

    fn insert_platform_internal_agent_config(
        &mut self,
        config: &PlatformInternalAgentConfig,
    ) -> Result<(), AgentManagementError> {
        let json =
            serde_json::to_string(config).map_err(|_| AgentManagementError::StorageFailure)?;
        let transaction = self.connection.transaction().map_err(storage_error)?;
        ensure_internal_agent_code_available(&transaction, None, &config.agent_code)?;
        transaction.execute(
            "INSERT INTO platform_internal_agent_configs (agent_code, config_json) VALUES (?1, ?2)",
            params![config.agent_code, json],
        ).map_err(storage_error)?;
        transaction
            .execute(
                "INSERT INTO agent_workspace_grant_policies VALUES(?1)",
                [&config.agent_code],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    fn insert_workspace_internal_agent_definition(
        &mut self,
        definition: &WorkspaceInternalAgentDefinition,
    ) -> Result<(), AgentManagementError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        ensure_internal_agent_code_available(
            &transaction,
            Some(&definition.workspace_code),
            &definition.agent_code,
        )?;
        transaction
            .execute(
                "INSERT INTO workspace_internal_agent_definitions
             (workspace_code, agent_code, name, role_prompt, created_at, updated_at, loop_code, avatar_id, group_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    definition.workspace_code,
                    definition.agent_code,
                    definition.name,
                    definition.role_prompt,
                    timestamp_to_millis(&definition.created_at),
                    timestamp_to_millis(&definition.updated_at),
                    definition.loop_code,
                    definition.avatar_id,
                    definition.group_name
                ],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    fn list_platform_internal_agent_configs(
        &mut self,
    ) -> Result<Vec<PlatformInternalAgentConfig>, AgentManagementError> {
        let mut statement = self
            .connection
            .prepare("SELECT config_json FROM platform_internal_agent_configs ORDER BY agent_code")
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?;
        rows.map(|row| {
            serde_json::from_str(&row.map_err(storage_error)?)
                .map_err(|_| AgentManagementError::StorageFailure)
        })
        .collect()
    }
    fn upsert_platform_internal_agent_config(
        &mut self,
        config: &PlatformInternalAgentConfig,
    ) -> Result<(), AgentManagementError> {
        let json =
            serde_json::to_string(config).map_err(|_| AgentManagementError::StorageFailure)?;
        self.connection.execute("INSERT INTO platform_internal_agent_configs (agent_code, config_json) VALUES (?1, ?2)
            ON CONFLICT(agent_code) DO UPDATE SET config_json = excluded.config_json", params![config.agent_code, json]).map_err(storage_error)?;
        Ok(())
    }
    fn delete_workspace_internal_agent_definition(
        &mut self,
        workspace: &str,
        agent: &str,
    ) -> Result<(), AgentManagementError> {
        self.connection.execute("DELETE FROM workspace_internal_agent_definitions WHERE workspace_code = ?1 AND agent_code = ?2", params![workspace, agent]).map_err(storage_error)?;
        Ok(())
    }
    fn list_workspace_internal_agent_definitions(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceInternalAgentDefinition>, AgentManagementError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, agent_code, name, role_prompt, created_at, updated_at, loop_code, avatar_id, group_name
             FROM workspace_internal_agent_definitions WHERE workspace_code = ?1
             ORDER BY created_at, agent_code",
            )
            .map_err(storage_error)?;
        statement
            .query_map([workspace_code], |row| {
                Ok(WorkspaceInternalAgentDefinition {
                    workspace_code: row.get(0)?,
                    agent_code: row.get(1)?,
                    loop_code: row.get(6)?,
                    avatar_id: row.get(7)?,
                    group_name: row.get(8)?,
                    name: row.get(2)?,
                    role_prompt: row.get(3)?,
                    created_at: timestamp_from_millis(row.get(4)?),
                    updated_at: timestamp_from_millis(row.get(5)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn upsert_workspace_internal_agent_definition(
        &mut self,
        definition: &WorkspaceInternalAgentDefinition,
    ) -> Result<(), AgentManagementError> {
        self.connection.execute(
            "INSERT INTO workspace_internal_agent_definitions
             (workspace_code, agent_code, name, role_prompt, created_at, updated_at, loop_code, avatar_id, group_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(workspace_code, agent_code) DO UPDATE SET
             name = excluded.name, role_prompt = excluded.role_prompt, loop_code = excluded.loop_code, avatar_id = excluded.avatar_id, group_name = excluded.group_name, updated_at = excluded.updated_at",
            params![definition.workspace_code, definition.agent_code, definition.name,
                definition.role_prompt, timestamp_to_millis(&definition.created_at),
                timestamp_to_millis(&definition.updated_at), definition.loop_code, definition.avatar_id, definition.group_name],
        ).map_err(storage_error)?;
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

    fn delete_system_agent(&mut self, agent_code: &str) -> Result<bool, AgentManagementError> {
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        transaction
            .execute(
                "DELETE FROM workspace_system_agent_policies WHERE agent_code = ?1",
                [agent_code],
            )
            .map_err(storage_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM system_agent_registries WHERE agent_code = ?1",
                [agent_code],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)
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
                "SELECT workspace_code, agent_code, tool_assignment_mode, mcp_access_mode, tool_exposure_mode, expanded_domain_persistence_mode, max_steps, max_tool_calls, max_parallel_tool_calls, created_at, updated_at, overridden_fields
                 FROM workspace_internal_agent_policies
                 WHERE workspace_code = ?1
                 ORDER BY agent_code ASC"
            }
            None => {
                "SELECT workspace_code, agent_code, tool_assignment_mode, mcp_access_mode, tool_exposure_mode, expanded_domain_persistence_mode, max_steps, max_tool_calls, max_parallel_tool_calls, created_at, updated_at, overridden_fields
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
                overridden_fields: serde_json::from_str(&row.get::<_, String>(11)?).map_err(
                    |e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            11,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    },
                )?,
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
                    updated_at,
                    overridden_fields
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(workspace_code, agent_code) DO UPDATE SET
                    tool_assignment_mode = excluded.tool_assignment_mode,
                    mcp_access_mode = excluded.mcp_access_mode,
                    tool_exposure_mode = excluded.tool_exposure_mode,
                    expanded_domain_persistence_mode = excluded.expanded_domain_persistence_mode,
                    max_steps = excluded.max_steps,
                    max_tool_calls = excluded.max_tool_calls,
                    max_parallel_tool_calls = excluded.max_parallel_tool_calls,
                    overridden_fields = excluded.overridden_fields,
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
                    serde_json::to_string(&policy.overridden_fields)
                        .map_err(|_| AgentManagementError::StorageFailure)?,
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

impl AgentManagementRepository for SqliteAgentManagementRepository {
    fn list_internal_agent_groups(
        &mut self,
        scope: &str,
    ) -> Result<Vec<String>, AgentManagementError> {
        let scope = scope.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.list_internal_agent_groups(&scope)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn save_internal_agent_group(
        &mut self,
        scope: &str,
        name: &str,
    ) -> Result<(), AgentManagementError> {
        let scope = scope.to_owned();
        let name = name.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .save_internal_agent_group(&scope, &name)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }

    fn list_tool_permissions(
        &mut self,
    ) -> Result<Vec<tron_harness_capability::ToolPermissionSetting>, AgentManagementError> {
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.list_tool_permissions()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn replace_tool_permissions(
        &mut self,
        settings: &[tron_harness_capability::ToolPermissionSetting],
    ) -> Result<(), AgentManagementError> {
        let settings = settings.to_vec();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .replace_tool_permissions(&settings)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_discovered_tools(
        &mut self,
    ) -> Result<Vec<crate::domain::agent::ToolCatalogRecord>, AgentManagementError> {
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.list_discovered_tools()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn remember_discovered_tool(
        &mut self,
        tool: &crate::domain::agent::ToolCatalogRecord,
    ) -> Result<(), AgentManagementError> {
        let tool = tool.clone();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.remember_discovered_tool(&tool)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }

    fn list_loop_designs(
        &mut self,
    ) -> Result<Vec<crate::domain::agent::loop_design::LoopDesign>, AgentManagementError> {
        self.connection.call(
            move |connection| SqliteAgentManagementRepositorySql { connection }.list_loop_designs(),
            || AgentManagementError::StorageFailure,
        )
    }
    fn save_loop_design(
        &mut self,
        design: &crate::domain::agent::loop_design::LoopDesign,
        expected: u32,
    ) -> Result<(), AgentManagementError> {
        let design = design.clone();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .save_loop_design(&design, expected)
            },
            || AgentManagementError::StorageFailure,
        )
    }
    fn delete_loop_design(&mut self, code: &str) -> Result<(), AgentManagementError> {
        let code = code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.delete_loop_design(&code)
            },
            || AgentManagementError::StorageFailure,
        )
    }

    fn agent_workspace_grants(
        &mut self,
        agent: &str,
    ) -> Result<Option<Vec<String>>, AgentManagementError> {
        let agent = agent.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.agent_workspace_grants(&agent)
            },
            || AgentManagementError::StorageFailure,
        )
    }
    fn replace_agent_workspace_grants(
        &mut self,
        agent: &str,
        workspaces: &[String],
    ) -> Result<(), AgentManagementError> {
        let (agent, workspaces) = (agent.to_owned(), workspaces.to_vec());
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .replace_agent_workspace_grants(&agent, &workspaces)
            },
            || AgentManagementError::StorageFailure,
        )
    }

    fn insert_platform_internal_agent_config(
        &mut self,
        config: &PlatformInternalAgentConfig,
    ) -> Result<(), AgentManagementError> {
        let config = config.clone();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .insert_platform_internal_agent_config(&config)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }

    fn insert_workspace_internal_agent_definition(
        &mut self,
        definition: &WorkspaceInternalAgentDefinition,
    ) -> Result<(), AgentManagementError> {
        let definition = definition.clone();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .insert_workspace_internal_agent_definition(&definition)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }

    fn list_platform_internal_agent_configs(
        &mut self,
    ) -> Result<Vec<PlatformInternalAgentConfig>, AgentManagementError> {
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .list_platform_internal_agent_configs()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_platform_internal_agent_config(
        &mut self,
        config: &PlatformInternalAgentConfig,
    ) -> Result<(), AgentManagementError> {
        let config = config.clone();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .upsert_platform_internal_agent_config(&config)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_workspace_internal_agent_definition(
        &mut self,
        workspace: &str,
        agent: &str,
    ) -> Result<(), AgentManagementError> {
        let workspace = workspace.to_owned();
        let agent = agent.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .delete_workspace_internal_agent_definition(&workspace, &agent)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_workspace_internal_agent_definitions(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceInternalAgentDefinition>, AgentManagementError> {
        let workspace_code = workspace_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .list_workspace_internal_agent_definitions(&workspace_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }

    fn upsert_workspace_internal_agent_definition(
        &mut self,
        definition: &WorkspaceInternalAgentDefinition,
    ) -> Result<(), AgentManagementError> {
        let definition = definition.clone();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .upsert_workspace_internal_agent_definition(&definition)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_system_agents(
        &mut self,
    ) -> Result<Vec<SystemAgentRegistryEntry>, AgentManagementError> {
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.list_system_agents()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_system_agent(
        &mut self,
        agent: &SystemAgentRegistryEntry,
    ) -> Result<(), AgentManagementError> {
        let agent = agent.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.upsert_system_agent(&agent)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_system_agent(&mut self, agent_code: &str) -> Result<bool, AgentManagementError> {
        let agent_code = agent_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }.delete_system_agent(&agent_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_workspace_system_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceSystemAgentPolicy>, AgentManagementError> {
        let workspace_code = workspace_code.map(ToOwned::to_owned);
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .list_workspace_system_policies(workspace_code.as_deref())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_workspace_system_policy(
        &mut self,
        policy: &WorkspaceSystemAgentPolicy,
    ) -> Result<(), AgentManagementError> {
        let policy = policy.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .upsert_workspace_system_policy(&policy)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_workspace_system_policy(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<bool, AgentManagementError> {
        let workspace_code = workspace_code.to_owned();
        let agent_code = agent_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .delete_workspace_system_policy(&workspace_code, &agent_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_workspace_internal_agent_policies(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentPolicy>, AgentManagementError> {
        let workspace_code = workspace_code.map(ToOwned::to_owned);
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .list_workspace_internal_agent_policies(workspace_code.as_deref())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_workspace_internal_agent_policy(
        &mut self,
        policy: &WorkspaceInternalAgentPolicy,
    ) -> Result<(), AgentManagementError> {
        let policy = policy.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .upsert_workspace_internal_agent_policy(&policy)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_workspace_internal_agent_policy(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<bool, AgentManagementError> {
        let workspace_code = workspace_code.to_owned();
        let agent_code = agent_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .delete_workspace_internal_agent_policy(&workspace_code, &agent_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_workspace_internal_agent_tool_assignments(
        &mut self,
        workspace_code: Option<&str>,
        agent_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentToolAssignment>, AgentManagementError> {
        let workspace_code = workspace_code.map(ToOwned::to_owned);
        let agent_code = agent_code.map(ToOwned::to_owned);
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .list_workspace_internal_agent_tool_assignments(
                        workspace_code.as_deref(),
                        agent_code.as_deref(),
                    )
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_workspace_internal_agent_tool_assignment(
        &mut self,
        assignment: &WorkspaceInternalAgentToolAssignment,
    ) -> Result<(), AgentManagementError> {
        let assignment = assignment.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .upsert_workspace_internal_agent_tool_assignment(&assignment)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_workspace_internal_agent_tool_assignments(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<(), AgentManagementError> {
        let workspace_code = workspace_code.to_owned();
        let agent_code = agent_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .delete_workspace_internal_agent_tool_assignments(&workspace_code, &agent_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_workspace_internal_agent_mcp_server_assignments(
        &mut self,
        workspace_code: Option<&str>,
        agent_code: Option<&str>,
    ) -> Result<Vec<WorkspaceInternalAgentMcpServerAssignment>, AgentManagementError> {
        let workspace_code = workspace_code.map(ToOwned::to_owned);
        let agent_code = agent_code.map(ToOwned::to_owned);
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .list_workspace_internal_agent_mcp_server_assignments(
                        workspace_code.as_deref(),
                        agent_code.as_deref(),
                    )
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_workspace_internal_agent_mcp_server_assignment(
        &mut self,
        assignment: &WorkspaceInternalAgentMcpServerAssignment,
    ) -> Result<(), AgentManagementError> {
        let assignment = assignment.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .upsert_workspace_internal_agent_mcp_server_assignment(&assignment)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_workspace_internal_agent_mcp_server_assignments(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<(), AgentManagementError> {
        let workspace_code = workspace_code.to_owned();
        let agent_code = agent_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteAgentManagementRepositorySql { connection }
                    .delete_workspace_internal_agent_mcp_server_assignments(
                        &workspace_code,
                        &agent_code,
                    )
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
