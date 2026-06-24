impl<R, S, E> RagIngestService<R, S, E>
where
    R: RagIngestRepository + RagConfigRepository,
    S: RagStrategyRegistry,
    E: RagEmbeddingRuntime + RagDocumentSourceRuntime,
{
    fn execute_extract_tree_stage(
        &self,
        request: &RagIngestRequest,
        bundle: &mut ArtifactBundle,
        store: &mut RagArtifactStore,
    ) -> Result<(), RagError> {
        let prepared = bundle.prepared_payload()?.clone();
        let tree = build_tree_extracted_artifact(request, &prepared);
        let graph = build_tree_graph_artifact(request, &prepared);
        bundle.append(
            request,
            ARTIFACT_TREE_GRAPH,
            RagArtifactPayload::TreeGraph(graph.clone()),
            json!({
                "nodeCount": graph.get("nodes").and_then(Value::as_array).map_or(0, Vec::len)
            }),
        );
        store.put_json("tree.extracted", tree);
        store.put_json("graph.related", graph);
        publish_ingest_stage_event(
            &self.event_bus,
            "extractTree",
            BusOutcome::Completed,
            prepared.units.len(),
            request,
        );
        Ok(())
    }

    async fn execute_summarize_tree_stage(
        &self,
        request: &RagIngestRequest,
        node: &CompiledRagPipelineNode,
        bundle: &mut ArtifactBundle,
        store: &mut RagArtifactStore,
    ) -> Result<(), RagError> {
        let prepared = bundle.prepared_payload()?.clone();
        let tree_extracted = store.get::<Value>("tree.extracted")?.clone();
        let extracted_nodes = tree_extracted_nodes(&tree_extracted)?;
        let children_by_parent = tree_parent_child_edges(&tree_extracted, &extracted_nodes)?;
        let tree_nodes = match node.strategy_id.as_str() {
            "RAG_TREE_EXTRACTIVE_SUMMARY_V1" => extracted_nodes
                .iter()
                .map(|tree_node| extractive_summary_tree_node(request, &prepared, tree_node))
                .collect::<Vec<_>>(),
            "RAG_TREE_LLM_SUMMARY_V1" => {
                self.execute_llm_summarize_tree(
                    request,
                    node,
                    &prepared,
                    &extracted_nodes,
                    &children_by_parent,
                )
                    .await?
            }
            _ => return Err(RagError::IngestProfileInvalid),
        };
        let count = tree_nodes.len();
        bundle.append(
            request,
            ARTIFACT_TREE_SUMMARY_RESULT,
            RagArtifactPayload::TreeSummaryResult(RagTreeIndexArtifact {
                tree_nodes,
                embedding_status: "PENDING".into(),
                warnings: Vec::new(),
                fatal_error: None,
            }),
            json!({ "nodeCount": count }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "summarizeTree",
            BusOutcome::Completed,
            count,
            request,
        );
        Ok(())
    }

    async fn execute_llm_summarize_tree(
        &self,
        request: &RagIngestRequest,
        node: &CompiledRagPipelineNode,
        prepared: &PreparedIngestPayload,
        tree_nodes: &[TreeExtractedNode],
        children_by_parent: &BTreeMap<String, Vec<&TreeExtractedNode>>,
    ) -> Result<Vec<RagTreeNodeWrite>, RagError> {
        let model_code = llm_summary_model_code(node)?;
        let mut summaries: BTreeMap<String, String> = BTreeMap::new();
        let mut tree_nodes_by_code: BTreeMap<String, RagTreeNodeWrite> = BTreeMap::new();
        let mut ordered_nodes = tree_nodes.iter().collect::<Vec<_>>();
        ordered_nodes.sort_by(|left, right| {
            right
                .level
                .cmp(&left.level)
                .then_with(|| left.ordinal.cmp(&right.ordinal))
        });
        for tree_node in ordered_nodes {
            let child_summaries = children_by_parent
                .get(&tree_node.tree_node_code)
                .into_iter()
                .flat_map(|children| children.iter())
                .filter_map(|child| {
                    summaries
                        .get(child.tree_node_code.as_str())
                        .map(|summary| (child.title.as_str(), summary.as_str()))
                })
                .collect::<Vec<_>>();
            let direct_text_units = direct_text_units(prepared, tree_node, children_by_parent);
            let prompt =
                llm_tree_summary_prompt(&tree_node.title, &child_summaries, &direct_text_units);
            let result = self
                .tree_summary_runtime
                .as_ref()
                .ok_or(RagError::IngestStageFailed)?
                .summarize_tree(RagTreeSummaryCall {
                    workspace_code: request.workspace_code.clone(),
                    model_code: Some(model_code.clone()),
                    prompt,
                })
                .await
                .map_err(|_| RagError::IngestStageFailed)?;
            let summary_text = result.summary_text.trim().to_string();
            if summary_text.is_empty() {
                return Err(RagError::IngestStageFailed);
            }
            let tree_node = llm_summary_tree_node(
                request,
                prepared,
                tree_node,
                summary_text.clone(),
                result.model_code,
            );
            summaries.insert(tree_node.tree_node_code.clone(), summary_text);
            tree_nodes_by_code.insert(tree_node.tree_node_code.clone(), tree_node);
        }
        Ok(tree_nodes
            .iter()
            .filter_map(|tree_node| tree_nodes_by_code.remove(&tree_node.tree_node_code))
            .collect())
    }

    fn execute_tree_fulltext_index_stage(
        &self,
        request: &RagIngestRequest,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        let mut result = bundle.tree_summary_result()?.clone();
        for node in &mut result.tree_nodes {
            node.summary_fulltext_status = "INDEXED".into();
        }
        let count = result.tree_nodes.len();
        bundle.append(
            request,
            ARTIFACT_TREE_FULLTEXT_INDEX_RESULT,
            RagArtifactPayload::TreeFulltextIndexResult(result),
            json!({ "nodeCount": count, "status": "INDEXED" }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "treeFulltextIndex",
            BusOutcome::Completed,
            count,
            request,
        );
        Ok(())
    }

    async fn execute_tree_vector_index_stage(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        let mut result = bundle.tree_summary_result()?.clone();
        let mut embedding_failed = false;
        let mut fatal_error = None;
        let mut warnings = Vec::new();
        for node in &mut result.tree_nodes {
            match self
                .embedding_runtime
                .embed(RagEmbeddingCall {
                    workspace_code: request.workspace_code.clone(),
                    model_code: compiled.embedding_model_code.clone(),
                    input: node.summary_text.clone(),
                })
                .await
            {
                Ok(embedding) => {
                    node.summary_embedding_model_code = Some(embedding.model_code);
                    node.summary_embedding_version = Some(compiled.embedding_version.clone());
                    node.summary_embedding_text_hash = Some(stable_hash(&node.summary_text));
                    node.summary_embedding_dimension = Some(embedding.embedding.len() as i64);
                    node.summary_embedding = Some(encode_embedding(&embedding.embedding));
                    node.summary_embedding_status = "INDEXED".into();
                }
                Err(_) if allows_vector_degrade(&compiled.degrade_policy) => {
                    embedding_failed = true;
                    node.summary_embedding_status = "FAILED".into();
                }
                Err(_) => {
                    embedding_failed = true;
                    node.summary_embedding_status = "FAILED".into();
                    fatal_error = Some(RagError::EmbeddingFailed);
                    break;
                }
            }
        }
        if embedding_failed && fatal_error.is_none() {
            warnings.push(RagIngestWarning {
                stage_id: "treeVectorIndex".into(),
                code: "RAG_TREE_VECTOR_INDEX_WARNING".into(),
                message: "Tree vector indexing failed; tree fulltext remains available".into(),
            });
        }
        result.embedding_status = if embedding_failed {
            "FAILED".into()
        } else {
            "INDEXED".into()
        };
        result.warnings = warnings;
        result.fatal_error = fatal_error;
        let count = result.tree_nodes.len();
        let outcome = if embedding_failed {
            BusOutcome::Failure
        } else {
            BusOutcome::Completed
        };
        let fatal_error = result.fatal_error.clone();
        bundle.append(
            request,
            ARTIFACT_TREE_VECTOR_INDEX_RESULT,
            RagArtifactPayload::TreeVectorIndexResult(result),
            json!({ "nodeCount": count }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "treeVectorIndex",
            outcome,
            count,
            request,
        );
        if let Some(error) = fatal_error {
            return Err(error);
        }
        Ok(())
    }

    fn execute_persist_tree_stage(
        &self,
        request: &RagIngestRequest,
        bundle: &mut ArtifactBundle,
        store: &mut RagArtifactStore,
    ) -> Result<(), RagError> {
        let mut result = bundle.tree_summary_result()?.clone();
        if let Ok(fulltext_result) = bundle.tree_fulltext_result() {
            merge_tree_fulltext_status(&mut result, fulltext_result);
        }
        if let Ok(vector_result) = bundle.tree_vector_result() {
            merge_tree_vector_status(&mut result, vector_result);
        }
        if result
            .tree_nodes
            .iter()
            .any(|node| node.covered_text_unit_codes.is_empty())
        {
            return Err(RagError::IngestStageFailed);
        }
        bundle.append(
            request,
            ARTIFACT_TREE_PERSIST_RESULT,
            RagArtifactPayload::TreePersistResult(result.clone()),
            json!({
                "sourceCode": request.source.code,
                "sourceVersion": request.source.version,
                "nodeCount": result.tree_nodes.len()
            }),
        );
        store.put_json(
            "tree.writePayload",
            json!({
                "sourceCode": request.source.code,
                "sourceVersion": request.source.version,
                "nodeCount": result.tree_nodes.len()
            }),
        );
        store.put_json("tree.persisted", json!({"nodeCount": result.tree_nodes.len()}));
        publish_ingest_stage_event(
            &self.event_bus,
            "persistTree",
            BusOutcome::Completed,
            1,
            request,
        );
        Ok(())
    }
}

fn llm_summary_model_code(node: &CompiledRagPipelineNode) -> Result<String, RagError> {
    node.config
        .as_ref()
        .and_then(|config| config.get("modelCode"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(RagError::IngestProfileInvalid)
}

#[derive(Debug, Clone)]
struct TreeExtractedNode {
    tree_node_code: String,
    parent_node_code: String,
    path_key: String,
    level: i64,
    ordinal: i64,
    title: String,
    covered_text_unit_codes: Vec<String>,
}

fn tree_extracted_nodes(tree_extracted: &Value) -> Result<Vec<TreeExtractedNode>, RagError> {
    tree_extracted
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or(RagError::IngestStageFailed)?
        .iter()
        .map(tree_extracted_node)
        .collect()
}

fn tree_extracted_node(node: &Value) -> Result<TreeExtractedNode, RagError> {
    Ok(TreeExtractedNode {
        tree_node_code: required_string(node, "treeNodeCode")?,
        parent_node_code: required_string(node, "parentNodeCode")?,
        path_key: required_string(node, "pathKey")?,
        level: required_i64(node, "level")?,
        ordinal: required_i64(node, "ordinal")?,
        title: required_string(node, "title")?,
        covered_text_unit_codes: required_string_array(node, "coveredTextUnitCodes")?,
    })
}

fn tree_parent_child_edges<'a>(
    tree_extracted: &Value,
    tree_nodes: &'a [TreeExtractedNode],
) -> Result<BTreeMap<String, Vec<&'a TreeExtractedNode>>, RagError> {
    let mut nodes_by_code = BTreeMap::new();
    for tree_node in tree_nodes {
        if nodes_by_code
            .insert(tree_node.tree_node_code.as_str(), tree_node)
            .is_some()
        {
            return Err(RagError::IngestStageFailed);
        }
    }

    let mut children_by_parent: BTreeMap<String, Vec<&TreeExtractedNode>> = BTreeMap::new();
    let mut linked_child_codes = BTreeSet::new();
    for edge in tree_extracted
        .get("edges")
        .and_then(Value::as_array)
        .ok_or(RagError::IngestStageFailed)?
    {
        if required_string(edge, "relationship")? != "PARENT_OF" {
            continue;
        }
        let parent_code = required_string(edge, "from")?;
        let child_code = required_string(edge, "to")?;
        let child = *nodes_by_code
            .get(child_code.as_str())
            .ok_or(RagError::IngestStageFailed)?;
        if child.parent_node_code != parent_code {
            return Err(RagError::IngestStageFailed);
        }
        children_by_parent
            .entry(parent_code)
            .or_default()
            .push(child);
        linked_child_codes.insert(child_code);
    }
    for tree_node in tree_nodes
        .iter()
        .filter(|tree_node| tree_node.parent_node_code != "0")
    {
        if !linked_child_codes.contains(&tree_node.tree_node_code) {
            return Err(RagError::IngestStageFailed);
        }
    }
    for children in children_by_parent.values_mut() {
        children.sort_by_key(|tree_node| tree_node.ordinal);
    }
    Ok(children_by_parent)
}

fn required_string(value: &Value, field: &str) -> Result<String, RagError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(RagError::IngestStageFailed)
}

fn required_i64(value: &Value, field: &str) -> Result<i64, RagError> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or(RagError::IngestStageFailed)
}

fn required_string_array(value: &Value, field: &str) -> Result<Vec<String>, RagError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or(RagError::IngestStageFailed)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToOwned::to_owned)
                .ok_or(RagError::IngestStageFailed)
        })
        .collect()
}

