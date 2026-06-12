use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{
    application::{
        graph::{GraphSemanticProjection, PreparedGraphEdgeWrite, PreparedGraphNodeWrite},
        rag::{RagPersistIngestCommand, RagTextUnitWrite},
    },
    domain::{
        graph::{GraphEdgeWrite, GraphNodeWrite, GraphPropertyValue},
        rag::{RagCorpusRecord, RagError},
    },
};

use super::{
    GraphBackedRagRepository,
    model::{
        GraphRuntimeTarget, decode_embedding, fold_text_units, graph_main_node_code,
        graph_property_name_for_logical, graph_segment_node_code, json_string,
        legacy_graph_skill_node_code, map_graph_error, node_label, normalize_related_code,
        optional_graph_target_for_segment, optional_str, push_text_prop, required_str,
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
        let node_code = self.graph_main_node_code_for_write(&scope, &command)?;
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
        push_domain_field_properties(corpus, "textUnit", &command, &mut properties)?;
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

    pub(super) fn persist_graph_segmented(
        &self,
        corpus: &RagCorpusRecord,
        command: RagPersistIngestCommand,
    ) -> Result<(), RagError> {
        let source_target = optional_graph_target_for_segment(corpus, "source")?;
        let text_unit_target = optional_graph_target_for_segment(corpus, "textUnit")?;
        let fulltext_target = optional_graph_target_for_segment(corpus, "fulltext")?
            .or_else(|| text_unit_target.clone());
        let vector_target = optional_graph_target_for_segment(corpus, "vector")?;
        let mut related_anchors = Vec::new();

        if let Some(target) = source_target.as_ref() {
            let node_code = graph_segment_node_code("source", &command, None);
            let properties = source_properties(corpus, &command, &node_code)?;
            let node = GraphNodeWrite {
                node_code: node_code.clone(),
                labels: vec![target.node_label.clone()],
                properties,
                full_text: command.title.clone(),
                embedding_text: None,
                embedding_model_code: None,
            }
            .validate()
            .map_err(map_graph_error)?;
            self.upsert_segment_node(target, &command, PreparedGraphNodeWrite::plain(node))?;
            related_anchors.push((target.clone(), node_code));
        }

        for unit in &command.text_units {
            let text_unit_node_code =
                graph_segment_node_code("textUnit", &command, Some(unit.text_unit_code.as_str()));
            if let Some(target) = text_unit_target.as_ref() {
                let node_code = text_unit_node_code.clone();
                let properties =
                    text_unit_properties(corpus, &command, unit, &node_code, &text_unit_node_code)?;
                let node = GraphNodeWrite {
                    node_code: node_code.clone(),
                    labels: vec![target.node_label.clone()],
                    properties,
                    full_text: Some(unit.text.clone()),
                    embedding_text: None,
                    embedding_model_code: None,
                }
                .validate()
                .map_err(map_graph_error)?;
                self.upsert_segment_node(target, &command, PreparedGraphNodeWrite::plain(node))?;
                related_anchors.push((target.clone(), node_code));
            }

            if let Some(target) = fulltext_target.as_ref() {
                let node_code =
                    graph_segment_node_code("fulltext", &command, Some(&unit.text_unit_code));
                let properties =
                    text_unit_properties(corpus, &command, unit, &node_code, &text_unit_node_code)?;
                let node = GraphNodeWrite {
                    node_code: node_code.clone(),
                    labels: vec![target.node_label.clone()],
                    properties,
                    full_text: Some(unit.search_text.clone()),
                    embedding_text: None,
                    embedding_model_code: None,
                }
                .validate()
                .map_err(map_graph_error)?;
                self.upsert_segment_node(target, &command, PreparedGraphNodeWrite::plain(node))?;
                related_anchors.push((target.clone(), node_code));
            }

            if let Some(target) = vector_target.as_ref() {
                let node_code =
                    graph_segment_node_code("vector", &command, Some(&unit.text_unit_code));
                let properties =
                    text_unit_properties(corpus, &command, unit, &node_code, &text_unit_node_code)?;
                let semantic_projection = unit
                    .embedding
                    .as_deref()
                    .map(decode_embedding)
                    .filter(|embedding| !embedding.is_empty())
                    .map(GraphSemanticProjection::from_values);
                let semantic_model_code = unit
                    .embedding_model_code
                    .clone()
                    .filter(|value| !value.trim().is_empty());
                let node = GraphNodeWrite {
                    node_code: node_code.clone(),
                    labels: vec![target.node_label.clone()],
                    properties,
                    full_text: Some(unit.search_text.clone()),
                    embedding_text: Some(unit.search_text.clone()),
                    embedding_model_code: semantic_model_code.clone(),
                }
                .validate()
                .map_err(map_graph_error)?;
                self.upsert_segment_node(
                    target,
                    &command,
                    PreparedGraphNodeWrite {
                        command: node,
                        semantic_model_code,
                        semantic_projection,
                        semantic_slices: Vec::new(),
                    },
                )?;
                related_anchors.push((target.clone(), node_code));
            }
        }

        for (target, node_code) in related_anchors {
            let scope = self.resolve_graph_scope_for_namespace(
                &target.namespace,
                &command.workspace_code,
                &command.scope.scope_type,
                &command.scope.scope_code,
            )?;
            self.persist_related_graph(
                &scope,
                &node_code,
                command.related_graph.as_ref(),
                &command.graph_write_policy,
            )?;
        }
        Ok(())
    }

    fn upsert_segment_node(
        &self,
        target: &GraphRuntimeTarget,
        command: &RagPersistIngestCommand,
        write: PreparedGraphNodeWrite,
    ) -> Result<(), RagError> {
        let scope = self.resolve_graph_scope_for_namespace(
            &target.namespace,
            &command.workspace_code,
            &command.scope.scope_type,
            &command.scope.scope_code,
        )?;
        let node_code = write.command.node_code.clone();
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
        Ok(())
    }

    fn persist_related_graph(
        &self,
        scope: &crate::domain::graph::ResolvedGraphScope,
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
        let edges = related_graph
            .get("edges")
            .and_then(Value::as_array)
            .ok_or(RagError::IngestRequestInvalid)?;
        if nodes.len() > policy.max_nodes || edges.len() > policy.max_edges {
            return Err(RagError::IngestRequestInvalid);
        }

        let mut refs = BTreeMap::from([
            ("$target".to_string(), target_node_code.to_string()),
            ("$ragNode".to_string(), target_node_code.to_string()),
            ("$source".to_string(), target_node_code.to_string()),
            ("$skill".to_string(), target_node_code.to_string()),
        ]);
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
            let edge_code = stable_graph_code("RAG_RELATED_EDGE", &[&from, relationship, &to]);
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

    fn graph_main_node_code_for_write(
        &self,
        scope: &crate::domain::graph::ResolvedGraphScope,
        command: &RagPersistIngestCommand,
    ) -> Result<String, RagError> {
        let node_code = graph_main_node_code(command);
        if self
            .graph_store
            .get_node(scope, &node_code)
            .map_err(map_graph_error)?
            .is_some()
        {
            return Ok(node_code);
        }
        let legacy_node_code = legacy_graph_skill_node_code(command);
        if legacy_node_code == node_code {
            return Ok(node_code);
        }
        if self
            .graph_store
            .get_node(scope, &legacy_node_code)
            .map_err(map_graph_error)?
            .is_some()
        {
            return Ok(legacy_node_code);
        }
        Ok(node_code)
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

