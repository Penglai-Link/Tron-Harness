use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{
    application::{
        graph::{GraphSemanticProjection, PreparedGraphEdgeWrite, PreparedGraphNodeWrite},
        rag::RagPersistIngestCommand,
    },
    domain::{
        graph::{GraphEdgeWrite, GraphNodeWrite, GraphPropertyValue},
        rag::{RagCorpusRecord, RagError},
    },
};

use super::{
    GraphBackedRagRepository,
    model::{
        decode_embedding, fold_text_units, graph_skill_node_code, json_string, map_graph_error,
        node_label, normalize_related_code, optional_str, push_text_prop, required_str,
        stable_graph_code,
    },
};

impl GraphBackedRagRepository {
    pub(super) fn persist_graph_single_node(
        &self,
        corpus: &RagCorpusRecord,
        command: RagPersistIngestCommand,
    ) -> Result<(), RagError> {
        let scope = self.resolve_graph_scope(
            corpus,
            &command.workspace_code,
            &command.scope.scope_type,
            &command.scope.scope_code,
        )?;
        let node_code = graph_skill_node_code(&command);
        let text = fold_text_units(&command);
        let first_unit = command.text_units.first();
        let mut properties = BTreeMap::new();
        push_text_prop(&mut properties, "corpusId", &command.corpus_id);
        push_text_prop(&mut properties, "sourceCode", &command.source_code);
        push_text_prop(&mut properties, "sourceVersion", &command.source_version);
        push_text_prop(&mut properties, "sourceType", &command.source_type);
        push_text_prop(&mut properties, "sourceStatus", &command.source_status);
        push_text_prop(&mut properties, "scopeType", &command.scope.scope_type);
        push_text_prop(&mut properties, "scopeCode", &command.scope.scope_code);
        push_text_prop(&mut properties, "ragWorkspaceCode", &command.workspace_code);
        push_text_prop(&mut properties, "targetCode", &node_code);
        push_text_prop(&mut properties, "textUnitCode", &node_code);
        push_text_prop(
            &mut properties,
            "title",
            command.title.as_deref().unwrap_or(""),
        );
        push_text_prop(&mut properties, "text", &text);
        push_text_prop(&mut properties, "searchText", &text);
        push_text_prop(&mut properties, "headingPath", "");
        push_text_prop(&mut properties, "textUnitStatus", "ACTIVE");
        push_text_prop(&mut properties, "fulltextStatus", "INDEXED");
        push_text_prop(
            &mut properties,
            "embeddingStatus",
            first_unit
                .map(|unit| unit.embedding_status.as_str())
                .unwrap_or("FAILED"),
        );
        push_text_prop(
            &mut properties,
            "embeddingModelCode",
            first_unit
                .and_then(|unit| unit.embedding_model_code.as_deref())
                .unwrap_or(""),
        );
        push_text_prop(
            &mut properties,
            "metadataJson",
            &json_string(&command.metadata)?,
        );
        push_text_prop(
            &mut properties,
            "sourceMetadataJson",
            &json_string(&command.metadata)?,
        );
        push_domain_field_properties(corpus, &command, &mut properties)?;
        properties.insert(
            "tokenCount".into(),
            GraphPropertyValue::Integer(
                command.text_units.iter().map(|unit| unit.token_count).sum(),
            ),
        );
        if let Some(embedding) = first_unit.and_then(|unit| unit.embedding.as_deref()) {
            push_text_prop(
                &mut properties,
                "embeddingJson",
                &json_string(&decode_embedding(embedding))?,
            );
        }

        let semantic_projection = first_unit
            .and_then(|unit| unit.embedding.as_deref())
            .map(decode_embedding)
            .filter(|embedding| !embedding.is_empty())
            .map(GraphSemanticProjection::from_values);
        let semantic_model_code = first_unit
            .and_then(|unit| unit.embedding_model_code.clone())
            .filter(|value| !value.trim().is_empty());
        let node = GraphNodeWrite {
            node_code: node_code.clone(),
            labels: vec![node_label(corpus)?],
            properties,
            full_text: Some(text.clone()),
            embedding_text: Some(text),
            embedding_model_code: semantic_model_code.clone(),
        }
        .validate()
        .map_err(map_graph_error)?;
        let write = PreparedGraphNodeWrite {
            command: node,
            semantic_model_code,
            semantic_projection,
            semantic_slices: Vec::new(),
        };
        if self
            .graph_store
            .get_node(&scope, &node_code)
            .map_err(map_graph_error)?
            .is_some()
        {
            self.graph_store
                .update_node(&scope, write)
                .map_err(map_graph_error)?;
        } else {
            self.graph_store
                .create_node(&scope, write)
                .map_err(map_graph_error)?;
        }
        self.persist_related_graph(
            &scope,
            &node_code,
            command.related_graph.as_ref(),
            &command.graph_write_policy,
        )?;
        Ok(())
    }

