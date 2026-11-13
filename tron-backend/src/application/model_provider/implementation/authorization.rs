use super::*;

impl ModelProviderService {
    pub async fn create_authorization_session(
        &self,
        command: CreateProviderAuthorizationCommand,
    ) -> Result<ProviderAuthorizationSessionView, ModelProviderError> {
        let (adapter, existing_client_id, server_base_url) = {
            let this = self.clone();
            let worker_command = command.clone();
            this.connection.clone().call(
                move |connection| {
                    let command = worker_command;
                    let row = read_connection(&connection, command.provider_code)?;
                    let server_base_url = match command
                        .server_base_url
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        Some(value) => normalize_server_base_url(Some(value))?,
                        None => row
                            .as_ref()
                            .map(|row| row.server_base_url.clone())
                            .unwrap_or_else(|| PERSON_AI_DEFAULT_SERVER_BASE_URL.into()),
                    };
                    let existing_client_id = row
                        .as_ref()
                        .and_then(|row| row.credentials_encrypted.as_deref())
                        .and_then(|encrypted| this.decrypt_credentials(encrypted).ok())
                        .map(|credentials| credentials.client_id);
                    Ok((
                        this.adapter(command.provider_code)?,
                        existing_client_id,
                        server_base_url,
                    ))
                },
                || ModelProviderError::Storage,
            )?
        };
        let remote = match adapter
            .create_authorization(
                &server_base_url,
                PERSON_AI_CLIENT_NAME,
                existing_client_id.as_deref(),
                &self.device_id,
            )
            .await
        {
            Ok(remote) => remote,
            Err(ModelProviderError::DeviceUnbound) => {
                self.clear_credentials_after_device_unbound()?;
                return Err(ModelProviderError::DeviceUnbound);
            }
            Err(error) => return Err(error),
        };
        let now = now_millis();
        let view = ProviderAuthorizationSessionView {
            provider_code: command.provider_code,
            session_id: remote.session_id.clone(),
            login_url: remote.login_url.clone(),
            status: remote.status.clone(),
            expires_at: remote.expires_at,
            created_at: now,
            updated_at: now,
        };

        let this = self.clone();
        let worker_command = command.clone();
        let worker_server_base_url = server_base_url.clone();
        let worker_now = now.clone();
        this.connection.clone().call(move |connection| { let command=worker_command;  let server_base_url=worker_server_base_url; let now=worker_now;
        connection
            .execute(
                "INSERT INTO provider_connections (
                    provider_code, server_base_url, status, credentials_encrypted, authorized_at,
                    authorization_expires_at, last_sync_at, last_error, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, NULL, NULL, NULL, NULL, NULL, ?4, ?4)
                 ON CONFLICT(provider_code) DO UPDATE SET
                    server_base_url = CASE
                        WHEN provider_connections.credentials_encrypted IS NULL THEN excluded.server_base_url
                        ELSE provider_connections.server_base_url
                    END,
                    status = CASE
                        WHEN provider_connections.credentials_encrypted IS NULL THEN excluded.status
                        ELSE provider_connections.status
                    END,
                    updated_at = excluded.updated_at",
                params![
                    command.provider_code.storage_value(),
                    server_base_url,
                    ProviderConnectionStatus::PendingAuthorization.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "INSERT INTO provider_authorization_sessions (
                    session_id, provider_code, server_base_url, client_name, login_url, status,
                    expires_at, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
                 ON CONFLICT(session_id) DO UPDATE SET status = excluded.status, updated_at = excluded.updated_at",
                params![
                    view.session_id,
                    command.provider_code.storage_value(),
                    server_base_url,
                    PERSON_AI_CLIENT_NAME,
                    view.login_url,
                    view.status,
                    view.expires_at,
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(view)
     },||ModelProviderError::Storage)
    }

    pub async fn poll_authorization_session(
        &self,
        command: CompleteProviderAuthorizationCommand,
    ) -> Result<CompleteProviderAuthorizationResult, ModelProviderError> {
        // Authorizing an account must not wait indefinitely on upstream services.
        // All awaited work finishes before credentials are committed below.
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.complete_authorization_session(command),
        )
        .await
        .map_err(|_| ModelProviderError::Remote("Person-AI authorization check timed out".into()))?
    }

