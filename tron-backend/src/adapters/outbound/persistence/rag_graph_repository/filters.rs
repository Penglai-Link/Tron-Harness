use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::{
    graph::{GraphFilterOperator, GraphPropertyFilter, GraphPropertyValue},
    rag::{RagCorpusRecord, RagError},
};

use super::model::graph_property_name_for_logical;

pub(super) fn append_graph_filters(
    filters: &mut Vec<GraphPropertyFilter>,
    corpus: &RagCorpusRecord,
    raw: &Value,
    segment: &str,
) -> Result<(), RagError> {
    let object = raw.as_object().ok_or(RagError::SearchFilterInvalid)?;
    let schema = parse_graph_filter_schema(&corpus.filter_schema);
    for (field, value) in object {
        if matches!(field.as_str(), "sourceVersion" | "ragWorkspaceCode") {
            continue;
        }
        let schema_field = schema.get(field).ok_or(RagError::SearchFilterInvalid)?;
        let property_name =
            graph_property_name_for_logical(corpus, segment, field).unwrap_or(field);
        append_single_graph_filter(filters, property_name, schema_field, value)?;
    }
    Ok(())
}

pub(super) fn graph_filter(property_name: &str, value: &str) -> GraphPropertyFilter {
    GraphPropertyFilter {
        property_name: property_name.to_string(),
        operator: GraphFilterOperator::Equals,
        value: GraphPropertyValue::Text(value.to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphRagFilterType {
    String,
    Number,
    Boolean,
}

#[derive(Debug, Clone)]
struct GraphRagFilterField {
    field_type: GraphRagFilterType,
    operators: BTreeSet<String>,
}

fn parse_graph_filter_schema(value: &Value) -> BTreeMap<String, GraphRagFilterField> {
    value
        .get("fields")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .map(|(name, raw)| {
                    let field_type = match raw.get("type").and_then(Value::as_str) {
                        Some("integer" | "number") => GraphRagFilterType::Number,
                        Some("boolean") => GraphRagFilterType::Boolean,
                        _ => GraphRagFilterType::String,
                    };
                    let operators = raw
                        .get("operators")
                        .and_then(Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .map(ToOwned::to_owned)
                                .collect::<BTreeSet<_>>()
                        })
                        .unwrap_or_else(|| BTreeSet::from(["eq".to_string()]));
                    (
                        name.clone(),
                        GraphRagFilterField {
                            field_type,
                            operators,
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn append_single_graph_filter(
    filters: &mut Vec<GraphPropertyFilter>,
    property_name: &str,
    schema_field: &GraphRagFilterField,
    raw: &Value,
) -> Result<(), RagError> {
    let operator = raw.get("op").and_then(Value::as_str).unwrap_or("eq");
    if operator == "and" {
        let values = raw
            .get("value")
            .and_then(Value::as_array)
            .ok_or(RagError::SearchFilterInvalid)?;
        for value in values {
            append_single_graph_filter(filters, property_name, schema_field, value)?;
        }
        return Ok(());
    }
    if !schema_field.operators.contains(operator) {
        return Err(RagError::SearchFilterInvalid);
    }
    let graph_operator = graph_filter_operator(operator)?;
    let value = raw.get("value").unwrap_or(raw);
    filters.push(GraphPropertyFilter {
        property_name: property_name.to_string(),
        operator: graph_operator,
        value: graph_filter_value(schema_field.field_type, value)?,
    });
    Ok(())
}

fn graph_filter_operator(operator: &str) -> Result<GraphFilterOperator, RagError> {
    match operator {
        "eq" => Ok(GraphFilterOperator::Equals),
        "gt" => Ok(GraphFilterOperator::GreaterThan),
        "gte" => Ok(GraphFilterOperator::GreaterThanOrEquals),
        "lt" => Ok(GraphFilterOperator::LessThan),
        "lte" => Ok(GraphFilterOperator::LessThanOrEquals),
        "contains" => Ok(GraphFilterOperator::Contains),
        "in" => Err(RagError::SearchRouteUnavailable),
        _ => Err(RagError::SearchFilterInvalid),
    }
}

fn graph_filter_value(
    field_type: GraphRagFilterType,
    value: &Value,
) -> Result<GraphPropertyValue, RagError> {
    match field_type {
        GraphRagFilterType::String => value
            .as_str()
            .map(|value| GraphPropertyValue::Text(value.to_string()))
            .ok_or(RagError::SearchFilterInvalid),
        GraphRagFilterType::Number => {
            if let Some(value) = value.as_i64() {
                Ok(GraphPropertyValue::Integer(value))
            } else {
                value
                    .as_f64()
                    .map(GraphPropertyValue::Decimal)
                    .ok_or(RagError::SearchFilterInvalid)
            }
        }
        GraphRagFilterType::Boolean => value
            .as_bool()
            .map(GraphPropertyValue::Boolean)
            .ok_or(RagError::SearchFilterInvalid),
    }
}