    fn persist_related_graph(
        &self,
        scope: &crate::domain::graph::ResolvedGraphScope,
        skill_node_code: &str,
        related_graph: Option<&Value>,
        graph_write_policy: &Value,
    ) -> Result<(), RagError> {
        let Some(related_graph) = related_graph else {
            return Ok(());
        };
        let policy = related_policy(graph_write_policy)?;
        let nodes = related_graph
            .get("nodes")
            .and_then(Value::as_array)
            .ok_or(RagError::IngestRequestInvalid)?;
        let edges = related_graph
            .get("edges")
            .and_then(Value::as_array)
            .ok_or(RagError::IngestRequestInvalid)?;
        if nodes.len() > policy.max_nodes || edges.len() > policy.max_edges {
            return Err(RagError::IngestRequestInvalid);
        }

        let mut refs = BTreeMap::from([("$skill".to_string(), skill_node_code.to_string())]);
        for node in nodes {
            let reference = required_str(node, "ref")?;
            let kind = required_str(node, "kind")?;
            let label = required_str(node, "label")?;
            if !valid_business_code(kind)
                || policy
                    .allowed_kinds
                    .as_ref()
                    .is_some_and(|allowed| !allowed.contains(kind))
            {
                return Err(RagError::IngestRequestInvalid);
            }
            let code = normalize_related_code(required_str(node, "code")?)?;
            let mut properties = custom_related_properties(node)?;
            push_text_prop(&mut properties, "relatedKind", kind);
            push_text_prop(&mut properties, "relatedRef", reference);
            push_text_prop(
                &mut properties,
                "title",
                optional_str(node, "title").unwrap_or(code.as_str()),
            );
            push_text_prop(
                &mut properties,
                "text",
                optional_str(node, "text").unwrap_or(""),
            );
            let write = GraphNodeWrite {
                node_code: code.clone(),
                labels: vec![label.to_string()],
                properties,
                full_text: optional_str(node, "text").map(ToOwned::to_owned),
                embedding_text: None,
                embedding_model_code: None,
            }
            .validate()
            .map_err(map_graph_error)?;
            let prepared = PreparedGraphNodeWrite::plain(write);
            if self
                .graph_store
                .get_node(scope, &code)
                .map_err(map_graph_error)?
                .is_some()
            {
                self.graph_store
                    .update_node(scope, prepared)
                    .map_err(map_graph_error)?;
            } else {
                self.graph_store
                    .create_node(scope, prepared)
                    .map_err(map_graph_error)?;
            }
            refs.insert(reference.to_string(), code);
        }

        for edge in edges {
            let from = refs
                .get(required_str(edge, "from")?)
                .ok_or(RagError::IngestRequestInvalid)?
                .clone();
            let to = refs
                .get(required_str(edge, "to")?)
                .ok_or(RagError::IngestRequestInvalid)?
                .clone();
            let relationship = required_str(edge, "relationship")?;
            if !valid_business_code(relationship)
                || policy
                    .allowed_relationships
                    .as_ref()
                    .is_some_and(|allowed| !allowed.contains(relationship))
            {
                return Err(RagError::IngestRequestInvalid);
            }
            let edge_code = stable_graph_code("RAG_SKILL_EDGE", &[&from, relationship, &to]);
            let properties = custom_edge_properties(edge)?;
            let write = GraphEdgeWrite {
                edge_code: edge_code.clone(),
                source_node_code: from,
                target_node_code: to,
                relationship: relationship.to_string(),
                properties,
                full_text: None,
                embedding_text: None,
                embedding_model_code: None,
            }
            .validate()
            .map_err(map_graph_error)?;
            let prepared = PreparedGraphEdgeWrite::plain(write);
            if self
                .graph_store
                .get_edge(scope, &edge_code)
                .map_err(map_graph_error)?
                .is_some()
            {
                self.graph_store
                    .update_edge(scope, prepared)
                    .map_err(map_graph_error)?;
            } else {
                self.graph_store
                    .create_edge(scope, prepared)
                    .map_err(map_graph_error)?;
            }
        }
        Ok(())
    }
}

struct RelatedPolicy {
    allowed_kinds: Option<BTreeSet<String>>,
    allowed_relationships: Option<BTreeSet<String>>,
    max_nodes: usize,
    max_edges: usize,
}

