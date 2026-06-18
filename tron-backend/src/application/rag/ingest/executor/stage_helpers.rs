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
