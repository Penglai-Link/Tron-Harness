use std::{fs, path::Path, time::Duration};

use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};

use crate::{
    application::prompt::PromptRepository,
    domain::prompt::{
        Prompt, PromptCategory, PromptError, PromptScopeKind, PromptSourceKind, PromptTranslation,
        PromptRuntimeProvider, PromptTranslationStatus, PromptVariable, PromptVariableSourceKind,
        PromptVariableType,
    },
    shared::{
        id::SnowflakeId,
        time::{timestamp_from_millis, timestamp_to_millis},
    },
};

use super::migrations::{
    PROMPT_ACTION_CATEGORY_MIGRATION, PROMPT_CATALOG_MIGRATION, PROMPT_VARIABLE_BINDINGS_MIGRATION,
    PROMPT_VARIABLE_METADATA_MIGRATION, PROMPTS_MIGRATION,
};

pub struct SqlitePromptRepository {
    connection: Connection,
}

impl SqlitePromptRepository {
    pub fn connect(database_url: &str) -> Result<Self, PromptError> {
        ensure_database_parent(database_url)?;
        let mut connection = if database_url == ":memory:" || database_url.starts_with("file:") {
            Connection::open_with_flags(
                database_url,
                OpenFlags::SQLITE_OPEN_READ_WRITE
                    | OpenFlags::SQLITE_OPEN_CREATE
                    | OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open(database_url)
        }
        .map_err(storage_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(storage_error)?;
        connection
            .execute_batch(PROMPTS_MIGRATION)
            .map_err(storage_error)?;
        migrate_prompt_variable_metadata(&mut connection)?;
        migrate_prompt_catalog(&mut connection)?;
        migrate_prompt_category(&mut connection)?;
        migrate_prompt_variable_bindings(&mut connection)?;
        Ok(Self { connection })
    }
}

impl PromptRepository for SqlitePromptRepository {
    fn list_prompts(&mut self) -> Result<Vec<Prompt>, PromptError> {
        let mut prompts =
            {
                let mut statement = self.connection.prepare(
                "SELECT id, scope_kind, workspace_code, prompt_code, display_name, category,
                        prompt_body, description, enabled, source_kind, source_prompt_code,
                        allow_inheritance, version,
                        created_at, updated_at, built_in, runtime_required, catalog_key
                 FROM prompts
                 ORDER BY scope_kind, workspace_code, display_name, prompt_code",
            ).map_err(storage_error)?;
                statement
                    .query_map([], decode_prompt)
                    .map_err(storage_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage_error)?
            };
        for prompt in &mut prompts {
            prompt.variables = load_variables(&self.connection, &prompt.prompt_code)?;
            prompt.translations = load_translations(&self.connection, &prompt.prompt_code)?;
        }
        Ok(prompts)
    }

    fn create_prompt(&mut self, prompt: &Prompt) -> Result<(), PromptError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        insert_prompt(&transaction, prompt)?;
        replace_variables(&transaction, prompt)?;
        replace_translations(&transaction, prompt)?;
        transaction.commit().map_err(storage_error)
    }

    fn update_prompt(&mut self, prompt: &Prompt, expected_version: u64) -> Result<(), PromptError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        let updated = transaction
            .execute(
                "UPDATE prompts
             SET display_name = ?2, prompt_body = ?3, description = ?4, enabled = ?5,
                 source_kind = ?6, source_prompt_code = ?7, allow_inheritance = ?8,
                 version = ?9, updated_at = ?10
             WHERE prompt_code = ?1 AND version = ?11",
                params![
                    prompt.prompt_code,
                    prompt.display_name,
                    prompt.prompt_body,
                    prompt.description,
                    bool_to_int(prompt.enabled),
                    source_kind_to_storage(prompt.source_kind),
                    prompt.source_prompt_code,
                    prompt.allow_inheritance.map(bool_to_int),
                    prompt.version,
                    timestamp_to_millis(&prompt.updated_at),
                    expected_version,
                ],
            )
            .map_err(storage_error)?;
        if updated == 0 {
            return Err(PromptError::VersionConflict);
        }
        replace_variables(&transaction, prompt)?;
        replace_translations(&transaction, prompt)?;
        transaction.commit().map_err(storage_error)
    }

    fn delete_prompt(&mut self, prompt_code: &str) -> Result<bool, PromptError> {
        self.connection
            .execute(
                "DELETE FROM prompts WHERE prompt_code = ?1",
                params![prompt_code],
            )
            .map(|count| count > 0)
            .map_err(storage_error)
    }
}

