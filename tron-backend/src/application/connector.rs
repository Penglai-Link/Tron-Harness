use std::sync::Arc;

use async_trait::async_trait;
use eliteforge_agent_connector::{
    Connection, ConnectionResolver, ConnectorError, ConnectorProvider, ConnectorService,
    Credential, Page, PageRequest, Principal, ProviderDescriptor, Repository,
};

use crate::{
    application::{variable::VariableCipher, workspace::WorkspaceService},
    domain::{
        connector::{ConnectionError, ConnectionView, StoredConnection},
        variable::EncryptedVariableValue,
    },
    shared::id::{IdGenerator, SnowflakedSnowflakeIdProvider},
};

pub trait ConnectionRepository: Send + Sync {
    fn list(&self, workspace_code: &str, user_code: &str) -> Result<Vec<StoredConnection>, ConnectionError>;
    fn get(&self, code: &str) -> Result<StoredConnection, ConnectionError>;
    fn save(&self, connection: &StoredConnection) -> Result<(), ConnectionError>;
    fn delete(&self, code: &str) -> Result<(), ConnectionError>;
}

struct HostResolver {
    repository: Arc<dyn ConnectionRepository>,
    cipher: Arc<dyn VariableCipher>,
    workspaces: WorkspaceService,
}

fn credential_aad(workspace: &str, owner: &str, code: &str) -> String {
    format!("connector:{workspace}:{owner}:{code}")
}

#[async_trait]
impl ConnectionResolver for HostResolver {
    async fn resolve(
        &self,
        principal: &Principal,
        connection_id: &str,
    ) -> Result<(Connection, Credential), ConnectorError> {
        if principal.id.is_empty()
            || !matches!(
                self.workspaces.workspace_is_active(&principal.scope),
                Ok(true)
            )
        {
            return Err(ConnectorError::Forbidden);
        }
        let stored = self
            .repository
            .get(connection_id)
            .map_err(|error| match error {
                ConnectionError::NotFound => ConnectorError::NotFound,
                _ => ConnectorError::Unavailable,
            })?;
        if stored.workspace_code != principal.scope || stored.owner_user_code != principal.id {
            return Err(ConnectorError::Forbidden);
        }
        let encrypted: EncryptedVariableValue = serde_json::from_str(&stored.encrypted_credential)
            .map_err(|_| ConnectorError::Configuration)?;
        let token = self
            .cipher
            .decrypt(
                &encrypted,
                credential_aad(&stored.workspace_code, &stored.owner_user_code, &stored.connection_code).as_bytes(),
            )
            .map_err(|_| ConnectorError::Authentication)?;
        Ok((
            Connection {
                id: stored.connection_code,
                provider: stored.provider_id,
                endpoint: stored.endpoint,
            },
            Credential::new(token.into_bytes()),
        ))
    }
}

#[derive(Clone)]
pub struct IntegrationService {
    repository: Arc<dyn ConnectionRepository>,
    cipher: Arc<dyn VariableCipher>,
    workspaces: WorkspaceService,
    connectors: Arc<ConnectorService>,
    ids: Arc<dyn IdGenerator>,
}

impl IntegrationService {
    pub(crate) fn connector_service(&self) -> Arc<ConnectorService> { self.connectors.clone() }
    pub fn new(
        repository: impl ConnectionRepository + 'static,
        cipher: Arc<dyn VariableCipher>,
        workspaces: WorkspaceService,
        providers: Vec<Arc<dyn ConnectorProvider>>,
    ) -> Result<Self, ConnectionError> {
        let repository: Arc<dyn ConnectionRepository> = Arc::new(repository);
        let resolver = Arc::new(HostResolver {
            repository: repository.clone(),
            cipher: cipher.clone(),
            workspaces: workspaces.clone(),
        });
        let connectors =
            ConnectorService::new(providers, resolver).map_err(ConnectionError::Remote)?;
        Ok(Self {
            repository,
            cipher,
            workspaces,
            connectors: Arc::new(connectors),
            ids: Arc::new(SnowflakedSnowflakeIdProvider::default()),
        })
    }

    fn check_workspace(&self, workspace_code: &str) -> Result<(), ConnectionError> {
        match self.workspaces.workspace_is_active(workspace_code) {
            Ok(true) => Ok(()),
            _ => Err(ConnectionError::WorkspaceInvalid),
        }
    }

    pub fn providers(&self) -> Vec<ProviderDescriptor> {
        self.connectors.providers()
    }

    pub fn list(&self, workspace_code: &str, user_code: &str) -> Result<Vec<ConnectionView>, ConnectionError> {
        self.check_workspace(workspace_code)?;
        if user_code.is_empty() { return Err(ConnectionError::NotFound); }
        Ok(self
            .repository
            .list(workspace_code, user_code)?
            .into_iter()
            .map(ConnectionView::from)
            .collect())
    }

    pub fn contains(&self, workspace_code: &str, user_code: &str, code: &str) -> bool {
        self.check_workspace(workspace_code).is_ok()
            && !user_code.is_empty()
            && self.repository.get(code).is_ok_and(|value| value.workspace_code == workspace_code && value.owner_user_code == user_code)
    }

