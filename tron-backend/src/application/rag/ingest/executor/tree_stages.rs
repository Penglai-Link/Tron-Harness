use std::time::Duration;

use tokio::time::timeout;

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
        let (tree_nodes, warnings) = match node.strategy_id.as_str() {
            "RAG_TREE_EXTRACTIVE_SUMMARY_V1" => {
                let tree_nodes = extracted_nodes
                    .iter()
                    .map(|tree_node| extractive_summary_tree_node(request, &prepared, tree_node))
                    .collect::<Vec<_>>();
                (tree_nodes, Vec::new())
            }
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
                warnings,
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
    ) -> Result<(Vec<RagTreeNodeWrite>, Vec<RagIngestWarning>), RagError> {
        let summary_config = llm_summary_config(node)?;
        let tree_summary_runtime = self
            .tree_summary_runtime
            .as_ref()
            .ok_or(RagError::IngestStageFailed)?;
        let mut summaries: BTreeMap<String, String> = BTreeMap::new();
        let mut tree_nodes_by_code: BTreeMap<String, RagTreeNodeWrite> = BTreeMap::new();
        let mut warnings = Vec::new();
        let mut fallback_reason: Option<&'static str> = None;
        let ordered_nodes = tree_summary_post_order(tree_nodes, children_by_parent)?;
        for tree_node in ordered_nodes {
            if fallback_reason.is_some() {
                let fallback_node = llm_fallback_summary_tree_node(
                    request,
                    prepared,
                    tree_node,
                    "LLM_SUMMARY_SKIPPED_AFTER_FAILURE",
                );
                summaries.insert(
                    fallback_node.tree_node_code.clone(),
                    fallback_node.summary_text.clone(),
                );
                tree_nodes_by_code.insert(fallback_node.tree_node_code.clone(), fallback_node);
                continue;
            }

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
            let prompt = truncate_tree_summary_prompt(prompt, summary_config.max_prompt_chars);
            let summary_call = tree_summary_runtime.summarize_tree(RagTreeSummaryCall {
                workspace_code: request.workspace_code.clone(),
                model_code: summary_config.model_code.clone(),
                prompt,
            });
            let result = if let Some(timeout_seconds) = summary_config.timeout_seconds {
                match timeout(Duration::from_secs(timeout_seconds), summary_call).await {
                    Ok(result) => result,
                    Err(_) => {
                        fallback_reason = Some("LLM_SUMMARY_TIMEOUT");
                        warnings.push(llm_summary_fallback_warning("LLM_SUMMARY_TIMEOUT"));
                        let fallback_node = llm_fallback_summary_tree_node(
                            request,
                            prepared,
                            tree_node,
                            "LLM_SUMMARY_TIMEOUT",
                        );
                        summaries.insert(
                            fallback_node.tree_node_code.clone(),
                            fallback_node.summary_text.clone(),
                        );
                        tree_nodes_by_code
                            .insert(fallback_node.tree_node_code.clone(), fallback_node);
                        continue;
                    }
                }
            } else {
                summary_call.await
            };
            let result = match result {
                Ok(result) if !result.summary_text.trim().is_empty() => result,
                Ok(_) => {
                    fallback_reason = Some("LLM_SUMMARY_EMPTY_RESPONSE");
                    warnings.push(llm_summary_fallback_warning("LLM_SUMMARY_EMPTY_RESPONSE"));
                    let fallback_node = llm_fallback_summary_tree_node(
                        request,
                        prepared,
                        tree_node,
                        "LLM_SUMMARY_EMPTY_RESPONSE",
                    );
                    summaries.insert(
                        fallback_node.tree_node_code.clone(),
                        fallback_node.summary_text.clone(),
                    );
                    tree_nodes_by_code.insert(fallback_node.tree_node_code.clone(), fallback_node);
                    continue;
                }
                Err(_) => {
                    fallback_reason = Some("LLM_SUMMARY_PROVIDER_ERROR");
                    warnings.push(llm_summary_fallback_warning("LLM_SUMMARY_PROVIDER_ERROR"));
                    let fallback_node = llm_fallback_summary_tree_node(
                        request,
                        prepared,
                        tree_node,
                        "LLM_SUMMARY_PROVIDER_ERROR",
                    );
                    summaries.insert(
                        fallback_node.tree_node_code.clone(),
                        fallback_node.summary_text.clone(),
                    );
                    tree_nodes_by_code.insert(fallback_node.tree_node_code.clone(), fallback_node);
                    continue;
                }
            };
            let summary_text = result.summary_text.trim().to_string();
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
        Ok((
            tree_nodes
                .iter()
                .filter_map(|tree_node| tree_nodes_by_code.remove(&tree_node.tree_node_code))
                .collect(),
            warnings,
        ))
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
        publish_ingest_stage_event(&self.event_bus, "treeVectorIndex", outcome, count, request);
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
        store.put_json(
            "tree.persisted",
            json!({"nodeCount": result.tree_nodes.len()}),
        );
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

fn llm_fallback_summary_tree_node(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
    reason: &str,
) -> RagTreeNodeWrite {
    let mut node = extractive_summary_tree_node(request, prepared, tree_node);
    node.metadata = json!({
        "summaryStrategy": "RAG_TREE_LLM_SUMMARY_V1",
        "summaryFallbackStrategy": "EXTRACTIVE_DETERMINISTIC_V1",
        "summaryFallbackReason": reason,
        "summaryStatus": "FALLBACK_EXTRACTIVE"
    });
    node
}

fn llm_summary_fallback_warning(reason: &str) -> RagIngestWarning {
    RagIngestWarning {
        stage_id: "summarizeTree".into(),
        code: "RAG_TREE_LLM_SUMMARY_FALLBACK".into(),
        message: format!("LLM tree summary fell back to extractive summary: {reason}"),
    }
}

fn merge_tree_fulltext_status(target: &mut RagTreeIndexArtifact, fulltext: &RagTreeIndexArtifact) {
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
