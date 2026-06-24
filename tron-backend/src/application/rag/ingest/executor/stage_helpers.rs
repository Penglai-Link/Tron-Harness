fn ingest_stage_registry() -> BTreeMap<&'static str, IngestStageKind> {
    BTreeMap::from([
        ("validateSource", IngestStageKind::ValidateSource),
        ("preparePayload", IngestStageKind::PreparePayload),
        ("resolveSource", IngestStageKind::ResolveSource),
        ("parseDocument", IngestStageKind::ParseDocument),
        ("splitText", IngestStageKind::SplitText),
        ("fulltextIndex", IngestStageKind::FulltextIndex),
        ("vectorIndex", IngestStageKind::VectorIndex),
        ("extractTree", IngestStageKind::ExtractTree),
        ("summarizeTree", IngestStageKind::SummarizeTree),
        ("treeFulltextIndex", IngestStageKind::TreeFulltextIndex),
        ("treeVectorIndex", IngestStageKind::TreeVectorIndex),
        ("persistTree", IngestStageKind::PersistTree),
        ("persistSource", IngestStageKind::PersistSource),
    ])
}

fn build_tree_graph_artifact(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
) -> Value {
    let source_node_ref = format!("source:{}", request.source.code);
    let mut nodes = vec![json!({
        "ref": source_node_ref,
        "kind": "SOURCE",
        "label": "RAG_SOURCE",
        "code": request.source.code,
        "sourceCode": request.source.code,
        "sourceVersion": request.source.version,
        "sourceType": request.source.source_type,
        "title": prepared.title
    })];
    let mut edges = Vec::new();
    let mut previous_text_unit_ref: Option<String> = None;
    for unit in &prepared.units {
        let text_unit_ref = format!("text-unit:{}", unit.text_unit_code);
        nodes.push(json!({
            "ref": text_unit_ref,
            "kind": "TEXT_UNIT",
            "label": "RAG_TEXT_UNIT",
            "code": unit.text_unit_code,
            "title": unit.text_unit_code,
            "text": unit.text,
            "textUnitCode": unit.text_unit_code,
            "headingPath": unit.heading_path,
            "ordinal": unit.ordinal
        }));
        edges.push(json!({
            "from": source_node_ref,
            "to": text_unit_ref,
            "relationship": "CONTAINS"
        }));
        if let Some(previous) = previous_text_unit_ref {
            edges.push(json!({
                "from": previous,
                "to": text_unit_ref,
                "relationship": "NEXT"
            }));
        }
        previous_text_unit_ref = Some(text_unit_ref);
    }
    json!({
        "schemaVersion": "tree-graph-v1",
        "sourceCode": request.source.code,
        "sourceVersion": request.source.version,
        "nodes": nodes,
        "edges": edges
    })
}

