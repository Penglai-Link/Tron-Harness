//! Host credential bridge; Framework owns all remote tool implementations.
use super::*;
use eliteforge_agent_capability_personai::{AccountContext, PersonAiContextProvider};
use tokio::sync::watch;

struct ToolContext {
    providers: ModelProviderService,
    identity: String,
    account: Arc<AccountContext>,
    sender: watch::Sender<Option<Arc<AccountContext>>>,
}
impl PersonAiContextProvider for ToolContext {
    fn current(&self) -> Option<Arc<AccountContext>> {
        if self
            .providers
            .personai_tool_identity()
            .ok()
            .flatten()
            .as_deref()
            == Some(&self.identity)
        {
            Some(self.account.clone())
        } else {
            if self.sender.borrow().is_some() {
                self.sender.send_replace(None);
            }
            None
        }
    }
    fn subscribe(&self) -> watch::Receiver<Option<Arc<AccountContext>>> {
        self.sender.subscribe()
    }
}
impl ModelProviderService {
    fn personai_tool_identity(&self) -> Result<Option<String>, ModelProviderError> {
        let this = self.clone();
        self.connection.call(
            move |connection| {
                let Some(row) = read_connection(connection, ModelProviderCode::PersonAi)? else {
                    return Ok(None);
                };
                if !matches!(
                    row.status,
                    ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
                ) {
                    return Ok(None);
                }
                let Some(encrypted) = row.credentials_encrypted.as_deref() else {
                    return Ok(None);
                };
                let credentials = this.decrypt_credentials(encrypted)?;
                Ok(Some(this.sdk_attempt_for_row(&row, &credentials.token)))
            },
            || ModelProviderError::Storage,
        )
    }
    pub async fn personai_tool_context(
        &self,
        expected_scope: &str,
    ) -> Result<(String, String, Arc<dyn PersonAiContextProvider>), ModelProviderError> {
        if self
            .sdk_model_authorization_scope()
            .map_err(|e| ModelProviderError::Remote(e.to_string()))?
            != expected_scope
        {
            return Err(ModelProviderError::AuthenticationRequired);
        }
        let credentials = self.active_credentials(ModelProviderCode::PersonAi).await?;
        let this = self.clone();
        let supplied = credentials.clone();
        let expected = expected_scope.to_owned();
        let identity = self.connection.call(
            move |connection| {
                let row = read_connection(connection, ModelProviderCode::PersonAi)?
                    .ok_or(ModelProviderError::AuthenticationRequired)?;
                let current = row
                    .credentials_encrypted
                    .as_deref()
                    .map(|v| this.decrypt_credentials(v))
                    .transpose()?
                    .ok_or(ModelProviderError::AuthenticationRequired)?;
                if this.sdk_identity_scope(&row) != expected
                    || current.token != supplied.token
                    || current.server_base_url != supplied.server_base_url
                {
                    return Err(ModelProviderError::AuthenticationRequired);
                }
                Ok(this.sdk_attempt_for_row(&row, &supplied.token))
            },
            || ModelProviderError::Storage,
        )?;
        if self
            .sdk_model_authorization_scope()
            .map_err(|e| ModelProviderError::Remote(e.to_string()))?
            != expected_scope
        {
            return Err(ModelProviderError::AuthenticationRequired);
        }
        let account = Arc::new(AccountContext {
            account_scope: expected_scope.into(),
            revision: 1,
            authorization: credentials.context(),
        });
        let (sender, _) = watch::channel(Some(account.clone()));
        let provider = ToolContext {
            providers: self.clone(),
            identity: identity.clone(),
            account,
            sender,
        };
        if provider.current().is_none() {
            return Err(ModelProviderError::AuthenticationRequired);
        }
        Ok((identity, credentials.server_base_url, Arc::new(provider)))
    }
}
