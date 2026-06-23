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
    let source_node_id = format!("source:{}", request.source.code);
    let mut nodes = vec![json!({
        "id": source_node_id,
        "kind": "SOURCE",
        "sourceCode": request.source.code,
        "sourceVersion": request.source.version,
        "sourceType": request.source.source_type,
        "title": prepared.title
    })];
    let mut edges = Vec::new();
    let mut previous_text_unit_id: Option<String> = None;
    for unit in &prepared.units {
        let text_unit_id = format!("text-unit:{}", unit.text_unit_code);
        nodes.push(json!({
            "id": text_unit_id,
            "kind": "TEXT_UNIT",
            "textUnitCode": unit.text_unit_code,
            "headingPath": unit.heading_path,
            "ordinal": unit.ordinal
        }));
        edges.push(json!({
            "from": source_node_id,
            "to": text_unit_id,
            "relationship": "CONTAINS"
        }));
        if let Some(previous) = previous_text_unit_id {
            edges.push(json!({
                "from": previous,
                "to": text_unit_id,
                "relationship": "NEXT"
            }));
        }
        previous_text_unit_id = Some(text_unit_id);
    }
    json!({
        "schemaVersion": "tree-graph-v1",
        "sourceCode": request.source.code,
        "sourceVersion": request.source.version,
        "nodes": nodes,
        "edges": edges
    })
}

fn summarize_tree_node(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
    hint: &crate::application::rag::ingest_payload::RagTreeHintDraft,
) -> RagTreeNodeWrite {
    let covered_text = prepared
        .units
        .iter()
        .filter(|unit| hint.covered_text_unit_codes.contains(&unit.text_unit_code))
        .map(|unit| unit.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let summary_text = extractive_summary(&hint.title, &covered_text);
    RagTreeNodeWrite {
        tree_node_code: hint.tree_node_code.clone(),
        parent_node_code: hint.parent_node_code.clone(),
        source_code: request.source.code.clone(),
        source_version: request.source.version.clone(),
        scope_type: request.target_scope.scope_type.clone(),
        scope_code: request.target_scope.scope_code.clone(),
        path_key: hint.path_key.clone(),
        level: hint.level,
        ordinal: hint.ordinal,
        title: hint.title.clone(),
        summary_token_count: token_count(&summary_text),
        summary_content_hash: stable_hash(&summary_text),
        summary_text,
        summary_fulltext_status: "PENDING".into(),
        summary_embedding_status: "PENDING".into(),
        summary_embedding_model_code: None,
        summary_embedding_version: None,
        summary_embedding_text_hash: None,
        summary_embedding_dimension: None,
        summary_embedding: None,
        covered_text_unit_count: hint.covered_text_unit_codes.len() as i64,
        covered_text_unit_codes: hint.covered_text_unit_codes.clone(),
        metadata: json!({ "summaryStrategy": "EXTRACTIVE_DETERMINISTIC_V1" }),
        status: "ACTIVE".into(),
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
