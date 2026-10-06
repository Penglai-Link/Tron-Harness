use super::*;

impl ModelProviderService {
    pub fn connect(
        database_url: &str,
        cipher: Arc<dyn VariableCipher>,
    ) -> Result<Self, ModelProviderError> {
        let connection = open_connection(database_url)?;
        connection
            .execute_batch(MODEL_PROVIDER_MIGRATION)
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute_batch(PERSON_AI_PROFILE_CREDENTIALS_MIGRATION)
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute_batch(include_str!(
                "../../../../migrations/20260917_0001_create_chat_auto_model_routes.sql"
            ))
            .map_err(|_| ModelProviderError::Storage)?;
        migrate_provider_catalog_model_schema(&connection)
            .map_err(|_| ModelProviderError::Storage)?;
        remove_legacy_person_ai_embedding_catalog_models(&connection)?;
        trim_provider_sync_runs(&connection, ModelProviderCode::PersonAi)?;
        migrate_legacy_default_model_selections(&connection)?;
        migrate_legacy_person_ai_profile_credentials(&connection)?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS person_ai_device_identity (
                    singleton_id INTEGER PRIMARY KEY CHECK(singleton_id = 1),
                    device_id TEXT NOT NULL
                )",
            )
            .map_err(|_| ModelProviderError::Storage)?;
        let saved_device_id = connection
            .query_row(
                "SELECT device_id FROM person_ai_device_identity WHERE singleton_id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| ModelProviderError::Storage)?;
        let device_id = match saved_device_id {
            Some(device_id) => device_id,
            None => {
                let device_id = crab_serverapi::detect_device_id();
                connection
                    .execute(
                    "INSERT OR IGNORE INTO person_ai_device_identity(singleton_id, device_id) VALUES (1, ?1)",
                    [&device_id],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                device_id
            }
        };
        let device_unbound = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM provider_connections
                    WHERE provider_code = ?1 AND last_error = ?2
                )",
                params![
                    ModelProviderCode::PersonAi.storage_value(),
                    "PersonAI device was unbound; login again",
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| ModelProviderError::Storage)?;
        if sqlite_table_exists(&connection, "default_model_selections")? {
            connection
                .execute(
                    "DELETE FROM default_model_selections WHERE provider_code = 'PersonAi' AND route_code = 'ChatAuto'",
                    [],
                )
                .map_err(|_| ModelProviderError::Storage)?;
        }
        let adapter: Arc<dyn ModelProviderAdapter> = Arc::new(PersonAiAdapter);
        let mut adapters = HashMap::new();
        adapters.insert(adapter.code(), adapter);
        Ok(Self {
            model_executor: Default::default(),
            model_attempt_salt: Arc::new(eliteforge_agent_core::RunId::new_v4().to_string()),
            connection: Arc::new(Mutex::new(connection)),
            token_refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
            server_auth_events: broadcast::channel(16).0,
            device_unbound: Arc::new(AtomicBool::new(device_unbound)),
            device_id,
            cipher,
            adapters: Arc::new(adapters),
        })
    }

    pub(super) async fn fetch_catalog_snapshot(
        &self,
        provider_code: ModelProviderCode,
        credentials: &ProviderCredentials,
    ) -> Result<
        (
            Vec<ExternalCatalogModel>,
            Vec<ExternalMultimediaCatalogModel>,
        ),
        ModelProviderError,
    > {
        let adapter = self.adapter(provider_code)?;
        let chat_models = adapter.list_chat_models(credentials).await?;
        let multimedia_models = adapter.list_multimedia_models(credentials).await?;
        Ok((chat_models, multimedia_models))
    }

