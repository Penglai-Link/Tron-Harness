fn persist_fulltext_units(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    if !plan.stores_fulltext_carrier_rows() {
        return Ok(());
    }
    let now = now_millis();
    for unit in &command.text_units {
        let mut values = Vec::new();
        push_carrier_text_unit_values(
            &mut values,
            &plan.fulltext_columns,
            plan,
            command,
            unit,
            now,
        )?;
        execute_upsert(
            tx,
            plan.fulltext_storage_table(),
            values,
            &plan
                .fulltext_unique_key
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            &[Some("created_at")],
        )?;
    }
    Ok(())
}

fn push_carrier_text_unit_values(
    values: &mut Vec<(String, rusqlite::types::Value)>,
    columns: &std::collections::BTreeSet<String>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
    unit: &RagTextUnitWrite,
    now: i64,
) -> Result<(), RagError> {
    push_text(values, columns, "text_unit_code", &unit.text_unit_code);
    push_text(values, columns, "source_code", &command.source_code);
    push_text(values, columns, "source_version", &command.source_version);
    push_text(values, columns, "scope_type", &command.scope.scope_type);
    push_text(values, columns, "scope_code", &command.scope.scope_code);
    push_text(values, columns, "unit_type", &unit.unit_type);
    push_text(values, columns, "text", &unit.text);
    push_text(values, columns, "search_text", &unit.search_text);
    push_text(values, columns, "heading_path", &unit.heading_path);
    push_integer(values, columns, "ordinal", unit.ordinal as i64);
    push_integer(values, columns, "token_count", unit.token_count);
    push_text(values, columns, "status", &unit.status);
    push_text(values, columns, "content_hash", &unit.content_hash);
    push_dynamic_text(values, columns, "metadata", &json_string(&unit.metadata)?);
    push_integer(values, columns, "created_at", now);
    push_integer(values, columns, "updated_at", now);
    push_available_domain_values(values, columns, &plan.text_unit_field_mapping, command)?;
    Ok(())
}

fn persist_warnings(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let diagnostics_table = plan
        .diagnostics_table
        .as_deref()
        .ok_or(RagError::StorageBindingInvalid)?;
    let diagnostics_table = quote_identifier(diagnostics_table)?;
    let now = now_millis();
    for warning in &command.warnings {
        let record_code = format!(
            "{}:{}:{}:{}:{}:{}",
            command.corpus_id,
            command.profile_id,
            command.source_code,
            command.source_version,
            command.scope.scope_type,
            warning.stage_id
        );
        let sql = format!(
            "INSERT INTO {diagnostics_table} (
                record_type, record_code, corpus_id, profile_id, source_code, source_version,
                scope_type, scope_code, workspace_code, user_code, agent_code, stage_id,
                message, payload_json, created_at
            )
            VALUES ('INDEXING_WARNING', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT (record_code) DO UPDATE SET
                message = excluded.message,
                payload_json = excluded.payload_json,
                created_at = excluded.created_at"
        );
        tx.execute(
            &sql,
            params![
                record_code,
                command.corpus_id,
                command.profile_id,
                command.source_code,
                command.source_version,
                command.scope.scope_type,
                command.scope.scope_code,
                command.workspace_code,
                command.user_code,
                command.agent_code,
                warning.stage_id,
                warning.message,
                json_string(&serde_json::json!({ "code": warning.code }))?,
                now,
            ],
        )
        .map_err(storage_error)?;
    }
    Ok(())
}