fn related_policy(policy: &Value) -> Result<RelatedPolicy, RagError> {
    Ok(RelatedPolicy {
        allowed_kinds: optional_code_set(policy, "allowedNodeKinds")?,
        allowed_relationships: optional_code_set(policy, "allowedRelationships")?,
        max_nodes: optional_positive_usize(policy, "maxRelatedNodes").unwrap_or(50),
        max_edges: optional_positive_usize(policy, "maxRelatedEdges").unwrap_or(100),
    })
}

fn optional_code_set(policy: &Value, field: &str) -> Result<Option<BTreeSet<String>>, RagError> {
    let Some(values) = policy.get(field) else {
        return Ok(None);
    };
    let values = values.as_array().ok_or(RagError::StorageBindingInvalid)?;
    let mut codes = BTreeSet::new();
    for value in values {
        let code = value
            .as_str()
            .map(str::trim)
            .filter(|value| valid_business_code(value))
            .ok_or(RagError::StorageBindingInvalid)?;
        codes.insert(code.to_string());
    }
    Ok((!codes.is_empty()).then_some(codes))
}

fn optional_positive_usize(policy: &Value, field: &str) -> Option<usize> {
    policy
        .get(field)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
}

fn valid_business_code(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn push_domain_field_properties(
    corpus: &RagCorpusRecord,
    command: &RagPersistIngestCommand,
    properties: &mut BTreeMap<String, GraphPropertyValue>,
) -> Result<(), RagError> {
    for (logical_name, value) in &command.domain_fields {
        let property_name = corpus
            .field_mapping
            .get(logical_name)
            .ok_or(RagError::StorageBindingInvalid)?;
        if is_reserved_main_node_property(property_name) {
            return Err(RagError::StorageBindingInvalid);
        }
        properties.insert(property_name.clone(), json_graph_property_value(value)?);
    }
    Ok(())
}

fn is_reserved_main_node_property(name: &str) -> bool {
    matches!(
        name,
        "corpusId"
            | "sourceCode"
            | "sourceVersion"
            | "sourceType"
            | "sourceStatus"
            | "scopeType"
            | "scopeCode"
            | "ragWorkspaceCode"
            | "targetCode"
            | "textUnitCode"
            | "title"
            | "text"
            | "searchText"
            | "headingPath"
            | "textUnitStatus"
            | "fulltextStatus"
            | "embeddingStatus"
            | "embeddingModelCode"
            | "metadataJson"
            | "sourceMetadataJson"
            | "tokenCount"
            | "embeddingJson"
    )
}

fn custom_related_properties(
    node: &Value,
) -> Result<BTreeMap<String, GraphPropertyValue>, RagError> {
    let mut properties = BTreeMap::new();
    let Some(raw_properties) = node.get("properties") else {
        return Ok(properties);
    };
    let property_map = raw_properties
        .as_object()
        .ok_or(RagError::IngestRequestInvalid)?;
    for (name, value) in property_map {
        let name = name.trim();
        if name.is_empty() || is_reserved_related_property(name) {
            return Err(RagError::IngestRequestInvalid);
        }
        properties.insert(name.to_string(), json_graph_property_value(value)?);
    }
    Ok(properties)
}

fn custom_edge_properties(edge: &Value) -> Result<BTreeMap<String, GraphPropertyValue>, RagError> {
    let mut properties = BTreeMap::new();
    let Some(raw_properties) = edge.get("properties") else {
        return Ok(properties);
    };
    let property_map = raw_properties
        .as_object()
        .ok_or(RagError::IngestRequestInvalid)?;
    for (name, value) in property_map {
        let name = name.trim();
        if name.is_empty() || is_reserved_edge_property(name) {
            return Err(RagError::IngestRequestInvalid);
        }
        properties.insert(name.to_string(), json_graph_property_value(value)?);
    }
    Ok(properties)
}

fn is_reserved_related_property(name: &str) -> bool {
    matches!(name, "relatedKind" | "relatedRef" | "title" | "text")
}

fn is_reserved_edge_property(name: &str) -> bool {
    matches!(
        name,
        "edgeCode" | "sourceNodeCode" | "targetNodeCode" | "relationship"
    )
}

fn json_graph_property_value(value: &Value) -> Result<GraphPropertyValue, RagError> {
    match value {
        Value::String(value) => Ok(GraphPropertyValue::Text(value.clone())),
        Value::Number(value) => value
            .as_i64()
            .map(GraphPropertyValue::Integer)
            .or_else(|| value.as_f64().map(GraphPropertyValue::Decimal))
            .ok_or(RagError::IngestRequestInvalid),
        Value::Bool(value) => Ok(GraphPropertyValue::Boolean(*value)),
        _ => Err(RagError::IngestRequestInvalid),
    }
}
