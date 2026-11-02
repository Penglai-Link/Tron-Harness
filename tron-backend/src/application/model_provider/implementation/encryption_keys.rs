use super::*;

pub(super) fn is_encryption_key_failure(error: &eliteforge_agent_core::ModelError) -> bool {
    matches!(error, eliteforge_agent_core::ModelError::Provider(detail)
        if is_encryption_key_failure_detail(detail))
}

pub(super) fn is_encryption_key_failure_detail(
    detail: &eliteforge_agent_core::ProviderModelError,
) -> bool {
    detail.category == eliteforge_agent_core::ProviderErrorCategory::Authentication
        && detail.http_status == Some(400)
        && matches!(
            detail.error_type.as_deref(),
            Some("decrypt_error" | "DecryptFailed")
        )
}

impl ModelProviderService {
    pub(super) async fn recover_encryption_key_for_match(
        &self,
        expected_attempt: &str,
    ) -> Result<Option<ProviderCredentials>, ModelProviderError> {
        let _guard = self.token_refresh_lock.lock().await;
        let row = self
            .connection
            .call(
                |connection| read_connection(connection, ModelProviderCode::PersonAi),
                || ModelProviderError::Storage,
            )?
            .ok_or(ModelProviderError::ConnectionNotReady)?;
        if expected_attempt.split_once(':').map(|parts| parts.0)
            != Some(self.sdk_identity_scope(&row).as_str())
            || !(matches!(
                row.status,
                ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
            ) || (row.status == ProviderConnectionStatus::AuthRequired
                && row.last_error.as_deref()
                    == Some("PersonAI rejected access credentials; reauthorization required")))
        {
            return Err(ModelProviderError::ConnectionNotReady);
        }
        let mut credentials = self.decrypt_credentials(
            row.credentials_encrypted
                .as_deref()
                .ok_or(ModelProviderError::AuthenticationRequired)?,
        )?;
        if self.sdk_attempt_for_row(&row, &credentials.token) != expected_attempt {
            return Ok(Some(credentials));
        }

        let server = row.server_base_url.clone();
        let sessions = self.connection.call(
            move |connection| {
                let mut query = connection
                    .prepare(
                        "SELECT session_id FROM provider_authorization_sessions
                 WHERE provider_code='PersonAi' AND server_base_url=?1 AND status='Completed'
                 ORDER BY created_at DESC LIMIT 32",
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                query
                    .query_map([server], |row| row.get::<_, String>(0))
                    .map_err(|_| ModelProviderError::Storage)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| ModelProviderError::Storage)
            },
            || ModelProviderError::Storage,
        )?;
        let adapter = self.adapter(ModelProviderCode::PersonAi)?;
        let mut updated = false;
        for session in sessions {
            if let ExternalAuthorizationPoll::Completed(remote) = adapter
                .poll_authorization(&credentials.server_base_url, &session)
                .await?
            {
                if remote.client_id == credentials.client_id {
                    if remote.public_key.trim().is_empty()
                        || remote.public_key == credentials.public_key
                    {
                        return Ok(None);
                    }
                    // The login session's tokens may be old. Only adopt the
                    // current public key for this exact authorized device.
                    credentials.public_key = remote.public_key;
                    updated = true;
                    break;
                }
            }
        }
        if !updated {
            return Ok(None);
        }

        let this = self.clone();
        self.connection.call(move |connection| {
            let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(
                connection, rusqlite::TransactionBehavior::Immediate,
            ).map_err(|_| ModelProviderError::Storage)?;
            let active = read_connection(&transaction, ModelProviderCode::PersonAi)?
                .ok_or(ModelProviderError::ConnectionNotReady)?;
            if active.person_ai_user_id != row.person_ai_user_id
                || active.server_base_url != row.server_base_url
                || active.credentials_encrypted != row.credentials_encrypted
                || active.status != row.status
            {
                return Err(ModelProviderError::ConnectionNotReady);
            }
            let encrypted = this.encrypt_credentials(&credentials)?;
            let now = now_millis();
            transaction.execute(
                "UPDATE provider_connections SET credentials_encrypted=?1,status='Connected',last_error=NULL,updated_at=?2 WHERE provider_code='PersonAi'",
                params![encrypted, now],
            ).map_err(|_| ModelProviderError::Storage)?;
            if let Some(user) = active.person_ai_user_id.as_deref() {
                transaction.execute(
                    "UPDATE person_ai_profile_credentials SET credentials_encrypted=?1,status='Connected',last_error=NULL,updated_at=?2 WHERE person_ai_user_id=?3",
                    params![encrypted, now, user],
                ).map_err(|_| ModelProviderError::Storage)?;
            }
            transaction.commit().map_err(|_| ModelProviderError::Storage)?;
            Ok(Some(credentials))
        }, || ModelProviderError::Storage)
    }
}
