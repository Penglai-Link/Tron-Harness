fn map_corpus_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RagCorpusRecord> {
    let status = row.get::<_, String>(2)?;
    let capabilities_json = row.get::<_, String>(4)?;
    let filter_schema_json = row.get::<_, String>(5)?;
    let field_mapping_json = row.get::<_, String>(6)?;
    let storage_binding_json = row.get::<_, String>(7)?;

    let (field_mapping, source_field_mapping, text_unit_field_mapping) =
        parse_field_mapping(&field_mapping_json)?;
    Ok(RagCorpusRecord {
        corpus_id: row.get(0)?,
        display_name: row.get(1)?,
        status: RagStatus::try_from(status.as_str()).map_err(to_sql_error)?,
        description: row.get(3)?,
        capabilities: parse_json_value(&capabilities_json)?,
        filter_schema: parse_json_value(&filter_schema_json)?,
        field_mapping,
        source_field_mapping,
        text_unit_field_mapping,
        storage_binding: parse_storage_binding(&storage_binding_json)?,
    })
}

fn map_corpus_management_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RagCorpusManagementRecord> {
    let record = map_corpus_row(row)?;
    Ok(RagCorpusManagementRecord { record })
}

fn map_profile_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RagProfileRecord> {
    let profile_type = row.get::<_, String>(1)?;
    let status = row.get::<_, String>(2)?;
    let corpus_ids_json = row.get::<_, String>(4)?;
    let config_json = row.get::<_, String>(6)?;
    let filters_json = row.get::<_, String>(7)?;

    Ok(RagProfileRecord {
        profile_id: row.get(0)?,
        profile_type: RagProfileType::try_from(profile_type.as_str()).map_err(to_sql_error)?,
        status: RagStatus::try_from(status.as_str()).map_err(to_sql_error)?,
        primary_corpus_id: row.get(3)?,
        corpus_ids: parse_json_value(&corpus_ids_json)?,
        required_source_version: row.get(5)?,
        config: parse_json_value(&config_json)?,
        filters: parse_json_value(&filters_json)?,
        degrade_policy: row.get(8)?,
    })
}

fn parse_storage_binding(raw: &str) -> rusqlite::Result<RagStorageBinding> {
    let value: Value = parse_json_value(raw)?;
    let mode = value
        .get("mode")
        .and_then(Value::as_str)
        .map(str::trim)
        .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
    if mode != "TARGETED" {
        return Err(to_sql_error(RagError::StorageBindingInvalid));
    }
    let segment_bindings = parse_segment_bindings(value.get("bindings"))?;
    let segment_targets = parse_segment_targets(value.get("targets"))?;
    if segment_bindings.is_empty() || segment_targets.is_empty() {
        return Err(to_sql_error(RagError::StorageBindingInvalid));
    }
    Ok(RagStorageBinding {
        segment_bindings,
        segment_targets,
    })
}

fn parse_segment_bindings(value: Option<&Value>) -> rusqlite::Result<BTreeMap<String, String>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let object = value
        .as_object()
        .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
    object
        .iter()
        .map(|(segment, target_id)| {
            let target_id = target_id
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
            Ok((segment.clone(), target_id.to_string()))
        })
        .collect()
}

fn parse_segment_targets(
    value: Option<&Value>,
) -> rusqlite::Result<BTreeMap<String, RagStorageSegmentTarget>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let targets = value
        .as_array()
        .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
    targets
        .iter()
        .map(|target| {
            let id = target
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
            let backend = target
                .get("backend")
                .and_then(Value::as_str)
                .map(RagStorageBackend::try_from)
                .transpose()
                .map_err(to_sql_error)?
                .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
            Ok((
                id.to_string(),
                RagStorageSegmentTarget {
                    id: id.to_string(),
                    backend,
                    layout: target
                        .get("layout")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    table: target
                        .get("table")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    namespace: target
                        .get("namespace")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    node_label: target
                        .get("nodeLabel")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    code_strategy: target
                        .get("codeStrategy")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                },
            ))
        })
        .collect()
}

fn parse_json_value<T>(raw: &str) -> rusqlite::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(raw).map_err(|_| to_sql_error(RagError::StorageFailure))
}

