use super::*;

pub(crate) fn migrate_provider_catalog_model_schema(
    connection: &Connection,
) -> rusqlite::Result<()> {
    ensure_sqlite_column(
        connection,
        "provider_catalog_models",
        "quality_type",
        "TEXT NOT NULL DEFAULT 'High'",
    )?;
    for (column, definition) in [
        ("person_ai_user_id", "TEXT"),
        ("person_ai_username", "TEXT"),
    ] {
        ensure_sqlite_column(connection, "provider_connections", column, definition)?;
    }
    for (column, definition) in [
        ("protocols", "TEXT NOT NULL DEFAULT '[\"ChatCompletions\"]'"),
        ("input_coefficient", "REAL"),
        ("cache_hit_coefficient", "REAL"),
        ("output_coefficient", "REAL"),
    ] {
        ensure_sqlite_column(connection, "provider_catalog_models", column, definition)?;
    }
    let primary_key = {
        let mut statement = connection.prepare("PRAGMA table_info(provider_catalog_models)")?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    if !primary_key
        .iter()
        .any(|(name, position)| name == "upstream_vendor_code" && *position > 0)
    {
        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(
            connection,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        transaction.execute_batch(include_str!(
            "../../../../migrations/20261008_0006_provider_catalog_vendor_quality.sql"
        ))?;
        transaction.commit()?;
    }
    if sqlite_table_exists_raw(connection, "default_model_selections")? {
        ensure_sqlite_column(
            connection,
            "default_model_selections",
            "quality_type",
            "TEXT",
        )?;
    }
    if sqlite_table_exists_raw(connection, "provider_model_run_snapshots")? {
        ensure_sqlite_column(
            connection,
            "provider_model_run_snapshots",
            "quality_type",
            "TEXT",
        )?;
    }
    Ok(())
}

pub(super) fn ensure_sqlite_column(
    connection: &Connection,
    table_name: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    let columns = {
        let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
        statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<HashSet<_>, _>>()?
    };
    if !columns.contains(column) {
        connection.execute(
            &format!("ALTER TABLE {table_name} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

pub(super) type ProviderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ModelProviderError>> + Send + 'a>>;

pub(super) fn provider_quality_from_sdk(quality: QualityType) -> ProviderModelQuality {
    match quality {
        QualityType::High => ProviderModelQuality::High,
        QualityType::Middle => ProviderModelQuality::Middle,
        QualityType::Low => ProviderModelQuality::Low,
    }
}

pub(super) fn provider_quality_to_sdk(quality: ProviderModelQuality) -> QualityType {
    match quality {
        ProviderModelQuality::High => QualityType::High,
        ProviderModelQuality::Middle => QualityType::Middle,
        ProviderModelQuality::Low => QualityType::Low,
    }
}

pub(super) fn agent_protocol_from_sdk(protocol: ProtocolType) -> AgentModelProtocol {
    match protocol {
        ProtocolType::ChatCompletions => AgentModelProtocol::ChatCompletions,
        ProtocolType::Responses => AgentModelProtocol::Responses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_identity_migration_preserves_rows_and_is_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MODEL_PROVIDER_MIGRATION).unwrap();
        connection
            .execute_batch(
                "INSERT INTO provider_catalog_models (
                provider_code, model_code, model_name, upstream_vendor_code,
                upstream_vendor_name, protocols, quality_type, input_coefficient,
                status, first_seen_at, last_seen_at, updated_at
             ) VALUES
                ('PersonAi', 'shared-model', 'Shared Model', 'vendor-a', 'Vendor A',
                 '[\"ChatCompletions\",\"Responses\"]', 'High', 0.01, 'Available', 1, 2, 2),
                ('PersonAi', 'legacy-model', 'Legacy Model', NULL, NULL,
                 '[\"ChatCompletions\"]', 'Low', NULL, 'Unavailable', 3, 4, 4);
             INSERT INTO default_model_selections (
                scope_key, model_type, selection_kind, provider_code, model_code,
                quality_type, created_at, updated_at
             ) VALUES ('platform', 'TEXT', 'Fixed', 'PersonAi', 'shared-model', 'High', 1, 1);",
            )
            .unwrap();
        let before = read_chat_catalog_models(&connection, ModelProviderCode::PersonAi).unwrap();
        // The migration must also work inside the Framework's existing transaction.
        connection.execute_batch("BEGIN").unwrap();
        migrate_provider_catalog_model_schema(&connection).unwrap();
        assert!(!connection.is_autocommit());
        connection.execute_batch("COMMIT").unwrap();
        migrate_provider_catalog_model_schema(&connection).unwrap();
        let after = read_chat_catalog_models(&connection, ModelProviderCode::PersonAi).unwrap();
        assert_eq!(before, after);
        assert_eq!(after.len(), 2);
        assert_eq!(
            effective_default_selection(&connection, "", ModelType::Text).unwrap(),
            Some(ModelSelection::person_ai_fixed(
                "shared-model",
                ProviderModelQuality::High
            )),
        );
        // Both a different vendor and a different tier can coexist with the old row.
        connection.execute_batch(
            "INSERT INTO provider_catalog_models (
                provider_code, model_code, model_name, upstream_vendor_code,
                quality_type, status, first_seen_at, updated_at
             ) VALUES
                ('PersonAi', 'shared-model', 'Shared Model', 'vendor-b', 'High', 'Available', 5, 5),
                ('PersonAi', 'shared-model', 'Shared Model', 'vendor-a', 'Middle', 'Available', 5, 5);"
        ).unwrap();
        assert_eq!(
            read_chat_catalog_models(&connection, ModelProviderCode::PersonAi)
                .unwrap()
                .len(),
            4
        );
    }
}
