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
