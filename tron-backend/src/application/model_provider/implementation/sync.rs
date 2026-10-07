use super::*;

impl ModelProviderService {
    pub async fn sync_now(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderSyncRunView, ModelProviderError> {
        let now = now_millis();
        let sync_run_code = sync_run_code(now);
        {
            let this = self.clone();
            let worker_provider_code = provider_code.clone();
            let worker_now = now.clone();
            let worker_sync_run_code = sync_run_code.clone();
            this.connection.clone().call(move |connection| { let provider_code=worker_provider_code; let now=worker_now; let sync_run_code=worker_sync_run_code;
            let row = read_connection(&connection, provider_code)?
                .ok_or(ModelProviderError::ConnectionNotReady)?;
            if matches!(
                row.status,
                ProviderConnectionStatus::Disconnected
                    | ProviderConnectionStatus::PendingAuthorization
            ) {
                return Err(ModelProviderError::ConnectionNotReady);
            }
            connection
                .execute(
                    "INSERT INTO provider_sync_runs (
                        sync_run_code, provider_code, status, started_at, completed_at, model_count, error_summary
                     ) VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL)",
                    params![
                        sync_run_code,
                        provider_code.storage_value(),
                        ProviderSyncStatus::Running.storage_value(),
                        now,
                    ],
                )
                .map_err(|_| ModelProviderError::Storage)?;
            trim_provider_sync_runs(&connection, provider_code)?;

Ok(()) },||ModelProviderError::Storage)?
        }
        let credentials = match self.active_credentials(provider_code).await {
            Ok(credentials) => credentials,
            Err(error) => {
                self.finish_sync_failure(provider_code, &sync_run_code, &error)
                    .await?;
                return Err(error);
            }
        };
        let result = self
            .fetch_catalog_snapshot(provider_code, &credentials)
            .await;
        match result {
            Ok((catalog, multimedia_catalog)) => {
                let completed_at = now_millis();

                let this = self.clone();
                let worker_provider_code = provider_code.clone();
                let worker_now = now.clone();
                let worker_sync_run_code = sync_run_code.clone();
                this.connection.clone().call(move |connection| { let provider_code=worker_provider_code; let now=worker_now; let sync_run_code=worker_sync_run_code;
                let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate)
                    .map_err(|_| ModelProviderError::Storage)?;
                apply_catalog_snapshot(&transaction, provider_code, &catalog, completed_at)?;
                apply_multimedia_catalog_snapshot(
                    &transaction,
                    provider_code,
                    &multimedia_catalog,
                    completed_at,
                )?;
                transaction
                    .execute(
                        "UPDATE provider_connections SET status = ?2, last_sync_at = ?3, last_error = NULL, updated_at = ?3 WHERE provider_code = ?1",
                        params![
                            provider_code.storage_value(),
                            ProviderConnectionStatus::Connected.storage_value(),
                            completed_at,
                        ],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                transaction
                    .execute(
                        "UPDATE provider_sync_runs SET status = ?2, completed_at = ?3, model_count = ?4 WHERE sync_run_code = ?1",
                        params![
                            sync_run_code,
                            ProviderSyncStatus::Succeeded.storage_value(),
                            completed_at,
                            i64::try_from(catalog.len().saturating_add(multimedia_catalog.len()))
                                .unwrap_or(i64::MAX),
                        ],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                transaction
                    .commit()
                    .map_err(|_| ModelProviderError::Storage)?;
                Ok(ProviderSyncRunView {
                    sync_run_code,
                    provider_code,
                    status: ProviderSyncStatus::Succeeded,
                    started_at: now,
                    completed_at: Some(completed_at),
                    model_count: Some(catalog.len().saturating_add(multimedia_catalog.len())),
                    error_summary: None,
                })
             },||ModelProviderError::Storage)
            }
            Err(error) => {
                self.finish_sync_failure(provider_code, &sync_run_code, &error)
                    .await?;
                Err(error)
            }
        }
    }

    pub async fn refresh_due_connections(&self) {
        let should_refresh = self
            .get_connection(ModelProviderCode::PersonAi)
            .ok()
            .is_some_and(|connection| {
                matches!(
                    connection.status,
                    ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
                ) && connection
                    .authorization_expires_at
                    .is_some_and(|expires_at| {
                        expires_at <= now_millis().saturating_add(TOKEN_REFRESH_LEAD_MILLIS)
                    })
            });
        if should_refresh {
            let _ = self.active_credentials(ModelProviderCode::PersonAi).await;
        }
    }

    pub async fn sync_all_due_connections(&self) {
        self.refresh_due_connections().await;
        let connection = self.get_connection(ModelProviderCode::PersonAi);
        if connection.is_ok_and(|view| {
            matches!(
                view.status,
                ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
            )
        }) {
            let _ = self.sync_now(ModelProviderCode::PersonAi).await;
        }
    }

    pub fn start_background_sync(&self) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let service = self.clone();
        handle.spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60));
            interval.tick().await;
            loop {
                interval.tick().await;
                service.sync_all_due_connections().await;
            }
        });
    }

    pub fn disconnect(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderConnectionView, ModelProviderError> {
        let now = now_millis();

        let this = self.clone();
        let worker_provider_code = provider_code.clone();
        this.connection.clone().call(move |connection| { let provider_code=worker_provider_code;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_connections SET status = ?2, credentials_encrypted = NULL,
                 authorization_expires_at = NULL, last_error = NULL, updated_at = ?3 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    ProviderConnectionStatus::Disconnected.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_catalog_models SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    ProviderCatalogModelStatus::Unavailable.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_multimedia_catalog_models
                 SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    ProviderCatalogModelStatus::Unavailable.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "DELETE FROM default_model_selections WHERE provider_code = ?1",
                params![provider_code.storage_value()],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .commit()
            .map_err(|_| ModelProviderError::Storage)?;
        let row = read_connection(&connection, provider_code)?;
        Ok(connection_view(
            row,
            catalog_model_count(&connection, provider_code)?,
        ))
     },||ModelProviderError::Storage)
    }
}
