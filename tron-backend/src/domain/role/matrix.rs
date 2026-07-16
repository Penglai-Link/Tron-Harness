use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

pub const DEFAULT_ROLE_UPDATED_AT: &str = "2026-05-15T00:00:00Z";
pub const PERMISSION_DOMAINS: [&str; 7] = [
    "Workspace",
    "Models",
    "Tasks",
    "Memory",
    "Skills",
    "MCP",
    "Resources",
];
pub const PERMISSION_CODES: [&str; 4] = ["R", "W", "D", "A"];
pub const FIXED_SYSTEM_ROLE_CODES: [&str; 4] = ["ADMIN", "WORKSPACE_ADMIN", "DEVELOPER", "VIEWER"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleKind {
    System,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleManagedBy {
    Platform,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub id: SnowflakeId,
    pub code: String,
    pub name: String,
    pub description: String,
    pub kind: RoleKind,
    pub managed_by: RoleManagedBy,
    pub permissions: BTreeMap<String, Vec<String>>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleError {
    DuplicateRoleCode,
    DuplicateRoleName,
    InvalidPermissionCode,
    InvalidPermissionDomain,
    InvalidRoleCode,
    MissingRequiredSystemRole,
    RoleNameRequired,
    StorageFailure,
}

impl fmt::Display for RoleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRoleCode => formatter.write_str("duplicate role code"),
            Self::DuplicateRoleName => formatter.write_str("duplicate role name"),
            Self::InvalidPermissionCode => formatter.write_str("invalid permission code"),
            Self::InvalidPermissionDomain => formatter.write_str("invalid permission domain"),
            Self::InvalidRoleCode => formatter.write_str("invalid role code"),
            Self::MissingRequiredSystemRole => formatter.write_str("missing required system role"),
            Self::RoleNameRequired => formatter.write_str("role name is required"),
            Self::StorageFailure => formatter.write_str("role storage failure"),
        }
    }
}

impl Error for RoleError {}

pub fn permission_domains() -> &'static [&'static str] {
    &PERMISSION_DOMAINS
}

pub fn permission_codes() -> &'static [&'static str] {
    &PERMISSION_CODES
}

pub fn fixed_system_role_codes() -> &'static [&'static str] {
    &FIXED_SYSTEM_ROLE_CODES
}

pub fn seeded_roles() -> Vec<Role> {
    vec![
        Role {
            id: SnowflakeId::new(734_300_000_000_000_001),
            code: "ADMIN".into(),
            name: "Admin".into(),
            description: "Full System Access".into(),
            kind: RoleKind::System,
            managed_by: RoleManagedBy::Platform,
            permissions: full_permissions(),
            updated_at: DEFAULT_ROLE_UPDATED_AT.into(),
        },
        Role {
            id: SnowflakeId::new(734_300_000_000_000_002),
            code: "WORKSPACE_ADMIN".into(),
            name: "Workspace Admin".into(),
            description: "Workspace administration".into(),
            kind: RoleKind::System,
            managed_by: RoleManagedBy::Workspace,
            permissions: permissions_map(&[
                ("Workspace", &["R", "W"]),
                ("Models", &["R", "W"]),
                ("Tasks", &["R", "W"]),
                ("Memory", &["R"]),
                ("Skills", &["R", "W"]),
                ("MCP", &["R"]),
                ("Resources", &["R", "W"]),
            ]),
            updated_at: DEFAULT_ROLE_UPDATED_AT.into(),
        },
        Role {
            id: SnowflakeId::new(734_300_000_000_000_003),
            code: "DEVELOPER".into(),
            name: "Developer".into(),
            description: "Build and operate applications".into(),
            kind: RoleKind::System,
            managed_by: RoleManagedBy::Platform,
            permissions: permissions_map(&[
                ("Workspace", &["R"]),
                ("Models", &["R", "W"]),
                ("Tasks", &["R", "W"]),
                ("Memory", &["R"]),
                ("Skills", &["R", "W"]),
                ("MCP", &["R"]),
                ("Resources", &["R", "W"]),
            ]),
            updated_at: DEFAULT_ROLE_UPDATED_AT.into(),
        },
        Role {
            id: SnowflakeId::new(734_300_000_000_000_004),
            code: "VIEWER".into(),
            name: "Viewer".into(),
            description: "Read-only Access".into(),
            kind: RoleKind::System,
            managed_by: RoleManagedBy::Platform,
            permissions: permissions_map(&[
                ("Workspace", &["R"]),
                ("Models", &["R"]),
                ("Tasks", &["R"]),
                ("Memory", &["R"]),
                ("Skills", &["R"]),
                ("MCP", &["R"]),
                ("Resources", &["R"]),
            ]),
            updated_at: DEFAULT_ROLE_UPDATED_AT.into(),
        },
    ]
}