    async fn complete_authorization_session(
        &self,
        command: CompleteProviderAuthorizationCommand,
    ) -> Result<CompleteProviderAuthorizationResult, ModelProviderError> {
        let stored = {
            let this = self.clone();
            let worker_command = command.clone();
            this.connection.clone().call(
                move |connection| {
                    let command = worker_command;
                    Ok(read_authorization_session(
                        &connection,
                        &command.session_id,
                    )?)
                },
                || ModelProviderError::Storage,
            )?
        };
        let adapter = self.adapter(stored.provider_code)?;
        let authorization_poll = adapter
            .poll_authorization(&stored.server_base_url, &stored.session_id)
            .await;
        let authorization_poll = match authorization_poll {
            Ok(authorization_poll) => authorization_poll,
            Err(ModelProviderError::DeviceUnbound) => {
                self.clear_credentials_after_device_unbound()?;
                return Err(ModelProviderError::DeviceUnbound);
            }
            Err(error) => return Err(error),
        };
        match authorization_poll {
            ExternalAuthorizationPoll::Pending { status, expires_at } => {
                let now = now_millis();

                let this = self.clone();
                let worker_stored = stored.clone();
                let worker_now = now.clone();
                this.connection.clone().call(move |connection| { let stored=worker_stored;  let now=worker_now;
                connection
                    .execute(
                        "UPDATE provider_authorization_sessions SET status = ?2, expires_at = ?3, updated_at = ?4 WHERE session_id = ?1",
                        params![stored.session_id, status, expires_at, now],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                Err(ModelProviderError::AuthorizationPending)
             },||ModelProviderError::Storage)
            }
            ExternalAuthorizationPoll::Ended { status } => {
                let now = now_millis();

                let this = self.clone();
                let worker_stored = stored.clone();
                let worker_now = now.clone();
                this.connection.clone().call(move |connection| { let stored=worker_stored;  let now=worker_now;
                connection
                    .execute(
                        "UPDATE provider_authorization_sessions SET status = ?2, updated_at = ?3 WHERE session_id = ?1",
                        params![stored.session_id, status, now],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                Err(ModelProviderError::AuthorizationEnded(status))
             },||ModelProviderError::Storage)
            }
            ExternalAuthorizationPoll::Completed(credentials) => {
                let validated_identity = async {
                    let identity =
                        if let Some(expected) = command.expected_person_ai_user_id.as_deref() {
                            let identity = self.fetch_person_ai_identity(&credentials).await?;
                            if identity.0 != expected {
                                return Err(ModelProviderError::AuthorizationAccountMismatch);
                            }
                            Some(identity)
                        } else {
                            None
                        };
                    let (catalog, multimedia_catalog) = self
                        .fetch_catalog_snapshot(stored.provider_code, &credentials)
                        .await?;
                    // Identity validation proves authorization. Generating chat or
                    // embedding output here makes sign-in depend on model latency,
                    // availability and quota, even after browser consent succeeds.
                    let identity = match identity {
                        Some(identity) => identity,
                        None => self.fetch_person_ai_identity(&credentials).await?,
                    };
                    Ok::<_, ModelProviderError>((catalog, multimedia_catalog, identity))
                }
                .await;
                let (catalog, multimedia_catalog, (person_ai_user_id, person_ai_username)) =
                    match validated_identity {
                        Ok(validated_identity) => validated_identity,
                        Err(ModelProviderError::DeviceUnbound) => {
                            self.clear_credentials_after_device_unbound()?;
                            return Err(ModelProviderError::DeviceUnbound);
                        }
                        Err(error) => return Err(error),
                    };
                let encrypted = self.encrypt_credentials(&credentials)?;
                let now = now_millis();
                let run_code = sync_run_code(now);

                let this = self.clone();
                this.connection.clone().call(move |connection| {
                let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate)
                    .map_err(|_| ModelProviderError::Storage)?;
                if let Some(expected) = command.expected_person_ai_user_id.as_deref() {
                    let active = read_connection(&transaction, stored.provider_code)?;
                    if active.as_ref().and_then(|row| row.person_ai_user_id.as_deref()) != Some(expected) {
                        return Err(ModelProviderError::AuthorizationAccountMismatch);
                    }
                }
                replace_authorized_connection(
                    &transaction,
                    stored.provider_code,
                    &credentials,
                    &person_ai_user_id,
                    &person_ai_username,
                    &encrypted,
                    &stored.session_id,
                    &catalog,
                    &multimedia_catalog,
                    &run_code,
                    now,
                )?;
                transaction
                    .commit()
                    .map_err(|_| ModelProviderError::Storage)?;
                if stored.provider_code == ModelProviderCode::PersonAi {
                    this.clear_device_unbound_flag();
                }
                let session =
                    read_authorization_session(&connection, &stored.session_id)?.to_view();
                let row = read_connection(&connection, stored.provider_code)?;
                let count = catalog_model_count(&connection, stored.provider_code)?;
                Ok(CompleteProviderAuthorizationResult {
                    session,
                    connection: connection_view(row, count),
                })
             },||ModelProviderError::Storage)
            }
        }
    }

    pub(super) async fn fetch_person_ai_identity(
        &self,
        credentials: &ProviderCredentials,
    ) -> Result<(String, String), ModelProviderError> {
        let login = LoginClient::new(credentials.server_base_url.clone())
            .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
        let token_status = login
            .get_token_status(&credentials.token)
            .await
            .map_err(server_api_error)?;
        if token_status.is_device_unbound() {
            return Err(ModelProviderError::DeviceUnbound);
        }
        let person_ai_user_id = token_status
            .subject
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ModelProviderError::Remote(
                    "Person-AI token status did not contain a stable subject".into(),
                )
            })?;
        let account = AccountClient::new(credentials.server_base_url.clone())
            .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?
            .get_info(&credentials.context())
            .await
            .map_err(server_api_error)?;
        let person_ai_username = account
            .username
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ModelProviderError::Remote(
                    "Person-AI account info did not contain a username".into(),
                )
            })?;
        Ok((person_ai_user_id, person_ai_username))
    }
}
