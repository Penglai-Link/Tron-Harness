//! Durable computation and graph staging shared by knowledge bases, memories and skills.
//! Parse output -> extraction checkpoints -> scoped facts -> node vectors -> unpublished generation.
use super::*;
use crate::{
    application::rag::*,
    domain::rag::graph::{GraphPartition, RagGraphGeneration, stable_id},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct GraphInfrastructure {
    pub store: Arc<dyn RagGraphStore>,
    pub index: Arc<dyn RagGraphIndexRepository>,
    pub(super) maintenance: Arc<super::cleanup::GraphMaintenanceLifecycle>,
}
impl GraphInfrastructure {
    pub fn new(store: Arc<dyn RagGraphStore>, index: Arc<dyn RagGraphIndexRepository>) -> Self {
        Self {
            store,
            index,
            maintenance: Arc::new(super::cleanup::GraphMaintenanceLifecycle::default()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct GraphIndexContext {
    pub partition: GraphPartition,
    pub logical_source_code: String,
    pub source_code: String,
    pub source_version: String,
    pub task_id: String,
    pub configuration_hash: String,
}
impl GraphIndexContext {
    fn scope(&self) -> String {
        stable_id(
            "GRAPH_CACHE",
            &[&self.partition.key(), &self.logical_source_code],
        )
    }
}
pub(crate) struct GraphComputationCheckpoint<'a> {
    base: &'a dyn RagIndexCheckpoint,
    index: Option<&'a dyn RagGraphIndexRepository>,
    task: String,
    scope: String,
}
impl<'a> GraphComputationCheckpoint<'a> {
    pub(crate) fn new(
        infrastructure: Option<&'a GraphInfrastructure>,
        base: &'a dyn RagIndexCheckpoint,
        context: &GraphIndexContext,
        settings: &RagSettings,
    ) -> Result<Self, String> {
        if settings.graph && infrastructure.is_none() {
            return Err(crate::shared::i18n_message::MessageText::new(
                "rag.graph.storage_unconfigured",
            )
            .encode()
            .into());
        }
        Ok(Self {
            base,
            index: infrastructure.map(|g| g.index.as_ref()),
            task: context.task_id.clone(),
            scope: stable_id(
                "EXTRACTION",
                &[
                    &context.scope(),
                    settings.graph_model_code.as_deref().unwrap_or(""),
                    "graph-extraction-v2",
                ],
            ),
        })
    }
}
impl RagIndexCheckpoint for GraphComputationCheckpoint<'_> {
    fn validate(&self) -> Result<(), String> {
        self.base.validate()
    }
    fn stage(&self, stage: &str) -> Result<(), String> {
        self.base.stage(stage)
    }
    fn save_batch(&self, result: &RagEmbeddingBatchResult) -> Result<(), String> {
        self.base.save_batch(result)
    }
    fn load_graph(&self, key: &str) -> Result<Option<Value>, String> {
        self.validate()?;
        self.index
            .map(|index| {
                index.load_checkpoint(&self.task, &self.scope, &format!("Extraction:{key}"), true)
            })
            .transpose()
            .map(Option::flatten)
    }
    fn load_rejected_graph(&self, key: &str) -> Result<Option<Value>, String> {
        self.validate()?;
        self.index
            .map(|index| {
                index.load_checkpoint(
                    &self.task,
                    &self.scope,
                    &format!("RejectedExtraction:{key}"),
                    false,
                )
            })
            .transpose()
            .map(Option::flatten)
    }
    fn save_rejected_graph(&self, key: &str, value: &Value) -> Result<(), String> {
        self.validate()?;
        if let Some(index) = self.index {
            index.save_checkpoint(
                &self.task,
                &self.scope,
                &format!("RejectedExtraction:{key}"),
                value,
            )?;
        }
        self.validate()
    }
    fn save_graph(&self, key: &str, value: &Value) -> Result<(), String> {
        self.validate()?;
        if let Some(index) = self.index {
            index.save_checkpoint(&self.task, &self.scope, &format!("Extraction:{key}"), value)?;
        }
        self.validate()
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct PinnedNodeModel {
    model: String,
    fingerprint: String,
}
struct NodeVectorCheckpoint<'a> {
    base: &'a dyn RagIndexCheckpoint,
    index: &'a dyn RagGraphIndexRepository,
    context: &'a GraphIndexContext,
    scope: String,
    key: String,
    manifest: Mutex<GraphNodeVectorManifest>,
}
impl NodeVectorCheckpoint<'_> {
    fn save_manifest(&self) -> Result<(), String> {
        self.validate()?;
        let manifest = self.manifest.lock().map_err(|e| e.to_string())?;
        self.index.save_node_vector_checkpoint(
            &self.context.task_id,
            &self.scope,
            &self.key,
            manifest.completed_units,
            &manifest,
            None,
        )?;
        self.validate()
    }
}
impl RagIndexCheckpoint for NodeVectorCheckpoint<'_> {
    fn validate(&self) -> Result<(), String> {
        self.base.validate()
    }
    fn stage(&self, stage: &str) -> Result<(), String> {
        self.base.stage(stage)
    }
    fn save_batch(&self, result: &RagEmbeddingBatchResult) -> Result<(), String> {
        self.validate()?;
        let mut manifest = self.manifest.lock().map_err(|e| e.to_string())?;
        if result.model_code != manifest.model_code
            || (manifest.completed_units > 0 && manifest.provider_model != result.provider_model)
        {
            return Err(crate::shared::i18n_message::MessageText::new(
                "rag.graph.node_vector_model_mismatch",
            )
            .encode()
            .into());
        }
        let values = result
            .vectors
            .iter()
            .map(|v| v.values.clone())
            .collect::<Vec<_>>();
        let batch =
            GraphNodeVectorBatch::encode(manifest.completed_units, &values, manifest.dimension)?;
        let mut next = manifest.clone();
        next.provider_model = result.provider_model.clone();
        next.completed_units += batch.count;
        next.batch_count += 1;
        self.index.save_node_vector_checkpoint(
            &self.context.task_id,
            &self.scope,
            &self.key,
            manifest.completed_units,
            &next,
            Some(&batch),
        )?;
        *manifest = next;
        self.validate()
    }
}
impl<R: Clone, E: RagEmbeddingRuntime> RagService<R, E> {
    /// Publish a tested candidate by copying its verified projections; no model call or reparse.
    pub(crate) fn copy_graph_generation(
        &self,
        workspace: &str,
        origin: &GraphInfrastructure,
        id: &str,
        context: &GraphIndexContext,
        prepared: &RagPreparedIndex,
        checkpoint: &dyn RagIndexCheckpoint,
    ) -> Result<(), String> {
        checkpoint.validate()?;
        let destination = self.graph.as_ref().ok_or(
            crate::shared::i18n_message::MessageText::new("rag.graph.storage_unconfigured")
                .encode(),
        )?;
        let (manifest, projections) = origin.index.export_generation(id)?.ok_or(
            crate::shared::i18n_message::MessageText::new("rag.graph.candidate_missing").encode(),
        )?;
        if manifest.partition != context.partition
            || manifest.source_code != context.source_code
            || manifest.logical_source_code != context.logical_source_code
            || manifest.task_id != context.task_id
            || manifest.source_version != context.source_version
            || manifest.configuration_hash != context.configuration_hash
        {
            return Err(crate::shared::i18n_message::MessageText::new(
                "rag.graph.candidate_source_mismatch",
            )
            .encode()
            .into());
        }
        if self
            .runtime
            .embedding_fingerprint(workspace, Some(&manifest.model_code), manifest.dimension)?
            .as_deref()
            != Some(&manifest.model_fingerprint)
        {
            return Err(crate::shared::i18n_message::MessageText::new(
                "rag.graph.candidate_vector_space_changed",
            )
            .encode()
            .into());
        }
        let (facts, texts) = build_graph_generation(
            RagGraphGeneration {
                partition: context.partition.clone(),
                id: id.into(),
                source_code: context.source_code.clone(),
                source_version: context.source_version.clone(),
                task_id: context.task_id.clone(),
                nodes: vec![],
                edges: vec![],
            },
            &prepared.units,
            prepared.graph.as_ref().ok_or(
                crate::shared::i18n_message::MessageText::new(
                    "rag.graph.candidate_extraction_missing",
                )
                .encode(),
            )?,
        )?;
        let by_id = projections
            .iter()
            .map(|p| (p.entity_id.as_str(), p))
            .collect::<BTreeMap<_, _>>();
        if texts.len() != projections.len()
            || texts.iter().any(|p| {
                by_id
                    .get(p.entity_id.as_str())
                    .is_none_or(|v| v.text != p.text || v.chunk_codes != p.chunk_codes)
            })
        {
            return Err(crate::shared::i18n_message::MessageText::new(
                "rag.graph.candidate_projection_mismatch",
            )
            .encode()
            .into());
        }
        destination.index.begin_generation(&manifest)?;
        destination.store.stage_generation(&facts)?;
        checkpoint.validate()?;
        destination
            .index
            .stage_projections(&manifest, &projections)?;
        checkpoint.validate()
    }
    pub(crate) async fn stage_graph_generation(
        &self,
        workspace: &str,
        settings: &RagSettings,
        context: &GraphIndexContext,
        prepared: &RagPreparedIndex,
        checkpoint: &dyn RagIndexCheckpoint,
    ) -> Result<Option<String>, String> {
        if !settings.graph {
            return Ok(None);
        }
        let infrastructure = self.graph.as_ref().ok_or(
            crate::shared::i18n_message::MessageText::new("rag.graph.storage_unconfigured")
                .encode(),
        )?;
        checkpoint.stage("GRAPH_NODE_EMBEDDING")?;
        let scope = stable_id(
            "GRAPH_COMPUTATION",
            &[&context.scope(), &context.configuration_hash],
        );
        let identity_key = "NodeModel";
        let pinned: PinnedNodeModel = if let Some(saved) =
            infrastructure
                .index
                .load_checkpoint(&context.task_id, &scope, identity_key, false)?
        {
            serde_json::from_value(saved).map_err(|e| e.to_string())?
        } else {
            let model = match prepared.vectors.model_code.clone() {
                Some(model) => model,
                None => self
                    .runtime
                    .resolve_index_model(workspace, settings.embedding_model_code.as_deref())?
                    .ok_or(
                        crate::shared::i18n_message::MessageText::new(
                            "rag.graph.node_vector_model_unconfigured",
                        )
                        .encode(),
                    )?,
            };
            let fingerprint = self
                .runtime
                .embedding_fingerprint(workspace, Some(&model), settings.embedding_dimension)?
                .ok_or(
                    crate::shared::i18n_message::MessageText::new(
                        "rag.graph.node_vector_space_missing",
                    )
                    .encode(),
                )?;
            let pinned = PinnedNodeModel { model, fingerprint };
            infrastructure.index.save_checkpoint(
                &context.task_id,
                &scope,
                identity_key,
                &serde_json::to_value(&pinned).map_err(|e| e.to_string())?,
            )?;
            pinned
        };
        if self
            .runtime
            .embedding_fingerprint(workspace, Some(&pinned.model), settings.embedding_dimension)?
            .as_deref()
            != Some(&pinned.fingerprint)
        {
            return Err(crate::shared::i18n_message::MessageText::new(
                "rag.graph.node_vector_model_changed",
            )
            .encode()
            .into());
        }
        let id = stable_id(
            "GENERATION",
            &[
                "source-structure-v2",
                &context.partition.key(),
                &context.task_id,
                &context.source_code,
                &context.configuration_hash,
                &pinned.fingerprint,
            ],
        );
        let (facts, texts) = build_graph_generation(
            RagGraphGeneration {
                partition: context.partition.clone(),
                id: id.clone(),
                source_code: context.source_code.clone(),
                source_version: context.source_version.clone(),
                task_id: context.task_id.clone(),
                nodes: vec![],
                edges: vec![],
            },
            &prepared.units,
            prepared.graph.as_ref().ok_or(
                crate::shared::i18n_message::MessageText::new(
                    "rag.graph.extraction_checkpoint_missing",
                )
                .encode(),
            )?,
        )?;
        let mut manifest = GraphGenerationManifest {
            id: id.clone(),
            partition: context.partition.clone(),
            logical_source_code: context.logical_source_code.clone(),
            source_code: context.source_code.clone(),
            source_version: context.source_version.clone(),
            task_id: context.task_id.clone(),
            configuration_hash: context.configuration_hash.clone(),
            model_code: pinned.model.clone(),
            model_fingerprint: pinned.fingerprint.clone(),
            provider_model: None,
            dimension: settings.embedding_dimension,
            expected_projections: texts.len(),
        };
        checkpoint.validate()?;
        let inputs = texts.iter().map(|p| p.text.clone()).collect::<Vec<_>>();
        let input_key = stable_id(
            "NodeVectors",
            &[
                &serde_json::to_string(&inputs).map_err(|e| e.to_string())?,
                &pinned.fingerprint,
            ],
        );
        // Lexical switches and task generations do not change node embedding
        // input. Reuse only this source's published cache in the exact model space.
        let vector_scope = stable_id(
            "GRAPH_NODE_VECTORS",
            &[
                &context.scope(),
                &pinned.model,
                &pinned.fingerprint,
                &settings.embedding_dimension.to_string(),
            ],
        );
        // Batch decoding runs outside the async lease future and outside the
        // SQLite worker. No historical vector matrix is cloned per provider call.
        let (index, task, scope, key, model, fingerprint, dimension, count) = (
            infrastructure.index.clone(),
            context.task_id.clone(),
            vector_scope.clone(),
            input_key.clone(),
            pinned.model.clone(),
            pinned.fingerprint.clone(),
            settings.embedding_dimension,
            inputs.len(),
        );
        let restored = tokio::task::spawn_blocking(move || {
            index
                .load_node_vector_checkpoint(&task, &scope, &key)?
                .map(|saved| saved.restore(&model, &fingerprint, dimension, count))
                .transpose()
        })
        .await
        .map_err(|error| {
            format!(
                "{}",
                crate::shared::i18n_message::MessageText::new(
                    "rag.graph.vector_checkpoint_restore_failed"
                )
                .arg("detail", error.to_string())
                .encode()
            )
        })??;
        let (saved, restored_manifest) = restored.unwrap_or_else(|| {
            (
                RagPreparedVectors {
                    model_code: Some(pinned.model.clone()),
                    fingerprint: Some(pinned.fingerprint.clone()),
                    ..Default::default()
                },
                GraphNodeVectorManifest {
                    format_version: 1,
                    storage_task_id: context.task_id.clone(),
                    model_code: pinned.model.clone(),
                    fingerprint: pinned.fingerprint.clone(),
                    provider_model: None,
                    dimension: settings.embedding_dimension,
                    completed_units: 0,
                    batch_count: 0,
                },
            )
        });
        let node_checkpoint = NodeVectorCheckpoint {
            base: checkpoint,
            index: infrastructure.index.as_ref(),
            context,
            scope: vector_scope.clone(),
            key: input_key.clone(),
            manifest: Mutex::new(restored_manifest),
        };
        let vectors = self
            .prepare_vectors(
                workspace,
                Some(&pinned.model),
                &inputs,
                settings.embedding_dimension,
                saved,
                Some(&node_checkpoint),
            )
            .await?;
        // Published cache adoption also copies its batches to this task, so
        // retirement of the former generation cannot break future reuse.
        node_checkpoint.save_manifest()?;
        manifest.provider_model = vectors.provider_model.clone();
        let projections = texts
            .into_iter()
            .zip(vectors.vectors)
            .map(|(p, embedding)| PreparedGraphProjection {
                entity_id: p.entity_id,
                chunk_codes: p.chunk_codes,
                text: p.text,
                embedding,
            })
            .collect::<Vec<_>>();
        checkpoint.stage("GRAPH_STAGING")?;
        infrastructure.index.begin_generation(&manifest)?;
        let store = infrastructure.store.clone();
        // A large durable Grafeo write must not starve the ingestion lease future.
        tokio::task::spawn_blocking(move || store.stage_generation(&facts))
            .await
            .map_err(|error| {
                format!(
                    "{}",
                    crate::shared::i18n_message::MessageText::new(
                        "rag.graph.stage_generation_failed"
                    )
                    .arg("detail", error.to_string())
                    .encode()
                )
            })??;
        checkpoint.validate()?;
        infrastructure
            .index
            .stage_projections(&manifest, &projections)?;
        checkpoint.validate()?;
        Ok(Some(id))
    }
}
