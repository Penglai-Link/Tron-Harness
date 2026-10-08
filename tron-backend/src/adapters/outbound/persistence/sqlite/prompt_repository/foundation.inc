use std::{fs, path::Path, time::Duration};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::{
    application::prompt::PromptRepository,
    domain::prompt::{
        Prompt, PromptCategory, PromptError, PromptRuntimeProvider, PromptScopeKind,
        PromptSourceKind, PromptTranslation, PromptTranslationStatus, PromptVariable,
        PromptVariableSourceKind, PromptVariableType,
    },
    shared::{
        id::SnowflakeId,
        time::{timestamp_from_millis, timestamp_to_millis},
    },
};

use super::migrations::{
    PROMPT_ACTION_CATEGORY_MIGRATION, PROMPT_AGENT_ROLE_CATEGORY_MIGRATION,
    PROMPT_CATALOG_MIGRATION, PROMPT_VARIABLE_BINDINGS_MIGRATION,
    PROMPT_VARIABLE_METADATA_MIGRATION, PROMPTS_MIGRATION,
};

struct SqlitePromptRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqlitePromptRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqlitePromptRepositorySql<'a> {
    pub fn initialize(
        database_url: &str,
        mut connection: &'a mut Connection,
    ) -> Result<Self, PromptError> {
        ensure_database_parent(database_url)?;

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
        migrate_prompt_agent_role_category(&mut connection)?;
        Ok(Self { connection })
    }
}

impl PromptRepository for SqlitePromptRepositorySql<'_> {
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
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        insert_prompt(&transaction, prompt)?;
        replace_variables(&transaction, prompt)?;
        replace_translations(&transaction, prompt)?;
        transaction.commit().map_err(storage_error)
    }

    fn update_prompt(&mut self, prompt: &Prompt, expected_version: u64) -> Result<(), PromptError> {
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
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

impl SqlitePromptRepository {
    pub fn connect(database_url: &str) -> Result<Self, PromptError> {
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(database_url).map_err(storage_error)?;
        let database_url = database_url.to_owned();
        connection.call(
            move |connection| {
                SqlitePromptRepositorySql::initialize(&database_url, connection).map(|_| ())
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )?;
        Ok(Self { connection })
    }
}
impl PromptRepository for SqlitePromptRepository {
    fn list_prompts(&mut self) -> Result<Vec<Prompt>, PromptError> {
        self.connection.call(
            move |connection| SqlitePromptRepositorySql { connection }.list_prompts(),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn create_prompt(&mut self, prompt: &Prompt) -> Result<(), PromptError> {
        let prompt = prompt.to_owned();
        self.connection.call(
            move |connection| SqlitePromptRepositorySql { connection }.create_prompt(&prompt),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn update_prompt(&mut self, prompt: &Prompt, expected_version: u64) -> Result<(), PromptError> {
        let prompt = prompt.to_owned();
        self.connection.call(
            move |connection| {
                SqlitePromptRepositorySql { connection }.update_prompt(&prompt, expected_version)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn delete_prompt(&mut self, prompt_code: &str) -> Result<bool, PromptError> {
        let prompt_code = prompt_code.to_owned();
        self.connection.call(
            move |connection| SqlitePromptRepositorySql { connection }.delete_prompt(&prompt_code),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