fn insert_prompt(transaction: &Transaction<'_>, prompt: &Prompt) -> Result<(), PromptError> {
    transaction
        .execute(
            "INSERT INTO prompts (
            id, scope_kind, workspace_code, prompt_code, display_name, category, prompt_body,
            description, enabled, source_kind,
            source_prompt_code, allow_inheritance, version, created_at, updated_at,
            built_in, runtime_required, catalog_key
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                prompt.id.value(),
                scope_kind_to_storage(prompt.scope_kind),
                prompt.workspace_code,
                prompt.prompt_code,
                prompt.display_name,
                category_to_storage(prompt.category),
                prompt.prompt_body,
                prompt.description,
                bool_to_int(prompt.enabled),
                source_kind_to_storage(prompt.source_kind),
                prompt.source_prompt_code,
                prompt.allow_inheritance.map(bool_to_int),
                prompt.version,
                timestamp_to_millis(&prompt.created_at),
                timestamp_to_millis(&prompt.updated_at),
                bool_to_int(prompt.built_in),
                bool_to_int(prompt.runtime_required),
                prompt.catalog_key,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn replace_variables(transaction: &Transaction<'_>, prompt: &Prompt) -> Result<(), PromptError> {
    transaction
        .execute(
            "DELETE FROM prompt_variables WHERE prompt_code = ?1",
            params![prompt.prompt_code],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "DELETE FROM prompt_parameters WHERE prompt_code = ?1",
            params![prompt.prompt_code],
        )
        .map_err(storage_error)?;
    for (position, variable) in prompt.variables.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO prompt_variables (
                    prompt_code, variable_name, variable_type, description, source_kind,
                    managed_variable_key, runtime_provider, position
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    prompt.prompt_code,
                    variable.name,
                    variable.variable_type.code(),
                    variable.description,
                    variable.source_kind.code(),
                    variable.managed_variable_key,
                    variable.runtime_provider.map(PromptRuntimeProvider::code),
                    position
                ],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "INSERT INTO prompt_parameters (
                    prompt_code, parameter_name, parameter_type, description, position
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    prompt.prompt_code,
                    variable.name,
                    variable.variable_type.code(),
                    variable.description,
                    position
                ],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn replace_translations(transaction: &Transaction<'_>, prompt: &Prompt) -> Result<(), PromptError> {
    transaction
        .execute(
            "DELETE FROM prompt_translations WHERE prompt_code = ?1",
            params![prompt.prompt_code],
        )
        .map_err(storage_error)?;
    for translation in &prompt.translations {
        transaction
            .execute(
                "INSERT INTO prompt_translations (
                    prompt_code, locale, display_name, prompt_body, description,
                    translation_status, source_version, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    prompt.prompt_code,
                    translation.locale,
                    translation.display_name,
                    translation.prompt_body,
                    translation.description,
                    translation_status_to_storage(translation.status),
                    translation.source_version,
                    timestamp_to_millis(&translation.updated_at),
                ],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn load_variables(
    connection: &Connection,
    prompt_code: &str,
) -> Result<Vec<PromptVariable>, PromptError> {
    let mut statement = connection
        .prepare(
            "SELECT variable_name, variable_type, description, source_kind,
                    managed_variable_key, runtime_provider FROM prompt_variables
         WHERE prompt_code = ?1 ORDER BY position",
        )
        .map_err(storage_error)?;
    statement
        .query_map(params![prompt_code], |row| {
            Ok(PromptVariable {
                name: row.get(0)?,
                variable_type: variable_type_from_storage(&row.get::<_, String>(1)?)?,
                description: row.get(2)?,
                source_kind: variable_source_kind_from_storage(&row.get::<_, String>(3)?)?,
                managed_variable_key: row.get(4)?,
                runtime_provider: row
                    .get::<_, Option<String>>(5)?
                    .map(|value| runtime_provider_from_storage(&value))
                    .transpose()?,
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

fn load_translations(
    connection: &Connection,
    prompt_code: &str,
) -> Result<Vec<PromptTranslation>, PromptError> {
    let mut statement = connection
        .prepare(
            "SELECT locale, display_name, prompt_body, description,
                    translation_status, source_version, updated_at
             FROM prompt_translations WHERE prompt_code = ?1 ORDER BY locale",
        )
        .map_err(storage_error)?;
    statement
        .query_map(params![prompt_code], |row| {
            Ok(PromptTranslation {
                locale: row.get(0)?,
                display_name: row.get(1)?,
                prompt_body: row.get(2)?,
                description: row.get(3)?,
                status: translation_status_from_storage(&row.get::<_, String>(4)?)?,
                source_version: row.get(5)?,
                updated_at: timestamp_from_millis(row.get(6)?),
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

fn decode_prompt(row: &rusqlite::Row<'_>) -> rusqlite::Result<Prompt> {
    Ok(Prompt {
        id: SnowflakeId::new(row.get(0)?),
        scope_kind: scope_kind_from_storage(&row.get::<_, String>(1)?)?,
        workspace_code: row.get(2)?,
        prompt_code: row.get(3)?,
        display_name: row.get(4)?,
        category: category_from_storage(&row.get::<_, String>(5)?)?,
        prompt_body: row.get(6)?,
        description: row.get(7)?,
        enabled: row.get::<_, i64>(8)? != 0,
        source_kind: source_kind_from_storage(&row.get::<_, String>(9)?)?,
        source_prompt_code: row.get(10)?,
        allow_inheritance: row.get::<_, Option<i64>>(11)?.map(|value| value != 0),
        version: row.get(12)?,
        variables: Vec::new(),
        translations: Vec::new(),
        built_in: row.get::<_, i64>(15)? != 0,
        runtime_required: row.get::<_, i64>(16)? != 0,
        catalog_key: row.get(17)?,
        created_at: timestamp_from_millis(row.get(13)?),
        updated_at: timestamp_from_millis(row.get(14)?),
    })
}

fn scope_kind_to_storage(value: PromptScopeKind) -> &'static str {
    match value {
        PromptScopeKind::Platform => "PLATFORM",
        PromptScopeKind::Workspace => "WORKSPACE",
    }
}

fn scope_kind_from_storage(value: &str) -> rusqlite::Result<PromptScopeKind> {
    match value {
        "PLATFORM" => Ok(PromptScopeKind::Platform),
        "WORKSPACE" => Ok(PromptScopeKind::Workspace),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn category_to_storage(value: PromptCategory) -> &'static str {
    match value {
        PromptCategory::System => "SYSTEM",
        PromptCategory::Persona => "PERSONA",
        PromptCategory::Tool => "TOOL",
        PromptCategory::Template => "TEMPLATE",
        PromptCategory::Action => "ACTION",
    }
}

fn category_from_storage(value: &str) -> rusqlite::Result<PromptCategory> {
    match value {
        "SYSTEM" => Ok(PromptCategory::System),
        "PERSONA" => Ok(PromptCategory::Persona),
        "TOOL" => Ok(PromptCategory::Tool),
        "TEMPLATE" => Ok(PromptCategory::Template),
        "ACTION" => Ok(PromptCategory::Action),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn variable_type_from_storage(value: &str) -> rusqlite::Result<PromptVariableType> {
    match value {
        "STRING" => Ok(PromptVariableType::String),
        "NUMBER" => Ok(PromptVariableType::Number),
        "BOOLEAN" => Ok(PromptVariableType::Boolean),
        "JSON" => Ok(PromptVariableType::Json),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn variable_source_kind_from_storage(
    value: &str,
) -> rusqlite::Result<PromptVariableSourceKind> {
    match value {
        "MANAGED" => Ok(PromptVariableSourceKind::Managed),
        "RUNTIME" => Ok(PromptVariableSourceKind::Runtime),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn runtime_provider_from_storage(value: &str) -> rusqlite::Result<PromptRuntimeProvider> {
    PromptRuntimeProvider::from_code(value).ok_or(rusqlite::Error::InvalidQuery)
}

fn translation_status_to_storage(value: PromptTranslationStatus) -> &'static str {
    match value {
        PromptTranslationStatus::Published => "PUBLISHED",
        PromptTranslationStatus::Draft => "DRAFT",
        PromptTranslationStatus::UpdateAvailable => "UPDATE_AVAILABLE",
    }
}

fn translation_status_from_storage(value: &str) -> rusqlite::Result<PromptTranslationStatus> {
    match value {
        "PUBLISHED" => Ok(PromptTranslationStatus::Published),
        "DRAFT" => Ok(PromptTranslationStatus::Draft),
        "UPDATE_AVAILABLE" => Ok(PromptTranslationStatus::UpdateAvailable),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn source_kind_to_storage(value: PromptSourceKind) -> &'static str {
    match value {
        PromptSourceKind::Platform => "PLATFORM",
        PromptSourceKind::WorkspaceLocal => "WORKSPACE_LOCAL",
        PromptSourceKind::WorkspaceOverride => "WORKSPACE_OVERRIDE",
        PromptSourceKind::WorkspaceStatusPatch => "WORKSPACE_STATUS_PATCH",
    }
}

fn source_kind_from_storage(value: &str) -> rusqlite::Result<PromptSourceKind> {
    match value {
        "PLATFORM" => Ok(PromptSourceKind::Platform),
        "WORKSPACE_LOCAL" => Ok(PromptSourceKind::WorkspaceLocal),
        "WORKSPACE_OVERRIDE" => Ok(PromptSourceKind::WorkspaceOverride),
        "WORKSPACE_STATUS_PATCH" => Ok(PromptSourceKind::WorkspaceStatusPatch),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

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

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use super::{SqlitePromptRepository, migrate_prompt_variable_metadata};

    #[test]
    fn legacy_prompt_variables_are_migrated_to_typed_metadata() {
        let mut connection = Connection::open_in_memory().expect("legacy database should open");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE prompts (prompt_code TEXT PRIMARY KEY);
                 CREATE TABLE prompt_variables (
                    prompt_code TEXT NOT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
                    variable_name TEXT NOT NULL,
                    variable_type TEXT NOT NULL DEFAULT 'STRING'
                        CHECK (variable_type = 'STRING'),
                    position INTEGER NOT NULL,
                    PRIMARY KEY (prompt_code, variable_name)
                 );
                 CREATE INDEX idx_prompt_variables_prompt_position
                    ON prompt_variables(prompt_code, position);
                 INSERT INTO prompts (prompt_code) VALUES ('PROMPT-LEGACY');
                 INSERT INTO prompt_variables (
                    prompt_code, variable_name, variable_type, position
                 ) VALUES ('PROMPT-LEGACY', 'name', 'STRING', 0);",
            )
            .expect("legacy prompt schema should be created");

        migrate_prompt_variable_metadata(&mut connection)
            .expect("legacy variable metadata should migrate");

        let migrated = connection
            .query_row(
                "SELECT variable_type, description FROM prompt_variables
                 WHERE prompt_code = ?1 AND variable_name = ?2",
                params!["PROMPT-LEGACY", "name"],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("migrated variable should remain readable");
        assert_eq!(
            migrated,
            ("STRING".to_string(), "Variable name".to_string())
        );

        connection
            .execute(
                "INSERT INTO prompt_variables (
                    prompt_code, variable_name, variable_type, description, position
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["PROMPT-LEGACY", "count", "NUMBER", "Item count", 1],
            )
            .expect("migrated schema should accept supported variable types");

        migrate_prompt_variable_metadata(&mut connection)
            .expect("reapplying the migration should be safe");
        let preserved = connection
            .query_row(
                "SELECT variable_type, description FROM prompt_variables
                 WHERE prompt_code = ?1 AND variable_name = ?2",
                params!["PROMPT-LEGACY", "count"],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("reapplied migration should preserve typed metadata");
        assert_eq!(preserved, ("NUMBER".to_string(), "Item count".to_string()));
    }

    #[test]
    fn action_category_is_supported_by_the_prompt_schema() {
        let repository =
            SqlitePromptRepository::connect(":memory:").expect("prompt schema should migrate");
        repository
            .connection
            .execute(
                "INSERT INTO prompts (
                    id, scope_kind, workspace_code, prompt_code, display_name, category,
                    prompt_body, description, enabled, source_kind, source_prompt_code,
                    allow_inheritance, version, created_at, updated_at, built_in,
                    runtime_required, catalog_key
                ) VALUES (1, 'PLATFORM', NULL, 'PROMPT-ACTION', 'Action prompt', 'ACTION',
                    'Run {{command}}', NULL, 1, 'PLATFORM', NULL, NULL, 1, 1, 1, 0, 0, NULL)",
                [],
            )
            .expect("action prompt category should satisfy the schema");

        let category = repository
            .connection
            .query_row(
                "SELECT category FROM prompts WHERE prompt_code = 'PROMPT-ACTION'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("action prompt should be readable");
        assert_eq!(category, "ACTION");
    }
}
