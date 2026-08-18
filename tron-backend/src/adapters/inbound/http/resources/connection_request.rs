use serde::Deserialize;
use serde_json::Value;

use crate::{
    application::resource::{
        DataSourceCreateCommand, ServerAccountCreateCommand, ServerAccountCreateInput,
        ServerCreateCommand,
    },
    domain::resource::ServerAuthenticationType,
};

use super::request::ScopeCode;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataSourceCreateRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub definition_code: String,
    pub name: String,
    pub description: Option<String>,
    pub attributes: Value,
    pub allow_inheritance: Option<bool>,
}

impl From<DataSourceCreateRequest> for DataSourceCreateCommand {
    fn from(value: DataSourceCreateRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            definition_code: value.definition_code,
            name: value.name,
            description: value.description.unwrap_or_default(),
            attributes: value.attributes,
            allow_inheritance: value.allow_inheritance.unwrap_or(false),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountCreateInputRequest {
    pub account_code: String,
    pub name: String,
    pub description: Option<String>,
    pub username: String,
    pub authentication_type: ServerAuthenticationType,
    pub password_variable_key: Option<String>,
    pub private_key_variable_key: Option<String>,
    pub private_key_passphrase_variable_key: Option<String>,
    pub allow_inheritance: Option<bool>,
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
            allow_inheritance: value.allow_inheritance.unwrap_or(false),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerCreateRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub server_code: String,
    pub name: String,
    pub description: Option<String>,
    pub host: String,
    pub port: u16,
    pub allow_inheritance: Option<bool>,
    pub initial_account: ServerAccountCreateInputRequest,
}

impl From<ServerCreateRequest> for ServerCreateCommand {
    fn from(value: ServerCreateRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            server_code: value.server_code,
            name: value.name,
            description: value.description.unwrap_or_default(),
            host: value.host,
            port: value.port,
            allow_inheritance: value.allow_inheritance.unwrap_or(false),
            initial_account: value.initial_account.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountCreateRequest {
    pub scope_kind: ScopeCode,
    pub workspace_code: Option<String>,
    pub server_code: String,
    pub account: ServerAccountCreateInputRequest,
}

impl From<ServerAccountCreateRequest> for ServerAccountCreateCommand {
    fn from(value: ServerAccountCreateRequest) -> Self {
        Self {
            scope_kind: value.scope_kind.into(),
            workspace_code: value.workspace_code,
            server_code: value.server_code,
            account: value.account.into(),
        }
    }
}
