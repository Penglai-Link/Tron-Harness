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

fn storage_error(error: rusqlite::Error) -> RagError {
    eprintln!("RAG SQLite storage error: {error:?}");
    RagError::StorageFailure
}
