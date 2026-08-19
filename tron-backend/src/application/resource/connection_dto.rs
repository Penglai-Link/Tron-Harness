use serde::Serialize;
use serde_json::Value;

use crate::domain::resource::{ResourceKind, ServerAuthenticationType};

#[derive(Debug, Clone)]
pub struct DataSourceCreateCommand {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    pub description: String,
    pub attributes: Value,
    pub allow_inheritance: bool,
}

#[derive(Debug, Clone)]
pub struct DataSourceUpdateCommand {
    pub workspace_code: String,
    pub resource_code: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub attributes: Option<Value>,
    pub allow_inheritance: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct ServerAccountCreateInput {
    pub account_code: String,
    pub name: String,
    pub description: String,
    pub username: String,
    pub authentication_type: ServerAuthenticationType,
    pub password_variable_key: Option<String>,
    pub private_key_variable_key: Option<String>,
    pub private_key_passphrase_variable_key: Option<String>,
    pub allow_inheritance: bool,
}

#[derive(Debug, Clone)]
pub struct ServerCreateCommand {
    pub workspace_code: String,
    pub server_code: String,
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: u16,
    pub allow_inheritance: bool,
    pub initial_account: ServerAccountCreateInput,
}

#[derive(Debug, Clone)]
pub struct ServerAccountCreateCommand {
    pub workspace_code: String,
    pub server_code: String,
    pub account: ServerAccountCreateInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCommitOutcome {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCreateResult {
    pub server: ResourceCommitOutcome,
    pub initial_account: ResourceCommitOutcome,
    pub connectivity_verified: bool,
}
