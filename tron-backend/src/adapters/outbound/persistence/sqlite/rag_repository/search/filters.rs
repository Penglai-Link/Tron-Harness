use rusqlite::types::Value as SqlValue;
use serde_json::Value;

use crate::domain::rag::RagError;

use super::super::sql_plan::{SqliteRagCorpusPlan, SqliteRagValueType, quote_identifier};

pub(super) fn append_filter_clauses(
    plan: &SqliteRagCorpusPlan,
    filters: &Value,
    clauses: &mut Vec<String>,
    params: &mut Vec<SqlValue>,
) -> Result<(), RagError> {
    let object = filters.as_object().ok_or(RagError::SearchFilterInvalid)?;
    for (field, filter) in object {
        let field_plan = plan
            .filter_columns
            .get(field)
            .ok_or(RagError::SearchFilterInvalid)?;
        let column = format!(
            "{}.{}",
            field_plan.alias.as_str(),
            quote_identifier(&field_plan.column)?
        );
        let operator = filter
            .get("op")
            .and_then(Value::as_str)
            .ok_or(RagError::SearchFilterInvalid)?;
        let raw_value = filter.get("value").ok_or(RagError::SearchFilterInvalid)?;
        match operator {
            "and" => {
                for item in raw_value.as_array().ok_or(RagError::SearchFilterInvalid)? {
                    append_single_filter_clause(
                        &column,
                        field_plan.value_type,
                        item,
                        clauses,
                        params,
                    )?;
                }
            }
            _ => {
                append_single_filter_clause(
                    &column,
                    field_plan.value_type,
                    filter,
                    clauses,
                    params,
                )?;
            }
        }
    }
    Ok(())
}

fn append_single_filter_clause(
    column: &str,
    value_type: SqliteRagValueType,
    filter: &Value,
    clauses: &mut Vec<String>,
    params: &mut Vec<SqlValue>,
) -> Result<(), RagError> {
    let operator = filter
        .get("op")
        .and_then(Value::as_str)
        .ok_or(RagError::SearchFilterInvalid)?;
    let raw_value = filter.get("value").ok_or(RagError::SearchFilterInvalid)?;
    match operator {
        "eq" => {
            clauses.push(format!("{column} = ?"));
            params.push(sql_value(raw_value, value_type)?);
        }
        "in" => {
            let values = raw_value.as_array().ok_or(RagError::SearchFilterInvalid)?;
            if values.is_empty() {
                return Err(RagError::SearchFilterInvalid);
            }
            clauses.push(format!(
                "{column} IN ({})",
                values.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
            ));
            for value in values {
                params.push(sql_value(value, value_type)?);
            }
        }
        "gt" | "gte" | "lt" | "lte" => {
            clauses.push(format!("{column} {} ?", sql_comparator(operator)?));
            params.push(sql_value(raw_value, value_type)?);
        }
        _ => return Err(RagError::SearchFilterInvalid),
    }
    Ok(())
}

fn sql_comparator(operator: &str) -> Result<&'static str, RagError> {
    match operator {
        "gt" => Ok(">"),
        "gte" => Ok(">="),
        "lt" => Ok("<"),
        "lte" => Ok("<="),
        _ => Err(RagError::SearchFilterInvalid),
    }
}

fn sql_value(value: &Value, value_type: SqliteRagValueType) -> Result<SqlValue, RagError> {
    match value_type {
        SqliteRagValueType::String => value
            .as_str()
            .map(|value| SqlValue::Text(value.to_string()))
            .ok_or(RagError::SearchFilterInvalid),
        SqliteRagValueType::Integer => value
            .as_i64()
            .map(SqlValue::Integer)
            .ok_or(RagError::SearchFilterInvalid),
        SqliteRagValueType::Boolean => value
            .as_bool()
            .map(|value| SqlValue::Integer(i64::from(value)))
            .ok_or(RagError::SearchFilterInvalid),
    }
}
