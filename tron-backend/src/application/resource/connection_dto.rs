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

#[derive(Clone)]
pub struct ServerAccountCreateInput {
    pub account_code: String,
    pub name: String,
    pub description: String,
    pub username: String,
    pub authentication_type: ServerAuthenticationType,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub private_key_passphrase: Option<String>,
    pub allow_inheritance: bool,
}

#[derive(Clone)]
pub struct ServerCreateCommand {
    pub workspace_code: String,
    pub server_code: String,
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: u16,
    pub allow_inheritance: bool,
    /// Optional for the HTTP resource flow. Server accounts are managed as a
    /// separate resource and can be added after the server exists. Agent
    /// tools may still provide this value to keep the atomic create-and-verify
    /// workflow.
    pub initial_account: Option<ServerAccountCreateInput>,
}

#[derive(Clone)]
pub struct ServerAccountCreateCommand {
    pub workspace_code: String,
    pub account: ServerAccountCreateInput,
}

#[derive(Clone)]
pub struct ServerAccountUpdateCommand {
    pub workspace_code: String,
    pub account_code: String,
    pub name: String,
    pub description: String,
    pub username: String,
    pub authentication_type: ServerAuthenticationType,
    /// None preserves the existing credential when the authentication type is unchanged.
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub private_key_passphrase: Option<String>,
    pub allow_inheritance: bool,
}

#[derive(Debug, Clone)]
pub struct ServerAccountAssignCommand {
    pub workspace_code: String,
    pub account_code: String,
    pub server_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ServerAccountVerifyCommand {
    pub workspace_code: String,
    pub account_code: String,
    pub server_code: String,
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
    pub initial_account: Option<ResourceCommitOutcome>,
    pub connectivity_verified: bool,
}
