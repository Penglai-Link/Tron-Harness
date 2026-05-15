use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{
    application::role::{ReplaceRolesCommand, RoleDraft},
    domain::role::{RoleKind, RoleManagedBy},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceRolesRequest {
    pub roles: Vec<RoleDraftRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleDraftRequest {
    pub id: Option<String>,
    pub code: String,
    pub name: String,
    pub description: String,
    pub kind: RoleKind,
    pub managed_by: RoleManagedBy,
    pub permissions: BTreeMap<String, Vec<String>>,
    pub updated_at: Option<String>,
}

impl From<ReplaceRolesRequest> for ReplaceRolesCommand {
    fn from(request: ReplaceRolesRequest) -> Self {
        Self {
            roles: request.roles.into_iter().map(RoleDraft::from).collect(),
        }
    }
}

impl From<RoleDraftRequest> for RoleDraft {
    fn from(request: RoleDraftRequest) -> Self {
        Self {
            id: request.id,
            code: request.code,
            name: request.name,
            description: request.description,
            kind: request.kind,
            managed_by: request.managed_by,
            permissions: request.permissions,
            updated_at: request.updated_at,
        }
    }
}
