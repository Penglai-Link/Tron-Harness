use std::collections::BTreeMap;

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
    super::{
        GraphBackedRagRepository,
        model::{
            GraphRuntimeTarget, decode_embedding, fold_text_units, graph_main_node_code,
            graph_segment_node_code, json_string, legacy_graph_skill_node_code, map_graph_error,
            node_label, optional_graph_target_for_segment, push_text_prop, stable_graph_code,
        },
    },
    properties::{
        push_domain_field_properties, source_properties, text_unit_properties,
        tree_edge_properties, tree_node_properties,
    },
};

impl GraphBackedRagRepository {
    pub(in super::super) fn persist_graph_single_node(
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
        let embedding_text =
            semantic_embedding_text(&text, &semantic_model_code, &semantic_projection);
        let node = GraphNodeWrite {
            node_code: node_code.clone(),
            labels: vec![node_label(corpus)?],
            properties,
            full_text: Some(text.clone()),
            embedding_text,
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
        self.replace_contains_edges(&scope, &node_code)?;
        self.persist_related_graph(
            &scope,
            &node_code,
            command.related_graph.as_ref(),
            &command.graph_write_policy,
        )?;
        Ok(())
    }

    pub(in super::super) fn persist_graph_segmented(
        &self,
        corpus: &RagCorpusRecord,
        command: RagPersistIngestCommand,
    ) -> Result<(), RagError> {
        let source_target = optional_graph_target_for_segment(corpus, "source")?;
        let text_unit_target = optional_graph_target_for_segment(corpus, "textUnit")?;
        let fulltext_target = optional_graph_target_for_segment(corpus, "fulltext")?
            .or_else(|| text_unit_target.clone());
        let vector_target = optional_graph_target_for_segment(corpus, "vector")?;
        let tree_target = optional_graph_target_for_segment(corpus, "tree")?;
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
                let embedding_text = semantic_embedding_text(
                    &unit.search_text,
                    &semantic_model_code,
                    &semantic_projection,
                );
                let node = GraphNodeWrite {
                    node_code: node_code.clone(),
                    labels: vec![target.node_label.clone()],
                    properties,
                    full_text: Some(unit.search_text.clone()),
                    embedding_text,
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

        if let Some(target) = tree_target.as_ref() {
            for tree_node in &command.tree_nodes {
                let node_code = graph_segment_node_code(
                    "tree",
                    &command,
                    Some(tree_node.tree_node_code.as_str()),
                );
                let properties = tree_node_properties(corpus, &command, tree_node, &node_code)?;
                let semantic_projection = tree_node
                    .summary_embedding
                    .as_deref()
                    .map(decode_embedding)
                    .filter(|embedding| !embedding.is_empty())
                    .map(GraphSemanticProjection::from_values);
                let semantic_model_code = tree_node
                    .summary_embedding_model_code
                    .clone()
                    .filter(|value| !value.trim().is_empty());
                let embedding_text = semantic_embedding_text(
                    &tree_node.summary_text,
                    &semantic_model_code,
                    &semantic_projection,
                );
                let node = GraphNodeWrite {
                    node_code: node_code.clone(),
                    labels: vec![target.node_label.clone()],
                    properties,
                    full_text: Some(tree_node.summary_text.clone()),
                    embedding_text,
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
            self.replace_contains_edges(&scope, &node_code)?;
            self.persist_related_graph(
                &scope,
                &node_code,
                command.related_graph.as_ref(),
                &command.graph_write_policy,
            )?;
        }
        Ok(())
    }

    pub(in super::super) fn persist_graph_tree_nodes(
        &self,
        corpus: &RagCorpusRecord,
        command: RagPersistIngestCommand,
    ) -> Result<(), RagError> {
        let Some(target) = optional_graph_target_for_segment(corpus, "tree")? else {
            return Ok(());
        };
        if command.tree_nodes.is_empty() {
            return Ok(());
        }
        let scope = self.resolve_graph_scope_for_namespace(
            &target.namespace,
            &command.workspace_code,
            &command.scope.scope_type,
            &command.scope.scope_code,
        )?;

        for tree_node in &command.tree_nodes {
            let node_code =
                graph_segment_node_code("tree", &command, Some(&tree_node.tree_node_code));
            let semantic_projection = tree_node
                .summary_embedding
                .as_deref()
                .map(decode_embedding)
                .filter(|embedding| !embedding.is_empty())
                .map(GraphSemanticProjection::from_values);
            let semantic_model_code = tree_node
                .summary_embedding_model_code
                .clone()
                .filter(|value| !value.trim().is_empty());
            let node = GraphNodeWrite {
                node_code: node_code.clone(),
                labels: vec![target.node_label.clone()],
                properties: tree_node_properties(corpus, &command, tree_node, &node_code)?,
                full_text: Some(tree_node.summary_text.clone()),
                embedding_text: semantic_embedding_text(
                    &tree_node.summary_text,
                    &semantic_model_code,
                    &semantic_projection,
                ),
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
        }

        for tree_node in command
            .tree_nodes
            .iter()
            .filter(|node| node.parent_node_code != "0")
        {
            let source_node_code =
                graph_segment_node_code("tree", &command, Some(&tree_node.parent_node_code));
            let target_node_code =
                graph_segment_node_code("tree", &command, Some(&tree_node.tree_node_code));
            let edge_code = stable_graph_code(
                "RAG_TREE_EDGE",
                &[
                    &command.corpus_id,
                    &source_node_code,
                    "HAS_CHILD",
                    &target_node_code,
                ],
            );
            let edge = GraphEdgeWrite {
                edge_code: edge_code.clone(),
                source_node_code,
                target_node_code,
                relationship: "HAS_CHILD".into(),
                properties: tree_edge_properties(&command, tree_node),
                full_text: None,
                embedding_text: None,
                embedding_model_code: None,
            }
            .validate()
            .map_err(map_graph_error)?;
            let prepared = PreparedGraphEdgeWrite::plain(edge);
            if self
                .graph_store
                .get_edge(&scope, &edge_code)
                .map_err(map_graph_error)?
                .is_some()
            {
                self.graph_store
                    .update_edge(&scope, prepared)
                    .map_err(map_graph_error)?;
            } else {
                self.graph_store
                    .create_edge(&scope, prepared)
                    .map_err(map_graph_error)?;
            }
        }
        for tree_node in &command.tree_nodes {
            let node_code =
                graph_segment_node_code("tree", &command, Some(&tree_node.tree_node_code));
            self.replace_contains_edges(&scope, &node_code)?;
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

fn semantic_embedding_text(
    text: &str,
    semantic_model_code: &Option<String>,
    semantic_projection: &Option<GraphSemanticProjection>,
) -> Option<String> {
    if semantic_model_code.is_some() && semantic_projection.is_some() {
        Some(text.to_string())
    } else {
        None
    }
}
