use serde::Serialize;
use serde_json::Value;

use crate::domain::resource::{ResourceScopeKind, ServerAuthenticationType};

use super::ResourceActivationOutput;

#[derive(Clone)]
pub struct DataSourceCreateCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub resource_code: String,
    pub definition_code: String,
    pub name: String,
    pub description: String,
    pub attributes: Value,
    pub allow_inheritance: bool,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct ServerCreateCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub server_code: String,
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: u16,
    pub allow_inheritance: bool,
    pub initial_account: ServerAccountCreateInput,
}

#[derive(Clone)]
pub struct ServerAccountCreateCommand {
    pub scope_kind: ResourceScopeKind,
    pub workspace_code: Option<String>,
    pub server_code: String,
    pub account: ServerAccountCreateInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCreateResult {
    pub server: ResourceActivationOutput,
    pub initial_account: ResourceActivationOutput,
    pub connectivity_verified: bool,
}
