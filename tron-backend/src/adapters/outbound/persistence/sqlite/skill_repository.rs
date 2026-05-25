use std::{fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::skill::SkillRegistryRepository,
    domain::skill::{
        RegistrySkill, SkillError, SkillManagedBy, SkillSourceType, SkillUpdateStatus,
        WorkspaceSkillBinding,
    },
    shared::time::{
        option_timestamp_from_millis,
        option_timestamp_to_millis,
        timestamp_from_millis,
        timestamp_to_millis,
    },
};

use super::migrations::SKILL_REGISTRY_MIGRATION;

pub struct SqliteSkillRegistryRepository {
    connection: Connection,
}

impl SqliteSkillRegistryRepository {
    pub fn connect(database_url: &str) -> Result<Self, SkillError> {
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

        let mut repository = Self { connection };
        repository.apply_migrations()?;
        Ok(repository)
    }

    fn apply_migrations(&mut self) -> Result<(), SkillError> {
        self.connection
            .execute_batch(SKILL_REGISTRY_MIGRATION)
            .map_err(storage_error)?;
        ensure_workspace_skill_binding_enabled_column(&self.connection)
    }
}

impl SkillRegistryRepository for SqliteSkillRegistryRepository {
    fn list_registry_skills(&mut self) -> Result<Vec<RegistrySkill>, SkillError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT skill_name, source_ref, source_type, registry_path, declared_version, compatibility,
                        update_status, managed_by, last_checked_at, last_updated_at, last_error,
                        created_at, updated_at
                 FROM registry_skills
                 ORDER BY skill_name ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(RegistrySkill {
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
        self.connection
            .execute(
                "INSERT INTO registry_skills (
                    skill_name, source_ref, source_type, registry_path, declared_version,
                    compatibility, update_status, managed_by, last_checked_at, last_updated_at,
                    last_error, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(skill_name) DO UPDATE SET
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
                ],
            )
            .map_err(storage_error)?;
        Ok(())
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

    fn list_workspace_bindings(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceSkillBinding>, SkillError> {
        let sql = if workspace_code.is_some() {
            "SELECT workspace_code, skill_name, enabled, created_at, updated_at
             FROM workspace_skill_bindings
             WHERE workspace_code = ?1
             ORDER BY skill_name ASC"
        } else {
            "SELECT workspace_code, skill_name, enabled, created_at, updated_at
             FROM workspace_skill_bindings
             ORDER BY workspace_code ASC, skill_name ASC"
        };
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        if let Some(workspace_code) = workspace_code {
            statement
                .query_map(params![workspace_code], |row| {
                    Ok(WorkspaceSkillBinding {
                        workspace_code: row.get(0)?,
                        skill_name: row.get(1)?,
                        enabled: row.get::<_, i64>(2)? != 0,
                        created_at: timestamp_from_millis(row.get(3)?),
                        updated_at: timestamp_from_millis(row.get(4)?),
                    })
                })
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)
        } else {
            statement
                .query_map([], |row| {
                    Ok(WorkspaceSkillBinding {
                        workspace_code: row.get(0)?,
                        skill_name: row.get(1)?,
                        enabled: row.get::<_, i64>(2)? != 0,
                        created_at: timestamp_from_millis(row.get(3)?),
                        updated_at: timestamp_from_millis(row.get(4)?),
                    })
                })
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)
        }
    }

    fn upsert_workspace_binding(
        &mut self,
        binding: &WorkspaceSkillBinding,
    ) -> Result<(), SkillError> {
        self.connection
            .execute(
                "INSERT INTO workspace_skill_bindings (
                    workspace_code, skill_name, enabled, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(workspace_code, skill_name) DO UPDATE SET
                    enabled = excluded.enabled,
                    updated_at = excluded.updated_at",
                params![
                    binding.workspace_code,
                    binding.skill_name,
                    if binding.enabled { 1_i64 } else { 0_i64 },
                    timestamp_to_millis(&binding.created_at),
                    timestamp_to_millis(&binding.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_workspace_binding(
        &mut self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<bool, SkillError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM workspace_skill_bindings
                 WHERE workspace_code = ?1 AND skill_name = ?2",
                params![workspace_code, skill_name],
            )
            .map_err(storage_error)?;
        Ok(deleted_rows > 0)
    }

    fn delete_bindings_for_skill(&mut self, skill_name: &str) -> Result<(), SkillError> {
        self.connection
            .execute(
                "DELETE FROM workspace_skill_bindings WHERE skill_name = ?1",
                params![skill_name],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), SkillError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| SkillError::StorageFailure)?;
    }
    Ok(())
}

fn ensure_workspace_skill_binding_enabled_column(
    connection: &Connection,
) -> Result<(), SkillError> {
    let mut statement = connection
        .prepare("PRAGMA table_info(workspace_skill_bindings)")
        .map_err(storage_error)?;
    let has_enabled = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?
        .into_iter()
        .any(|name| name == "enabled");
    if !has_enabled {
        connection
            .execute(
                "ALTER TABLE workspace_skill_bindings ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1",
                [],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn source_type_to_storage(value: SkillSourceType) -> &'static str {
    match value {
        SkillSourceType::Marketplace => "Marketplace",
        SkillSourceType::GitHub => "GitHub",
        SkillSourceType::Git => "Git",
        SkillSourceType::Url => "Url",
        SkillSourceType::Local => "Local",
        SkillSourceType::Manual => "Manual",
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

fn managed_by_to_storage(value: SkillManagedBy) -> &'static str {
    match value {
        SkillManagedBy::Cli => "Cli",
        SkillManagedBy::Manual => "Manual",
    }
}

fn managed_by_from_storage(value: &str) -> Result<SkillManagedBy, rusqlite::Error> {
    match value {
        "Cli" => Ok(SkillManagedBy::Cli),
        "Manual" => Ok(SkillManagedBy::Manual),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn storage_error(_error: rusqlite::Error) -> SkillError {
    SkillError::StorageFailure
}
