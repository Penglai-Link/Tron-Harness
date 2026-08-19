use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::resource::{
        DataSourceCreateCommand, DataSourceUpdateCommand, ResourceBinaryDownloadCommand,
        ResourceBinaryUploadCommand, ResourceCreateCommand, ResourceDeleteCommand,
        ResourceGetQuery, ResourcePageQuery, ResourceUpdateCommand, ServerAccountCreateCommand,
        ServerAccountCreateInput, ServerCreateCommand,
    },
    domain::resource::{ResourceError, ResourceKind, ServerAuthenticationType, is_data_source_kind},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourcePageRequest {
    pub workspace_code: String,
    #[serde(default)]
    pub kind: Option<ResourceKind>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub current: Option<usize>,
    #[serde(default)]
    pub size: Option<usize>,
}

impl From<ResourcePageRequest> for ResourcePageQuery {
    fn from(value: ResourcePageRequest) -> Self {
        Self {
            workspace_code: value.workspace_code,
            kind: value.kind,
            search: value.search,
            current: value.current.unwrap_or(1),
            size: value.size.unwrap_or(20),
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
    #[serde(default = "default_true")]
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
    #[serde(default = "default_true")]
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
    #[serde(default = "default_true")]
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
    #[serde(default = "default_true")]
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

fn default_true() -> bool {
    true
}