fn direct_text_units(
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
    children_by_parent: &BTreeMap<String, Vec<&TreeExtractedNode>>,
) -> Vec<String> {
    let child_covered_codes = children_by_parent
        .get(&tree_node.tree_node_code)
        .into_iter()
        .flat_map(|children| children.iter())
        .flat_map(|child| child.covered_text_unit_codes.iter().cloned())
        .collect::<BTreeSet<_>>();
    prepared
        .units
        .iter()
        .filter(|unit| {
            tree_node
                .covered_text_unit_codes
                .contains(&unit.text_unit_code)
        })
        .filter(|unit| !child_covered_codes.contains(&unit.text_unit_code))
        .map(|unit| unit.text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect()
}

fn llm_tree_summary_prompt(
    title: &str,
    child_summaries: &[(&str, &str)],
    direct_text_units: &[String],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("Summarize this RAG tree node. Return only the summary text.\n");
    prompt.push_str("Node title:\n");
    prompt.push_str(title.trim());
    prompt.push('\n');
    if !child_summaries.is_empty() {
        prompt.push_str("\nChild summaries:\n");
        for (child_title, child_summary) in child_summaries {
            prompt.push_str("- ");
            prompt.push_str(child_title.trim());
            prompt.push_str(": ");
            prompt.push_str(child_summary.trim());
            prompt.push('\n');
        }
    }
    if !direct_text_units.is_empty() {
        prompt.push_str("\nDirect text units:\n");
        for (index, text) in direct_text_units.iter().enumerate() {
            prompt.push_str("[");
            prompt.push_str(&(index + 1).to_string());
            prompt.push_str("]\n");
            prompt.push_str(text);
            prompt.push('\n');
        }
    }
    prompt
}

fn llm_summary_tree_node(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
    summary_text: String,
    model_code: String,
) -> RagTreeNodeWrite {
    summary_tree_node_write(
        request,
        prepared,
        tree_node,
        summary_text,
        json!({
            "summaryStrategy": "RAG_TREE_LLM_SUMMARY_V1",
            "summaryModelCode": model_code,
            "summaryStatus": "SUMMARIZED"
        }),
    )
}

fn extractive_summary_tree_node(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
) -> RagTreeNodeWrite {
    let covered_text = prepared
        .units
        .iter()
        .filter(|unit| {
            tree_node
                .covered_text_unit_codes
                .contains(&unit.text_unit_code)
        })
        .map(|unit| unit.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let summary_text = extractive_summary(&tree_node.title, &covered_text);
    summary_tree_node_write(
        request,
        prepared,
        tree_node,
        summary_text,
        json!({ "summaryStrategy": "EXTRACTIVE_DETERMINISTIC_V1" }),
    )
}

fn summary_tree_node_write(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
    summary_text: String,
    metadata: Value,
) -> RagTreeNodeWrite {
    let covered_ordinals = prepared
        .units
        .iter()
        .filter(|unit| {
            tree_node
                .covered_text_unit_codes
                .contains(&unit.text_unit_code)
        })
        .map(|unit| unit.ordinal as i64)
        .collect::<Vec<_>>();
    RagTreeNodeWrite {
        tree_node_code: tree_node.tree_node_code.clone(),
        parent_node_code: tree_node.parent_node_code.clone(),
        source_code: request.source.code.clone(),
        source_version: request.source.version.clone(),
        scope_type: request.target_scope.scope_type.clone(),
        scope_code: request.target_scope.scope_code.clone(),
        path_key: tree_node.path_key.clone(),
        level: tree_node.level,
        ordinal: tree_node.ordinal,
        title: tree_node.title.clone(),
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
        covered_text_unit_count: tree_node.covered_text_unit_codes.len() as i64,
        covered_text_unit_codes: tree_node.covered_text_unit_codes.clone(),
        child_start_ordinal: covered_ordinals.iter().min().copied().unwrap_or(0),
        child_end_ordinal: covered_ordinals.iter().max().copied().unwrap_or(0),
        metadata,
        status: "ACTIVE".into(),
    }
}

fn merge_tree_fulltext_status(
    target: &mut RagTreeIndexArtifact,
    fulltext: &RagTreeIndexArtifact,
) {
    for node in &mut target.tree_nodes {
        if let Some(fulltext_node) = fulltext
            .tree_nodes
            .iter()
            .find(|candidate| candidate.tree_node_code == node.tree_node_code)
        {
            node.summary_fulltext_status = fulltext_node.summary_fulltext_status.clone();
        }
    }
}

fn merge_tree_vector_status(target: &mut RagTreeIndexArtifact, vector: &RagTreeIndexArtifact) {
    target.embedding_status = vector.embedding_status.clone();
    target.warnings.extend(vector.warnings.clone());
    target.fatal_error = vector.fatal_error.clone();
    for node in &mut target.tree_nodes {
        if let Some(vector_node) = vector
            .tree_nodes
            .iter()
            .find(|candidate| candidate.tree_node_code == node.tree_node_code)
        {
            node.summary_embedding_status = vector_node.summary_embedding_status.clone();
            node.summary_embedding_model_code = vector_node.summary_embedding_model_code.clone();
            node.summary_embedding_version = vector_node.summary_embedding_version.clone();
            node.summary_embedding_text_hash = vector_node.summary_embedding_text_hash.clone();
            node.summary_embedding_dimension = vector_node.summary_embedding_dimension;
            node.summary_embedding = vector_node.summary_embedding.clone();
        }
    }
}
