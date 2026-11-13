use serde::Deserialize;

use crate::{
    application::resource::{
        ResourceBinaryDownloadCommand, ResourceBinaryUploadCommand, ResourceCreateCommand,
        ResourceDeleteCommand, ResourceGetQuery, ResourcePageQuery, ResourceUpdateCommand,
    },
    domain::resource::{ResourceError, ResourceKind, ResourceSourceKind, validate_resource_code},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourcePageRequest {
    pub workspace_code: String,
    pub kinds: Vec<ResourceKind>,
    pub source_kind: Option<ResourceSourceKind>,
    pub search: Option<String>,
    pub current: usize,
    pub size: usize,
}

impl From<ResourcePageRequest> for ResourcePageQuery {
    fn from(value: ResourcePageRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            kinds: value.kinds,
            source_kind: value.source_kind,
            search: value.search,
            current: value.current,
            size: value.size,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceGetRequest {
    pub workspace_code: String,
    pub resource_code: String,
}

impl From<ResourceGetRequest> for ResourceGetQuery {
    fn from(value: ResourceGetRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceCreateRequest {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub allow_inheritance: bool,
}

impl TryFrom<ResourceCreateRequest> for ResourceCreateCommand {
    type Error = ResourceError;

    fn try_from(value: ResourceCreateRequest) -> Result<Self, Self::Error> {
        validate_non_blank(&value.workspace_code, "workspaceCode")?;
        validate_code(&value.resource_code, "resourceCode")?;
        validate_non_blank(&value.name, "name")?;
        if !value.kind.is_file_kind() {
            return Err(ResourceError::InvalidField {
                field: "kind".to_string(),
                reason: "must be a file resource kind".to_string(),
            });
        }
        Ok(Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            kind: value.kind,
            name: value.name,
            description: value.description.unwrap_or_default(),
            allow_inheritance: value.allow_inheritance,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceUpdateRequest {
    pub workspace_code: String,
    pub resource_code: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub allow_inheritance: Option<bool>,
}

impl TryFrom<ResourceUpdateRequest> for ResourceUpdateCommand {
    type Error = ResourceError;

    fn try_from(value: ResourceUpdateRequest) -> Result<Self, Self::Error> {
        validate_non_blank(&value.workspace_code, "workspaceCode")?;
        validate_code(&value.resource_code, "resourceCode")?;
        if value
            .name
            .as_deref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(ResourceError::InvalidField {
                field: "name".to_string(),
                reason: "must not be blank".to_string(),
            });
        }
        Ok(Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            name: value.name,
            description: value.description,
            allow_inheritance: value.allow_inheritance,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceDeleteRequest {
    pub workspace_code: String,
    pub resource_code: String,
}

impl From<ResourceDeleteRequest> for ResourceDeleteCommand {
    fn from(value: ResourceDeleteRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceBinaryDownloadRequest {
    pub workspace_code: String,
    pub resource_code: String,
}

impl From<ResourceBinaryDownloadRequest> for ResourceBinaryDownloadCommand {
    fn from(value: ResourceBinaryDownloadRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
        }
    }
}

fn validate_non_blank(value: &str, field: &str) -> Result<(), ResourceError> {
    if value.trim().is_empty() {
        return Err(ResourceError::InvalidField {
            field: field.to_string(),
            reason: "must not be blank".to_string(),
        });
    }
    Ok(())
}

fn validate_code(value: &str, field: &str) -> Result<(), ResourceError> {
    validate_resource_code(value).map_err(|error| match error {
        ResourceError::InvalidCode => ResourceError::InvalidField {
            field: field.to_string(),
            reason: "must match the resource code format".to_string(),
        },
        error => error,
    })
}

pub fn binary_upload_command(
    workspace_code: &str,
    resource_code: &str,
) -> Result<ResourceBinaryUploadCommand, ResourceError> {
    if workspace_code.trim().is_empty() {
        return Err(ResourceError::WorkspaceRequired);
    }
    validate_code(resource_code, "resourceCode")?;
    Ok(ResourceBinaryUploadCommand {
        workspace_code: workspace_code.to_string(),
        resource_code: resource_code.to_string(),
    })
}
