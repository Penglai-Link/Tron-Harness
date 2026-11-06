use serde::Serialize;

/// Host-owned connection metadata. The encrypted credential is never a DTO.
#[derive(Clone)]
pub struct StoredConnection {
    pub connection_code: String,
    pub workspace_code: String,
    pub owner_user_code: String,
    pub provider_id: String,
    pub name: String,
    pub endpoint: String,
    pub encrypted_credential: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub connection_code: String,
    pub workspace_code: String,
    pub provider_id: String,
    pub name: String,
    pub endpoint: String,
}

impl From<StoredConnection> for ConnectionView {
    fn from(value: StoredConnection) -> Self {
        Self {
            connection_code: value.connection_code,
            workspace_code: value.workspace_code,
            provider_id: value.provider_id,
            name: value.name,
            endpoint: value.endpoint,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ConnectionError {
    WorkspaceInvalid,
    InvalidInput,
    NotFound,
    DuplicateName,
    EncryptionUnavailable,
    StorageFailure,
    Remote(eliteforge_agent_connector::ConnectorError),
}
