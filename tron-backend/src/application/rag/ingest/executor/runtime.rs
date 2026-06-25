use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use serde_json::{Value, json};

use crate::domain::rag::RagError;

use super::{
    RagEmbeddingCall, RagEmbeddingRuntime, RagIngestRepository, RagIngestRequest, RagIngestService,
    RagIngestWarning, RagPersistIngestCommand, RagTreeNodeWrite, RagTreeSummaryCall,
    publish_ingest_stage_event,
};
use crate::application::rag::{
    CompiledRagIngestProfile, CompiledRagPipelineNode, RagArtifactStore, RagConfigRepository,
    RagDocumentParseCall, RagDocumentSourceCall, RagDocumentSourceRuntime, RagPipelineExecutor,
    RagStageExecutionResult, RagStageOutcome, RagStrategyRegistry,
    ingest_artifact::{
        ARTIFACT_FULLTEXT_INDEX_RESULT, ARTIFACT_PARSED_DOCUMENT, ARTIFACT_PERSIST_RESULT,
        ARTIFACT_RESOLVED_DOCUMENT, ARTIFACT_TEXT_UNIT_BATCH, ARTIFACT_TREE_FULLTEXT_INDEX_RESULT,
        ARTIFACT_TREE_GRAPH, ARTIFACT_TREE_PERSIST_RESULT, ARTIFACT_TREE_SUMMARY_RESULT,
        ARTIFACT_TREE_VECTOR_INDEX_RESULT, ARTIFACT_VALIDATED_SOURCE, ARTIFACT_VECTOR_INDEX_RESULT,
        ArtifactBundle, RagArtifactPayload, RagPersistResultArtifact, RagTreeIndexArtifact,
        RagVectorIndexArtifact,
    },
    ingest_payload::{PreparedIngestPayload, prepare_payload},
};
use crate::shared::event_bus::BusOutcome;

