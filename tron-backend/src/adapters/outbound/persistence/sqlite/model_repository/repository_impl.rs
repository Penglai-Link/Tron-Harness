use super::storage::*;
use super::*;

impl SqliteModelRepositorySql<'_> {
    pub(super) fn list_models(&mut self) -> Result<Vec<Model>, ModelError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, scope_kind, workspace_code, model_code, display_name, vendor, protocol, model_type,
                        model_capabilities, base_url, api_path, api_key_ref, model_name, enabled, description,
                        thinking_enabled, supported_reasoning_efforts, default_reasoning_effort,
                        structured_output_enabled, responses_stateful_enabled, temperature, max_tokens,
                        context_window_tokens, top_p, max_retries, last_verified_at, last_verify_status,
                        last_verify_message, source_kind, source_model_code, allow_inheritance, created_at,
                        updated_at
                 FROM models
                 ORDER BY scope_kind ASC, workspace_code ASC, display_name ASC, model_code ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                let model_type = model_type_from_storage(row.get::<_, String>(7)?.as_str())
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok(Model {
                    provider_id: None,
                    request_headers: Default::default(),
                    id: SnowflakeId::new(row.get::<_, i64>(0)?),
                    scope_kind: scope_kind_from_storage(row.get::<_, String>(1)?.as_str())
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    workspace_code: row.get(2)?,
                    model_code: row.get(3)?,
                    display_name: row.get(4)?,
                    vendor: row.get(5)?,
                    protocol: protocol_from_storage(row.get::<_, String>(6)?.as_str())
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    model_type,
                    model_capabilities: model_capabilities_from_storage(
                        row.get::<_, Option<String>>(8)?.as_deref(),
                        model_type,
                    )
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    base_url: row.get(9)?,
                    api_path: row.get(10)?,
                    api_key_ref: row.get(11)?,
                    model_name: row.get(12)?,
                    enabled: row.get::<_, i64>(13)? != 0,
                    description: row.get(14)?,
                    thinking_enabled: row.get::<_, i64>(15)? != 0,
                    supported_reasoning_efforts: supported_reasoning_efforts_from_storage(
                        row.get::<_, Option<String>>(16)?.as_deref(),
                    )
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    default_reasoning_effort: reasoning_effort_from_storage(
                        row.get::<_, Option<String>>(17)?.as_deref(),
                    )
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    structured_output_enabled: row.get::<_, i64>(18)? != 0,
                    responses_stateful_enabled: row.get::<_, i64>(19)? != 0,
                    temperature: row.get(20)?,
                    max_tokens: row.get::<_, Option<u32>>(21)?,
                    context_window_tokens: row.get::<_, Option<u32>>(22)?,
                    top_p: row.get(23)?,
                    max_retries: row.get::<_, Option<u32>>(24)?,
                    last_verified_at: option_timestamp_from_millis(row.get::<_, Option<i64>>(25)?),
                    last_verify_status: row
                        .get::<_, Option<String>>(26)?
                        .map(|value| verify_status_from_storage(value.as_str()))
                        .transpose()
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    last_verify_message: row.get(27)?,
                    source_kind: source_kind_from_storage(row.get::<_, String>(28)?.as_str())
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    source_model_code: row.get(29)?,
                    allow_inheritance: row.get::<_, Option<i64>>(30)?.map(|value| value != 0),
                    created_at: timestamp_from_millis(row.get(31)?),
                    updated_at: timestamp_from_millis(row.get(32)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    pub(super) fn next_id(&mut self) -> Result<i64, ModelError> {
        let max_id: i64 = self
            .connection
            .query_row("SELECT max(id) FROM models", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .map_err(storage_error)?
            .unwrap_or(MODEL_ID_SEED);
        Ok(max_id + 1)
    }

    pub(super) fn create_model(&mut self, model: &Model) -> Result<(), ModelError> {
        self.connection
            .execute(
                "INSERT INTO models (
                    id, scope_kind, workspace_code, model_code, display_name, vendor, protocol,
                    model_type, model_capabilities, base_url, api_path, api_key_ref, model_name, enabled,
                    description, thinking_enabled, supported_reasoning_efforts, default_reasoning_effort,
                    structured_output_enabled, responses_stateful_enabled, temperature, max_tokens,
                    context_window_tokens, top_p, max_retries, last_verified_at, last_verify_status,
                    last_verify_message, source_kind, source_model_code, allow_inheritance, created_at,
                    updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                    CASE WHEN EXISTS(SELECT 1 FROM configured_provider_models WHERE model_code=?4 OR model_code=?30) THEN '' ELSE ?10 END,
                    CASE WHEN EXISTS(SELECT 1 FROM configured_provider_models WHERE model_code=?4 OR model_code=?30) THEN NULL ELSE ?11 END,
                    CASE WHEN EXISTS(SELECT 1 FROM configured_provider_models WHERE model_code=?4 OR model_code=?30) THEN '' ELSE ?12 END,
                    ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31,
                    ?32, ?33
                )",
                params![
                    model.id.value(),
                    scope_kind_to_storage(model.scope_kind),
                    model.workspace_code,
                    model.model_code,
                    model.display_name,
                    model.vendor,
                    protocol_to_storage(model.protocol),
                    model_type_to_storage(model.model_type),
                    model_capabilities_to_storage(&model.model_capabilities)?,
                    model.base_url,
                    model.api_path,
                    model.api_key_ref,
                    model.model_name,
                    bool_to_int(model.enabled),
                    model.description,
                    bool_to_int(model.thinking_enabled),
                    supported_reasoning_efforts_to_storage(&model.supported_reasoning_efforts)?,
                    model.default_reasoning_effort.map(ReasoningEffort::storage_value),
                    bool_to_int(model.structured_output_enabled),
                    bool_to_int(model.responses_stateful_enabled),
                    model.temperature,
                    model.max_tokens,
                    model.context_window_tokens,
                    model.top_p,
                    model.max_retries,
                    option_timestamp_to_millis(model.last_verified_at.as_deref()),
                    model.last_verify_status.map(verify_status_to_storage),
                    model.last_verify_message,
                    source_kind_to_storage(model.source_kind),
                    model.source_model_code,
                    model.allow_inheritance.map(bool_to_int),
                    timestamp_to_millis(&model.created_at),
                    timestamp_to_millis(&model.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(super) fn update_model(&mut self, model: &Model) -> Result<(), ModelError> {
        let updated_rows = self
            .connection
            .execute(
                "UPDATE models
                 SET scope_kind = ?2,
                     workspace_code = ?3,
                     display_name = ?4,
                    vendor = ?5,
                    protocol = ?6,
                    model_type = ?7,
                     model_capabilities = ?8,
                     base_url = CASE WHEN EXISTS(SELECT 1 FROM configured_provider_models WHERE model_code=?1 OR model_code=?29) THEN '' ELSE ?9 END,
                    api_path = CASE WHEN EXISTS(SELECT 1 FROM configured_provider_models WHERE model_code=?1 OR model_code=?29) THEN NULL ELSE ?10 END,
                    api_key_ref = CASE WHEN EXISTS(SELECT 1 FROM configured_provider_models WHERE model_code=?1 OR model_code=?29) THEN '' ELSE ?11 END,
                    model_name = ?12,
                     enabled = ?13,
                     description = ?14,
                     thinking_enabled = ?15,
                     supported_reasoning_efforts = ?16,
                     default_reasoning_effort = ?17,
                     structured_output_enabled = ?18,
                     responses_stateful_enabled = ?19,
                     temperature = ?20,
                     max_tokens = ?21,
                     context_window_tokens = ?22,
                     top_p = ?23,
                     max_retries = ?24,
                     last_verified_at = ?25,
                     last_verify_status = ?26,
                     last_verify_message = ?27,
                     source_kind = ?28,
                     source_model_code = ?29,
                     allow_inheritance = ?30,
                     updated_at = ?31
                 WHERE model_code = ?1",
                params![
                    model.model_code,
                    scope_kind_to_storage(model.scope_kind),
                    model.workspace_code,
                    model.display_name,
                    model.vendor,
                    protocol_to_storage(model.protocol),
                    model_type_to_storage(model.model_type),
                    model_capabilities_to_storage(&model.model_capabilities)?,
                    model.base_url,
                    model.api_path,
                    model.api_key_ref,
                    model.model_name,
                    bool_to_int(model.enabled),
                    model.description,
                    bool_to_int(model.thinking_enabled),
                    supported_reasoning_efforts_to_storage(&model.supported_reasoning_efforts)?,
                    model
                        .default_reasoning_effort
                        .map(ReasoningEffort::storage_value),
                    bool_to_int(model.structured_output_enabled),
                    bool_to_int(model.responses_stateful_enabled),
                    model.temperature,
                    model.max_tokens,
                    model.context_window_tokens,
                    model.top_p,
                    model.max_retries,
                    option_timestamp_to_millis(model.last_verified_at.as_deref()),
                    model.last_verify_status.map(verify_status_to_storage),
                    model.last_verify_message,
                    source_kind_to_storage(model.source_kind),
                    model.source_model_code,
                    model.allow_inheritance.map(bool_to_int),
                    timestamp_to_millis(&model.updated_at),
                ],
            )
            .map_err(storage_error)?;
        if updated_rows == 0 {
            return Err(ModelError::NotFound);
        }
        Ok(())
    }

    pub(super) fn delete_model(&mut self, model_code: &str) -> Result<bool, ModelError> {
        let deleted_rows = self
            .connection
            .execute(
                "DELETE FROM models WHERE model_code = ?1",
                params![model_code],
            )
            .map_err(storage_error)?;
        Ok(deleted_rows > 0)
    }

    pub(super) fn list_workspace_default_assignments(
        &mut self,
    ) -> Result<Vec<WorkspaceDefaultModelAssignment>, ModelError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT workspace_code, model_type, model_code, updated_at
                 FROM workspace_default_models
                 ORDER BY workspace_code ASC, model_type ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(WorkspaceDefaultModelAssignment {
                    workspace_code: row.get(0)?,
                    model_type: model_type_from_storage(row.get::<_, String>(1)?.as_str())
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    model_code: row.get(2)?,
                    updated_at: timestamp_from_millis(row.get(3)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    pub(super) fn list_platform_default_assignments(
        &mut self,
    ) -> Result<Vec<PlatformDefaultModelAssignment>, ModelError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT model_type, model_code, updated_at
                 FROM platform_default_models
                 ORDER BY model_type ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(PlatformDefaultModelAssignment {
                    model_type: model_type_from_storage(row.get::<_, String>(0)?.as_str())
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    model_code: row.get(1)?,
                    updated_at: timestamp_from_millis(row.get(2)?),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    pub(super) fn upsert_workspace_default_assignment(
        &mut self,
        assignment: &WorkspaceDefaultModelAssignment,
    ) -> Result<(), ModelError> {
        self.connection
            .execute(
                "INSERT INTO workspace_default_models (workspace_code, model_type, model_code, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(workspace_code, model_type)
                 DO UPDATE SET model_code = excluded.model_code, updated_at = excluded.updated_at",
                params![
                    assignment.workspace_code,
                    model_type_to_storage(assignment.model_type),
                    assignment.model_code,
                    timestamp_to_millis(&assignment.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(super) fn upsert_platform_default_assignment(
        &mut self,
        assignment: &PlatformDefaultModelAssignment,
    ) -> Result<(), ModelError> {
        self.connection
            .execute(
                "INSERT INTO platform_default_models (model_type, model_code, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(model_type)
                 DO UPDATE SET model_code = excluded.model_code, updated_at = excluded.updated_at",
                params![
                    model_type_to_storage(assignment.model_type),
                    assignment.model_code,
                    timestamp_to_millis(&assignment.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(super) fn delete_workspace_default_assignments_by_model_code(
        &mut self,
        model_code: &str,
    ) -> Result<(), ModelError> {
        self.connection
            .execute(
                "DELETE FROM workspace_default_models WHERE model_code = ?1",
                params![model_code],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(super) fn delete_workspace_default_assignments_by_workspace_and_model_code(
        &mut self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<(), ModelError> {
        self.connection
            .execute(
                "DELETE FROM workspace_default_models
                 WHERE workspace_code = ?1 AND model_code = ?2",
                params![workspace_code, model_code],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(super) fn delete_platform_default_assignments_by_model_code(
        &mut self,
        model_code: &str,
    ) -> Result<(), ModelError> {
        self.connection
            .execute(
                "DELETE FROM platform_default_models WHERE model_code = ?1",
                params![model_code],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

impl ModelRepository for SqliteModelRepository {
    fn model_configuration_snapshot(
        &mut self,
    ) -> Result<
        (
            Vec<Model>,
            Vec<crate::domain::model::ConfiguredModelProvider>,
        ),
        ModelError,
    > {
        self.connection.call(
            |connection| {
                let transaction = connection.transaction().map_err(storage_error)?;
                let models = SqliteModelRepositorySql {
                    connection: &transaction,
                }
                .list_models()?;
                let providers = configured_providers::read_providers(&transaction)
                    .map_err(|_| ModelError::StorageFailure)?;
                transaction.commit().map_err(storage_error)?;
                Ok((models, providers))
            },
            || ModelError::StorageFailure,
        )
    }
    fn list_configured_providers(
        &mut self,
    ) -> Result<
        Vec<crate::domain::model::ConfiguredModelProvider>,
        crate::domain::model::ConfiguredProviderError,
    > {
        self.read_configured_providers()
    }
    fn save_configured_provider(
        &mut self,
        provider: &crate::domain::model::ConfiguredModelProvider,
        expected_updated_at: Option<i64>,
        models: &[Model],
    ) -> Result<(), crate::domain::model::ConfiguredProviderError> {
        self.write_configured_provider(provider, expected_updated_at, models)
    }
    fn delete_configured_provider(
        &mut self,
        provider_id: &str,
    ) -> Result<(), crate::domain::model::ConfiguredProviderError> {
        self.remove_configured_provider(provider_id)
    }

    fn list_models(&mut self) -> Result<Vec<Model>, ModelError> {
        self.connection.call(
            move |connection| SqliteModelRepositorySql { connection }.list_models(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn next_id(&mut self) -> Result<i64, ModelError> {
        self.connection.call(
            move |connection| SqliteModelRepositorySql { connection }.next_id(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn create_model(&mut self, model: &Model) -> Result<(), ModelError> {
        let model = model.to_owned();
        self.connection.call(
            move |connection| SqliteModelRepositorySql { connection }.create_model(&model),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_model(&mut self, model: &Model) -> Result<(), ModelError> {
        let model = model.to_owned();
        self.connection.call(
            move |connection| SqliteModelRepositorySql { connection }.update_model(&model),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_model(&mut self, model_code: &str) -> Result<bool, ModelError> {
        let model_code = model_code.to_owned();
        self.connection.call(
            move |connection| SqliteModelRepositorySql { connection }.delete_model(&model_code),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_workspace_default_assignments(
        &mut self,
    ) -> Result<Vec<WorkspaceDefaultModelAssignment>, ModelError> {
        self.connection.call(
            move |connection| {
                SqliteModelRepositorySql { connection }.list_workspace_default_assignments()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn list_platform_default_assignments(
        &mut self,
    ) -> Result<Vec<PlatformDefaultModelAssignment>, ModelError> {
        self.connection.call(
            move |connection| {
                SqliteModelRepositorySql { connection }.list_platform_default_assignments()
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_workspace_default_assignment(
        &mut self,
        assignment: &WorkspaceDefaultModelAssignment,
    ) -> Result<(), ModelError> {
        let assignment = assignment.to_owned();
        self.connection.call(
            move |connection| {
                SqliteModelRepositorySql { connection }
                    .upsert_workspace_default_assignment(&assignment)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn upsert_platform_default_assignment(
        &mut self,
        assignment: &PlatformDefaultModelAssignment,
    ) -> Result<(), ModelError> {
        let assignment = assignment.to_owned();
        self.connection.call(
            move |connection| {
                SqliteModelRepositorySql { connection }
                    .upsert_platform_default_assignment(&assignment)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_workspace_default_assignments_by_model_code(
        &mut self,
        model_code: &str,
    ) -> Result<(), ModelError> {
        let model_code = model_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteModelRepositorySql { connection }
                    .delete_workspace_default_assignments_by_model_code(&model_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_workspace_default_assignments_by_workspace_and_model_code(
        &mut self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<(), ModelError> {
        let workspace_code = workspace_code.to_owned();
        let model_code = model_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteModelRepositorySql { connection }
                    .delete_workspace_default_assignments_by_workspace_and_model_code(
                        &workspace_code,
                        &model_code,
                    )
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_platform_default_assignments_by_model_code(
        &mut self,
        model_code: &str,
    ) -> Result<(), ModelError> {
        let model_code = model_code.to_owned();
        self.connection.call(
            move |connection| {
                SqliteModelRepositorySql { connection }
                    .delete_platform_default_assignments_by_model_code(&model_code)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
