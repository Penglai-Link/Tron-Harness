
use rusqlite::{Connection, params};

use crate::{
    application::prompt::PromptRepository,
    domain::prompt::{
        Prompt, PromptCategory, PromptError, PromptRuntimeProvider,
        PromptTranslation, PromptTranslationStatus, PromptVariable,
        PromptVariableSourceKind, PromptVariableType,
    },
    shared::{
        id::SnowflakeId,
        time::{timestamp_from_millis, timestamp_to_millis},
    },
};

struct SqlitePromptRepositorySql<'a> {
    connection: &'a mut Connection,
}
pub struct SqlitePromptRepository {
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
}

impl<'a> SqlitePromptRepositorySql<'a> {
    pub fn initialize(
        _database_url: &str,
        connection: &'a mut Connection,
    ) -> Result<Self, PromptError> {
        crate::adapters::outbound::persistence::sqlite::schema::initialize_business(connection)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl PromptRepository for SqlitePromptRepositorySql<'_> {
    fn list_prompts(&mut self) -> Result<Vec<Prompt>, PromptError> {
        let mut prompts =
            {
                let mut statement = self.connection.prepare(
                "SELECT id, prompt_code, display_name, category, prompt_body, description,
                        enabled, version,
                        created_at, updated_at, built_in, runtime_required, catalog_key
                 FROM prompts
                 ORDER BY display_name, prompt_code",
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
                 version = ?6, updated_at = ?7
             WHERE prompt_code = ?1 AND version = ?8",
                params![
                    prompt.prompt_code,
                    prompt.display_name,
                    prompt.prompt_body,
                    prompt.description,
                    bool_to_int(prompt.enabled),
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