impl<R, S, E> RagIngestService<R, S, E>
where
    R: RagIngestRepository + RagConfigRepository,
    S: RagStrategyRegistry,
    E: RagEmbeddingRuntime + RagDocumentSourceRuntime,
{
    pub(super) async fn execute_ingest_stages(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
    ) -> Result<(RagPersistResultArtifact, Vec<RagStageExecutionResult>), RagError> {
        let pipeline = compiled
            .pipeline
            .as_ref()
            .ok_or(RagError::IngestProfileInvalid)?;
        let stage_registry = ingest_stage_registry();
        let mut store = RagArtifactStore::default();
        store.put_json("request.payload", request.payload.clone());
        store.put_json(
            "request.accessContext",
            json!({
                "userCode": request.access_context.user_code,
                "agentCode": request.access_context.agent_code,
                "workspaceCode": request.access_context.workspace_code
            }),
        );
        let mut stage_results = Vec::new();
        for level in RagPipelineExecutor::node_levels(pipeline) {
            for node in level {
                RagPipelineExecutor::ensure_inputs::<RagError>(node, &store)?;
                let started = std::time::Instant::now();
                let outcome = match self
                    .execute_ingest_pipeline_node(
                        request,
                        compiled,
                        bundle,
                        &stage_registry,
                        node,
                        &mut store,
                    )
                    .await
                {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        publish_ingest_stage_event(
                            &self.event_bus,
                            &node.execution_stage_id,
                            BusOutcome::Failure,
                            0,
                            request,
                        );
                        return Err(error);
                    }
                };
                for key in &node.output_artifact_keys {
                    if !store.contains(key) {
                        store.put_json(key.clone(), json!({"nodeId": node.node_id}));
                    }
                }
                let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                stage_results.push(RagPipelineExecutor::stage_result(
                    node,
                    outcome.status,
                    duration_ms,
                    outcome.warnings,
                    outcome.strategy_id,
                ));
            }
        }
        Ok((bundle.persist_result()?.clone(), stage_results))
    }

    async fn execute_ingest_pipeline_node(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
        stage_registry: &BTreeMap<&'static str, IngestStageKind>,
        node: &CompiledRagPipelineNode,
        store: &mut RagArtifactStore,
    ) -> Result<RagStageOutcome, RagError> {
        let kind = stage_registry
            .get(node.execution_stage_id.as_str())
            .ok_or(RagError::IngestProfileInvalid)?;
        match kind {
            IngestStageKind::ValidateSource => {
                let _payload = store.get::<Value>("request.payload")?;
                self.execute_validate_source_stage(request, bundle)?;
                store.put_json("source.validated", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::PreparePayload => {
                self.execute_prepare_payload_stage(request, bundle)?;
                store.put_json("payload.prepared", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::ResolveSource => {
                self.execute_resolve_source_stage(request, compiled, bundle)
                    .await?;
                store.put_json("document.resolved", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::ParseDocument => {
                self.execute_parse_document_stage(request, compiled, bundle)
                    .await?;
                store.put_json("document.parsed", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::SplitText => {
                self.execute_split_text_stage(request, compiled, bundle)?;
                store.put_json("textUnits.generated", json!({"nodeId": node.node_id}));
                let prepared = bundle.prepared_payload()?;
                store.put_json(
                    "tree.hints",
                    build_tree_hints_artifact(prepared, &compiled.split_strategy_id),
                );
            }
            IngestStageKind::FulltextIndex => {
                self.execute_fulltext_index_stage(request, bundle)?;
                store.put_json("index.fulltext", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::VectorIndex => {
                self.execute_vector_index_stage(request, compiled, bundle)
                    .await?;
                store.put_json("index.vector", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::ExtractTree => {
                self.execute_extract_tree_stage(request, bundle, store)?;
            }
            IngestStageKind::SummarizeTree => {
                self.execute_summarize_tree_stage(request, node, bundle, store)
                    .await?;
                store.put_json("tree.summarized", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::TreeFulltextIndex => {
                self.execute_tree_fulltext_index_stage(request, bundle)?;
                store.put_json("index.treeFulltext", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::TreeVectorIndex => {
                self.execute_tree_vector_index_stage(request, compiled, bundle)
                    .await?;
                store.put_json("index.treeVector", json!({"nodeId": node.node_id}));
            }
            IngestStageKind::PersistTree => {
                self.execute_persist_tree_stage(request, bundle, store)?;
            }
            IngestStageKind::PersistSource => {
                self.execute_persist_source_stage(request, compiled, bundle, store)?;
                store.put_json("source.persisted", json!({"nodeId": node.node_id}));
            }
        }
        Ok(RagStageOutcome::completed())
    }

    fn execute_prepare_payload_stage(
        &self,
        request: &RagIngestRequest,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        bundle.validated_source_payload()?;
        publish_ingest_stage_event(
            &self.event_bus,
            "preparePayload",
            BusOutcome::Completed,
            1,
            request,
        );
        Ok(())
    }

    fn execute_validate_source_stage(
        &self,
        request: &RagIngestRequest,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        let payload = bundle.raw_source_payload()?.clone();
        if request.source.source_type == "PROJECT_DOC_REF" {
            validate_project_doc_ref_request(request)?;
        }
        bundle.append(
            request,
            ARTIFACT_VALIDATED_SOURCE,
            RagArtifactPayload::ValidatedSource(payload),
            json!({ "sourceType": request.source.source_type }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "validateSource",
            BusOutcome::Completed,
            1,
            request,
        );
        Ok(())
    }

    async fn execute_resolve_source_stage(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        bundle.validated_source_payload()?;
        validate_project_doc_ref_request(request)?;
        let document_uri = document_uri(request, compiled)?;
        let format = document_format(request, compiled)?;
        if !compiled.supports_document_format(format) {
            return Err(RagError::DocumentFormatUnsupported);
        }
        let call = RagDocumentSourceCall {
            workspace_code: request.workspace_code.clone(),
            user_code: request.access_context.user_code.clone(),
            agent_code: request.access_context.agent_code.clone(),
            document_uri: document_uri.to_string(),
            max_bytes: compiled.source_resolver_max_bytes,
        };
        let resolved = if format == "MARKDOWN" {
            self.embedding_runtime
                .resolve_project_doc_markdown(call)
                .await?
        } else {
            self.embedding_runtime
                .resolve_project_doc_file(call)
                .await?
        };
        bundle.append(
            request,
            ARTIFACT_RESOLVED_DOCUMENT,
            RagArtifactPayload::ResolvedDocument(resolved),
            json!({
                "strategyId": compiled.source_resolver_strategy_id.as_deref(),
                "documentUri": document_uri
            }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "resolveSource",
            BusOutcome::Completed,
            1,
            request,
        );
        Ok(())
    }

    async fn execute_parse_document_stage(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        let format = document_format(request, compiled)?;
        if !compiled.supports_document_format(format) {
            return Err(RagError::DocumentFormatUnsupported);
        }
        let strategy_id = compiled
            .document_parser_strategy_id
            .as_deref()
            .unwrap_or("RAG_PARSE_MARKDOWN_DOCUMENT");
        let resolved = match strategy_id {
            "RAG_PARSE_MARKDOWN_DOCUMENT" => bundle.resolved_document()?.clone(),
            "RAG_PARSE_MINERU_DOCUMENT" => {
                let document_uri = document_uri(request, compiled)?;
                self.embedding_runtime
                    .parse_project_doc_with_mineru(RagDocumentParseCall {
                        workspace_code: request.workspace_code.clone(),
                        user_code: request.access_context.user_code.clone(),
                        agent_code: request.access_context.agent_code.clone(),
                        document_uri: document_uri.to_string(),
                        document_format: format.to_string(),
                        max_bytes: compiled.source_resolver_max_bytes,
                    })
                    .await?
            }
            _ => return Err(RagError::DocumentParseFailed),
        };
        if resolved.content_text.trim().is_empty() {
            return Err(RagError::DocumentParseFailed);
        }
        bundle.append(
            request,
            ARTIFACT_PARSED_DOCUMENT,
            RagArtifactPayload::ParsedDocument(resolved),
            json!({ "format": format }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "parseDocument",
            BusOutcome::Completed,
            1,
            request,
        );
        Ok(())
    }

    fn execute_split_text_stage(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        let prepared = prepare_payload(request, compiled, bundle)?;
        let count = prepared.units.len();
        bundle.append(
            request,
            ARTIFACT_TEXT_UNIT_BATCH,
            RagArtifactPayload::PreparedPayload(prepared),
            json!({ "count": count }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "splitText",
            BusOutcome::Completed,
            count,
            request,
        );
        Ok(())
    }

    fn execute_fulltext_index_stage(
        &self,
        request: &RagIngestRequest,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        let count = bundle.prepared_payload()?.units.len();
        bundle.append(
            request,
            ARTIFACT_FULLTEXT_INDEX_RESULT,
            RagArtifactPayload::FulltextIndexResult {
                text_unit_count: count,
            },
            json!({ "status": "INDEXED" }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "fulltextIndex",
            BusOutcome::Completed,
            count,
            request,
        );
        Ok(())
    }

    async fn execute_vector_index_stage(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        let prepared = bundle.prepared_payload()?.clone();
        let mut warnings = Vec::new();
        let mut embedding_failed = false;
        let mut fatal_error = None;
        let mut text_units = Vec::with_capacity(prepared.units.len());
        for unit in prepared.units {
            match self
                .embedding_runtime
                .embed(RagEmbeddingCall {
                    workspace_code: request.workspace_code.clone(),
                    model_code: compiled.embedding_model_code.clone(),
                    input: unit.index_text.clone(),
                })
                .await
            {
                Ok(result) => text_units.push(unit.with_embedding(
                    result.embedding,
                    result.model_code.as_str(),
                    &compiled.embedding_version,
                )),
                Err(_) if allows_vector_degrade(&compiled.degrade_policy) => {
                    embedding_failed = true;
                    text_units.push(unit.with_failed_embedding());
                }
                Err(_) => {
                    embedding_failed = true;
                    text_units.push(unit.with_failed_embedding());
                    fatal_error = Some(RagError::EmbeddingFailed);
                    break;
                }
            }
        }

        if embedding_failed && fatal_error.is_none() {
            warnings.push(RagIngestWarning {
                stage_id: "vectorIndex".into(),
                code: "RAG_VECTOR_INDEX_WARNING".into(),
                message: "Vector indexing failed; fulltext remains available".into(),
            });
        }
        let embedding_status = if embedding_failed {
            "FAILED"
        } else {
            "INDEXED"
        };
        let count = text_units.len();
        bundle.append(
            request,
            ARTIFACT_VECTOR_INDEX_RESULT,
            RagArtifactPayload::VectorIndexResult(RagVectorIndexArtifact {
                text_units,
                embedding_status: embedding_status.into(),
                warnings,
                fatal_error,
            }),
            json!({ "embeddingStatus": embedding_status }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "vectorIndex",
            if embedding_failed {
                BusOutcome::Failure
            } else {
                BusOutcome::Completed
            },
            count,
            request,
        );
        Ok(())
    }

    fn execute_persist_source_stage(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
        store: &RagArtifactStore,
    ) -> Result<(), RagError> {
        let prepared = bundle.prepared_payload()?.clone();
        let vector_result = bundle.vector_result()?.clone();
        let tree_result = bundle.tree_persist_result().ok().cloned();
        let mut warnings = vector_result.warnings.clone();
        if let Some(tree_result) = &tree_result {
            warnings.extend(tree_result.warnings.clone());
        }
        let source_status = if vector_result.fatal_error.is_some() {
            "FAILED"
        } else {
            "ACTIVE"
        };
        let text_unit_count = vector_result.text_units.len();
        self.repository.persist_ingest(RagPersistIngestCommand {
            corpus_id: request.corpus_id.clone(),
            profile_id: compiled.profile_id.clone(),
            source_code: request.source.code.clone(),
            source_version: request.source.version.clone(),
            source_type: request.source.source_type.clone(),
            scope: request.target_scope.clone(),
            source_status: source_status.into(),
            title: prepared.title,
            content_hash: prepared.content_hash,
            body_hash: prepared.body_hash,
            metadata: prepared.metadata,
            domain_fields: prepared.domain_fields,
            related_graph: store
                .get_optional_cloned::<Value>("graph.related")?
                .or_else(|| request.payload.get("relatedGraph").cloned()),
            graph_write_policy: compiled.graph_write_policy.clone(),
            text_units: vector_result.text_units,
            tree_nodes: tree_result
                .as_ref()
                .map(|result| result.tree_nodes.clone())
                .unwrap_or_default(),
            warnings: warnings.clone(),
            workspace_code: request.workspace_code.clone(),
            user_code: request.access_context.user_code.clone(),
            agent_code: request.access_context.agent_code.clone(),
        })?;
        let result = RagPersistResultArtifact {
            source_status: source_status.into(),
            text_unit_count,
            fulltext_status: "INDEXED".into(),
            embedding_status: vector_result.embedding_status,
            warnings,
            fatal_error: vector_result.fatal_error,
        };
        bundle.append(
            request,
            ARTIFACT_PERSIST_RESULT,
            RagArtifactPayload::PersistResult(result),
            json!({ "sourceStatus": source_status, "textUnitCount": text_unit_count }),
        );
        publish_ingest_stage_event(
            &self.event_bus,
            "persistSource",
            if source_status == "FAILED" {
                BusOutcome::Failure
            } else {
                BusOutcome::Completed
            },
            text_unit_count,
            request,
        );
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum IngestStageKind {
    ValidateSource,
    PreparePayload,
    ResolveSource,
    ParseDocument,
    SplitText,
    FulltextIndex,
    VectorIndex,
    ExtractTree,
    SummarizeTree,
    TreeFulltextIndex,
    TreeVectorIndex,
    PersistTree,
    PersistSource,
}
