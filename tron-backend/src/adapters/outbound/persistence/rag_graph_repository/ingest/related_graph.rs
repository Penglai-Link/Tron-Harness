use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{
    application::graph::{PreparedGraphEdgeWrite, PreparedGraphNodeWrite},
    domain::{
        graph::{
            GraphEdgeWrite, GraphNodeWrite, GraphQueryLimit, GraphTraversalClause,
            GraphTraversalDirection, GraphTraverseQuery, ResolvedGraphScope,
        },
        rag::RagError,
    },
};

use super::{
    super::{
        GraphBackedRagRepository,
        model::{
            map_graph_error, normalize_related_code, optional_str, push_text_prop, required_str,
            stable_graph_code,
        },
    },
    properties::{custom_edge_properties, custom_related_properties},
};

impl GraphBackedRagRepository {
    pub(super) fn replace_contains_edges(
        &self,
        scope: &ResolvedGraphScope,
        target_node_code: &str,
    ) -> Result<(), RagError> {
        let query = GraphTraverseQuery {
            start_node_codes: vec![target_node_code.to_string()],
            clauses: vec![GraphTraversalClause {
                direction: GraphTraversalDirection::Outgoing,
                relationships: vec!["CONTAINS".into()],
                min_depth: 1,
                max_depth: 1,
            }],
            filters: Vec::new(),
            limit: GraphQueryLimit::new(500).map_err(map_graph_error)?,
        }
        .validate()
        .map_err(map_graph_error)?;
        let paths = self
            .graph_store
            .traverse(scope, query)
            .map_err(map_graph_error)?;
        for edge_code in paths
            .into_iter()
            .flat_map(|path| path.edges)
            .filter(|edge| {
                edge.source_node_code == target_node_code && edge.relationship == "CONTAINS"
            })
            .map(|edge| edge.edge_code)
        {
            self.graph_store
                .delete_edge(scope, &edge_code)
                .map_err(map_graph_error)?;
        }
        Ok(())
    }

    pub(super) fn persist_related_graph(
        &self,
        scope: &ResolvedGraphScope,
        target_node_code: &str,
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
        let empty_existing_nodes = Vec::new();
        let existing_nodes = related_graph
            .get("existingNodes")
            .map(|value| value.as_array().ok_or(RagError::IngestRequestInvalid))
            .transpose()?
            .unwrap_or(&empty_existing_nodes);
        let edges = related_graph
            .get("edges")
            .and_then(Value::as_array)
            .ok_or(RagError::IngestRequestInvalid)?;
        if nodes.len() + existing_nodes.len() > policy.max_nodes || edges.len() > policy.max_edges {
            return Err(RagError::IngestRequestInvalid);
        }

        let mut refs = BTreeMap::from([
            ("$target".to_string(), target_node_code.to_string()),
            ("$ragNode".to_string(), target_node_code.to_string()),
            ("$source".to_string(), target_node_code.to_string()),
            ("$skill".to_string(), target_node_code.to_string()),
        ]);
        for node in existing_nodes {
            let reference = required_str(node, "ref")?;
            let code = normalize_related_code(required_str(node, "code")?)?;
            if self
                .graph_store
                .get_node(scope, &code)
                .map_err(map_graph_error)?
                .is_none()
            {
                return Err(RagError::IngestRequestInvalid);
            }
            insert_related_ref(&mut refs, reference, code)?;
        }

        let mut prepared_nodes = Vec::with_capacity(nodes.len());
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
            insert_related_ref(&mut refs, reference, code.clone())?;
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
            prepared_nodes.push(PreparedGraphNodeWrite::plain(write));
        }

        let mut prepared_edges = Vec::with_capacity(edges.len());
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
            let edge_code = stable_graph_code("RAG_RELATED_EDGE", &[&from, relationship, &to]);
            let properties = custom_edge_properties(edge)?;
            let write = GraphEdgeWrite {
                edge_code,
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
            prepared_edges.push(PreparedGraphEdgeWrite::plain(write));
        }

        for prepared in prepared_nodes {
            let code = prepared.command.node_code.clone();
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
        }

        for prepared in prepared_edges {
            let edge_code = prepared.command.edge_code.clone();
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

fn insert_related_ref(
    refs: &mut BTreeMap<String, String>,
    reference: &str,
    code: String,
) -> Result<(), RagError> {
    if reference.trim().is_empty() || refs.contains_key(reference) {
        return Err(RagError::IngestRequestInvalid);
    }
    refs.insert(reference.to_string(), code);
    Ok(())
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