pub fn validate_role_catalog(roles: &[Role]) -> Result<(), RoleError> {
    let mut seen_codes = BTreeSet::new();
    let mut seen_names = BTreeSet::new();
    let mut role_codes = BTreeSet::new();

    for role in roles {
        validate_role_code(&role.code)?;
        if role.name.trim().is_empty() {
            return Err(RoleError::RoleNameRequired);
        }
        if !seen_codes.insert(role.code.clone()) {
            return Err(RoleError::DuplicateRoleCode);
        }
        if !seen_names.insert(role.name.trim().to_ascii_lowercase()) {
            return Err(RoleError::DuplicateRoleName);
        }
        role_codes.insert(role.code.clone());

        if fixed_system_role_codes().contains(&role.code.as_str()) && role.kind != RoleKind::System
        {
            return Err(RoleError::MissingRequiredSystemRole);
        }

        for domain in permission_domains() {
            let codes = role.permissions.get(*domain).cloned().unwrap_or_default();
            for code in &codes {
                validate_permission_code(code)?;
            }
        }

        for domain in role.permissions.keys() {
            validate_permission_domain(domain)?;
        }
    }

    if fixed_system_role_codes()
        .iter()
        .any(|required_code| !role_codes.contains(*required_code))
    {
        return Err(RoleError::MissingRequiredSystemRole);
    }

    Ok(())
}

pub fn validate_role_code(code: &str) -> Result<(), RoleError> {
    if code.trim().is_empty() {
        return Err(RoleError::InvalidRoleCode);
    }
    if code
        .chars()
        .any(|ch| !(ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'))
    {
        return Err(RoleError::InvalidRoleCode);
    }
    Ok(())
}

pub fn validate_permission_domain(domain: &str) -> Result<(), RoleError> {
    if permission_domains().contains(&domain) {
        Ok(())
    } else {
        Err(RoleError::InvalidPermissionDomain)
    }
}

pub fn validate_permission_code(code: &str) -> Result<(), RoleError> {
    if permission_codes().contains(&code) {
        Ok(())
    } else {
        Err(RoleError::InvalidPermissionCode)
    }
}

pub fn normalize_permissions(
    input: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<String, Vec<String>>, RoleError> {
    let mut normalized = BTreeMap::new();

    for domain in permission_domains() {
        let values = input.get(*domain).cloned().unwrap_or_default();
        let mut unique = BTreeSet::new();
        let mut ordered = Vec::new();

        for code in permission_codes() {
            if values.iter().any(|value| value == code) && unique.insert((*code).to_string()) {
                ordered.push((*code).to_string());
            }
        }

        normalized.insert((*domain).to_string(), ordered);
    }

    for domain in input.keys() {
        validate_permission_domain(domain)?;
    }

    Ok(normalized)
}

fn full_permissions() -> BTreeMap<String, Vec<String>> {
    permissions_map(&[
        ("Workspace", &["R", "W", "D", "A"]),
        ("Models", &["R", "W", "D", "A"]),
        ("Tasks", &["R", "W", "D", "A"]),
        ("Memory", &["R", "W", "D", "A"]),
        ("Skills", &["R", "W", "D", "A"]),
        ("MCP", &["R", "W", "D", "A"]),
        ("Resources", &["R", "W", "D", "A"]),
    ])
}

fn permissions_map(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    let mut mapping = BTreeMap::new();

    for domain in permission_domains() {
        let codes = entries
            .iter()
            .find(|(entry_domain, _)| entry_domain == domain)
            .map(|(_, codes)| codes.iter().map(|value| (*value).to_string()).collect())
            .unwrap_or_default();
        mapping.insert((*domain).to_string(), codes);
    }

    mapping
}
