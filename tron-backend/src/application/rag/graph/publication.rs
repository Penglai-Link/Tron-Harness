//! Publication metadata and rebuildable search projections, never adjacency facts.
use crate::domain::rag::graph::GraphPartition;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphGenerationManifest {
    pub id: String,
    pub partition: GraphPartition,
    pub logical_source_code: String,
    pub source_code: String,
    pub source_version: String,
    pub task_id: String,
    pub configuration_hash: String,
    pub model_code: String,
    pub model_fingerprint: String,
    pub provider_model: Option<String>,
    pub dimension: usize,
    pub expected_projections: usize,
}
impl GraphGenerationManifest {
    pub fn validate(&self) -> Result<(), String> {
        if [
            &self.id,
            &self.partition.corpus_id,
            &self.partition.scope_type,
            &self.partition.scope_code,
            &self.logical_source_code,
            &self.source_code,
            &self.source_version,
            &self.task_id,
            &self.configuration_hash,
            &self.model_code,
            &self.model_fingerprint,
        ]
        .iter()
        .any(|v| v.trim().is_empty())
            || !(1..=8192).contains(&self.dimension)
        {
            return Err("graph generation model, configuration or identity is invalid".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct PreparedGraphProjection {
    pub entity_id: String,
    pub chunk_codes: Vec<String>,
    pub text: String,
    pub embedding: Vec<f32>,
}
impl PreparedGraphProjection {
    pub fn validate(&self, dimension: usize) -> Result<(), String> {
        if self.entity_id.trim().is_empty()
            || self.chunk_codes.is_empty()
            || self.chunk_codes.iter().any(|c| c.is_empty())
            || self.text.trim().is_empty()
            || self.embedding.len() != dimension
            || self.embedding.iter().any(|v| !v.is_finite())
        {
            return Err(
                "graph node projection is incomplete or has invalid vector geometry".into(),
            );
        }
        Ok(())
    }
}
pub trait RagGraphIndexRepository: Send + Sync {
    /// Atomically claim an unpublished generation and fence staging/publication.
    fn claim_cleanup(
        &self,
        now: i64,
        lease_ms: i64,
    ) -> Result<Option<super::GraphCleanupClaim>, String>;
    /// Called after durable graph deletion; a stale lease cannot finish or reschedule another worker's job.
    fn finish_cleanup(
        &self,
        claim: &super::GraphCleanupClaim,
        now: i64,
        failure: Option<&str>,
    ) -> Result<(), String>;
    fn sweep_computation(
        &self,
        now: i64,
        limit: usize,
    ) -> Result<super::GraphHousekeepingResult, String>;

    fn save_snapshot(
        &self,
        draft: super::GraphSnapshotDraft,
        now: i64,
    ) -> Result<super::GraphQuerySnapshot, super::GraphSnapshotError>;
    /// Callers must also re-resolve business grants before traversing this state.
    fn load_snapshot(
        &self,
        identity_hash: &str,
        cursor: &str,
        now: i64,
    ) -> Result<super::GraphQuerySnapshot, super::GraphSnapshotError>;
    /// Generations are resolved from the caller's trusted source authorization.
    /// The adapter additionally checks that each generation is still activated.
    fn search_nodes(&self, query: GraphNodeSearchQuery) -> Result<GraphNodeSearchPage, String>;
    fn load_checkpoint(
        &self,
        task_id: &str,
        cache_scope: &str,
        key: &str,
        reuse_published: bool,
    ) -> Result<Option<serde_json::Value>, String>;
    fn save_checkpoint(
        &self,
        task_id: &str,
        cache_scope: &str,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), String>;
    /// Restore one contiguous prefix and attach published reuse to the requesting task.
    fn load_node_vector_checkpoint(
        &self,
        task_id: &str,
        cache_scope: &str,
        key: &str,
    ) -> Result<Option<super::GraphNodeVectorCheckpoint>, String>;
    /// The immutable batch and its small advancing manifest commit atomically.
    fn save_node_vector_checkpoint(
        &self,
        task_id: &str,
        cache_scope: &str,
        key: &str,
        expected_completed: usize,
        manifest: &super::GraphNodeVectorManifest,
        batch: Option<&super::GraphNodeVectorBatch>,
    ) -> Result<(), String>;
    fn export_generation(
        &self,
        id: &str,
    ) -> Result<Option<(GraphGenerationManifest, Vec<PreparedGraphProjection>)>, String>;
    fn begin_generation(&self, manifest: &GraphGenerationManifest) -> Result<(), String>;
    /// Called only after durable Grafeo staging; FTS and vectors must both succeed.
    fn stage_projections(
        &self,
        manifest: &GraphGenerationManifest,
        projections: &[PreparedGraphProjection],
    ) -> Result<(), String>;
    fn active_generations(
        &self,
        partitions: &[GraphPartition],
    ) -> Result<Vec<GraphGenerationManifest>, String>;
}

#[derive(Clone, Debug)]
pub struct GraphNodeSearchQuery {
    pub generation_ids: Vec<String>,
    pub matching: GraphNodeMatch,
    pub limit: usize,
}

#[derive(Clone, Debug)]
pub enum GraphNodeMatch {
    /// Published, authorized entity samples for browsing without model calls.
    Browse {
        excluded_ids: BTreeSet<String>,
    },
    Fulltext(String),
    Vector {
        embedding: Vec<f32>,
        model_code: String,
        model_fingerprint: String,
        provider_model: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNodeSearchHit {
    pub entity_id: String,
    pub generation_id: String,
    pub chunk_codes: Vec<String>,
    pub score: f64,
}

#[derive(Clone, Debug, Default)]
pub struct GraphNodeSearchPage {
    pub hits: Vec<GraphNodeSearchHit>,
    pub has_more: bool,
    /// Native KNN has a bounded candidate window. Callers must report this limit.
    pub candidate_limit_reached: bool,
}