fn build_tree_hints_artifact(prepared: &PreparedIngestPayload, split_strategy_id: &str) -> Value {
    let root = prepared
        .tree_hints
        .iter()
        .find(|hint| hint.parent_node_code == "0")
        .map(tree_hint_json)
        .unwrap_or_else(|| json!({}));
    let outline = prepared
        .tree_hints
        .iter()
        .filter(|hint| hint.parent_node_code != "0")
        .map(tree_hint_json)
        .collect::<Vec<_>>();
    let leaf_attachments = prepared
        .units
        .iter()
        .map(|unit| {
            let tree_node_codes = prepared
                .tree_hints
                .iter()
                .filter(|hint| hint.covered_text_unit_codes.contains(&unit.text_unit_code))
                .map(|hint| hint.tree_node_code.clone())
                .collect::<Vec<_>>();
            json!({
                "textUnitCode": unit.text_unit_code,
                "treeNodeCodes": tree_node_codes,
                "headingPath": unit.heading_path,
                "ordinal": unit.ordinal
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schemaVersion": "rag-tree-hints-v1",
        "split": {
            "strategyId": split_strategy_id,
            "structureKind": tree_structure_kind(prepared)
        },
        "root": root,
        "outline": outline,
        "leafAttachments": leaf_attachments
    })
}

fn build_tree_extracted_artifact(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
) -> Value {
    let root = prepared
        .tree_hints
        .iter()
        .find(|hint| hint.parent_node_code == "0")
        .map(tree_hint_json)
        .unwrap_or_else(|| json!({}));
    let nodes = prepared
        .tree_hints
        .iter()
        .map(|hint| {
            json!({
                "kind": "TREE_NODE",
                "treeNodeCode": hint.tree_node_code,
                "parentNodeCode": hint.parent_node_code,
                "pathKey": hint.path_key,
                "level": hint.level,
                "ordinal": hint.ordinal,
                "title": hint.title,
                "coveredTextUnitCodes": hint.covered_text_unit_codes,
                "coveredTextUnitCount": hint.covered_text_unit_codes.len()
            })
        })
        .collect::<Vec<_>>();
    let edges = prepared
        .tree_hints
        .iter()
        .filter(|hint| hint.parent_node_code != "0")
        .map(|hint| {
            json!({
                "from": hint.parent_node_code,
                "to": hint.tree_node_code,
                "relationship": "PARENT_OF"
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schemaVersion": "rag-tree-extracted-v1",
        "sourceCode": request.source.code,
        "sourceVersion": request.source.version,
        "root": root,
        "nodes": nodes,
        "edges": edges
    })
}

fn tree_hint_json(hint: &crate::application::rag::ingest_payload::RagTreeHintDraft) -> Value {
    json!({
        "treeNodeCode": hint.tree_node_code,
        "parentNodeCode": hint.parent_node_code,
        "pathKey": hint.path_key,
        "level": hint.level,
        "ordinal": hint.ordinal,
        "title": hint.title,
        "coveredTextUnitCodes": hint.covered_text_unit_codes
    })
}

fn tree_structure_kind(prepared: &PreparedIngestPayload) -> &'static str {
    if prepared
        .tree_hints
        .iter()
        .any(|hint| hint.level > 0 && !hint.path_key.trim().is_empty())
    {
        "HEADING_TREE"
    } else {
        "FLAT_TREE"
    }
}

fn extractive_summary(title: &str, text: &str) -> String {
    let body = text
        .split(['\n', '.'])
        .map(str::trim)
        .find(|part| !part.is_empty())
        .unwrap_or(text.trim());
    let mut summary = title.trim().to_string();
    if !body.is_empty() {
        if !summary.is_empty() {
            summary.push_str(": ");
        }
        summary.push_str(body);
    }
    if summary.chars().count() > 512 {
        summary.chars().take(512).collect()
    } else {
        summary
    }
}

fn stable_hash(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn token_count(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

fn encode_embedding(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>()
}

fn validate_project_doc_ref_request(request: &RagIngestRequest) -> Result<(), RagError> {
    if request.source.source_type != "PROJECT_DOC_REF" {
        return Err(RagError::IngestRequestInvalid);
    }
    if request
        .payload
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return Err(RagError::IngestRequestInvalid);
    }
    if document_payload_string(&request.payload, "format")
        .or_else(|| payload_string(&request.payload, "format"))
        .is_none()
    {
        return Err(RagError::DocumentFormatUnsupported);
    }
    if document_payload_string(&request.payload, "uri")
        .or_else(|| payload_string(&request.payload, "documentUri"))
        .is_none()
    {
        return Err(RagError::IngestRequestInvalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::application::rag::{
        RagIngestAccessContext, RagIngestScope, RagIngestSource,
        ingest_payload::{PreparedIngestPayload, RagTextUnitDraft, RagTreeHintDraft},
    };

    #[test]
    fn split_text_tree_hints_artifact_uses_object_contract() {
        let prepared = prepared_tree_payload();

        let artifact = build_tree_hints_artifact(&prepared, "RAG_SPLIT_MARKDOWN_HEADING_V1");

        assert_eq!(artifact["schemaVersion"], json!("rag-tree-hints-v1"));
        assert_eq!(
            artifact["split"],
            json!({
                "strategyId": "RAG_SPLIT_MARKDOWN_HEADING_V1",
                "structureKind": "HEADING_TREE"
            })
        );
        assert_eq!(artifact["root"]["treeNodeCode"], json!("tree-root"));
        assert_eq!(artifact["outline"][0]["treeNodeCode"], json!("tree-child"));
        assert_eq!(
            artifact["leafAttachments"],
            json!([
                {
                    "textUnitCode": "source:v1:0",
                    "treeNodeCodes": ["tree-root", "tree-child"],
                    "headingPath": "Intro",
                    "ordinal": 0
                }
            ])
        );
        assert!(
            artifact.as_array().is_none(),
            "tree.hints should be an object, not the internal Vec shape"
        );
    }

    #[test]
    fn extract_tree_artifact_uses_tree_node_shape_not_related_graph_shape() {
        let request = tree_request();
        let prepared = prepared_tree_payload();

        let artifact = build_tree_extracted_artifact(&request, &prepared);

        assert_eq!(artifact["schemaVersion"], json!("rag-tree-extracted-v1"));
        assert_eq!(artifact["root"]["treeNodeCode"], json!("tree-root"));
        assert!(
            artifact["nodes"].as_array().is_some_and(|nodes| nodes.iter().all(
                |node| node["kind"] == json!("TREE_NODE")
                    && node.get("treeNodeCode").is_some()
                    && node.get("textUnitCode").is_none()
            )),
            "tree.extracted nodes should describe tree nodes instead of text-unit graph nodes"
        );
        assert!(
            artifact["edges"].as_array().is_some_and(|edges| edges.iter().any(
                |edge| edge["from"] == json!("tree-root")
                    && edge["to"] == json!("tree-child")
                    && edge["relationship"] == json!("PARENT_OF")
            )),
            "tree.extracted should describe parent-child tree edges"
        );
    }

    fn prepared_tree_payload() -> PreparedIngestPayload {
        PreparedIngestPayload {
            title: Some("Source title".into()),
            content_hash: "content-hash".into(),
            body_hash: None,
            metadata: json!({}),
            domain_fields: Default::default(),
            units: vec![RagTextUnitDraft {
                text_unit_code: "source:v1:0".into(),
                text: "# Intro\nAlways cite source titles.".into(),
                index_text: "Intro\nAlways cite source titles.".into(),
                heading_path: "Intro".into(),
                ordinal: 0,
                content_hash: "unit-hash".into(),
            }],
            tree_hints: vec![
                RagTreeHintDraft {
                    tree_node_code: "tree-root".into(),
                    parent_node_code: "0".into(),
                    path_key: "/".into(),
                    level: 0,
                    ordinal: 0,
                    title: "Source title".into(),
                    covered_text_unit_codes: vec!["source:v1:0".into()],
                },
                RagTreeHintDraft {
                    tree_node_code: "tree-child".into(),
                    parent_node_code: "tree-root".into(),
                    path_key: "Intro".into(),
                    level: 1,
                    ordinal: 1,
                    title: "Intro".into(),
                    covered_text_unit_codes: vec!["source:v1:0".into()],
                },
            ],
        }
    }

    fn tree_request() -> RagIngestRequest {
        RagIngestRequest {
            corpus_id: "memory-card".into(),
            ingest_profile_id: "memory-card-tree".into(),
            workspace_code: "TRON_CORE".into(),
            access_context: RagIngestAccessContext {
                user_code: "USR-LARGE-ID".into(),
                agent_code: "AGENT_CODEX".into(),
                workspace_code: "TRON_CORE".into(),
            },
            target_scope: RagIngestScope {
                scope_type: "SPACE".into(),
                scope_code: "TRON_CORE".into(),
            },
            source: RagIngestSource {
                code: "source".into(),
                version: "v1".into(),
                source_type: "MEMORY_CARD".into(),
            },
            payload: json!({}),
        }
    }
}

fn document_uri<'a>(
    request: &'a RagIngestRequest,
    compiled: &CompiledRagIngestProfile,
) -> Result<&'a str, RagError> {
    if compiled.payload_strategy_id == "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1" {
        return document_payload_string(&request.payload, "uri")
            .ok_or(RagError::IngestRequestInvalid);
    }
    payload_string(&request.payload, "documentUri").ok_or(RagError::IngestRequestInvalid)
}

fn document_format<'a>(
    request: &'a RagIngestRequest,
    compiled: &CompiledRagIngestProfile,
) -> Result<&'a str, RagError> {
    if compiled.payload_strategy_id == "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1" {
        return document_payload_string(&request.payload, "format")
            .ok_or(RagError::DocumentFormatUnsupported);
    }
    payload_string(&request.payload, "format").ok_or(RagError::DocumentFormatUnsupported)
}

fn payload_string<'a>(payload: &'a Value, field: &str) -> Option<&'a str> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn document_payload_string<'a>(payload: &'a Value, field: &str) -> Option<&'a str> {
    payload
        .get("document")
        .and_then(|document| document.get(field))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn allows_vector_degrade(degrade_policy: &str) -> bool {
    matches!(
        degrade_policy,
        "SKIP_VECTOR_ON_UNAVAILABLE"
            | "SKIP_VECTOR_WITH_WARNING"
            | "PARTIAL_ALLOWED"
            | "PARTIAL_INDEX_ALLOWED"
    )
}
