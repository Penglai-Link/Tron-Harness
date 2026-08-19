use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::resource::{
        DataSourceCreateCommand, DataSourceUpdateCommand, ResourceBinaryDownloadCommand,
        ResourceBinaryUploadCommand, ResourceCreateCommand, ResourceDeleteCommand,
        ResourceGetQuery, ResourcePageQuery, ResourceUpdateCommand, ServerAccountCreateCommand,
        ServerAccountCreateInput, ServerCreateCommand,
    },
    domain::resource::{
        ResourceError, ResourceKind, ResourceSourceKind, ServerAuthenticationType,
        is_connection_managed_kind, is_data_source_kind,
    },
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

impl From<ResourceCreateRequest> for ResourceCreateCommand {
    fn from(value: ResourceCreateRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            kind: value.kind,
            name: value.name,
            description: value.description.unwrap_or_default(),
            allow_inheritance: value.allow_inheritance,
        }
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

impl From<ResourceUpdateRequest> for ResourceUpdateCommand {
    fn from(value: ResourceUpdateRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            name: value.name,
            description: value.description,
            allow_inheritance: value.allow_inheritance,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceDeleteRequest {
    pub workspace_code: String,
    pub resource_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionDeleteRequest {
    pub workspace_code: String,
    pub resource_code: String,
}

impl From<ConnectionDeleteRequest> for ResourceDeleteCommand {
    fn from(value: ConnectionDeleteRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionPageRequest {
    pub workspace_code: String,
    pub kinds: Vec<ResourceKind>,
    pub search: Option<String>,
    pub current: usize,
    pub size: usize,
}

impl TryFrom<ConnectionPageRequest> for ResourcePageQuery {
    type Error = ResourceError;

    fn try_from(value: ConnectionPageRequest) -> Result<Self, Self::Error> {
        if value
            .kinds
            .iter()
            .any(|kind| !is_connection_managed_kind(kind.as_str()))
        {
            return Err(ResourceError::InvalidInput);
        }
        Ok(Self {
            workspace_code: value.workspace_code,
            kinds: value.kinds,
            source_kind: None,
            search: value.search,
            current: value.current,
            size: value.size,
        })
    }
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataSourceCreateRequest {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub attributes: Value,
    pub allow_inheritance: bool,
}

impl TryFrom<DataSourceCreateRequest> for DataSourceCreateCommand {
    type Error = ResourceError;

    fn try_from(value: DataSourceCreateRequest) -> Result<Self, ResourceError> {
        if !is_data_source_kind(value.kind.as_str()) {
            return Err(ResourceError::InvalidInput);
        }
        Ok(Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            kind: value.kind,
            name: value.name,
            description: value.description.unwrap_or_default(),
            attributes: value.attributes,
            allow_inheritance: value.allow_inheritance,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataSourceUpdateRequest {
    pub workspace_code: String,
    pub resource_code: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub attributes: Option<Value>,
    #[serde(default)]
    pub allow_inheritance: Option<bool>,
}

impl From<DataSourceUpdateRequest> for DataSourceUpdateCommand {
    fn from(value: DataSourceUpdateRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            name: value.name,
            description: value.description,
            attributes: value.attributes,
            allow_inheritance: value.allow_inheritance,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountCreateInputRequest {
    pub account_code: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub username: String,
    pub authentication_type: ServerAuthenticationType,
    #[serde(default)]
    pub password_variable_key: Option<String>,
    #[serde(default)]
    pub private_key_variable_key: Option<String>,
    #[serde(default)]
    pub private_key_passphrase_variable_key: Option<String>,
    pub allow_inheritance: bool,
}

impl From<ServerAccountCreateInputRequest> for ServerAccountCreateInput {
    fn from(value: ServerAccountCreateInputRequest) -> Self {
        Self {
            account_code: value.account_code,
            name: value.name,
            description: value.description.unwrap_or_default(),
            username: value.username,
            authentication_type: value.authentication_type,
            password_variable_key: value.password_variable_key,
            private_key_variable_key: value.private_key_variable_key,
            private_key_passphrase_variable_key: value.private_key_passphrase_variable_key,
            allow_inheritance: value.allow_inheritance,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerCreateRequest {
    pub workspace_code: String,
    pub server_code: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub host: String,
    pub port: u16,
    pub allow_inheritance: bool,
    pub initial_account: ServerAccountCreateInputRequest,
}

impl From<ServerCreateRequest> for ServerCreateCommand {
    fn from(value: ServerCreateRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            server_code: value.server_code,
            name: value.name,
            description: value.description.unwrap_or_default(),
            host: value.host,
            port: value.port,
            allow_inheritance: value.allow_inheritance,
            initial_account: value.initial_account.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountCreateRequest {
    pub workspace_code: String,
    pub server_code: String,
    pub account: ServerAccountCreateInputRequest,
}

impl From<ServerAccountCreateRequest> for ServerAccountCreateCommand {
    fn from(value: ServerAccountCreateRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            server_code: value.server_code,
            account: value.account.into(),
        }
    }
}

pub fn binary_upload_command(
    workspace_code: &str,
    resource_code: &str,
) -> Result<ResourceBinaryUploadCommand, ResourceError> {
    if workspace_code.trim().is_empty() {
        return Err(ResourceError::WorkspaceRequired);
    }
    Ok(ResourceBinaryUploadCommand {
        workspace_code: workspace_code.to_string(),
        resource_code: resource_code.to_string(),
    })
}