    pub fn save(
        &self,
        workspace_code: &str,
        user_code: &str,
        code: Option<&str>,
        provider: &str,
        name: &str,
        endpoint: &str,
        token: Option<&str>,
    ) -> Result<ConnectionView, ConnectionError> {
        self.check_workspace(workspace_code)?;
        if user_code.is_empty() { return Err(ConnectionError::NotFound); }
        let name = name.trim();
        let endpoint = endpoint.trim().trim_end_matches('/');
        if name.is_empty()
            || name.chars().count() > 200
            || !self.connectors.providers().iter().any(|p| p.id == provider)
        {
            return Err(ConnectionError::InvalidInput);
        }
        let url = url::Url::parse(endpoint).map_err(|_| ConnectionError::InvalidInput)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(ConnectionError::InvalidInput);
        }
        if (provider == "github" && endpoint != "https://api.github.com")
            || (provider == "gitee" && endpoint != "https://gitee.com")
        {
            return Err(ConnectionError::InvalidInput);
        }
        let existing = match code {
            Some(code) => {
                let value = self.repository.get(code)?;
                if value.workspace_code != workspace_code || value.owner_user_code != user_code || value.provider_id != provider {
                    return Err(ConnectionError::NotFound);
                }
                Some(value)
            }
            None => None,
        };
        let connection_code = match &existing {
            Some(value) => value.connection_code.clone(),
            None => format!(
                "CONNECTOR-{}",
                self.ids
                    .next_id()
                    .map_err(|_| ConnectionError::StorageFailure)?
                    .value()
            ),
        };
        let encrypted_credential = match token.filter(|token| !token.trim().is_empty()) {
            Some(token) => {
                let encrypted = self
                    .cipher
                    .encrypt(
                        token,
                        credential_aad(workspace_code, user_code, &connection_code).as_bytes(),
                    )
                    .map_err(|_| ConnectionError::EncryptionUnavailable)?;
                serde_json::to_string(&encrypted).map_err(|_| ConnectionError::StorageFailure)?
            }
            None => existing
                .as_ref()
                .map(|value| value.encrypted_credential.clone())
                .ok_or(ConnectionError::InvalidInput)?,
        };
        let stored = StoredConnection {
            connection_code,
            workspace_code: workspace_code.into(),
            owner_user_code: user_code.into(),
            provider_id: provider.into(),
            name: name.into(),
            endpoint: endpoint.into(),
            encrypted_credential,
        };
        self.repository.save(&stored)?;
        Ok(stored.into())
    }

    pub fn delete(&self, workspace_code: &str, user_code: &str, code: &str) -> Result<(), ConnectionError> {
        self.check_workspace(workspace_code)?;
        let connection = self.repository.get(code)?;
        if user_code.is_empty() || connection.workspace_code != workspace_code || connection.owner_user_code != user_code {
            return Err(ConnectionError::NotFound);
        }
        self.repository.delete(code)
    }

    async fn connect(
        &self,
        workspace_code: &str,
        user_code: &str,
        code: &str,
    ) -> Result<Arc<dyn eliteforge_agent_connector::ConnectedProvider>, ConnectionError> {
        self.connectors
            .connect(
                &Principal {
                    id: user_code.into(),
                    scope: workspace_code.into(),
                },
                code,
            )
            .await
            .map_err(ConnectionError::Remote)
    }

    pub async fn check(
        &self,
        workspace_code: &str,
        user_code: &str,
        code: &str,
    ) -> Result<(), ConnectionError> {
        self.connect(workspace_code, user_code, code)
            .await?
            .check()
            .await
            .map_err(ConnectionError::Remote)
    }

    pub async fn repositories(
        &self,
        workspace_code: &str,
        user_code: &str,
        code: &str,
        query: &str,
        page: PageRequest,
    ) -> Result<Page<Repository>, ConnectionError> {
        let connected = self.connect(workspace_code, user_code, code).await?;
        connected
            .repositories()
            .ok_or(ConnectionError::Remote(ConnectorError::Unsupported))?
            .search(query, page)
            .await
            .map_err(ConnectionError::Remote)
    }

    /// Fetch current provider metadata instead of trusting a client-supplied
    /// clone URL. Connection resolution is scoped to the requesting user.
    pub async fn repository(
        &self,
        workspace_code: &str,
        user_code: &str,
        code: &str,
        repository_id: &str,
    ) -> Result<Repository, ConnectionError> {
        self.connect(workspace_code, user_code, code)
            .await?
            .repositories()
            .ok_or(ConnectionError::Remote(ConnectorError::Unsupported))?
            .get(repository_id)
            .await
            .map_err(ConnectionError::Remote)
    }

    /// Only the host may obtain a credential, after the same ownership check
    /// used by connector calls. Never return this value through an API DTO.
    pub(crate) fn git_credential(
        &self,
        workspace_code: &str,
        user_code: &str,
        code: &str,
    ) -> Result<(StoredConnection, String), ConnectionError> {
        self.check_workspace(workspace_code)?;
        let stored = self.repository.get(code)?;
        if user_code.is_empty() || stored.workspace_code != workspace_code || stored.owner_user_code != user_code {
            return Err(ConnectionError::NotFound);
        }
        let encrypted: EncryptedVariableValue = serde_json::from_str(&stored.encrypted_credential)
            .map_err(|_| ConnectionError::StorageFailure)?;
        let token = self.cipher.decrypt(
            &encrypted,
            credential_aad(workspace_code, user_code, code).as_bytes(),
        ).map_err(|_| ConnectionError::EncryptionUnavailable)?;
        Ok((stored, token))
    }
}
