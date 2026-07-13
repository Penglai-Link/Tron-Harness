use std::{fs, path::Path};

use rusqlite::{Connection, OpenFlags, Transaction, params};

use crate::{
    application::prompt::PromptRepository,
    domain::prompt::{
        Prompt, PromptCategory, PromptError, PromptScopeKind, PromptSourceKind, PromptVariable,
    },
    shared::{
        id::SnowflakeId,
        time::{timestamp_from_millis, timestamp_to_millis},
    },
};

use super::migrations::PROMPTS_MIGRATION;

pub struct SqlitePromptRepository {
    connection: Connection,
}

impl SqlitePromptRepository {
    pub fn connect(database_url: &str) -> Result<Self, PromptError> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
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
            .execute_batch(PROMPTS_MIGRATION)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }
}

impl PromptRepository for SqlitePromptRepository {
    fn list_prompts(&mut self) -> Result<Vec<Prompt>, PromptError> {
        let mut prompts =
            {
                let mut statement = self.connection.prepare(
                "SELECT id, scope_kind, workspace_code, prompt_code, display_name, category,
                        prompt_body, description, model_code, temperature, max_output_tokens,
                        enabled, source_kind, source_prompt_code, allow_inheritance, version,
                        created_at, updated_at
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
        }
        Ok(prompts)
    }

    fn create_prompt(&mut self, prompt: &Prompt) -> Result<(), PromptError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        insert_prompt(&transaction, prompt)?;
        replace_variables(&transaction, prompt)?;
        transaction.commit().map_err(storage_error)
    }

    fn update_prompt(&mut self, prompt: &Prompt, expected_version: u64) -> Result<(), PromptError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        let updated = transaction
            .execute(
                "UPDATE prompts
             SET display_name = ?2, prompt_body = ?3, description = ?4, model_code = ?5,
                 temperature = ?6, max_output_tokens = ?7, enabled = ?8, source_kind = ?9,
                 source_prompt_code = ?10, allow_inheritance = ?11, version = ?12, updated_at = ?13
             WHERE prompt_code = ?1 AND version = ?14",
                params![
                    prompt.prompt_code,
                    prompt.display_name,
                    prompt.prompt_body,
                    prompt.description,
                    prompt.model_code,
                    prompt.temperature,
                    prompt.max_output_tokens,
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
    transaction.execute(
        "INSERT INTO prompts (
            id, scope_kind, workspace_code, prompt_code, display_name, category, prompt_body,
            description, model_code, temperature, max_output_tokens, enabled, source_kind,
            source_prompt_code, allow_inheritance, version, created_at, updated_at
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
            prompt.model_code,
            prompt.temperature,
            prompt.max_output_tokens,
            bool_to_int(prompt.enabled),
            source_kind_to_storage(prompt.source_kind),
            prompt.source_prompt_code,
            prompt.allow_inheritance.map(bool_to_int),
            prompt.version,
            timestamp_to_millis(&prompt.created_at),
            timestamp_to_millis(&prompt.updated_at),
        ],
    ).map_err(storage_error)?;
    Ok(())
}

fn replace_variables(transaction: &Transaction<'_>, prompt: &Prompt) -> Result<(), PromptError> {
    transaction
        .execute(
            "DELETE FROM prompt_variables WHERE prompt_code = ?1",
            params![prompt.prompt_code],
        )
        .map_err(storage_error)?;
    for (position, variable) in prompt.variables.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO prompt_variables (prompt_code, variable_name, variable_type, position)
             VALUES (?1, ?2, ?3, ?4)",
                params![
                    prompt.prompt_code,
                    variable.name,
                    variable.variable_type,
                    position
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
            "SELECT variable_name, variable_type FROM prompt_variables
         WHERE prompt_code = ?1 ORDER BY position",
        )
        .map_err(storage_error)?;
    statement
        .query_map(params![prompt_code], |row| {
            Ok(PromptVariable {
                name: row.get(0)?,
                variable_type: row.get(1)?,
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
        model_code: row.get(8)?,
        temperature: row.get(9)?,
        max_output_tokens: row.get(10)?,
        enabled: row.get::<_, i64>(11)? != 0,
        source_kind: source_kind_from_storage(&row.get::<_, String>(12)?)?,
        source_prompt_code: row.get(13)?,
        allow_inheritance: row.get::<_, Option<i64>>(14)?.map(|value| value != 0),
        version: row.get(15)?,
        variables: Vec::new(),
        created_at: timestamp_from_millis(row.get(16)?),
        updated_at: timestamp_from_millis(row.get(17)?),
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
    }
}

fn category_from_storage(value: &str) -> rusqlite::Result<PromptCategory> {
    match value {
        "SYSTEM" => Ok(PromptCategory::System),
        "PERSONA" => Ok(PromptCategory::Persona),
        "TOOL" => Ok(PromptCategory::Tool),
        "TEMPLATE" => Ok(PromptCategory::Template),
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

fn storage_error(_: impl std::fmt::Debug) -> PromptError {
    PromptError::StorageFailure
}
