use rusqlite::{Connection, params};

use crate::{
    application::skill::SkillRegistryRepository,
    domain::skill::{
        AgentSkillAssignment, RegistrySkill, SkillError, SkillManagedBy, SkillRagIndexRecord,
        SkillRagIndexStatus, SkillSourceType, SkillUpdateStatus,
    },
    shared::time::{
        option_timestamp_from_millis, option_timestamp_to_millis, timestamp_from_millis,
        timestamp_to_millis,
    },
};

struct SqliteSkillRegistryRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqliteSkillRegistryRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqliteSkillRegistryRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, SkillError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl SqliteSkillRegistryRepositorySql<'_> {
    fn list_registry_skills(&mut self) -> Result<Vec<RegistrySkill>, SkillError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT skill_name, source_ref, source_type, registry_path, declared_version, compatibility,
                        update_status, managed_by, last_checked_at, last_updated_at, last_error,
                        created_at, updated_at, provider_pack_id, available
                 FROM registry_skills
                 ORDER BY skill_name ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(RegistrySkill {
                    provider_pack_id: row.get(13)?,
                    available: row.get(14)?,
                    skill_name: row.get(0)?,
                    source_ref: row.get(1)?,
                    source_type: source_type_from_storage(&row.get::<_, String>(2)?)?,
                    registry_path: row.get(3)?,
                    declared_version: row.get(4)?,
                    update_status: update_status_from_storage(&row.get::<_, String>(6)?)?,
                    managed_by: managed_by_from_storage(&row.get::<_, String>(7)?)?,
                    last_checked_at: option_timestamp_from_millis(row.get(8)?),
                    last_updated_at: option_timestamp_from_millis(row.get(9)?),
                    last_error: row.get(10)?,
                    created_at: timestamp_from_millis(row.get(11)?),
                    updated_at: timestamp_from_millis(row.get(12)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn upsert_registry_skill(&mut self, skill: &RegistrySkill) -> Result<(), SkillError> {
        upsert_skill(self.connection, skill)
    }
    fn delete_registry_skill(&mut self, skill_name: &str) -> Result<bool, SkillError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM registry_skills WHERE skill_name = ?1",
                params![skill_name],
            )
            .map_err(storage_error)?;
        Ok(deleted_rows > 0)
    }

    fn list_assignments(&mut self) -> Result<Vec<AgentSkillAssignment>, SkillError> {
        let mut q = self
            .connection
            .prepare("SELECT agent_code,skill_name,enabled FROM agent_skill_assignments")
            .map_err(storage_error)?;
        q.query_map([], |r| {
            Ok(AgentSkillAssignment {
                agent_code: r.get(0)?,
                skill_name: r.get(1)?,
                enabled: r.get(2)?,
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
    }
    fn replace_agent_skills(&mut self, agent: &str, names: &[String]) -> Result<(), SkillError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        // Disabled rows are tombstones: scans must never restore a user's deselection.
        tx.execute("INSERT OR IGNORE INTO agent_skill_assignments SELECT ?1,skill_name,0 FROM registry_skills",[agent]).map_err(storage_error)?;
        tx.execute(
            "UPDATE agent_skill_assignments SET enabled=0 WHERE agent_code=?1",
            [agent],
        )
        .map_err(storage_error)?;
        let selected = serde_json::to_string(names).map_err(|_| SkillError::StorageFailure)?;
        let unavailable: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM json_each(?1) selected WHERE NOT EXISTS(SELECT 1 FROM registry_skills s WHERE s.skill_name=selected.value AND s.available=1))",
            [&selected], |row| row.get(0),
        ).map_err(storage_error)?;
        if unavailable {
            return Err(SkillError::Unavailable);
        }
        tx.execute("INSERT INTO agent_skill_assignments SELECT ?1,value,1 FROM json_each(?2) WHERE true ON CONFLICT(agent_code,skill_name) DO UPDATE SET enabled=1", params![agent,selected]).map_err(storage_error)?;
        tx.commit().map_err(storage_error)
    }
    fn replace_skill_agents(&mut self, name: &str, agents: &[String]) -> Result<(), SkillError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        let available: bool = tx
            .query_row(
                "SELECT available FROM registry_skills WHERE skill_name=?1",
                [name],
                |r| r.get(0),
            )
            .map_err(|_| SkillError::NotFound)?;
        if !available && !agents.is_empty() {
            return Err(SkillError::Unavailable);
        }
        tx.execute(
            "UPDATE agent_skill_assignments SET enabled=0 WHERE skill_name=?1",
            [name],
        )
        .map_err(storage_error)?;
        for agent in agents {
            tx.execute("INSERT INTO agent_skill_assignments VALUES(?1,?2,1) ON CONFLICT(agent_code,skill_name) DO UPDATE SET enabled=1",params![agent,name]).map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)
    }
    fn initialize_agent_skills(&mut self, agent: &str) -> Result<(), SkillError> {
        initialize_pack_skill_assignments(self.connection, &serde_json::json!([agent]).to_string())
    }
    fn delete_pack_skills(&mut self, pack: &str) -> Result<Vec<String>, SkillError> {
        let names = self
            .list_registry_skills()?
            .into_iter()
            .filter(|s| s.provider_pack_id.as_deref() == Some(pack))
            .map(|s| s.skill_name)
            .collect::<Vec<_>>();
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "DELETE FROM registry_skills WHERE provider_pack_id=?1",
            [pack],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(names)
    }

    fn get_rag_index_state(
        &mut self,
        skill_name: &str,
    ) -> Result<Option<SkillRagIndexRecord>, SkillError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT skill_name, source_version, desired_enabled,
                        vector_status, graph_status, content_hash, structure_hash,
                        last_indexed_at, last_error, updated_at
                 FROM skill_rag_index_states
                 WHERE skill_name = ?1",
            )
            .map_err(storage_error)?;
        match statement.query_row(params![skill_name], map_rag_index_state_row) {
            Ok(state) => Ok(Some(state)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(storage_error(error)),
        }
    }

    fn list_rag_index_states(&mut self) -> Result<Vec<SkillRagIndexRecord>, SkillError> {
        let mut q = self.connection.prepare("SELECT skill_name,source_version,desired_enabled,vector_status,graph_status,content_hash,structure_hash,last_indexed_at,last_error,updated_at FROM skill_rag_index_states ORDER BY skill_name").map_err(storage_error)?;
        q.query_map([], map_rag_index_state_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn delete_rag_index_state(&mut self, name: &str) -> Result<(), SkillError> {
        self.connection
            .execute(
                "DELETE FROM skill_rag_index_states WHERE skill_name=?1",
                [name],
            )
            .map_err(storage_error)?;
        Ok(())
    }
    fn upsert_rag_index_state(&mut self, state: &SkillRagIndexRecord) -> Result<(), SkillError> {
        self.connection
            .execute(
                "INSERT INTO skill_rag_index_states (
                    skill_name, source_version, desired_enabled,
                    vector_status, graph_status, content_hash, structure_hash,
                    last_indexed_at, last_error, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(skill_name) DO UPDATE SET
                    source_version = excluded.source_version,
                    desired_enabled = excluded.desired_enabled,
                    vector_status = excluded.vector_status,
                    graph_status = excluded.graph_status,
                    content_hash = excluded.content_hash,
                    structure_hash = excluded.structure_hash,
                    last_indexed_at = excluded.last_indexed_at,
                    last_error = excluded.last_error,
                    updated_at = excluded.updated_at",
                params![
                    state.skill_name,
                    state.source_version,
                    if state.desired_enabled { 1_i64 } else { 0_i64 },
                    rag_status_to_storage(state.vector_status),
                    rag_status_to_storage(state.graph_status),
                    state.content_hash,
                    state.structure_hash,
                    option_timestamp_to_millis(state.last_indexed_at.as_deref()),
                    state.last_error,
                    timestamp_to_millis(&state.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn upsert_skill(connection: &Connection, skill: &RegistrySkill) -> Result<(), SkillError> {
    connection
        .execute(
            "INSERT INTO registry_skills (
                    skill_name, source_ref, source_type, registry_path, declared_version,
                    compatibility, update_status, managed_by, last_checked_at, last_updated_at,
                    last_error, created_at, updated_at, provider_pack_id, available
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                 ON CONFLICT(skill_name) DO UPDATE SET
                    provider_pack_id = excluded.provider_pack_id,
                    available = excluded.available,
                    source_ref = excluded.source_ref,
                    source_type = excluded.source_type,
                    registry_path = excluded.registry_path,
                    declared_version = excluded.declared_version,
                    compatibility = excluded.compatibility,
                    update_status = excluded.update_status,
                    managed_by = excluded.managed_by,
                    last_checked_at = excluded.last_checked_at,
                    last_updated_at = excluded.last_updated_at,
                    last_error = excluded.last_error,
                    updated_at = excluded.updated_at",
            params![
                skill.skill_name,
                skill.source_ref,
                source_type_to_storage(skill.source_type),
                skill.registry_path,
                skill.declared_version,
                Option::<String>::None,
                update_status_to_storage(skill.update_status),
                managed_by_to_storage(skill.managed_by),
                option_timestamp_to_millis(skill.last_checked_at.as_deref()),
                option_timestamp_to_millis(skill.last_updated_at.as_deref()),
                skill.last_error,
                timestamp_to_millis(&skill.created_at),
                timestamp_to_millis(&skill.updated_at),
                skill.provider_pack_id,
                skill.available,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn map_rag_index_state_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillRagIndexRecord> {
    Ok(SkillRagIndexRecord {
        skill_name: row.get(0)?,
        source_version: row.get(1)?,
        desired_enabled: row.get::<_, i64>(2)? != 0,
        vector_status: rag_status_from_storage(&row.get::<_, String>(3)?)?,
        graph_status: rag_status_from_storage(&row.get::<_, String>(4)?)?,
        content_hash: row.get(5)?,
        structure_hash: row.get(6)?,
        last_indexed_at: option_timestamp_from_millis(row.get(7)?),
        last_error: row.get(8)?,
        updated_at: timestamp_from_millis(row.get(9)?),
    })
}

fn source_type_to_storage(value: SkillSourceType) -> &'static str {
    match value {
        SkillSourceType::Marketplace => "Marketplace",
        SkillSourceType::GitHub => "GitHub",
        SkillSourceType::Git => "Git",
        SkillSourceType::Url => "Url",
        SkillSourceType::Local => "Local",
        SkillSourceType::Manual => "Manual",
        SkillSourceType::BuiltIn => "BuiltIn",
    }
}

fn source_type_from_storage(value: &str) -> Result<SkillSourceType, rusqlite::Error> {
    match value {
        "Marketplace" => Ok(SkillSourceType::Marketplace),
        "GitHub" => Ok(SkillSourceType::GitHub),
        "Git" => Ok(SkillSourceType::Git),
        "Url" => Ok(SkillSourceType::Url),
        "Local" => Ok(SkillSourceType::Local),
        "Manual" => Ok(SkillSourceType::Manual),
        "BuiltIn" => Ok(SkillSourceType::BuiltIn),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn update_status_to_storage(value: SkillUpdateStatus) -> &'static str {
    match value {
        SkillUpdateStatus::Unknown => "Unknown",
        SkillUpdateStatus::UpToDate => "UpToDate",
        SkillUpdateStatus::UpdateAvailable => "UpdateAvailable",
        SkillUpdateStatus::Manual => "Manual",
        SkillUpdateStatus::Error => "Error",
    }
}

fn update_status_from_storage(value: &str) -> Result<SkillUpdateStatus, rusqlite::Error> {
    match value {
        "Unknown" => Ok(SkillUpdateStatus::Unknown),
        "UpToDate" => Ok(SkillUpdateStatus::UpToDate),
        "UpdateAvailable" => Ok(SkillUpdateStatus::UpdateAvailable),
        "Manual" => Ok(SkillUpdateStatus::Manual),
        "Error" => Ok(SkillUpdateStatus::Error),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn rag_status_to_storage(value: SkillRagIndexStatus) -> &'static str {
    match value {
        SkillRagIndexStatus::Pending => "PENDING",
        SkillRagIndexStatus::Indexed => "INDEXED",
        SkillRagIndexStatus::Disabled => "DISABLED",
        SkillRagIndexStatus::Warning => "WARNING",
        SkillRagIndexStatus::Failed => "FAILED",
        SkillRagIndexStatus::Stale => "STALE",
    }
}

fn rag_status_from_storage(value: &str) -> Result<SkillRagIndexStatus, rusqlite::Error> {
    match value {
        "PENDING" => Ok(SkillRagIndexStatus::Pending),
        "INDEXED" => Ok(SkillRagIndexStatus::Indexed),
        "DISABLED" => Ok(SkillRagIndexStatus::Disabled),
        "WARNING" => Ok(SkillRagIndexStatus::Warning),
        "FAILED" => Ok(SkillRagIndexStatus::Failed),
        "STALE" => Ok(SkillRagIndexStatus::Stale),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn managed_by_to_storage(value: SkillManagedBy) -> &'static str {
    match value {
        SkillManagedBy::Cli => "Cli",
        SkillManagedBy::Manual => "Manual",
        SkillManagedBy::BuiltIn => "BuiltIn",
    }
}

fn managed_by_from_storage(value: &str) -> Result<SkillManagedBy, rusqlite::Error> {
    match value {
        "Cli" => Ok(SkillManagedBy::Cli),
        "Manual" => Ok(SkillManagedBy::Manual),
        "BuiltIn" => Ok(SkillManagedBy::BuiltIn),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn storage_error(_error: rusqlite::Error) -> SkillError {
    SkillError::StorageFailure
}

impl SqliteSkillRegistryRepository {
    pub fn connect(database_url: &str) -> Result<Self, SkillError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl SkillRegistryRepository for SqliteSkillRegistryRepository {
    fn list_registry_skills(&mut self) -> Result<Vec<RegistrySkill>, SkillError> {
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.list_registry_skills()
            },
            || SkillError::StorageFailure,
        )
    }
    fn upsert_registry_skill(&mut self, skill: &RegistrySkill) -> Result<(), SkillError> {
        let skill = skill.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.upsert_registry_skill(&skill)
            },
            || SkillError::StorageFailure,
        )
    }
    fn upsert_registry_skills(&mut self, skills: &[RegistrySkill]) -> Result<(), SkillError> {
        let skills = skills.to_vec();
        self.connection.call(
            move |connection| {
                let tx = super::framework_storage::business_transaction(
                    connection,
                    rusqlite::TransactionBehavior::Immediate,
                )
                .map_err(storage_error)?;
                for skill in skills {
                    upsert_skill(&tx, &skill)?;
                }
                tx.commit().map_err(storage_error)
            },
            || SkillError::StorageFailure,
        )
    }
    fn delete_registry_skill(&mut self, skill_name: &str) -> Result<bool, SkillError> {
        let skill_name = skill_name.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.delete_registry_skill(&skill_name)
            },
            || SkillError::StorageFailure,
        )
    }
    fn list_assignments(&mut self) -> Result<Vec<AgentSkillAssignment>, SkillError> {
        self.connection.call(
            move |connection| SqliteSkillRegistryRepositorySql { connection }.list_assignments(),
            || SkillError::StorageFailure,
        )
    }
    fn replace_agent_skills(&mut self, agent: &str, names: &[String]) -> Result<(), SkillError> {
        let agent = agent.to_owned();
        let names = names.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.replace_agent_skills(&agent, &names)
            },
            || SkillError::StorageFailure,
        )
    }
    fn replace_skill_agents(&mut self, name: &str, agents: &[String]) -> Result<(), SkillError> {
        let name = name.to_owned();
        let agents = agents.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.replace_skill_agents(&name, &agents)
            },
            || SkillError::StorageFailure,
        )
    }
    fn initialize_agents_skills(&mut self, agents: &[String]) -> Result<(), SkillError> {
        let agents = serde_json::to_string(agents).map_err(|_| SkillError::StorageFailure)?;
        self.connection.call(
            move |connection| initialize_pack_skill_assignments(connection, &agents),
            || SkillError::StorageFailure,
        )
    }
    fn initialize_agent_skills(&mut self, agent: &str) -> Result<(), SkillError> {
        let agent = agent.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.initialize_agent_skills(&agent)
            },
            || SkillError::StorageFailure,
        )
    }
    fn delete_pack_skills(&mut self, pack: &str) -> Result<Vec<String>, SkillError> {
        let pack = pack.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.delete_pack_skills(&pack)
            },
            || SkillError::StorageFailure,
        )
    }
    fn get_rag_index_state(
        &mut self,
        skill_name: &str,
    ) -> Result<Option<SkillRagIndexRecord>, SkillError> {
        let skill_name = skill_name.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.get_rag_index_state(&skill_name)
            },
            || SkillError::StorageFailure,
        )
    }
    fn list_rag_index_states(&mut self) -> Result<Vec<SkillRagIndexRecord>, SkillError> {
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.list_rag_index_states()
            },
            || SkillError::StorageFailure,
        )
    }
    fn delete_rag_index_state(&mut self, name: &str) -> Result<(), SkillError> {
        let name = name.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.delete_rag_index_state(&name)
            },
            || SkillError::StorageFailure,
        )
    }
    fn upsert_rag_index_state(&mut self, state: &SkillRagIndexRecord) -> Result<(), SkillError> {
        let state = state.to_owned();
        self.connection.call(
            move |connection| {
                SqliteSkillRegistryRepositorySql { connection }.upsert_rag_index_state(&state)
            },
            || SkillError::StorageFailure,
        )
    }
}