fn to_sql_error(error: RagError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn serialize_json_value<T>(value: &T) -> Result<String, RagError>
where
    T: serde::Serialize,
{
    serde_json::to_string(value).map_err(|_| RagError::StorageFailure)
}

fn serialize_storage_binding_value(value: &RagStorageBinding) -> Result<String, RagError> {
    let targets = value
        .segment_targets
        .values()
        .map(|target| {
            json!({
                "id": target.id,
                "backend": target.backend.as_str(),
                "layout": target.layout,
                "table": target.table,
                "namespace": target.namespace,
                "nodeLabel": target.node_label,
                "codeStrategy": target.code_strategy,
            })
        })
        .collect::<Vec<_>>();
    let object = serde_json::Map::from_iter([
        ("mode".to_string(), json!("TARGETED")),
        ("bindings".to_string(), json!(value.segment_bindings)),
        ("targets".to_string(), Value::Array(targets)),
    ]);
    serialize_json_value(&Value::Object(object))
}

fn serialize_field_mapping(record: &RagCorpusRecord) -> Result<String, RagError> {
    let value = if record.text_unit_field_mapping.is_empty() {
        json!(record.source_field_mapping)
    } else {
        json!({
            "source": record.source_field_mapping,
            "textUnit": record.text_unit_field_mapping,
        })
    };
    serialize_json_value(&value)
}

fn current_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn ensure_database_parent(database_url: &str) -> Result<(), RagError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| RagError::StorageFailure)?;
    }
    Ok(())
}

fn ensure_legacy_memory_source_domain_columns(connection: &Connection) -> Result<(), RagError> {
    let columns = table_columns_for_migration(connection, "rag_memory_sources")?;
    if columns.is_empty() {
        return Ok(());
    }

    for (column, definition) in [
        ("memory_type", "TEXT"),
        ("memory_status", "TEXT"),
        ("review_status", "TEXT"),
        ("version", "TEXT"),
        ("superseded_by", "TEXT"),
        ("conflict_group", "TEXT"),
        ("duplicate_group", "TEXT"),
        ("injectable", "INTEGER NOT NULL DEFAULT 1"),
        ("importance_score", "INTEGER"),
        ("confidence_score", "INTEGER"),
        ("quality_score", "INTEGER"),
        ("pin_level", "INTEGER NOT NULL DEFAULT 0"),
        ("expires_at", "INTEGER"),
        ("derived_source_type", "TEXT"),
        ("derived_source_code", "TEXT"),
    ] {
        if !columns.contains(column) {
            connection
                .execute(
                    &format!("ALTER TABLE rag_memory_sources ADD COLUMN {column} {definition}"),
                    [],
                )
                .map_err(storage_error)?;
        }
    }

    Ok(())
}

fn table_columns_for_migration(
    connection: &Connection,
    table_name: &str,
) -> Result<BTreeSet<String>, RagError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(storage_error)
}

