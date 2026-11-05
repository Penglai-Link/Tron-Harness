impl ResourceRepository for SqliteResourceRepository {
    fn list_files(&mut self) -> Result<Vec<ResourceFileRecord>, ResourceError> {
        self.connection.call(
            move |connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT workspace_code, resource_code, kind, name, description,
                        file_name, file_extension, byte_size, storage_locator, allow_inheritance,
                        revision, created_at, updated_at
                 FROM resource_files
                 ORDER BY workspace_code, resource_code",
                    )
                    .map_err(storage_error)?;
                statement
                    .query_map([], file_from_row)
                    .map_err(storage_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage_error)
            },
            || ResourceError::StorageFailure,
        )
    }

    fn insert_file(&mut self, record: &ResourceFileRecord) -> Result<(), ResourceError> {
        let record = record.to_owned();
        self.connection.call(move |connection| {
            let record = &record;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &record.workspace_code,
            &record.resource_code,
            record.kind.as_str(),
        )?;
        transaction
            .execute(
                "INSERT INTO resource_files (
                    workspace_code, resource_code, kind, name, description,
                    file_name, file_extension, byte_size, storage_locator, allow_inheritance,
                    revision, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.kind.as_str(),
                    record.name,
                    record.description,
                    record.file_name,
                    record.file_extension,
                    record.byte_size as i64,
                    record.storage_locator,
                    bool_to_int(record.allow_inheritance),
                    record.revision as i64,
                    timestamp_to_millis(&record.created_at),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)

        }, || ResourceError::StorageFailure)
    }

    fn update_file(&mut self, record: &ResourceFileRecord) -> Result<bool, ResourceError> {
        let record = record.to_owned();
        self.connection.call(
            move |connection| {
                let record = &record;
                let changed = connection
                    .execute(
                        "UPDATE resource_files
                 SET kind = ?3, name = ?4, description = ?5, file_name = ?6,
                     file_extension = ?7, byte_size = ?8, storage_locator = ?9,
                     allow_inheritance = ?10, updated_at = ?11, revision = revision + 1
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?12",
                        params![
                            record.workspace_code,
                            record.resource_code,
                            record.kind.as_str(),
                            record.name,
                            record.description,
                            record.file_name,
                            record.file_extension,
                            record.byte_size as i64,
                            record.storage_locator,
                            bool_to_int(record.allow_inheritance),
                            timestamp_to_millis(&record.updated_at),
                            record.revision as i64,
                        ],
                    )
                    .map_err(storage_error)?;
                Ok(changed > 0)
            },
            || ResourceError::StorageFailure,
        )
    }

    fn delete_file(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError> {
        let workspace_code = workspace_code.to_owned();
        let resource_code = resource_code.to_owned();
        self.connection.call(move |connection| {
            let workspace_code = &workspace_code;
            let resource_code = &resource_code;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM resource_files
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?3",
                params![workspace_code, resource_code, revision as i64],
            )
            .map_err(storage_error)?;
        if deleted > 0 {
            release_resource_code(&transaction, workspace_code, resource_code)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)

        }, || ResourceError::StorageFailure)
    }

    fn list_data_source_connections(
        &mut self,
    ) -> Result<Vec<DataSourceConnectionRecord>, ResourceError> {
        self.connection.call(
            move |connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT workspace_code, resource_code, kind, name, description,
                        attributes_json, allow_inheritance, revision, created_at, updated_at
                 FROM resource_data_source_connections
                 ORDER BY workspace_code, resource_code",
                    )
                    .map_err(storage_error)?;
                statement
                    .query_map([], data_source_from_row)
                    .map_err(storage_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage_error)
            },
            || ResourceError::StorageFailure,
        )
    }

    fn insert_data_source_connection(
        &mut self,
        record: &DataSourceConnectionRecord,
    ) -> Result<(), ResourceError> {
        let record = record.to_owned();
        self.connection.call(move |connection| {
            let record = &record;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &record.workspace_code,
            &record.resource_code,
            record.kind.as_str(),
        )?;
        transaction
            .execute(
                "INSERT INTO resource_data_source_connections (
                    workspace_code, resource_code, kind, name, description,
                    attributes_json, allow_inheritance, revision, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    record.workspace_code,
                    record.resource_code,
                    record.kind.as_str(),
                    record.name,
                    record.description,
                    value_to_storage(&record.attributes)?,
                    bool_to_int(record.allow_inheritance),
                    record.revision as i64,
                    timestamp_to_millis(&record.created_at),
                    timestamp_to_millis(&record.updated_at),
                ],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)

        }, || ResourceError::StorageFailure)
    }

    fn update_data_source_connection(
        &mut self,
        record: &DataSourceConnectionRecord,
    ) -> Result<bool, ResourceError> {
        let record = record.to_owned();
        self.connection.call(
            move |connection| {
                let record = &record;
                let changed = connection
                    .execute(
                        "UPDATE resource_data_source_connections
                 SET name = ?3, description = ?4, attributes_json = ?5,
                     allow_inheritance = ?6, updated_at = ?7, revision = revision + 1
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?8",
                        params![
                            record.workspace_code,
                            record.resource_code,
                            record.name,
                            record.description,
                            value_to_storage(&record.attributes)?,
                            bool_to_int(record.allow_inheritance),
                            timestamp_to_millis(&record.updated_at),
                            record.revision as i64,
                        ],
                    )
                    .map_err(storage_error)?;
                Ok(changed > 0)
            },
            || ResourceError::StorageFailure,
        )
    }

    fn delete_data_source_connection(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError> {
        let workspace_code = workspace_code.to_owned();
        let resource_code = resource_code.to_owned();
        self.connection.call(move |connection| {
            let workspace_code = &workspace_code;
            let resource_code = &resource_code;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM resource_data_source_connections
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?3",
                params![workspace_code, resource_code, revision as i64],
            )
            .map_err(storage_error)?;
        if deleted > 0 {
            release_resource_code(&transaction, workspace_code, resource_code)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)

        }, || ResourceError::StorageFailure)
    }

    fn list_servers(&mut self) -> Result<Vec<ServerRecord>, ResourceError> {
        self.connection.call(
            move |connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT workspace_code, resource_code, name, description, host, port,
                        allow_inheritance, revision, created_at, updated_at
                 FROM resource_servers
                 ORDER BY workspace_code, resource_code",
                    )
                    .map_err(storage_error)?;
                statement
                    .query_map([], server_from_row)
                    .map_err(storage_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage_error)
            },
            || ResourceError::StorageFailure,
        )
    }

    fn insert_server(&mut self, record: &ServerRecord) -> Result<(), ResourceError> {
        let record = record.to_owned();
        self.connection.call(move |connection| {
            let record = &record;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &record.workspace_code,
            &record.resource_code,
            ResourceKind::ServerConnection.as_str(),
        )?;
        insert_server_record(&transaction, record)?;
        transaction.commit().map_err(storage_error)

        }, || ResourceError::StorageFailure)
    }

    fn insert_server_bundle(
        &mut self,
        server: &ServerRecord,
        initial_account: &ServerAccountRecord,
    ) -> Result<(), ResourceError> {
        let server = server.to_owned();
        let initial_account = initial_account.to_owned();
        self.connection.call(move |connection| {
            let server = &server;
            let initial_account = &initial_account;
        if server.workspace_code != initial_account.workspace_code
            || server.resource_code == initial_account.resource_code
            || initial_account.servers != vec![ServerAccountServer {
                workspace_code: server.workspace_code.clone(),
                server_code: server.resource_code.clone(),
            }]
        {
            return Err(ResourceError::InvalidInput);
        }
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &server.workspace_code,
            &server.resource_code,
            ResourceKind::ServerConnection.as_str(),
        )?;
        claim_resource_code(
            &transaction,
            &initial_account.workspace_code,
            &initial_account.resource_code,
            ResourceKind::ServerAccount.as_str(),
        )?;
        insert_server_record(&transaction, server)?;
        insert_server_account_record(&transaction, initial_account)?;
        transaction.commit().map_err(storage_error)

        }, || ResourceError::StorageFailure)
    }

    fn update_server(&mut self, record: &ServerRecord) -> Result<bool, ResourceError> {
        let record = record.to_owned();
        self.connection.call(
            move |connection| {
                let record = &record;
                let changed = connection
                    .execute(
                        "UPDATE resource_servers
                 SET name = ?3, description = ?4, host = ?5, port = ?6,
                     allow_inheritance = ?7, updated_at = ?8, revision = revision + 1
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?9",
                        params![
                            record.workspace_code,
                            record.resource_code,
                            record.name,
                            record.description,
                            record.host,
                            record.port as i64,
                            bool_to_int(record.allow_inheritance),
                            timestamp_to_millis(&record.updated_at),
                            record.revision as i64,
                        ],
                    )
                    .map_err(storage_error)?;
                Ok(changed > 0)
            },
            || ResourceError::StorageFailure,
        )
    }

    fn delete_server_if_no_accounts(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError> {
        let workspace_code = workspace_code.to_owned();
        let resource_code = resource_code.to_owned();
        self.connection.call(move |connection| {
            let workspace_code = &workspace_code;
            let resource_code = &resource_code;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        let has_accounts = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM resource_server_account_bindings
                    WHERE server_workspace_code = ?1 AND server_code = ?2
                )",
                params![workspace_code, resource_code],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)?
            != 0;
        if has_accounts {
            return Err(ResourceError::ServerHasAccounts);
        }
        let deleted = transaction
            .execute(
                "DELETE FROM resource_servers
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?3",
                params![workspace_code, resource_code, revision as i64],
            )
            .map_err(storage_error)?;
        if deleted > 0 {
            release_resource_code(&transaction, workspace_code, resource_code)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)

        }, || ResourceError::StorageFailure)
    }

    fn list_server_accounts(&mut self) -> Result<Vec<ServerAccountRecord>, ResourceError> {
        self.connection.call(
            move |connection| {
                let mut statement = connection.prepare(
                    "SELECT workspace_code, resource_code, name, description, username,
                     authentication_type, encrypted_credentials, allow_inheritance,
                     revision, created_at, updated_at FROM resource_server_accounts
                     ORDER BY workspace_code, resource_code",
                ).map_err(storage_error)?;
                let mut accounts = statement.query_map([], server_account_from_row)
                    .map_err(storage_error)?.collect::<Result<Vec<_>, _>>().map_err(storage_error)?;
                let mut statement = connection.prepare(
                    "SELECT account_workspace_code, account_code, server_workspace_code, server_code
                     FROM resource_server_account_bindings ORDER BY server_workspace_code, server_code",
                ).map_err(storage_error)?;
                let bindings = statement.query_map([], |row| Ok((
                    (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    ServerAccountServer { workspace_code: row.get(2)?, server_code: row.get(3)? },
                ))).map_err(storage_error)?;
                let mut by_account = std::collections::BTreeMap::<_, Vec<_>>::new();
                for binding in bindings {
                    let (key, server) = binding.map_err(storage_error)?;
                    by_account.entry(key).or_default().push(server);
                }
                for account in &mut accounts {
                    account.servers = by_account.remove(&(account.workspace_code.clone(), account.resource_code.clone())).unwrap_or_default();
                }
                Ok(accounts)
            },
            || ResourceError::StorageFailure,
        )
    }

    fn insert_server_account(&mut self, record: &ServerAccountRecord) -> Result<(), ResourceError> {
        let record = record.to_owned();
        self.connection.call(move |connection| {
            let record = &record;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        claim_resource_code(
            &transaction,
            &record.workspace_code,
            &record.resource_code,
            ResourceKind::ServerAccount.as_str(),
        )?;
        insert_server_account_record(&transaction, record)?;
        transaction.commit().map_err(storage_error)

        }, || ResourceError::StorageFailure)
    }

    fn update_server_account(&mut self, record: &ServerAccountRecord) -> Result<bool, ResourceError> {
        let record = record.to_owned();
        self.connection.call(move |connection| {
            let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
            let changed = transaction.execute(
                "UPDATE resource_server_accounts SET name = ?3, description = ?4, username = ?5,
                 authentication_type = ?6, encrypted_credentials = ?7, allow_inheritance = ?8,
                 updated_at = ?9, revision = revision + 1
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?10",
                params![record.workspace_code, record.resource_code, record.name, record.description,
                    record.username, authentication_type_to_storage(record.authentication_type),
                    record.encrypted_credentials, bool_to_int(record.allow_inheritance),
                    timestamp_to_millis(&record.updated_at), record.revision as i64],
            ).map_err(storage_error)?;
            if changed == 0 { return Ok(false); }
            replace_server_account_bindings(&transaction, &record)?;
            transaction.commit().map_err(storage_error)?;
            Ok(true)
        }, || ResourceError::StorageFailure)
    }

    fn delete_server_account(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError> {
        let workspace_code = workspace_code.to_owned();
        let resource_code = resource_code.to_owned();
        self.connection.call(move |connection| {
            let workspace_code = &workspace_code;
            let resource_code = &resource_code;
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM resource_server_accounts
                 WHERE workspace_code = ?1 AND resource_code = ?2 AND revision = ?3",
                params![workspace_code, resource_code, revision as i64],
            )
            .map_err(storage_error)?;
        if deleted > 0 {
            release_resource_code(&transaction, workspace_code, resource_code)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(deleted > 0)

        }, || ResourceError::StorageFailure)
    }

    fn insert_chat_session_attachment(
        &mut self,
        record: &ChatSessionAttachmentRecord,
    ) -> Result<(), ResourceError> {
        let record = record.to_owned();
        self.connection.call(
            move |connection| {
                let record = &record;
                connection
                    .execute(
                        "INSERT INTO resource_chat_session_attachments (
                    workspace_code, session_code, attachment_code, file_name,
                    content_type, byte_size, storage_locator, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            record.workspace_code,
                            record.session_code,
                            record.attachment_code,
                            record.file_name,
                            record.content_type,
                            record.byte_size as i64,
                            record.storage_locator,
                            timestamp_to_millis(&record.created_at),
                        ],
                    )
                    .map_err(storage_error)?;
                Ok(())
            },
            || ResourceError::StorageFailure,
        )
    }

    fn get_chat_session_attachment(
        &mut self,
        workspace_code: &str,
        session_code: &str,
        attachment_code: &str,
    ) -> Result<ChatSessionAttachmentRecord, ResourceError> {
        let workspace_code = workspace_code.to_owned();
        let session_code = session_code.to_owned();
        let attachment_code = attachment_code.to_owned();
        self.connection.call(
            move |connection| {
                let workspace_code = &workspace_code;
                let session_code = &session_code;
                let attachment_code = &attachment_code;
                connection
                    .query_row(
                        "SELECT workspace_code, session_code, attachment_code, file_name,
                        content_type, byte_size, storage_locator, created_at
                 FROM resource_chat_session_attachments
                 WHERE workspace_code = ?1 AND session_code = ?2 AND attachment_code = ?3",
                        params![workspace_code, session_code, attachment_code],
                        chat_session_attachment_from_row,
                    )
                    .optional()
                    .map_err(storage_error)?
                    .ok_or(ResourceError::NotFound)
            },
            || ResourceError::StorageFailure,
        )
    }

    fn list_chat_session_attachments(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<ChatSessionAttachmentRecord>, ResourceError> {
        let session_code = session_code.to_owned();
        self.connection.call(
            move |connection| {
                let session_code = &session_code;
                let mut statement = connection
                    .prepare(
                        "SELECT workspace_code, session_code, attachment_code, file_name,
                        content_type, byte_size, storage_locator, created_at
                 FROM resource_chat_session_attachments
                 WHERE session_code = ?1
                 ORDER BY created_at, id",
                    )
                    .map_err(storage_error)?;
                statement
                    .query_map([session_code], chat_session_attachment_from_row)
                    .map_err(storage_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage_error)
            },
            || ResourceError::StorageFailure,
        )
    }
}
