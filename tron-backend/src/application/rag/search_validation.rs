use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::rag::{RagCorpusRecord, RagError};

use super::{
    CompiledRetrievalProfile, RagIngestAccessContext, RagSearchRequest, RagSearchScope,
    merge_search_filters,
};

pub(super) fn validate_request(request: &RagSearchRequest) -> Result<(), RagError> {
    for value in [
        request.profile_id.as_str(),
        request.workspace_code.as_str(),
        request.query.as_str(),
        request.access_context.user_code.as_str(),
        request.access_context.agent_code.as_str(),
        request.access_context.workspace_code.as_str(),
        request.context.strategy.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(RagError::SearchRequestInvalid);
        }
    }
    if request.workspace_code != request.access_context.workspace_code || request.limit == 0 {
        return Err(RagError::SearchRequestInvalid);
    }
    if request
        .context
        .return_fields
        .iter()
        .any(|field| field.trim().is_empty())
    {
        return Err(RagError::SearchRequestInvalid);
    }
    if request
        .context
        .graph_expansion
        .relationships
        .iter()
        .any(|relationship| !valid_relationship_code(relationship))
    {
        return Err(RagError::SearchRequestInvalid);
    }
    Ok(())
}

pub(super) fn validate_return_fields(
    fields: &[String],
    corpora: &BTreeMap<String, RagCorpusRecord>,
) -> Result<(), RagError> {
    if fields.is_empty() {
        return Ok(());
    }
    let available = corpora
        .values()
        .flat_map(corpus_return_field_names)
        .collect::<BTreeSet<_>>();
    for field in fields {
        if is_reserved_return_field(field) || !available.contains(field) {
            return Err(RagError::SearchRequestInvalid);
        }
    }
    Ok(())
}

pub(super) fn filters_and_version(
    corpus: &RagCorpusRecord,
    profile_filters: &Value,
    request: &RagSearchRequest,
    compiled: &CompiledRetrievalProfile,
) -> Result<(Value, String), RagError> {
    let merged = merge_search_filters(corpus, profile_filters, &request.filters)?;
    let source_version = merged
        .get("sourceVersion")
        .map(source_version_filter_value)
        .transpose()?
        .unwrap_or_else(|| compiled.required_source_version.clone());
    Ok((merged, source_version))
}

pub(super) fn search_scopes(access_context: &RagIngestAccessContext) -> Vec<RagSearchScope> {
    vec![
        RagSearchScope {
            scope_type: "USER".into(),
            scope_code: access_context.user_code.clone(),
        },
        RagSearchScope {
            scope_type: "AGENT".into(),
            scope_code: access_context.agent_code.clone(),
        },
        RagSearchScope {
            scope_type: "SPACE".into(),
            scope_code: access_context.workspace_code.clone(),
        },
    ]
}

fn corpus_return_field_names(corpus: &RagCorpusRecord) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.extend(corpus.field_mapping.keys().cloned());
    names.extend(corpus.source_field_mapping.keys().cloned());
    names.extend(corpus.text_unit_field_mapping.keys().cloned());
    names.extend(
        corpus
            .filter_schema
            .get("fields")
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|fields| fields.keys().cloned()),
    );
    names
        .into_iter()
        .filter(|field| !is_reserved_return_field(field))
        .collect()
}

fn is_reserved_return_field(field: &str) -> bool {
    matches!(
        field,
        "corpusId"
            | "sourceCode"
            | "sourceVersion"
            | "title"
            | "score"
            | "text"
            | "metadata"
            | "targetType"
            | "targetCode"
            | "routeIds"
            | "scopeType"
            | "scopeCode"
            | "preview"
    )
}

fn source_version_filter_value(value: &Value) -> Result<String, RagError> {
    let raw = value.get("value").ok_or(RagError::SearchFilterInvalid)?;
    if let Some(version) = raw.as_str() {
        return Ok(version.to_string());
    }
    let versions = raw.as_array().ok_or(RagError::SearchFilterInvalid)?;
    match versions.as_slice() {
        [version] => version
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or(RagError::SearchFilterInvalid),
        _ => Err(RagError::SearchFilterInvalid),
    }
}

fn valid_relationship_code(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}
