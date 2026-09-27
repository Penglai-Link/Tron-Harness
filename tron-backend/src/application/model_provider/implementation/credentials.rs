use super::*;

impl ModelProviderService {
    pub(super) fn default_selection(
        &self,
        workspace_code: &str,
        model_type: ModelType,
    ) -> Result<Option<ModelSelection>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        effective_default_selection(&connection, workspace_code, model_type)
    }

    pub(super) fn person_ai_selection_from_legacy_code(
        &self,
        model_code: &str,
    ) -> Result<Option<ModelSelection>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(
            read_catalog_model(&connection, ModelProviderCode::PersonAi, model_code, None)?.map(
                |model| {
                    ModelSelection::person_ai_fixed(
                        model_code,
                        model
                            .quality_type
                            .expect("chat catalog models always have a quality"),
                    )
                },
            ),
        )
    }

    pub(super) fn require_active_connection(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<(), ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let row = read_connection(&connection, provider_code)?
            .ok_or(ModelProviderError::ConnectionNotReady)?;
        if matches!(
            row.status,
            ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
        ) {
            Ok(())
        } else if row.status == ProviderConnectionStatus::AuthRequired {
            Err(ModelProviderError::AuthenticationRequired)
        } else {
            Err(ModelProviderError::ConnectionNotReady)
        }
    }

    pub(super) async fn active_credentials(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        let row = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ModelProviderError::Storage)?;
            read_connection(&connection, provider_code)?
                .ok_or(ModelProviderError::ConnectionNotReady)?
        };
        if row.status == ProviderConnectionStatus::AuthRequired {
            return Err(ModelProviderError::AuthenticationRequired);
        }
        if !matches!(
            row.status,
            ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
        ) {
            return Err(ModelProviderError::ConnectionNotReady);
        }
        let encrypted = row
            .credentials_encrypted
            .as_deref()
            .ok_or(ModelProviderError::AuthenticationRequired)?;
        let credentials = match self.decrypt_credentials(encrypted) {
            Ok(credentials) => credentials,
            Err(_) => {
                self.mark_auth_required(provider_code, "credential decryption failed")?;
                return Err(ModelProviderError::AuthenticationRequired);
            }
        };
        if credentials
            .authorization_expires_at
            .is_some_and(|expires_at| {
                expires_at > now_millis().saturating_add(TOKEN_REFRESH_LEAD_MILLIS)
            })
        {
            return Ok(credentials);
        }
        self.refresh_credentials_for_request(provider_code, &credentials.token, false)
            .await
    }

    pub(super) async fn refresh_credentials_after_unauthorized(
        &self,
        provider_code: ModelProviderCode,
        rejected_token: &str,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        self.refresh_credentials_for_request(provider_code, rejected_token, true)
            .await
    }

    pub(super) async fn refresh_credentials_for_request(
        &self,
        provider_code: ModelProviderCode,
        expected_token: &str,
        force: bool,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        let _refresh_guard = self.token_refresh_lock.lock().await;
        let row = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ModelProviderError::Storage)?;
            read_connection(&connection, provider_code)?
                .ok_or(ModelProviderError::ConnectionNotReady)?
        };
        if !matches!(
            row.status,
            ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
        ) {
            return Err(if row.status == ProviderConnectionStatus::AuthRequired {
                ModelProviderError::AuthenticationRequired
            } else {
                ModelProviderError::ConnectionNotReady
            });
        }
        let encrypted = row
            .credentials_encrypted
            .as_deref()
            .ok_or(ModelProviderError::AuthenticationRequired)?;
        let mut credentials = self.decrypt_credentials(encrypted).map_err(|_| {
            let _ = self.mark_auth_required(provider_code, "credential decryption failed");
            ModelProviderError::AuthenticationRequired
        })?;

        // Another request may already have refreshed this token while we were
        // waiting for the lock. Reuse its persisted credentials rather than
        // rotating the same refresh token a second time.
        if credentials.token != expected_token {
            return Ok(credentials);
        }
        if !force
            && credentials
                .authorization_expires_at
                .is_some_and(|expires_at| {
                    expires_at > now_millis().saturating_add(TOKEN_REFRESH_LEAD_MILLIS)
                })
        {
            return Ok(credentials);
        }

        let refreshed = match self
            .adapter(provider_code)?
            .refresh_credentials(&credentials.server_base_url, &credentials.refresh_token)
            .await
        {
            Ok(refreshed) if !refreshed.token.trim().is_empty() => refreshed,
            Err(ModelProviderError::DeviceUnbound) => {
                self.clear_credentials_after_device_unbound()?;
                return Err(ModelProviderError::DeviceUnbound);
            }
            Ok(_) | Err(_) => {
                self.mark_auth_required(provider_code, "token refresh failed")?;
                return Err(ModelProviderError::AuthenticationRequired);
            }
        };
        credentials.token = refreshed.token;
        credentials.refresh_token = refreshed.refresh_token;
        credentials.authorization_expires_at = refreshed.authorization_expires_at;
        self.persist_refreshed_credentials(provider_code, &credentials)?;
        Ok(credentials)
    }

    pub(super) fn persist_refreshed_credentials(
        &self,
        provider_code: ModelProviderCode,
        credentials: &ProviderCredentials,
    ) -> Result<(), ModelProviderError> {
        let encrypted = self.encrypt_credentials(credentials)?;
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let active = read_connection(&connection, provider_code)?
            .ok_or(ModelProviderError::ConnectionNotReady)?;
        connection
            .execute(
                "UPDATE provider_connections SET credentials_encrypted = ?2, authorization_expires_at = ?3,
                 status = ?4, last_error = NULL, updated_at = ?5 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    encrypted,
                    credentials.authorization_expires_at,
                    ProviderConnectionStatus::Connected.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        if provider_code == ModelProviderCode::PersonAi {
            if let Some(person_ai_user_id) = active
                .person_ai_user_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                connection
                    .execute(
                        "UPDATE person_ai_profile_credentials
                         SET credentials_encrypted = ?2, authorization_expires_at = ?3,
                             status = ?4, last_error = NULL, updated_at = ?5
                         WHERE person_ai_user_id = ?1",
                        params![
                            person_ai_user_id,
                            encrypted,
                            credentials.authorization_expires_at,
                            ProviderConnectionStatus::Connected.storage_value(),
                            now,
                        ],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
            }
        }
        Ok(())
    }

    pub(super) fn mark_auth_required_after_rejected_credentials(&self, error: &ModelProviderError) {
        if matches!(error, ModelProviderError::DeviceUnbound) {
            if let Err(storage_error) = self.clear_credentials_after_device_unbound() {
                eprintln!("[person-ai] failed to clear unbound credentials: {storage_error}");
            }
            return;
        }
        if matches!(error, ModelProviderError::Unauthorized)
            || is_person_ai_authorization_key_error(error)
        {
            let reason = if matches!(error, ModelProviderError::Unauthorized) {
                "Person-AI rejected the refreshed access token; reauthorization required"
            } else {
                "Person-AI rejected the authorization key; reauthorization required"
            };
            let _ = self.mark_auth_required(ModelProviderCode::PersonAi, reason);
        }
    }

    pub(super) fn clear_credentials_after_device_unbound(&self) -> Result<(), ModelProviderError> {
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_connections SET status = ?2, credentials_encrypted = NULL,
                 authorization_expires_at = NULL, last_error = ?3, updated_at = ?4 WHERE provider_code = ?1",
                params![
                    ModelProviderCode::PersonAi.storage_value(),
                    ProviderConnectionStatus::AuthRequired.storage_value(),
                    "PersonAI device was unbound; login again",
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE person_ai_profile_credentials SET credentials_encrypted = '',
                 status = ?1, authorization_expires_at = NULL,
                 last_error = ?2, updated_at = ?3",
                params![
                    ProviderConnectionStatus::AuthRequired.storage_value(),
                    "PersonAI device was unbound; login again",
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "DELETE FROM provider_authorization_sessions WHERE provider_code = ?1",
                [ModelProviderCode::PersonAi.storage_value()],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_catalog_models SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
                params![
                    ModelProviderCode::PersonAi.storage_value(),
                    ProviderCatalogModelStatus::Unavailable.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_multimedia_catalog_models SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
                params![
                    ModelProviderCode::PersonAi.storage_value(),
                    ProviderCatalogModelStatus::Unavailable.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .commit()
            .map_err(|_| ModelProviderError::Storage)?;
        self.device_unbound.store(true, Ordering::SeqCst);
        let _ = self.server_auth_events.send(());
        Ok(())
    }

    pub(super) async fn finish_sync_failure(
        &self,
        provider_code: ModelProviderCode,
        sync_run_code: &str,
        error: &ModelProviderError,
    ) -> Result<(), ModelProviderError> {
        if matches!(error, ModelProviderError::DeviceUnbound) {
            self.clear_credentials_after_device_unbound()?;
        }
        let now = now_millis();
        let summary = safe_error_summary(error);
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let status = if matches!(error, ModelProviderError::AuthenticationRequired) {
            ProviderConnectionStatus::AuthRequired
        } else {
            ProviderConnectionStatus::Degraded
        };
        connection
            .execute(
                "UPDATE provider_connections SET status = ?2, last_error = ?3, updated_at = ?4 WHERE provider_code = ?1",
                params![provider_code.storage_value(), status.storage_value(), summary, now],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "UPDATE provider_sync_runs SET status = ?2, completed_at = ?3, error_summary = ?4 WHERE sync_run_code = ?1",
                params![sync_run_code, ProviderSyncStatus::Failed.storage_value(), now, summary],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    pub(super) fn mark_auth_required(
        &self,
        provider_code: ModelProviderCode,
        message: &str,
    ) -> Result<(), ModelProviderError> {
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "UPDATE provider_connections SET status = ?2, last_error = ?3, updated_at = ?4 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    ProviderConnectionStatus::AuthRequired.storage_value(),
                    message,
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        if provider_code == ModelProviderCode::PersonAi {
            if let Some(person_ai_user_id) = read_connection(&connection, provider_code)?
                .and_then(|row| row.person_ai_user_id)
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
            {
                connection
                    .execute(
                        "UPDATE person_ai_profile_credentials
                         SET status = ?2, last_error = ?3, updated_at = ?4
                         WHERE person_ai_user_id = ?1",
                        params![
                            person_ai_user_id,
                            ProviderConnectionStatus::AuthRequired.storage_value(),
                            message,
                            now,
                        ],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
            }
        }
        Ok(())
    }

    pub(super) fn adapter(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<Arc<dyn ModelProviderAdapter>, ModelProviderError> {
        self.adapters
            .get(&provider_code)
            .cloned()
            .ok_or(ModelProviderError::ConnectionNotReady)
    }

    pub(super) fn encrypt_credentials(
        &self,
        credentials: &ProviderCredentials,
    ) -> Result<String, ModelProviderError> {
        encrypt_credentials(self.cipher.as_ref(), credentials)
    }

    pub(super) fn decrypt_credentials(
        &self,
        encrypted: &str,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        decrypt_credentials(self.cipher.as_ref(), encrypted)
    }
}
