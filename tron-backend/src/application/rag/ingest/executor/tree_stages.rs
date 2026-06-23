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
        let graph = build_tree_graph_artifact(request, &prepared);
        bundle.append(
            request,
            ARTIFACT_TREE_GRAPH,
            RagArtifactPayload::TreeGraph(graph.clone()),
            json!({
                "nodeCount": graph.get("nodes").and_then(Value::as_array).map_or(0, Vec::len)
            }),
        );
        store.put_json("tree.extracted", graph.clone());
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

    fn execute_summarize_tree_stage(
        &self,
        request: &RagIngestRequest,
        bundle: &mut ArtifactBundle,
        _store: &mut RagArtifactStore,
    ) -> Result<(), RagError> {
        let prepared = bundle.prepared_payload()?.clone();
        let tree_nodes = prepared
            .tree_hints
            .iter()
            .map(|hint| summarize_tree_node(request, &prepared, hint))
            .collect::<Vec<_>>();
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
        let mut result = bundle.tree_fulltext_result()?.clone();
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
        let result = bundle.tree_vector_result()?.clone();
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
