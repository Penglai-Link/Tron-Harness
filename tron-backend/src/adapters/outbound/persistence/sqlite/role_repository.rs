use std::{collections::BTreeMap, fs, path::Path};

use rusqlite::{Connection, params};

use crate::{
    application::role::RoleRepository,
    domain::role::{Role, RoleError, RoleKind, RoleManagedBy},
};

use super::migrations::{ROLE_PERMISSIONS_MIGRATION, ROLES_MIGRATION};

pub struct SqliteRoleRepository {
    connection: Connection,
}

impl SqliteRoleRepository {
    pub fn connect(database_url: &str) -> Result<Self, RoleError> {
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

    fn apply_migrations(&mut self) -> Result<(), RoleError> {
        self.connection
            .execute_batch(ROLES_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(ROLE_PERMISSIONS_MIGRATION)
            .map_err(storage_error)
    }

    fn permission_codes(&self, role_id: i64) -> Result<BTreeMap<String, Vec<String>>, RoleError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT permission_domain, permission_code
                 FROM role_permissions
                 WHERE role_id = ?1
                 ORDER BY permission_domain ASC, id ASC",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([role_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        let mut permissions = BTreeMap::new();
        for (domain, code) in rows {
            permissions
                .entry(domain)
                .or_insert_with(Vec::new)
                .push(code);
        }
        Ok(permissions)
    }

    fn kind_from_storage(value: &str) -> Result<RoleKind, RoleError> {
        match value {
            "system" => Ok(RoleKind::System),
            "custom" => Ok(RoleKind::Custom),
            _ => Err(RoleError::StorageFailure),
        }
    }

    fn kind_to_storage(value: RoleKind) -> &'static str {
        match value {
            RoleKind::System => "system",
            RoleKind::Custom => "custom",
        }
    }

    fn managed_by_from_storage(value: &str) -> Result<RoleManagedBy, RoleError> {
        match value {
            "platform" => Ok(RoleManagedBy::Platform),
            "workspace" => Ok(RoleManagedBy::Workspace),
            _ => Err(RoleError::StorageFailure),
        }
    }

    fn managed_by_to_storage(value: RoleManagedBy) -> &'static str {
        match value {
            RoleManagedBy::Platform => "platform",
            RoleManagedBy::Workspace => "workspace",
        }
    }
}

impl RoleRepository for SqliteRoleRepository {
    fn list_roles(&mut self) -> Result<Vec<Role>, RoleError> {
        let role_rows = {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT id, role_code, name, description, kind, managed_by, updated_at
                     FROM roles
                     ORDER BY CASE kind WHEN 'system' THEN 0 ELSE 1 END, name ASC",
                )
                .map_err(storage_error)?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                })
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?
        };

        role_rows
            .into_iter()
            .map(
                |(id, code, name, description, kind, managed_by, updated_at)| {
                    Ok(Role {
                        id: id.into(),
                        code,
                        name,
                        description,
                        kind: Self::kind_from_storage(&kind)?,
                        managed_by: Self::managed_by_from_storage(&managed_by)?,
                        permissions: self.permission_codes(id)?,
                        updated_at,
                    })
                },
            )
            .collect()
    }

    fn next_id(&mut self) -> Result<i64, RoleError> {
        let max_id: i64 = self
            .connection
            .query_row(
                "SELECT max(id) FROM (
                    SELECT id FROM roles
                    UNION ALL SELECT id FROM role_permissions
                )",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(734_300_000_000_000_000);
        Ok(max_id + 1)
    }

    fn replace_roles(&mut self, roles: &[Role]) -> Result<Vec<Role>, RoleError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute("DELETE FROM role_permissions", [])
            .map_err(storage_error)?;
        tx.execute("DELETE FROM roles", []).map_err(storage_error)?;

        let mut next_permission_id = roles
            .iter()
            .map(|role| role.id.value())
            .max()
            .unwrap_or(734_300_000_000_000_000)
            + 1;

        for role in roles {
            tx.execute(
                "INSERT INTO roles (id, role_code, name, description, kind, managed_by, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    role.id.value(),
                    role.code,
                    role.name,
                    role.description,
                    Self::kind_to_storage(role.kind),
                    Self::managed_by_to_storage(role.managed_by),
                    role.updated_at,
                    role.updated_at,
                ],
            )
            .map_err(storage_error)?;

            for (domain, codes) in &role.permissions {
                for code in codes {
                    tx.execute(
                        "INSERT INTO role_permissions (id, role_id, role_code, permission_domain, permission_code, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            next_permission_id,
                            role.id.value(),
                            role.code,
                            domain,
                            code,
                            role.updated_at,
                        ],
                    )
                    .map_err(storage_error)?;
                    next_permission_id += 1;
                }
            }
        }

        tx.commit().map_err(storage_error)?;
        self.list_roles()
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), RoleError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| RoleError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(_error: rusqlite::Error) -> RoleError {
    RoleError::StorageFailure
}
