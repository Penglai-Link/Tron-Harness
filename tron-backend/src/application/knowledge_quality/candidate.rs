use super::*;
use super::{
    runtime::{BudgetState, QualityCheckpoint, QualityRuntime},
    snapshot::QualitySnapshot,
};
use crate::{
    adapters::outbound::persistence::sqlite::{
        SqliteRagRepository, StoredChunkContext, stored_contexts,
    },
    application::{
        document::{ChunkConfig, LocatedChunk, split_document_with_strategies},
        knowledge_base::{
            chunk_context::{ContextKey, expand_result},
            hash,
        },
        rag::*,
    },
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub(crate) type ExperimentRag = RagService<Arc<Mutex<SqliteRagRepository>>, QualityRuntime>;

pub(crate) struct TestedDocument {
    pub original: crate::application::knowledge_base::KnowledgeBaseDocument,
    pub chunks: Vec<LocatedChunk>,
    pub contexts: Vec<StoredChunkContext>,
    pub prepared: RagPreparedIndex,
    pub source: String,
    pub task_id: String,
    pub graph_id: Option<String>,
    pub graph_context: crate::application::rag::graph::GraphIndexContext,
}
pub(crate) struct CandidateIndex {
    pub rag: ExperimentRag,
    pub documents: Vec<TestedDocument>,
    pub contexts: BTreeMap<ContextKey, StoredChunkContext>,
    pub ranges: Vec<AuthorizedRagRange>,
    pub settings: RagSettings,
    pub chunk_count: usize,
    pub warnings: Vec<String>,
    pub _directory: tempfile::TempDir,
}
#[derive(Default)]
pub(crate) struct ReusedVectors {
    values: BTreeMap<String, Vec<f32>>,
}

pub(crate) fn pinned_settings(
    snapshot: &QualitySnapshot,
) -> Result<RagSettings, KnowledgeBaseError> {
    let mut settings = snapshot.settings.clone();
    if settings.vector {
        let first = &snapshot.documents[0].document;
        if first.embedding_model_code.is_none()
            || first.embedding_dimension != Some(settings.embedding_dimension)
            || first.embedding_space_fingerprint.is_none()
            || snapshot.documents.iter().any(|doc| {
                doc.document.embedding_model_code != first.embedding_model_code
                    || doc.document.embedding_dimension != first.embedding_dimension
                    || doc.document.embedding_space_fingerprint != first.embedding_space_fingerprint
                    || doc.document.embedding_provider_model != first.embedding_provider_model
            })
        {
            return Err(invalid(
                "knowledge.quality.diagnostic.embedding_space_mismatch",
            ));
        }
        settings.embedding_model_code = first.embedding_model_code.clone();
    }
    Ok(settings)
}

pub(crate) fn ranges(
    snapshot: &QualitySnapshot,
    settings: &RagSettings,
    sources: Vec<String>,
    vectors: Option<&RagPreparedVectors>,
) -> Vec<AuthorizedRagRange> {
    let first = &snapshot.documents[0].document;
    vec![AuthorizedRagRange {
        corpus_id: snapshot.library.id.clone(),
        scopes: vec![RagSearchScope {
            scope_type: "SPACE".into(),
            scope_code: "PLATFORM".into(),
        }],
        source_version: "v1".into(),
        filters: serde_json::json!({"sourceCode":{"op":"in","value":sources}}),
        embedding_model_code: settings.embedding_model_code.clone(),
        embedding_dimension: settings.vector.then_some(settings.embedding_dimension),
        provider_model: if settings.vector {
            vectors
                .and_then(|v| v.provider_model.clone())
                .or_else(|| first.embedding_provider_model.clone())
        } else {
            None
        },
        embedding_fingerprint: if settings.vector {
            vectors
                .and_then(|v| v.fingerprint.clone())
                .or_else(|| first.embedding_space_fingerprint.clone())
        } else {
            None
        },
        return_fields: vec![],
        fulltext: settings.fulltext,
        vector: settings.vector,
        graph: settings.graph,
    }]
}

impl KnowledgeQualityService {
    pub(crate) async fn build_candidate(
        &self,
        user: &str,
        run: &QualityOptimizationRun,
        snapshot: &QualitySnapshot,
        config: &ChunkConfig,
        trial: usize,
        budget: &Arc<BudgetState>,
        reuse: &mut ReusedVectors,
    ) -> Result<CandidateIndex, KnowledgeBaseError> {
        let settings = pinned_settings(snapshot)?;
        let directory = tempfile::Builder::new()
            .prefix("tron-retrieval-evaluation-")
            .tempdir()
            .map_err(invalid_error)?;
        let repo = Arc::new(Mutex::new(SqliteRagRepository::connect(
            directory
                .path()
                .join("candidate.sqlite3")
                .to_str()
                .ok_or_else(|| invalid("knowledge.quality.temporary_path_invalid"))?,
        )?));
        let mut corpus = snapshot.corpus.clone();
        corpus.capabilities["settings"] = serde_json::to_value(&settings).map_err(invalid_error)?;
        repo.save_managed_corpus(&corpus)?;
        let runtime = QualityRuntime {
            inner: self.host.rag.runtime_handle(),
            budget: budget.clone(),
        };
        let graph_core = crate::adapters::outbound::persistence::GrafeoStoreCore::connect(
            crate::adapters::outbound::persistence::GrafeoAdapterConfig {
                storage_path: directory
                    .path()
                    .join("candidate.grafeo")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .map_err(invalid_error)?;
        let graph_index =
            crate::adapters::outbound::persistence::SqliteRagGraphIndexRepository::open(
                directory
                    .path()
                    .join("candidate.sqlite3")
                    .to_str()
                    .ok_or_else(|| invalid("knowledge.quality.candidate_path_invalid"))?,
            )
            .map_err(invalid_error)?;
        let graph = crate::application::rag::graph::GraphInfrastructure::new(
            Arc::new(crate::adapters::outbound::persistence::GrafeoRagStore::new(
                graph_core,
            )),
            Arc::new(graph_index),
        );
        let rag = self
            .host
            .rag
            .with_runtime(runtime)
            .with_repository(repo.clone())
            .with_graph_infrastructure(graph);
        if settings.vector {
            let actual = RagEmbeddingRuntime::embedding_fingerprint(
                &rag.runtime_handle(),
                "PLATFORM",
                settings.embedding_model_code.as_deref(),
                settings.embedding_dimension,
            )
            .map_err(invalid_error)?;
            if actual != snapshot.documents[0].document.embedding_space_fingerprint {
                return Err(invalid("knowledge.quality.candidate_vector_model_changed"));
            }
        }
        let mut documents = Vec::new();
        let mut contexts = BTreeMap::new();
        let mut chunk_count = 0;
        let mut warnings = Vec::new();
        for document in &snapshot.documents {
            budget.check().map_err(invalid_error)?;
            let mut chunked = split_document_with_strategies(
                &document.parsed,
                config,
                &self.host.chunk_strategies,
                &document.document.id,
                &document.document.file_name,
            )
            .map_err(|error| {
                invalid_with(
                    crate::shared::i18n_message::MessageText::new(
                        "knowledge.quality.chunk_strategy_failed",
                    )
                    .arg("error", error.to_string()),
                )
            })?;
            if settings.vector
                && self
                    .host
                    .llm
                    .embedding_accepts_images(
                        "PLATFORM",
                        settings.embedding_model_code.as_deref().ok_or_else(|| {
                            invalid("knowledge.quality.candidate_vector_model_unpinned")
                        })?,
                    )
                    .map_err(invalid_error)?
            {
                crate::application::knowledge_base::image_index::append_image_chunks(
                    &document.parsed,
                    &document.document.file_name,
                    &mut chunked.chunks,
                )?;
            }
            warnings.extend(chunked.warnings.iter().map(|warning| {
                crate::shared::i18n_message::MessageText::new("knowledge.quality.candidate_warning")
                    .arg("number", (trial + 1).to_string())
                    .arg("warning", warning.clone())
                    .encode()
            }));
            if chunked.chunks.is_empty() {
                return Err(invalid("knowledge.quality.candidate_no_chunks"));
            }
            chunk_count += chunked.chunks.len();
            if chunk_count > 2000 {
                return Err(invalid("knowledge.quality.candidate_chunk_limit"));
            }
            let task_id = format!("quality_{}_{}_{}", run.id, trial, document.document.id);
            let source = format!("{}_{}_1", document.document.id, task_id);
            let units = chunked
                .chunks
                .iter()
                .map(|chunk| unit(&source, &document.document, chunk))
                .collect::<Vec<_>>();
            let vectors = if settings.vector {
                let model = settings
                    .embedding_model_code
                    .as_deref()
                    .ok_or_else(|| invalid("knowledge.quality.candidate_vector_model_unpinned"))?;
                let inputs =
                    self.host
                        .index_embedding_inputs(user, "PLATFORM", &chunked.chunks, model)?;
                let fingerprint = document
                    .document
                    .embedding_space_fingerprint
                    .as_deref()
                    .ok_or_else(|| invalid("knowledge.quality.candidate_vector_space_missing"))?;
                // Reuse image vectors by the exact media input hash, never by its display text.
                if let Some(source) = document.document.active_source_code.as_deref() {
                    reuse
                        .values
                        .extend(self.host.rag.repository_handle().cached_vectors(
                            &snapshot.library.id,
                            source,
                            "PLATFORM",
                            fingerprint,
                            settings.embedding_dimension,
                        )?);
                }
                let completed = RagPreparedVectors {
                    model_code: Some(model.into()),
                    provider_model: document.document.embedding_provider_model.clone(),
                    fingerprint: Some(fingerprint.into()),
                    vectors: vec![],
                };
                let prepared = rag
                    .prepare_content_vectors(
                        "PLATFORM",
                        Some(model),
                        &inputs,
                        settings.embedding_dimension,
                        completed,
                        &reuse.values,
                        Some(&QualityCheckpoint(budget.clone())),
                    )
                    .await
                    .map_err(invalid_error)?;
                if prepared.fingerprint.as_deref() != Some(fingerprint)
                    || prepared.model_code.as_deref() != Some(model)
                    || prepared.provider_model != document.document.embedding_provider_model
                {
                    return Err(invalid("knowledge.quality.candidate_model_changed"));
                }
                for (input, vector) in inputs.iter().zip(&prepared.vectors) {
                    reuse
                        .values
                        .insert(embedding_input_hash(input), vector.clone());
                }
                prepared
            } else {
                Default::default()
            };
            let configuration = quality_configuration(
                config,
                &run.id,
                &settings,
                &document.document,
                vectors.model_code.as_deref(),
                vectors.fingerprint.as_deref(),
            );
            let graph_context = crate::application::rag::graph::GraphIndexContext {
                partition: crate::domain::rag::graph::GraphPartition {
                    corpus_id: snapshot.library.id.clone(),
                    scope_type: "SPACE".into(),
                    scope_code: "PLATFORM".into(),
                },
                logical_source_code: document.document.id.clone(),
                source_code: source.clone(),
                source_version: "v1".into(),
                task_id: task_id.clone(),
                configuration_hash: hash(&configuration.to_string()),
            };
            let quality_checkpoint = QualityCheckpoint(budget.clone());
            let graph_checkpoint = crate::application::rag::graph::GraphComputationCheckpoint::new(
                rag.graph_infrastructure(),
                &quality_checkpoint,
                &graph_context,
                &settings,
            )
            .map_err(invalid_error)?;
            let prepared = rag
                .prepare_index(
                    "PLATFORM",
                    &settings,
                    units,
                    vectors,
                    None,
                    &graph_checkpoint,
                )
                .await
                .map_err(invalid_error)?;
            let graph_id = rag
                .stage_graph_generation(
                    "PLATFORM",
                    &settings,
                    &graph_context,
                    &prepared,
                    &graph_checkpoint,
                )
                .await
                .map_err(invalid_error)?;
            let stored = stored_contexts(&chunked.contexts, &chunked.chunks);
            for context in &stored {
                contexts.insert(
                    (
                        snapshot.library.id.clone(),
                        source.clone(),
                        context.id.clone(),
                    ),
                    context.clone(),
                );
            }
            repo.persist_ingest(persist_command(
                user,
                &snapshot.library.id,
                &document.document,
                &source,
                &task_id,
                &settings,
                &prepared,
                "ACTIVE",
                graph_id.as_deref().map(|id| (id, &graph_context)),
            ))?;
            documents.push(TestedDocument {
                original: document.document.clone(),
                chunks: chunked.chunks,
                contexts: stored,
                prepared,
                source,
                task_id,
                graph_id,
                graph_context,
            });
        }
        let sources = documents.iter().map(|doc| doc.source.clone()).collect();
        let query_ranges = ranges(
            snapshot,
            &settings,
            sources,
            documents.first().map(|doc| &doc.prepared.vectors),
        );
        Ok(CandidateIndex {
            rag,
            documents,
            contexts,
            ranges: query_ranges,
            settings,
            chunk_count,
            warnings,
            _directory: directory,
        })
    }
}

fn unit(
    source: &str,
    document: &crate::application::knowledge_base::KnowledgeBaseDocument,
    chunk: &LocatedChunk,
) -> RagTextUnitWrite {
    RagTextUnitWrite {
        text_unit_code: format!("{source}_{}", chunk.ordinal),
        unit_type: if chunk.location.block_kind.as_deref() == Some("image_vector") {
            "IMAGE"
        } else {
            "LEAF"
        }
        .into(),
        text: chunk.text.clone(),
        search_text: chunk.index_text.clone(),
        heading_path: serde_json::to_string(&chunk.heading_path).unwrap_or_default(),
        ordinal: chunk.ordinal,
        token_count: eliteforge_agent_core::Content::from(chunk.index_text.clone())
            .estimated_tokens() as i64,
        content_hash: hash(&chunk.text),
        status: "ACTIVE".into(),
        fulltext_status: String::new(),
        embedding_model_code: None,
        embedding_version: None,
        embedding_text_hash: None,
        embedding_dimension: None,
        embedding: None,
        embedding_status: String::new(),
        metadata: serde_json::json!({"documentId":document.id,"resourceCode":document.resource_code,"resourceRevision":document.resource_revision,"location":chunk.location,"fileName":document.file_name,"sourceFactsVersion":"normalized-blocks-v3","sourceSpans":chunk.source_spans,"sourceLocations":chunk.source_locations,"contextId":chunk.context_id}),
    }
}
pub(crate) fn quality_configuration(
    config: &ChunkConfig,
    run: &str,
    settings: &RagSettings,
    document: &crate::application::knowledge_base::KnowledgeBaseDocument,
    model: Option<&str>,
    fingerprint: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({"chunkConfig":config,"optimizationRunId":run,"ragSettings":settings,"parsingTier":document.parsing_tier,"ocrMode":document.ocr_mode,
        "parsePlan":document.parse_plan,"embeddingModel":model,"embeddingFingerprint":fingerprint})
}

pub(crate) fn persist_command(
    user: &str,
    kb: &str,
    document: &crate::application::knowledge_base::KnowledgeBaseDocument,
    source: &str,
    task: &str,
    settings: &RagSettings,
    prepared: &RagPreparedIndex,
    status: &str,
    graph_binding: Option<(&str, &crate::application::rag::graph::GraphIndexContext)>,
) -> RagPersistIngestCommand {
    RagPersistIngestCommand {
        corpus_id: kb.into(),
        task_id: task.into(),
        source_code: source.into(),
        source_version: "v1".into(),
        scope: RagIngestScope {
            scope_type: "SPACE".into(),
            scope_code: "PLATFORM".into(),
        },
        source_status: status.into(),
        title: Some(document.name.clone()),
        content_hash: hash(source),
        body_hash: None,
        metadata: serde_json::json!({"documentId":document.id,"resourceCode":document.resource_code,"resourceRevision":document.resource_revision,"embeddingProviderModel":prepared.vectors.provider_model,"embeddingFingerprint":prepared.vectors.fingerprint,"indexSettings":settings,"graphGenerationId":graph_binding.map(|b|b.0),"graphConfigurationHash":graph_binding.map(|b|b.1.configuration_hash.as_str()),"logicalSourceCode":document.id}),
        domain_fields: Default::default(),
        text_units: prepared.units.clone(),
        tree_nodes: vec![],
        warnings: vec![],
        workspace_code: "PLATFORM".into(),
        user_code: user.into(),
        agent_code: String::new(),
    }
}

pub(crate) struct ExperimentRunner<'a> {
    pub service: ExperimentRag,
    pub host: Option<&'a KnowledgeBaseService>,
    pub user: &'a str,
    pub snapshot: &'a QualitySnapshot,
    pub ranges: Vec<AuthorizedRagRange>,
    pub contexts: Option<&'a BTreeMap<ContextKey, StoredChunkContext>>,
    pub rerank: Option<String>,
    pub budget: Arc<BudgetState>,
}
#[async_trait::async_trait]
impl RetrievalEvaluationRunner for ExperimentRunner<'_> {
    async fn retrieve(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<RetrievedEvidence>, RetrievalEvaluationError> {
        self.budget.check().map_err(RetrievalEvaluationError)?;
        let mut result = self
            .service
            .recall_authorized("PLATFORM", query, &self.ranges, limit, None)
            .await
            .map_err(|error| RetrievalEvaluationError(error.to_string()))?;
        self.service
            .assemble_source_bundles(&self.ranges, &mut result);
        if let Some(host) = self.host {
            host.expand_context_evidence(self.user, &mut result)
                .map_err(|error| RetrievalEvaluationError(error.to_string()))?;
        }
        if let Some(contexts) = self.contexts {
            expand_result(&mut result, contexts);
        }
        let result = self
            .service
            .rank_evidence("PLATFORM", query, result, limit, self.rerank.as_deref())
            .await
            .map_err(|error| RetrievalEvaluationError(error.to_string()))?;
        if let Some(stage) = result.stages.iter().find(|stage| stage.status == "FAILED") {
            return Err(RetrievalEvaluationError(
                crate::shared::i18n_message::MessageText::new(
                    "knowledge.quality.retrieval_stage_failed",
                )
                .arg("stage", &stage.channel)
                .arg("detail", stage.error.as_deref().unwrap_or_default())
                .encode(),
            ));
        }
        let mut evidence = Vec::new();
        for mut hit in result.hits {
            let members = match hit.bundle.take() {
                Some(bundle) if !bundle.display.is_empty() => bundle.display,
                Some(bundle) => std::iter::once(hit).chain(bundle.members).collect(),
                None => vec![hit],
            };
            let mut documents = BTreeMap::<String, Vec<ChunkSourceSpan>>::new();
            for hit in members {
                let document_id = hit.row.text_metadata["documentId"]
                    .as_str()
                    .ok_or_else(|| {
                        RetrievalEvaluationError(
                            "knowledge.quality.evidence_document_missing".into(),
                        )
                    })?
                    .to_owned();
                let raw = hit
                    .row
                    .text_metadata
                    .get("contextSpans")
                    .or_else(|| hit.row.text_metadata.get("sourceSpans"));
                let spans: Vec<ChunkSourceSpan> = match raw.filter(|raw| {
                    hit.row.text_metadata["sourceFactsVersion"] == "normalized-blocks-v3"
                        && raw.as_array().is_some_and(|rows| !rows.is_empty())
                }) {
                    Some(raw) => serde_json::from_value(raw.clone()).map_err(|_| {
                        RetrievalEvaluationError(
                            "knowledge.quality.evidence_range_invalid_retrieval".into(),
                        )
                    })?,
                    None if hit.row.text_metadata["location"]["blockKind"] == "image_vector" => {
                        continue;
                    }
                    None => legacy_spans(self.snapshot, &document_id, hit.text())?,
                };
                documents.entry(document_id).or_default().extend(spans);
            }
            for (document_id, mut spans) in documents {
                spans.sort_by(|a, b| {
                    (&a.block_id, a.start, a.end).cmp(&(&b.block_id, b.start, b.end))
                });
                let mut merged: Vec<ChunkSourceSpan> = Vec::new();
                for span in spans {
                    if let Some(last) = merged
                        .last_mut()
                        .filter(|last| last.block_id == span.block_id && span.start <= last.end)
                    {
                        last.end = last.end.max(span.end);
                    } else {
                        merged.push(span);
                    }
                }
                evidence.push(RetrievedEvidence {
                    document_id,
                    spans: merged,
                });
            }
        }
        Ok(evidence)
    }
}

fn legacy_spans(
    snapshot: &QualitySnapshot,
    document_id: &str,
    text: &str,
) -> Result<Vec<ChunkSourceSpan>, RetrievalEvaluationError> {
    let document = snapshot
        .documents
        .iter()
        .find(|document| document.document.id == document_id)
        .ok_or_else(|| {
            RetrievalEvaluationError("knowledge.quality.evidence_foreign_document".into())
        })?;
    let body = document
        .input
        .blocks
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let matches = body.match_indices(text).collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(RetrievalEvaluationError(
            "knowledge.quality.chunks_missing_source".into(),
        ));
    }
    let start = body[..matches[0].0].chars().count();
    let end = start + text.chars().count();
    let mut offset = 0;
    let mut spans = Vec::new();
    for block in &document.input.blocks {
        let finish = offset + block.text.chars().count();
        let a = start.max(offset);
        let b = end.min(finish);
        if a < b {
            spans.push(ChunkSourceSpan {
                block_id: block.id.clone(),
                start: a - offset,
                end: b - offset,
            });
        }
        offset = finish + 1;
    }
    Ok(spans)
}
