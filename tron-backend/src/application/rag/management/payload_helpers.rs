fn capabilities_from_value(value: &Value) -> RagCorpusCapabilitiesPayload {
    RagCorpusCapabilitiesPayload {
        fulltext: value
            .get("fulltext")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        vector: value
            .get("vector")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        tree: value.get("tree").and_then(Value::as_bool).unwrap_or(false),
        graph: value.get("graph").and_then(Value::as_bool).unwrap_or(false),
    }
}

fn metadata_groups_from_value(value: &Value) -> Vec<RagMetadataGroup> {
    value
        .get("metadataGroups")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value(item.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn capabilities_value(
    capabilities: &RagCorpusCapabilitiesPayload,
    metadata_groups: &[RagMetadataGroup],
) -> Value {
    json!({
        "fulltext": capabilities.fulltext,
        "vector": capabilities.vector,
        "tree": capabilities.tree,
        "graph": capabilities.graph,
        "metadataGroups": metadata_groups,
    })
}

fn derive_text_unit_mapping(field_mapping: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    field_mapping
        .iter()
        .filter(|(logical_name, _)| {
            matches!(
                logical_name.as_str(),
                "sourceCode"
                    | "sourceVersion"
                    | "scopeType"
                    | "scopeCode"
                    | "text"
                    | "textUnitStatus"
            )
        })
        .map(|(logical_name, column)| (logical_name.clone(), column.clone()))
        .collect()
}

fn storage_binding_payload_from_record(binding: &RagStorageBinding) -> RagStorageBindingPayload {
    let targets = binding
        .segment_targets
        .values()
        .map(|target| RagStorageTargetPayload {
            id: target.id.clone(),
            backend: target.backend.as_str().into(),
            layout: target.layout.clone(),
            table: target.table.clone(),
            namespace: target.namespace.clone(),
            node_label: target.node_label.clone(),
            code_strategy: target.code_strategy.clone(),
        })
        .collect();
    RagStorageBindingPayload {
        mode: "TARGETED".into(),
        targets,
        bindings: binding.segment_bindings.clone(),
    }
}

fn validation_issue(code: &str, message: &str, field: Option<&str>) -> RagValidationIssueView {
    RagValidationIssueView {
        code: code.to_string(),
        message: message.to_string(),
        field: field.map(ToOwned::to_owned),
    }
}

fn first_validation_error(errors: &[RagValidationIssueView]) -> RagError {
    match errors.first().map(|error| error.code.as_str()) {
        Some("RAG_STORAGE_GRAPH_BINDING_REQUIRED") => RagError::StorageGraphBindingRequired,
        Some("RAG_STORAGE_TOPOLOGY_UNSUPPORTED") => RagError::StorageTopologyUnsupported,
        Some("RAG_FILTER_FIELD_INVALID") => RagError::FilterFieldInvalid,
        Some("RAG_FILTER_OPERATOR_UNSUPPORTED") => RagError::FilterOperatorUnsupported,
        Some("RAG_PROFILE_STRATEGY_INVALID") => RagError::ProfileStrategyInvalid,
        Some("RAG_PROFILE_CORPUS_INVALID") => RagError::ProfileCorpusInvalid,
        _ => RagError::ManagementRequestInvalid,
    }
}

fn merge_field_mappings(field_mapping: &RagFieldMappingPayload) -> BTreeMap<String, String> {
    let mut merged = field_mapping.source.clone();
    merged.extend(field_mapping.text_unit.clone());
    merged
}

fn dedup_strings(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert((*value).to_string()))
        .map(ToOwned::to_owned)
        .collect()
}

fn infer_route_type(strategy_id: &str) -> &'static str {
    if strategy_id.contains("STRUCTURED_FILTER") {
        "STRUCTURED_FILTER"
    } else if strategy_id == "TREE_FULLTEXT_V1" {
        "TREE_FULLTEXT"
    } else if strategy_id == "TREE_VECTOR_V1" {
        "TREE_VECTOR"
    } else if strategy_id.contains("VECTOR") {
        "VECTOR"
    } else if strategy_id.contains("FULLTEXT") {
        "FULLTEXT"
    } else if strategy_id.contains("GRAPH") {
        "GRAPH"
    } else {
        "FULLTEXT"
    }
}

