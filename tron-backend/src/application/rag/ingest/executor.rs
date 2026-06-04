use serde_json::{Value, json};

use crate::domain::rag::RagError;

use super::{
    RagEmbeddingCall, RagEmbeddingRuntime, RagIngestRepository, RagIngestRequest, RagIngestService,
    RagIngestWarning, RagPersistIngestCommand, publish_ingest_stage_event,
};
use crate::application::rag::{
    CompiledRagIngestProfile, RagConfigRepository, RagDocumentSourceCall, RagDocumentSourceRuntime,
    RagStrategyRegistry,
    ingest_artifact::{
        ARTIFACT_FULLTEXT_INDEX_RESULT, ARTIFACT_PARSED_DOCUMENT, ARTIFACT_PERSIST_RESULT,
        ARTIFACT_RESOLVED_DOCUMENT, ARTIFACT_TEXT_UNIT_BATCH, ARTIFACT_VALIDATED_SOURCE,
        ARTIFACT_VECTOR_INDEX_RESULT, ArtifactBundle, RagArtifactPayload, RagPersistResultArtifact,
        RagVectorIndexArtifact,
    },
    ingest_payload::prepare_payload,
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
    ) -> Result<RagPersistResultArtifact, RagError> {
        for stage_id in &compiled.stage_order {
            match stage_id.as_str() {
                "validateSource" => self.execute_validate_source_stage(request, bundle)?,
                "preparePayload" => self.execute_prepare_payload_stage(request, bundle)?,
                "resolveSource" => {
                    self.execute_resolve_source_stage(request, compiled, bundle)
                        .await?
                }
                "parseDocument" => self.execute_parse_document_stage(request, compiled, bundle)?,
                "splitText" => self.execute_split_text_stage(request, compiled, bundle)?,
                "fulltextIndex" => self.execute_fulltext_index_stage(request, bundle)?,
                "vectorIndex" => {
                    self.execute_vector_index_stage(request, compiled, bundle)
                        .await?
                }
                "persistSource" => {
                    self.execute_persist_source_stage(request, compiled, bundle)?;
                }
                _ => return Err(RagError::IngestProfileInvalid),
            }
        }
        Ok(bundle.persist_result()?.clone())
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
        let resolved = self
            .embedding_runtime
            .resolve_project_doc_markdown(RagDocumentSourceCall {
                workspace_code: request.workspace_code.clone(),
                user_code: request.access_context.user_code.clone(),
                agent_code: request.access_context.agent_code.clone(),
                document_uri: document_uri.to_string(),
                max_bytes: compiled.source_resolver_max_bytes,
            })
            .await?;
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

    fn execute_parse_document_stage(
        &self,
        request: &RagIngestRequest,
        compiled: &CompiledRagIngestProfile,
        bundle: &mut ArtifactBundle,
    ) -> Result<(), RagError> {
        let format = document_format(request, compiled)?;
        if format != compiled.document_format {
            return Err(RagError::DocumentFormatUnsupported);
        }
        let resolved = bundle.resolved_document()?.clone();
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
                    input: unit.text.clone(),
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
    ) -> Result<(), RagError> {
        let prepared = bundle.prepared_payload()?.clone();
        let vector_result = bundle.vector_result()?.clone();
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
            related_graph: request.payload.get("relatedGraph").cloned(),
            graph_write_policy: compiled.graph_write_policy.clone(),
            text_units: vector_result.text_units,
            warnings: vector_result.warnings.clone(),
            workspace_code: request.workspace_code.clone(),
            user_code: request.access_context.user_code.clone(),
            agent_code: request.access_context.agent_code.clone(),
        })?;
        let result = RagPersistResultArtifact {
            source_status: source_status.into(),
            text_unit_count,
            fulltext_status: "INDEXED".into(),
            embedding_status: vector_result.embedding_status,
            warnings: vector_result.warnings,
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
