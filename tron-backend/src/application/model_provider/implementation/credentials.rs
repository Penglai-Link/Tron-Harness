use super::*;

impl ModelProviderService {
    pub(super) fn default_selection(
        &self,
        workspace_code: &str,
        model_type: ModelType,
    ) -> Result<Option<ModelSelection>, ModelProviderError> {
        let this = self.clone();
        let worker_workspace_code = workspace_code.to_owned();
        let worker_model_type = model_type.clone();
        this.connection.clone().call(
            move |connection| {
                let workspace_code = &worker_workspace_code;
                let model_type = worker_model_type;
                effective_default_selection(&connection, workspace_code, model_type)
            },
            || ModelProviderError::Storage,
        )
    }

    pub(super) fn person_ai_selection_from_legacy_code(
        &self,
        model_code: &str,
    ) -> Result<Option<ModelSelection>, ModelProviderError> {
        let this = self.clone();
        let worker_model_code = model_code.to_owned();
        this.connection.clone().call(
            move |connection| {
                let model_code = &worker_model_code;
                Ok(
                    read_catalog_model(&connection, ModelProviderCode::PersonAi, model_code, None)?
                        .map(|model| {
                            ModelSelection::person_ai_fixed(
                                model_code,
                                model
                                    .quality_type
                                    .expect("chat catalog models always have a quality"),
                            )
                        }),
                )
            },
            || ModelProviderError::Storage,
        )
    }