    pub fn list_connections(&self) -> Result<Vec<ProviderConnectionView>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let existing = read_connection(&connection, ModelProviderCode::PersonAi)?;
        Ok(vec![connection_view(
            existing,
            catalog_model_count(&connection, ModelProviderCode::PersonAi)?,
        )])
    }

    /// Fetch the current Person-AI account information, including the latest
    /// signed avatar URL. The SDK deliberately refreshes the credentials before
    /// this call when necessary; callers must not persist the returned avatar URL.
    pub async fn get_person_ai_account_info(&self) -> Result<AccountInfo, ModelProviderError> {
        let credentials = self.active_credentials(ModelProviderCode::PersonAi).await?;
        let result = AccountClient::new(credentials.server_base_url.clone())
            .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?
            .get_info(&credentials.context())
            .await;
        match result {
            Ok(info) => Ok(info),
            Err(error) => {
                let mapped = server_api_error(error);
                self.mark_auth_required_after_rejected_credentials(&mapped);
                Err(mapped)
            }
        }
    }

    pub fn subscribe_server_auth_events(&self) -> broadcast::Receiver<()> {
        self.server_auth_events.subscribe()
    }

    pub fn device_unbound(&self) -> bool {
        self.device_unbound.load(Ordering::SeqCst)
    }

    pub async fn heartbeat_active_profile(&self) -> Result<(), ModelProviderError> {
        let credentials = self.active_credentials(ModelProviderCode::PersonAi).await?;
        let account = AccountClient::new(credentials.server_base_url.clone())
            .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
        match account.heartbeat(&credentials.context()).await {
            Ok(()) => Ok(()),
            Err(ServerApiError::DeviceUnbound) => {
                self.clear_credentials_after_device_unbound()?;
                Err(ModelProviderError::DeviceUnbound)
            }
            Err(error) => Err(server_api_error(error)),
        }
    }

    pub fn start_person_ai_heartbeat(&self) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let service = self.clone();
        handle.spawn(async move {
            if let Err(error) = service.heartbeat_active_profile().await
                && !matches!(
                    error,
                    ModelProviderError::AuthenticationRequired
                        | ModelProviderError::ConnectionNotReady
                )
            {
                eprintln!("[person-ai] device heartbeat failed: {error}");
            }
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5 * 60));
            interval.tick().await;
            loop {
                interval.tick().await;
                if let Err(error) = service.heartbeat_active_profile().await
                    && !matches!(
                        error,
                        ModelProviderError::AuthenticationRequired
                            | ModelProviderError::ConnectionNotReady
                    )
                {
                    eprintln!("[person-ai] device heartbeat failed: {error}");
                }
            }
        });
    }

    pub fn clear_device_unbound_flag(&self) {
        self.device_unbound.store(false, Ordering::SeqCst);
    }

    /// Returns the refreshed token belonging to the currently active profile.
    /// The caller is the local desktop frontend; credentials remain encrypted at rest.
    pub async fn get_person_ai_access_token(&self) -> Result<String, ModelProviderError> {
        Ok(self
            .active_credentials(ModelProviderCode::PersonAi)
            .await?
            .token)
    }

    /// Makes a selected local profile the active Person-AI runtime identity.
    ///
    /// The shared provider row remains the runtime entry point for model calls,
    /// while encrypted credentials are owned by the Person-AI user id. A profile
    /// without stored credentials must never inherit the previously active
    /// account's token.
    pub fn activate_person_ai_profile(
        &self,
        person_ai_user_id: &str,
        person_ai_username: &str,
    ) -> Result<(), ModelProviderError> {
        let person_ai_user_id = person_ai_user_id.trim();
        let person_ai_username = person_ai_username.trim();
        if person_ai_user_id.is_empty() || person_ai_username.is_empty() {
            return Err(ModelProviderError::AuthenticationRequired);
        }

        let now = now_millis();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let transaction = connection
            .transaction()
            .map_err(|_| ModelProviderError::Storage)?;

        if let Some(credentials) =
            read_person_ai_profile_credentials(&transaction, person_ai_user_id)?
        {
            transaction
                .execute(
                    "INSERT INTO provider_connections (
                        provider_code, person_ai_user_id, person_ai_username, server_base_url,
                        status, credentials_encrypted, authorized_at, authorization_expires_at,
                        last_sync_at, last_error, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?7, ?7)
                     ON CONFLICT(provider_code) DO UPDATE SET
                        person_ai_user_id = excluded.person_ai_user_id,
                        person_ai_username = excluded.person_ai_username,
                        server_base_url = excluded.server_base_url,
                        status = excluded.status,
                        credentials_encrypted = excluded.credentials_encrypted,
                        authorized_at = excluded.authorized_at,
                        authorization_expires_at = excluded.authorization_expires_at,
                        last_error = excluded.last_error,
                        updated_at = excluded.updated_at",
                    params![
                        ModelProviderCode::PersonAi.storage_value(),
                        person_ai_user_id,
                        person_ai_username,
                        credentials.server_base_url,
                        credentials.status.storage_value(),
                        credentials.credentials_encrypted,
                        credentials.authorized_at,
                        credentials.authorization_expires_at,
                        credentials.last_error,
                    ],
                )
                .map_err(|_| ModelProviderError::Storage)?;
        } else {
            transaction
                .execute(
                    "INSERT INTO provider_connections (
                        provider_code, person_ai_user_id, person_ai_username, server_base_url,
                        status, credentials_encrypted, authorized_at, authorization_expires_at,
                        last_sync_at, last_error, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, NULL, NULL, ?6, ?7, ?7)
                     ON CONFLICT(provider_code) DO UPDATE SET
                        person_ai_user_id = excluded.person_ai_user_id,
                        person_ai_username = excluded.person_ai_username,
                        server_base_url = excluded.server_base_url,
                        status = excluded.status,
                        credentials_encrypted = NULL,
                        authorized_at = NULL,
                        authorization_expires_at = NULL,
                        last_error = excluded.last_error,
                        updated_at = excluded.updated_at",
                    params![
                        ModelProviderCode::PersonAi.storage_value(),
                        person_ai_user_id,
                        person_ai_username,
                        PERSON_AI_DEFAULT_SERVER_BASE_URL,
                        ProviderConnectionStatus::AuthRequired.storage_value(),
                        "credentials are not available for the active Person-AI profile",
                        now,
                    ],
                )
                .map_err(|_| ModelProviderError::Storage)?;
        }
        transaction
            .commit()
            .map_err(|_| ModelProviderError::Storage)
    }

    pub fn set_person_ai_identity(
        &self,
        person_ai_user_id: &str,
        person_ai_username: &str,
    ) -> Result<(), ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "UPDATE provider_connections
                 SET person_ai_user_id = ?1, person_ai_username = ?2, updated_at = ?3
                 WHERE provider_code = 'PersonAi'",
                params![
                    person_ai_user_id.trim(),
                    person_ai_username.trim(),
                    now_millis()
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    pub fn get_connection(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderConnectionView, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let row = read_connection(&connection, provider_code)?;
        Ok(connection_view(
            row,
            catalog_model_count(&connection, provider_code)?,
        ))
    }

    pub async fn get_account_balance(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderAccountBalanceView, ModelProviderError> {
        let credentials = self.active_credentials(provider_code).await?;
        match provider_code {
            ModelProviderCode::PersonAi => {
                let client = AuthorizedClientApi::new(credentials.server_base_url.clone())
                    .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
                let context = credentials.context();
                let request = client
                    .new_gateway_request(Method::GET, "/account/info", &context)
                    .map_err(server_api_error)?;
                let response = client
                    .execute_for_raw_json(request, Some(&context), "获取账户信息")
                    .await
                    .map_err(|error| {
                        eprintln!(
                            "[person-ai] account profile request failed: {}",
                            account_info_error_kind(&error)
                        );
                        let mapped = account_info_error(error);
                        self.mark_auth_required_after_rejected_credentials(&mapped);
                        mapped
                    })?;
                parse_person_ai_account_info(response)
            }
        }
    }

    pub fn list_catalog_models(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<Vec<ProviderCatalogModelView>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        read_catalog_models(&connection, provider_code)
    }

    pub fn list_sync_runs(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<Vec<ProviderSyncRunView>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        read_sync_runs(&connection, provider_code)
    }

    pub fn get_run_snapshot(
        &self,
        run_code: &str,
    ) -> Result<Option<ProviderModelRunSnapshotView>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        read_run_snapshot(&connection, run_code)
    }

    pub fn get_authorization_session(
        &self,
        session_id: &str,
    ) -> Result<ProviderAuthorizationSessionView, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(read_authorization_session(&connection, session_id)?.to_view())
    }
}
