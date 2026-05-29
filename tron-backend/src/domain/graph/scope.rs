use crate::domain::workspace::{WorkspaceLineageItem, validate_workspace_code};

use super::GraphScopeError;

pub const STORAGE_ABSENT_SENTINEL: &str = "__NONE__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphScopeCommand {
    pub namespace: String,
    pub workspace_code: String,
    pub app_code: Option<String>,
    pub user_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphScope {
    pub namespace: String,
    pub workspace_code: String,
    pub app_code: Option<String>,
    pub user_code: Option<String>,
}

impl GraphScope {
    pub fn from_command(command: GraphScopeCommand) -> Result<Self, GraphScopeError> {
        let namespace = normalize_required(command.namespace, GraphScopeError::InvalidNamespace)?;
        validate_namespace(&namespace)?;
        let workspace_code = normalize_required(
            command.workspace_code,
            GraphScopeError::InvalidWorkspaceCode,
        )?;
        validate_workspace_code(&workspace_code)
            .map_err(|_| GraphScopeError::InvalidWorkspaceCode)?;

        Ok(Self {
            namespace,
            workspace_code,
            app_code: normalize_optional_dimension(command.app_code)?,
            user_code: normalize_optional_dimension(command.user_code)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGraphScope {
    pub schema: String,
    pub named_graph: String,
    pub root_workspace_code: String,
    pub current_workspace_code: String,
    pub visible_workspace_codes: Vec<String>,
    pub storage_dimensions: GraphStorageDimensions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStorageDimensions {
    pub namespace: String,
    pub workspace_code: String,
    pub app_code: String,
    pub user_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphScopeDimension {
    App,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphStorageProperty {
    Namespace,
    WorkspaceCode,
    AppCode,
    UserCode,
    CreatedAt,
    UpdatedAt,
}

impl GraphStorageProperty {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Namespace => "namespace",
            Self::WorkspaceCode => "workspaceCode",
            Self::AppCode => "appCode",
            Self::UserCode => "userCode",
            Self::CreatedAt => "createdAt",
            Self::UpdatedAt => "updatedAt",
        }
    }

    pub fn is_reserved(property_name: &str) -> bool {
        Self::from_storage_name(property_name).is_some()
    }

    pub fn from_storage_name(property_name: &str) -> Option<Self> {
        match property_name {
            "namespace" => Some(Self::Namespace),
            "workspaceCode" => Some(Self::WorkspaceCode),
            "appCode" => Some(Self::AppCode),
            "userCode" => Some(Self::UserCode),
            "createdAt" => Some(Self::CreatedAt),
            "updatedAt" => Some(Self::UpdatedAt),
            _ => None,
        }
    }
}

pub fn ensure_not_reserved_storage_property(property_name: &str) -> Result<(), GraphScopeError> {
    if GraphStorageProperty::is_reserved(property_name.trim()) {
        return Err(GraphScopeError::ReservedPropertyName);
    }

    Ok(())
}

pub fn resolve_storage_scope(
    scope: GraphScope,
    path: &[WorkspaceLineageItem],
) -> Result<ResolvedGraphScope, GraphScopeError> {
    let root_workspace_code = path
        .first()
        .ok_or(GraphScopeError::EmptyWorkspaceLineage)?
        .workspace_code
        .clone();
    let visible_workspace_codes = path
        .iter()
        .map(|item| item.workspace_code.clone())
        .collect::<Vec<_>>();
    let current_workspace_code = scope.workspace_code.clone();
    let namespace = scope.namespace.clone();

    Ok(ResolvedGraphScope {
        schema: namespace.clone(),
        named_graph: root_workspace_code.clone(),
        root_workspace_code,
        current_workspace_code: current_workspace_code.clone(),
        visible_workspace_codes,
        storage_dimensions: GraphStorageDimensions {
            namespace,
            workspace_code: current_workspace_code,
            app_code: scope
                .app_code
                .unwrap_or_else(|| STORAGE_ABSENT_SENTINEL.to_owned()),
            user_code: scope
                .user_code
                .unwrap_or_else(|| STORAGE_ABSENT_SENTINEL.to_owned()),
        },
    })
}

fn normalize_required(value: String, error: GraphScopeError) -> Result<String, GraphScopeError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(error);
    }

    Ok(value.to_owned())
}

fn normalize_optional_dimension(value: Option<String>) -> Result<Option<String>, GraphScopeError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value == STORAGE_ABSENT_SENTINEL {
        return Err(GraphScopeError::ReservedSentinelValue);
    }

    Ok(Some(value.to_owned()))
}

fn validate_namespace(namespace: &str) -> Result<(), GraphScopeError> {
    if namespace
        .chars()
        .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
    {
        return Err(GraphScopeError::InvalidNamespace);
    }

    Ok(())
}
