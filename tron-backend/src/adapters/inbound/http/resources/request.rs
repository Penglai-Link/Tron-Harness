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
        is_connection_managed_kind, is_data_source_kind, validate_resource_code,
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
            return Err(ResourceError::InvalidField {
                field: "kinds".to_string(),
                reason: "must contain only connection resource kinds".to_string(),
            });
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
            return Err(ResourceError::InvalidField {
                field: "kind".to_string(),
                reason: "must be a data-source connection kind".to_string(),
            });
        }
        if value.name.trim().is_empty() {
            return Err(ResourceError::InvalidField {
                field: "name".to_string(),
                reason: "must not be blank".to_string(),
            });
        }
        validate_code(&value.resource_code, "resourceCode")?;
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

impl TryFrom<DataSourceUpdateRequest> for DataSourceUpdateCommand {
    type Error = ResourceError;

    fn try_from(value: DataSourceUpdateRequest) -> Result<Self, Self::Error> {
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
            attributes: value.attributes,
            allow_inheritance: value.allow_inheritance,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountCreateInputRequest {
    pub account_code: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub username: String,
    pub authentication_type: ServerAuthenticationType,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub private_key: Option<String>,
    #[serde(default)]
    pub private_key_passphrase: Option<String>,
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
            password: value.password,
            private_key: value.private_key,
            private_key_passphrase: value.private_key_passphrase,
            allow_inheritance: value.allow_inheritance,
        }
    }
}

#[derive(Deserialize)]
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
    #[serde(default)]
    pub initial_account: Option<ServerAccountCreateInputRequest>,
}

impl TryFrom<ServerCreateRequest> for ServerCreateCommand {
    type Error = ResourceError;

    fn try_from(value: ServerCreateRequest) -> Result<Self, Self::Error> {
        validate_non_blank(&value.workspace_code, "workspaceCode")?;
        validate_code(&value.server_code, "serverCode")?;
        validate_non_blank(&value.name, "name")?;
        validate_non_blank(&value.host, "host")?;
        let initial_account = value
            .initial_account
            .map(|initial_account| {
                validate_code(&initial_account.account_code, "initialAccount.accountCode")?;
                validate_non_blank(&initial_account.name, "initialAccount.name")?;
                validate_non_blank(&initial_account.username, "initialAccount.username")?;
                Ok(initial_account.into())
            })
            .transpose()?;
        Ok(Self {
            workspace_code: value.workspace_code,
            server_code: value.server_code,
            name: value.name,
            description: value.description.unwrap_or_default(),
            host: value.host,
            port: value.port,
            allow_inheritance: value.allow_inheritance,
            initial_account,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountCreateRequest {
    pub workspace_code: String,
    pub account: ServerAccountCreateInputRequest,
}

impl TryFrom<ServerAccountCreateRequest> for ServerAccountCreateCommand {
    type Error = ResourceError;

    fn try_from(value: ServerAccountCreateRequest) -> Result<Self, Self::Error> {
        validate_non_blank(&value.workspace_code, "workspaceCode")?;
        validate_code(&value.account.account_code, "account.accountCode")?;
        validate_non_blank(&value.account.name, "account.name")?;
        validate_non_blank(&value.account.username, "account.username")?;
        Ok(Self {
            workspace_code: value.workspace_code,
            account: value.account.into(),
        })
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
