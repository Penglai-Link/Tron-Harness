use std::collections::BTreeMap;

use serde::Serialize;

use crate::domain::role::{Role, RoleKind, RoleManagedBy};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleDto {
    id: String,
    code: String,
    name: String,
    description: String,
    kind: RoleKind,
    managed_by: RoleManagedBy,
    permissions: BTreeMap<String, Vec<String>>,
    updated_at: String,
}

impl From<Role> for RoleDto {
    fn from(role: Role) -> Self {
        Self {
            id: role.id.as_decimal_string(),
            code: role.code,
            name: role.name,
            description: role.description,
            kind: role.kind,
            managed_by: role.managed_by,
            permissions: role.permissions,
            updated_at: role.updated_at,
        }
    }
}