fn storage_error(error: rusqlite::Error) -> RagError {
    eprintln!("RAG SQLite storage error: {error:?}");
    RagError::StorageFailure
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use rusqlite::params;

    use crate::{
        application::rag::{RagConfigRepository, RagProfileListQuery},
        domain::rag::{RagProfileType, RagStatus},
    };

    use super::SqliteRagRepository;

    #[test]
    fn skill_corpus_migration_seeds_embedded_table_and_profiles() {
        let repository = SqliteRagRepository::connect(":memory:").unwrap();

        let table_names = repository.rag_table_names().unwrap();
        assert!(table_names.contains("rag_skill_text_units"));

        let columns = repository.table_columns("rag_skill_text_units").unwrap();
        assert!(columns.contains("scope_code"));
        assert!(columns.contains("skill_name"));
        assert!(columns.contains("skill_description"));
        assert!(!columns.contains("workspace_code"));

        let corpus = repository.get_corpus("skill").unwrap().unwrap();
        assert_eq!(corpus.status, RagStatus::Enabled);
        assert_eq!(
            corpus.storage_binding.sqlite_table_for_segment("textUnit"),
            Some("rag_skill_text_units")
        );
        assert_eq!(
            corpus.field_mapping.get("skillName").map(String::as_str),
            Some("skill_name")
        );
        assert!(!corpus.field_mapping.contains_key("workspaceCode"));

        let profiles = repository
            .list_profiles(RagProfileListQuery {
                profile_type: None,
                status: Some(RagStatus::Enabled),
                corpus_id: Some("skill".into()),
            })
            .unwrap();
        let profile_ids = profiles
            .iter()
            .map(|profile| profile.profile_id.as_str())
            .collect::<Vec<_>>();
        assert!(profile_ids.contains(&"skill-ingest-v1"));
        assert!(profile_ids.contains(&"skill-related-v1"));
        assert!(
            profiles
                .iter()
                .any(|profile| profile.profile_type == RagProfileType::Ingest)
        );
        let ingest_profile = profiles
            .iter()
            .find(|profile| profile.profile_id == "skill-ingest-v1")
            .expect("skill ingest profile should be seeded");
        assert_eq!(
            ingest_profile
                .config
                .get("stages")
                .and_then(|stages| stages.get("preparePayload"))
                .and_then(|stage| stage.get("strategyId"))
                .and_then(serde_json::Value::as_str),
            Some("RAG_TEXT_CONTRACT_PAYLOAD_V1")
        );
        assert_eq!(
            corpus
                .source_field_mapping
                .get("skillName")
                .map(String::as_str),
            Some("skill_name")
        );
        assert!(
            profiles
                .iter()
                .any(|profile| profile.profile_type == RagProfileType::Retrieval)
        );
    }

    #[test]
    fn connect_backfills_storage_binding_from_transitional_storage_topology() {
        let database_path = std::env::temp_dir().join(format!(
            "rag-storage-topology-backfill-{}-{}.sqlite3",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let database_url = database_path.display().to_string();
        let repository = SqliteRagRepository::connect(&database_url).unwrap();
        repository
            .connection
            .execute_batch(
                "ALTER TABLE rag_corpora ADD COLUMN storage_topology_json TEXT;
                 DELETE FROM rag_corpora WHERE corpus_id = 'legacy-local';",
            )
            .unwrap();
        let topology = r#"{"mode":"TARGETED","bindings":{"source":"source-store","textUnit":"text-store","vector":"vector-store","diagnostics":"diagnostics-store"},"targets":[{"id":"source-store","backend":"SQLITE","layout":"TABLE","table":"rag_legacy_sources"},{"id":"text-store","backend":"SQLITE","layout":"TABLE","table":"rag_legacy_text_units"},{"id":"vector-store","backend":"SQLITE","layout":"TABLE","table":"rag_legacy_vectors"},{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}"#;
        repository
            .connection
            .execute(
                "INSERT INTO rag_corpora (
                    corpus_id,
                    display_name,
                    status,
                    description,
                    capabilities_json,
                    filter_schema_json,
                    field_mapping_json,
                    storage_binding_json,
                    storage_topology_json,
                    created_at,
                    updated_at
                 )
                 VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, 1, 1)",
                params![
                    "legacy-local",
                    "Legacy Local",
                    "enabled",
                    r#"{"fulltext":true,"vector":true,"graph":false}"#,
                    r#"{"fields":{}}"#,
                    "{}",
                    r#"{"backend":"SQLITE","layout":"SPLIT","sourceTable":"rag_legacy_sources","textUnitTable":"rag_legacy_text_units","vectorTable":"rag_legacy_vectors"}"#,
                    topology,
                ],
            )
            .unwrap();
        drop(repository);

        let repository = SqliteRagRepository::connect(&database_url).unwrap();
        let corpus = repository.get_corpus("legacy-local").unwrap().unwrap();

        assert_eq!(
            corpus.storage_binding.sqlite_table_for_segment("source"),
            Some("rag_legacy_sources")
        );
        assert_eq!(
            corpus.storage_binding.sqlite_table_for_segment("textUnit"),
            Some("rag_legacy_text_units")
        );
        assert_eq!(
            corpus.storage_binding.sqlite_table_for_segment("vector"),
            Some("rag_legacy_vectors")
        );
        let _ = std::fs::remove_file(database_path);
    }
}
