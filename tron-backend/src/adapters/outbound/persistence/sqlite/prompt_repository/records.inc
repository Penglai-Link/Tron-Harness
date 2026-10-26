fn insert_prompt(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    prompt: &Prompt,
) -> Result<(), PromptError> {
    transaction
        .execute(
            "INSERT INTO prompts (
            id, prompt_code, display_name, category, prompt_body,
            description, enabled, version, created_at, updated_at,
            built_in, runtime_required, catalog_key
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                prompt.id.value(),
                prompt.prompt_code,
                prompt.display_name,
                category_to_storage(prompt.category),
                prompt.prompt_body,
                prompt.description,
                bool_to_int(prompt.enabled),
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

fn replace_variables(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    prompt: &Prompt,
) -> Result<(), PromptError> {
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

fn replace_translations(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    prompt: &Prompt,
) -> Result<(), PromptError> {
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
) -> Result<std::collections::BTreeMap<String, Vec<PromptVariable>>, PromptError> {
    let mut statement = connection
        .prepare(
            "SELECT variable_name, variable_type, description, source_kind,
                    managed_variable_key, runtime_provider, prompt_code FROM prompt_variables
         ORDER BY prompt_code, position",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(6)?,
                PromptVariable {
                    name: row.get(0)?,
                    variable_type: variable_type_from_storage(&row.get::<_, String>(1)?)?,
                    description: row.get(2)?,
                    source_kind: variable_source_kind_from_storage(&row.get::<_, String>(3)?)?,
                    managed_variable_key: row.get(4)?,
                    runtime_provider: row
                        .get::<_, Option<String>>(5)?
                        .map(|value| runtime_provider_from_storage(&value))
                        .transpose()?,
                },
            ))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    let mut grouped = std::collections::BTreeMap::<String, Vec<PromptVariable>>::new();
    for (code, record) in rows {
        grouped.entry(code).or_default().push(record);
    }
    Ok(grouped)
}

fn load_translations(
    connection: &Connection,
) -> Result<std::collections::BTreeMap<String, Vec<PromptTranslation>>, PromptError> {
    let mut statement = connection
        .prepare(
            "SELECT locale, display_name, prompt_body, description,
                    translation_status, source_version, updated_at, prompt_code
             FROM prompt_translations ORDER BY prompt_code, locale",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(7)?,
                PromptTranslation {
                    locale: row.get(0)?,
                    display_name: row.get(1)?,
                    prompt_body: row.get(2)?,
                    description: row.get(3)?,
                    status: translation_status_from_storage(&row.get::<_, String>(4)?)?,
                    source_version: row.get(5)?,
                    updated_at: timestamp_from_millis(row.get(6)?),
                },
            ))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    let mut grouped = std::collections::BTreeMap::<String, Vec<PromptTranslation>>::new();
    for (code, record) in rows {
        grouped.entry(code).or_default().push(record);
    }
    Ok(grouped)
}

fn decode_prompt(row: &rusqlite::Row<'_>) -> rusqlite::Result<Prompt> {
    Ok(Prompt {
        id: SnowflakeId::new(row.get(0)?),
        prompt_code: row.get(1)?,
        display_name: row.get(2)?,
        category: category_from_storage(&row.get::<_, String>(3)?)?,
        prompt_body: row.get(4)?,
        description: row.get(5)?,
        enabled: row.get::<_, i64>(6)? != 0,
        version: row.get(7)?,
        variables: Vec::new(),
        translations: Vec::new(),
        built_in: row.get::<_, i64>(10)? != 0,
        runtime_required: row.get::<_, i64>(11)? != 0,
        catalog_key: row.get(12)?,
        created_at: timestamp_from_millis(row.get(8)?),
        updated_at: timestamp_from_millis(row.get(9)?),
    })
}
