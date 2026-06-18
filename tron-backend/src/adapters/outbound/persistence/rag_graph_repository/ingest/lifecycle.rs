use crate::{
    application::{graph::PreparedGraphNodeWrite, rag::RagDeactivateSourceCommand},
    domain::{
        graph::{GraphNodeWrite, GraphPropertyValue},
        rag::{RagCorpusRecord, RagError},
    },
};

use super::super::{
    GraphBackedRagRepository,
    model::{graph_main_node_code_parts, legacy_graph_skill_node_code_parts, map_graph_error},
};

impl GraphBackedRagRepository {
    pub(in super::super) fn deactivate_graph_single_node(
        &self,
        corpus: &RagCorpusRecord,
        command: RagDeactivateSourceCommand,
    ) -> Result<(), RagError> {
        let scope = self.resolve_graph_scope(
            corpus,
            &command.workspace_code,
            &command.scope.scope_type,
            &command.scope.scope_code,
        )?;
        let node_codes = [
            graph_main_node_code_parts(
                &command.corpus_id,
                &command.source_code,
                &command.source_version,
                &command.scope.scope_type,
                &command.scope.scope_code,
            ),
            legacy_graph_skill_node_code_parts(
                &command.corpus_id,
                &command.source_code,
                &command.source_version,
                &command.scope.scope_type,
                &command.scope.scope_code,
            ),
        ];
        for node_code in node_codes {
            let Some(node) = self
                .graph_store
                .get_node(&scope, &node_code)
                .map_err(map_graph_error)?
            else {
                continue;
            };
            let mut properties = node.properties;
            properties.insert(
                "sourceStatus".into(),
                GraphPropertyValue::Text("DISABLED".into()),
            );
            properties.insert(
                "textUnitStatus".into(),
                GraphPropertyValue::Text("DISABLED".into()),
            );
            let write = GraphNodeWrite {
                node_code,
                labels: node.labels,
                properties,
                full_text: node.full_text,
                embedding_text: node.embedding_text,
                embedding_model_code: None,
            }
            .validate()
            .map_err(map_graph_error)?;
            self.graph_store
                .update_node(&scope, PreparedGraphNodeWrite::plain(write))
                .map_err(map_graph_error)?;
        }
        Ok(())
    }
}