    pub(super) fn require_active_connection(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<(), ModelProviderError> {
        let this = self.clone();
        let worker_provider_code = provider_code.clone();
        this.connection.clone().call(
            move |connection| {
                let provider_code = worker_provider_code;
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
            },
            || ModelProviderError::Storage,
        )
    }

    pub(super) async fn active_credentials(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        let row = {
            let this = self.clone();
            let worker_provider_code = provider_code.clone();
            this.connection.clone().call(
                move |connection| {
                    let provider_code = worker_provider_code;
                    Ok({
                        read_connection(&connection, provider_code)?
                            .ok_or(ModelProviderError::ConnectionNotReady)?
                    })
                },
                || ModelProviderError::Storage,
            )?
        };
        let retry_rejected_access = row.status == ProviderConnectionStatus::AuthRequired
            && row.last_error.as_deref()
                == Some("PersonAI rejected access credentials; reauthorization required");
        if row.status == ProviderConnectionStatus::AuthRequired && !retry_rejected_access {
            return Err(ModelProviderError::AuthenticationRequired);
        }
        if !retry_rejected_access
            && !matches!(
                row.status,
                ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
            )
        {
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
        // Recover credentials marked by older versions as a rejected account
        // before attempting an unrelated access-token refresh.
        if retry_rejected_access {
            if let Some(recovered) = self.recover_encryption_key_for_match(
                &self.sdk_attempt_for_row(&row, &credentials.token),
            ).await? {
                return Ok(recovered);
            }
        }
        if !retry_rejected_access
            && credentials
                .authorization_expires_at
                .is_some_and(|expires_at| {
                    expires_at > now_millis().saturating_add(TOKEN_REFRESH_LEAD_MILLIS)
                })
        {
            return Ok(credentials);
        }
        self.refresh_credentials_for_match(
            provider_code,
            &self.sdk_attempt_for_row(&row, &credentials.token),
            retry_rejected_access,
            true,
        )
        .await
    }




    pub(super) async fn refresh_credentials_for_match(
        &self,
        provider_code: ModelProviderCode,
        expected_token: &str,
        force: bool,
        fingerprint: bool,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        let _refresh_guard = self.token_refresh_lock.lock().await;
        let row = {
            let this = self.clone();
            let worker_provider_code = provider_code.clone();
            this.connection.clone().call(
                move |connection| {
                    let provider_code = worker_provider_code;
                    Ok({
                        read_connection(&connection, provider_code)?
                            .ok_or(ModelProviderError::ConnectionNotReady)?
                    })
                },
                || ModelProviderError::Storage,
            )?
        };
        let retry_rejected_access = row.status == ProviderConnectionStatus::AuthRequired
            && row.last_error.as_deref()
                == Some("PersonAI rejected access credentials; reauthorization required");
        if !retry_rejected_access
            && !matches!(
                row.status,
                ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
            )
        {
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
        let credentials = self.decrypt_credentials(encrypted).map_err(|_| {
            let _ = self.mark_auth_required(provider_code, "credential decryption failed");
            ModelProviderError::AuthenticationRequired
        })?;

        if fingerprint
            && expected_token.split_once(':').map(|parts| parts.0)
                != Some(self.sdk_identity_scope(&row).as_str())
        {
            return Err(ModelProviderError::ConnectionNotReady);
        }
        // Another request may already have refreshed this token while we were
        // waiting for the lock. Reuse its persisted credentials rather than
        // rotating the same refresh token a second time.
        if (if fingerprint {
            self.sdk_attempt_for_row(&row, &credentials.token)
        } else {
            credentials.token.clone()
        }) != expected_token
        {
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

        let refreshed = self
            .adapter(provider_code)?
            .refresh_credentials(&credentials.server_base_url, &credentials.refresh_token)
            .await;
        // Activation does not take the async refresh lock. Check the exact
        // identity and original encrypted credentials while holding the SQLite
        // write transaction, before applying either success or failure.

        let this = self.clone();
        let worker_provider_code = provider_code.clone();
        let worker_row = row.clone();
        let worker_credentials = credentials.clone();
        this.connection.clone().call(move |connection| { let provider_code=worker_provider_code; let row=worker_row;  let mut credentials=worker_credentials;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| ModelProviderError::Storage)?;
        let active = read_connection(&transaction, provider_code)?
            .ok_or(ModelProviderError::ConnectionNotReady)?;
        if active.person_ai_user_id != row.person_ai_user_id
            || active.server_base_url != row.server_base_url
            || active.credentials_encrypted != row.credentials_encrypted
            || active.status != row.status
        {
            return Err(ModelProviderError::ConnectionNotReady);
        }
        let now = now_millis();
        let refreshed = match refreshed {
            Ok(refreshed) if !refreshed.token.trim().is_empty() => refreshed,
            Err(error) if !matches!(error, ModelProviderError::Unauthorized | ModelProviderError::AuthenticationRequired | ModelProviderError::DeviceUnbound)
                && !is_person_ai_authorization_key_error(&error) => {
                // A timeout, upstream outage or local failure does not revoke
                // the refresh credential. Leave it available for the next attempt.
                return Err(error);
            }
            result => {
                let unbound = matches!(result, Err(ModelProviderError::DeviceUnbound));
                let reason = if unbound {
                    "PersonAI device was unbound; login again"
                } else {
                    "token refresh failed"
                };
                transaction.execute(
                    "UPDATE provider_connections SET status = ?2, last_error = ?3, updated_at = ?4,
                     credentials_encrypted = CASE WHEN ?5 THEN NULL ELSE credentials_encrypted END,
                     authorization_expires_at = CASE WHEN ?5 THEN NULL ELSE authorization_expires_at END
                     WHERE provider_code = ?1",
                    params![provider_code.storage_value(), ProviderConnectionStatus::AuthRequired.storage_value(), reason, now, unbound],
                ).map_err(|_| ModelProviderError::Storage)?;
                if let Some(user_id) = active.person_ai_user_id.as_deref() {
                    transaction.execute(
                        "UPDATE person_ai_profile_credentials SET status = ?2, last_error = ?3, updated_at = ?4,
                         credentials_encrypted = CASE WHEN ?5 THEN '' ELSE credentials_encrypted END,
                         authorization_expires_at = CASE WHEN ?5 THEN NULL ELSE authorization_expires_at END
                         WHERE person_ai_user_id = ?1",
                        params![user_id, ProviderConnectionStatus::AuthRequired.storage_value(), reason, now, unbound],
                    ).map_err(|_| ModelProviderError::Storage)?;
                }
                if unbound {
                    this.invalidate_unbound_catalog(&transaction, now)?;
                }
                transaction
                    .commit()
                    .map_err(|_| ModelProviderError::Storage)?;
                if unbound {
                    this.device_unbound.store(true, Ordering::SeqCst);
                    let _ = this.server_auth_events.send(());
                }
                return Err(if unbound {
                    ModelProviderError::DeviceUnbound
                } else {
                    ModelProviderError::AuthenticationRequired
                });
            }
        };
        credentials.token = refreshed.token;
        credentials.refresh_token = refreshed.refresh_token;
        credentials.authorization_expires_at = refreshed.authorization_expires_at;
        let encrypted = this.encrypt_credentials(&credentials)?;
        transaction.execute(
            "UPDATE provider_connections SET credentials_encrypted = ?2, authorization_expires_at = ?3,
             status = ?4, last_error = NULL, updated_at = ?5 WHERE provider_code = ?1",
            params![provider_code.storage_value(), encrypted, credentials.authorization_expires_at, ProviderConnectionStatus::Connected.storage_value(), now],
        ).map_err(|_| ModelProviderError::Storage)?;
        if let Some(user_id) = active.person_ai_user_id.as_deref() {
            transaction.execute(
                "UPDATE person_ai_profile_credentials SET credentials_encrypted = ?2, authorization_expires_at = ?3,
                 status = ?4, last_error = NULL, updated_at = ?5 WHERE person_ai_user_id = ?1",
                params![user_id, encrypted, credentials.authorization_expires_at, ProviderConnectionStatus::Connected.storage_value(), now],
            ).map_err(|_| ModelProviderError::Storage)?;
        }
        transaction
            .commit()
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(credentials)
     },||ModelProviderError::Storage)
    }

    pub(super) fn invalidate_unbound_catalog(
        &self,
        transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
        now: i64,
    ) -> Result<(), ModelProviderError> {
        transaction
            .execute(
                "DELETE FROM provider_authorization_sessions WHERE provider_code=?1",
                [ModelProviderCode::PersonAi.storage_value()],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        for table in [
            "provider_catalog_models",
            "provider_multimedia_catalog_models",
        ] {
            transaction
                .execute(
                    &format!("UPDATE {table} SET status=?2,updated_at=?3 WHERE provider_code=?1"),
                    params![
                        ModelProviderCode::PersonAi.storage_value(),
                        ProviderCatalogModelStatus::Unavailable.storage_value(),
                        now
                    ],
                )
                .map_err(|_| ModelProviderError::Storage)?;
        }
        Ok(())
    }

    pub(super) fn mark_auth_required_after_rejected_credentials(&self, error: &ModelProviderError) {
        if matches!(error, ModelProviderError::DeviceUnbound) {
            if let Err(storage_error) = self.clear_credentials_after_device_unbound() {
                tracing::error!("[person-ai] failed to clear unbound credentials: {storage_error}");
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

        let this = self.clone();
        let worker_now = now.clone();
        this.connection.clone().call(move |connection| { let now=worker_now;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate)
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
        this.device_unbound.store(true, Ordering::SeqCst);
        let _ = this.server_auth_events.send(());
        Ok(())
     },||ModelProviderError::Storage)
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

        let this = self.clone();
        let worker_provider_code = provider_code.clone();
        let worker_sync_run_code = sync_run_code.to_owned();
        let worker_error = error.to_owned();
        let worker_now = now.clone();
        this.connection.clone().call(move |connection| { let provider_code=worker_provider_code; let sync_run_code=&worker_sync_run_code; let error=&worker_error; let now=worker_now;
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
     },||ModelProviderError::Storage)
    }

    pub(super) fn mark_auth_required(
        &self,
        provider_code: ModelProviderCode,
        message: &str,
    ) -> Result<(), ModelProviderError> {
        let now = now_millis();

        let this = self.clone();
        let worker_provider_code = provider_code.clone();
        let worker_message = message.to_owned();
        this.connection.clone().call(move |connection| { let provider_code=worker_provider_code; let message=&worker_message;
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
     },||ModelProviderError::Storage)
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
