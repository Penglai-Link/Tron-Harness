fn ensure_database_parent(database_url: &str) -> Result<(), PromptError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| PromptError::StorageFailure)?;
    }
    Ok(())
}

fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, PromptError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|value| value == column_name))
}

fn migrate_prompt_variable_metadata(connection: &mut Connection) -> Result<(), PromptError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    if column_exists(&transaction, "prompt_variables", "description")? {
        return transaction.commit().map_err(storage_error);
    }
    transaction
        .execute_batch(PROMPT_VARIABLE_METADATA_MIGRATION)
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)
}

fn migrate_prompt_catalog(connection: &mut Connection) -> Result<(), PromptError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    if !column_exists(&transaction, "prompts", "built_in")? {
        transaction
            .execute_batch(
                "ALTER TABLE prompts ADD COLUMN built_in INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE prompts ADD COLUMN runtime_required INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE prompts ADD COLUMN catalog_key TEXT;
                 CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_catalog_key
                     ON prompts(catalog_key) WHERE catalog_key IS NOT NULL;",
            )
            .map_err(storage_error)?;
    }
    transaction
        .execute_batch(PROMPT_CATALOG_MIGRATION)
        .map_err(storage_error)?;
    transaction
        .execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_catalog_key
                 ON prompts(catalog_key) WHERE catalog_key IS NOT NULL;
             INSERT OR IGNORE INTO prompt_parameters (
                 prompt_code, parameter_name, parameter_type, description, position
             )
             SELECT prompt_code, variable_name, variable_type, description, position
             FROM prompt_variables;
             INSERT OR IGNORE INTO prompt_translations (
                 prompt_code, locale, display_name, prompt_body, description,
                 translation_status, source_version, updated_at
             )
             SELECT prompt_code, 'en-US', display_name, prompt_body, description,
                    'PUBLISHED', version, updated_at
             FROM prompts;
             INSERT OR IGNORE INTO prompt_translations (
                 prompt_code, locale, display_name, prompt_body, description,
                 translation_status, source_version, updated_at
             )
             SELECT prompt_code, 'zh-CN', display_name, prompt_body, description,
                    'DRAFT', version, updated_at
             FROM prompts;",
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)
}

fn migrate_prompt_category(connection: &mut Connection) -> Result<(), PromptError> {
    let schema = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'prompts'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    if schema
        .as_deref()
        .is_some_and(|sql| sql.contains("'ACTION'"))
    {
        return Ok(());
    }
    connection
        .execute_batch(PROMPT_ACTION_CATEGORY_MIGRATION)
        .map_err(storage_error)
}

fn migrate_prompt_variable_bindings(connection: &mut Connection) -> Result<(), PromptError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    if !column_exists(&transaction, "prompt_variables", "source_kind")? {
        transaction
            .execute_batch(PROMPT_VARIABLE_BINDINGS_MIGRATION)
            .map_err(storage_error)?;
    }
    repair_prompt_variable_bindings(&transaction)?;
    transaction.commit().map_err(storage_error)
}

fn repair_prompt_variable_bindings(transaction: &Transaction<'_>) -> Result<(), PromptError> {
    for definition in crate::domain::prompt::builtin_prompts() {
        for parameter in &definition.parameters {
            transaction
                .execute(
                    "UPDATE prompt_variables
                     SET source_kind = 'RUNTIME', managed_variable_key = NULL,
                         runtime_provider = ?3
                     WHERE prompt_code = ?1 AND variable_name = ?2",
                    rusqlite::params![
                        definition.prompt_code,
                        parameter.name,
                        parameter.runtime_provider.code(),
                    ],
                )
                .map_err(storage_error)?;
        }
    }

    if column_exists(transaction, "platform_variables", "is_sensitive")?
        && column_exists(transaction, "workspace_variables", "is_sensitive")?
    {
        transaction
            .execute(
                "UPDATE prompt_variables
                 SET source_kind = 'MANAGED', managed_variable_key = variable_name,
                     runtime_provider = NULL
                 WHERE source_kind = 'MANAGED'
                   AND managed_variable_key IS NULL
                   AND runtime_provider IS NULL
                   AND (
                       EXISTS (
                           SELECT 1 FROM platform_variables
                           WHERE variable_key = prompt_variables.variable_name
                             AND is_sensitive = 0
                             AND trim(variable_value) <> ''
                       )
                       OR EXISTS (
                           SELECT 1
                           FROM workspace_variables
                           JOIN prompts ON prompts.workspace_code = workspace_variables.workspace_code
                           WHERE prompts.prompt_code = prompt_variables.prompt_code
                             AND workspace_variables.variable_key = prompt_variables.variable_name
                             AND workspace_variables.is_sensitive = 0
                             AND trim(workspace_variables.variable_value) <> ''
                       )
                   )",
                [],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn storage_error(_: impl std::fmt::Debug) -> PromptError {
    PromptError::StorageFailure
}