fn initialize_pack_skill_assignments(
    connection: &mut Connection,
    agents: &str,
) -> Result<(), SkillError> {
    use crate::domain::computer_use::{CONTROLLERS, PACK_ID, SKILL};
    // Platform variants share the legacy pack id as a prefix.
    let tx = connection.transaction().map_err(storage_error)?;
    tx.execute(
        "INSERT OR IGNORE INTO agent_skill_assignments
         SELECT a.value,s.skill_name,
                CASE WHEN (s.provider_pack_id=?2 OR s.provider_pack_id GLOB (?2 || '.*') OR s.skill_name=?3) AND a.value NOT IN (?4,?5) THEN 0 ELSE s.available END
         FROM json_each(?1) a CROSS JOIN registry_skills s WHERE s.provider_pack_id IS NOT NULL",
        params![agents, PACK_ID, SKILL, CONTROLLERS[0], CONTROLLERS[1]],
    ).map_err(storage_error)?;
    tx.execute(
        "UPDATE agent_skill_assignments SET enabled=0
         WHERE enabled=1 AND agent_code NOT IN (?3,?4)
           AND skill_name IN (SELECT skill_name FROM registry_skills WHERE provider_pack_id=?1 OR provider_pack_id GLOB (?1 || '.*') OR skill_name=?2)",
        params![PACK_ID, SKILL, CONTROLLERS[0], CONTROLLERS[1]],
    ).map_err(storage_error)?;
    tx.commit().map_err(storage_error)
}
