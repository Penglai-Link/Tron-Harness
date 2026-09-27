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