fn strategy_matches_route_type<S>(
    route_type: &str,
    strategy_id: &str,
    strategy_registry: &S,
) -> bool
where
    S: RagStrategyRegistry,
{
    if strategy_id.trim().is_empty()
        || strategy_registry.resolve_retriever(strategy_id).status
            != StrategyRegistrationStatus::Enabled
    {
        return false;
    }
    match route_type {
        "FULLTEXT" => strategy_id.contains("FULLTEXT"),
        "VECTOR" => strategy_id.contains("VECTOR"),
        "TREE_FULLTEXT" => strategy_id == "TREE_FULLTEXT_V1",
        "TREE_VECTOR" => strategy_id == "TREE_VECTOR_V1",
        "GRAPH" => strategy_id.contains("GRAPH"),
        "STRUCTURED_FILTER" => strategy_id.contains("STRUCTURED_FILTER"),
        _ => false,
    }
}

fn profile_description(config: &Value) -> String {
    config
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn filters_to_rows(filters: &Value) -> Vec<RagRetrievalFilterPayload> {
    filters
        .as_object()
        .map(|items| {
            items
                .iter()
                .map(|(field, value)| {
                    let operator = value
                        .get("op")
                        .and_then(Value::as_str)
                        .unwrap_or("eq")
                        .to_string();
                    let raw_value = value.get("value").cloned().unwrap_or(Value::Null);
                    let string_value = raw_value
                        .as_str()
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| raw_value.to_string().trim_matches('"').to_string());
                    RagRetrievalFilterPayload {
                        field: field.clone(),
                        operator,
                        value: string_value,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn topic_policy_code(config: &Value) -> String {
    let Some(policy) = config.get("topicPolicy").and_then(Value::as_object) else {
        return "NONE".into();
    };
    if !policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return "NONE".into();
    }
    policy
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("NONE")
        .to_string()
}

fn graph_expansion_policy_code(config: &Value) -> String {
    let Some(policy) = config
        .get("graphExpansionPolicy")
        .and_then(Value::as_object)
    else {
        return String::new();
    };
    if policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "OUTGOING_DEPTH_1".into()
    } else {
        String::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilterSchemaField {
    field_type: String,
    operators: BTreeSet<String>,
}

fn parse_filter_schema(schema: &Value) -> BTreeMap<String, FilterSchemaField> {
    schema
        .get("fields")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .map(|(name, value)| {
                    let operators = value
                        .get("operators")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .map(ToOwned::to_owned)
                                .collect()
                        })
                        .unwrap_or_else(|| BTreeSet::from(["eq".into(), "in".into()]));
                    let field_type = value
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("string")
                        .to_string();
                    (
                        name.clone(),
                        FilterSchemaField {
                            field_type,
                            operators,
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_filter_schema_entry(
    field: &str,
    corpus_ids: &[String],
    corpora_by_id: &BTreeMap<String, &RagCorpusRecord>,
) -> Option<(String, BTreeSet<String>)> {
    for corpus_id in corpus_ids {
        let corpus = corpora_by_id.get(corpus_id)?;
        let schema = parse_filter_schema(&corpus.filter_schema);
        if let Some(entry) = schema.get(field) {
            return Some((entry.field_type.clone(), entry.operators.clone()));
        }
    }
    None
}

fn extract_filter_operator(raw_value: &Value) -> (&str, &Value) {
    if let Some(object) = raw_value.as_object()
        && let (Some(operator), Some(value)) = (
            object.get("op").and_then(Value::as_str),
            object.get("value"),
        )
    {
        return (operator, value);
    }
    if raw_value.is_array() {
        ("in", raw_value)
    } else {
        ("eq", raw_value)
    }
}

fn filter_value_matches(field_type: &str, operator: &str, value: &Value) -> bool {
    if operator == "in" {
        return value.as_array().is_some_and(|items| {
            items
                .iter()
                .all(|item| single_value_matches(field_type, item))
        });
    }
    single_value_matches(field_type, value)
}

fn single_value_matches(field_type: &str, value: &Value) -> bool {
    match field_type {
        "integer" | "number" => value.as_i64().is_some(),
        "boolean" => value.is_boolean(),
        _ => value.is_string(),
    }
}

fn coerce_filter_value(field_type: &str, raw_value: &str) -> Result<Value, RagError> {
    match field_type {
        "integer" | "number" => raw_value
            .trim()
            .parse::<i64>()
            .map(Value::from)
            .map_err(|_| RagError::ManagementRequestInvalid),
        "boolean" => raw_value
            .trim()
            .parse::<bool>()
            .map(Value::from)
            .map_err(|_| RagError::ManagementRequestInvalid),
        _ => Ok(Value::String(raw_value.to_string())),
    }
}
