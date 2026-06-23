use std::collections::{BTreeMap, BTreeSet};

use crate::domain::rag::{RagCorpusRecord, RagError, RagStorageBackend, RagStorageSegmentTarget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RagStorageWritePlan {
    pub(super) operations: Vec<RagStorageWriteOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RagStorageWriteOperation {
    pub(super) backend: RagStorageBackend,
    pub(super) target_id: String,
    pub(super) target: RagStorageSegmentTarget,
    pub(super) segments: BTreeSet<String>,
}

impl RagStorageWritePlan {
    pub(super) fn compile(corpus: &RagCorpusRecord) -> Result<Self, RagError> {
        let mut by_target = BTreeMap::<String, RagStorageWriteOperation>::new();
        for segment in ["source", "textUnit", "fulltext", "vector", "tree"] {
            let Some((target_id, target)) = resolve_segment_target(corpus, segment)? else {
                continue;
            };
            by_target
                .entry(target_id.clone())
                .and_modify(|operation| {
                    operation.segments.insert(segment.to_string());
                })
                .or_insert_with(|| RagStorageWriteOperation {
                    backend: target.backend,
                    target_id,
                    target,
                    segments: BTreeSet::from([segment.to_string()]),
                });
        }
        Ok(Self {
            operations: by_target.into_values().collect(),
        })
    }
}

fn resolve_segment_target(
    corpus: &RagCorpusRecord,
    segment: &str,
) -> Result<Option<(String, RagStorageSegmentTarget)>, RagError> {
    let target_id = corpus
        .storage_binding
        .segment_bindings
        .get(segment)
        .or_else(|| {
            if segment == "fulltext" {
                corpus.storage_binding.segment_bindings.get("textUnit")
            } else {
                None
            }
        });
    let Some(target_id) = target_id else {
        return Ok(None);
    };
    let Some(target) = corpus.storage_binding.segment_targets.get(target_id) else {
        return Err(RagError::StorageBindingInvalid);
    };
    Ok(Some((target_id.clone(), target.clone())))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use crate::domain::rag::{
        RagCorpusRecord, RagStatus, RagStorageBackend, RagStorageBinding, RagStorageSegmentTarget,
    };

    use super::RagStorageWritePlan;

    #[test]
    fn write_plan_groups_segments_by_concrete_storage_target() {
        let plan = RagStorageWritePlan::compile(&corpus_with_split_targets()).unwrap();

        let operations = plan
            .operations
            .iter()
            .map(|operation| {
                (
                    operation.backend,
                    operation.target_id.as_str(),
                    operation
                        .target
                        .table
                        .as_deref()
                        .or(operation.target.namespace.as_deref())
                        .unwrap_or(""),
                    operation.segments.iter().cloned().collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            operations,
            vec![
                (
                    RagStorageBackend::Sqlite,
                    "source-store",
                    "rag_generic_sources",
                    vec!["source".to_string()]
                ),
                (
                    RagStorageBackend::Graph,
                    "text-unit-graph",
                    "rag_text_units",
                    vec!["fulltext".to_string(), "textUnit".to_string()]
                ),
                (
                    RagStorageBackend::Graph,
                    "vector-graph",
                    "rag_vectors",
                    vec!["vector".to_string()]
                ),
            ]
        );
    }

    #[test]
    fn write_plan_falls_fulltext_back_to_text_unit_target() {
        let mut corpus = corpus_with_split_targets();
        corpus.storage_binding.segment_bindings.remove("fulltext");

        let plan = RagStorageWritePlan::compile(&corpus).unwrap();
        let text_unit_operation = plan
            .operations
            .iter()
            .find(|operation| operation.target_id == "text-unit-graph")
            .unwrap();

        assert_eq!(
            text_unit_operation
                .segments
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["fulltext".to_string(), "textUnit".to_string()]
        );
    }

    fn corpus_with_split_targets() -> RagCorpusRecord {
        RagCorpusRecord {
            corpus_id: "generic-write-plan".into(),
            display_name: "Generic Write Plan".into(),
            status: RagStatus::Enabled,
            description: None,
            capabilities: json!({ "fulltext": true, "vector": true, "graph": true }),
            filter_schema: json!({ "fields": {} }),
            field_mapping: BTreeMap::new(),
            source_field_mapping: BTreeMap::new(),
            text_unit_field_mapping: BTreeMap::new(),
            storage_binding: RagStorageBinding {
                segment_bindings: BTreeMap::from([
                    ("source".into(), "source-store".into()),
                    ("textUnit".into(), "text-unit-graph".into()),
                    ("fulltext".into(), "text-unit-graph".into()),
                    ("vector".into(), "vector-graph".into()),
                ]),
                segment_targets: BTreeMap::from([
                    (
                        "source-store".into(),
                        RagStorageSegmentTarget {
                            id: "source-store".into(),
                            backend: RagStorageBackend::Sqlite,
                            layout: "TABLE".into(),
                            table: Some("rag_generic_sources".into()),
                            namespace: None,
                            node_label: None,
                            code_strategy: None,
                        },
                    ),
                    (
                        "text-unit-graph".into(),
                        RagStorageSegmentTarget {
                            id: "text-unit-graph".into(),
                            backend: RagStorageBackend::Graph,
                            layout: "GRAPH_SINGLE_NODE".into(),
                            table: None,
                            namespace: Some("rag_text_units".into()),
                            node_label: Some("RagTextUnit".into()),
                            code_strategy: None,
                        },
                    ),
                    (
                        "vector-graph".into(),
                        RagStorageSegmentTarget {
                            id: "vector-graph".into(),
                            backend: RagStorageBackend::Graph,
                            layout: "GRAPH_SINGLE_NODE".into(),
                            table: None,
                            namespace: Some("rag_vectors".into()),
                            node_label: Some("RagVector".into()),
                            code_strategy: None,
                        },
                    ),
                ]),
            },
        }
    }
}
