//! Shared vector preparation. Publication happens only after all configured indexes succeed.
mod content;
mod extraction;
use super::{RagEmbeddingBatchResult, RagEmbeddingRuntime, RagService};
pub use content::embedding_input_hash;

pub trait RagIndexCheckpoint: Send + Sync {
    fn validate(&self) -> Result<(), String>;
    fn save_batch(&self, result: &RagEmbeddingBatchResult) -> Result<(), String>;
    fn load_graph(&self, _key: &str) -> Result<Option<serde_json::Value>, String> {
        Ok(None)
    }
    fn save_graph(&self, _key: &str, _graph: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }
    /// Rejected output is diagnostic input for bounded repair, never a valid extraction cache.
    fn load_rejected_graph(&self, _key: &str) -> Result<Option<serde_json::Value>, String> {
        Ok(None)
    }
    fn save_rejected_graph(&self, _key: &str, _value: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }
    fn stage(&self, _stage: &str) -> Result<(), String> {
        self.validate()
    }
}

/// Input count and conservative token budget for one provider request.
/// Content::estimated_tokens uses UTF-8 byte length, matching Framework admission.
#[derive(Clone, Copy, Debug)]
pub struct RagEmbeddingBatchLimits {
    pub max_inputs: usize,
    pub max_tokens: usize,
}
impl Default for RagEmbeddingBatchLimits {
    fn default() -> Self {
        let config = crate::config::EmbeddingInvocationConfig::default();
        Self {
            max_inputs: config.max_batch_inputs,
            max_tokens: config.max_batch_tokens,
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RagPreparedVectors {
    pub vectors: Vec<Vec<f32>>,
    pub provider_model: Option<String>,
    pub model_code: Option<String>,
    pub fingerprint: Option<String>,
}

impl<R: Clone, E: RagEmbeddingRuntime> RagService<R, E> {
    pub async fn prepare_vectors(
        &self,
        workspace: &str,
        model: Option<&str>,
        inputs: &[String],
        dimension: usize,
        completed: RagPreparedVectors,
        checkpoint: Option<&dyn RagIndexCheckpoint>,
    ) -> Result<RagPreparedVectors, String> {
        let contents = inputs
            .iter()
            .cloned()
            .map(eliteforge_agent_framework::Content::from)
            .collect::<Vec<_>>();
        self.prepare_content_vectors(
            workspace,
            model,
            &contents,
            dimension,
            completed,
            &Default::default(),
            checkpoint,
        )
        .await
    }
}

fn valid(values: &[f32], dimension: usize) -> bool {
    values.len() == dimension && values.iter().all(|value| value.is_finite())
}

impl<R: Clone, E: super::RagGraphRuntime> RagService<R, E> {
    pub async fn prepare_graph(
        &self,
        workspace: &str,
        model: &str,
        units: &[super::RagTextUnitWrite],
        checkpoint: Option<&dyn RagIndexCheckpoint>,
    ) -> Result<serde_json::Value, String> {
        self.prepare_graph_with_cache(workspace, model, units, checkpoint, &Default::default())
            .await
    }
    pub async fn prepare_graph_with_cache(
        &self,
        workspace: &str,
        model: &str,
        units: &[super::RagTextUnitWrite],
        checkpoint: Option<&dyn RagIndexCheckpoint>,
        cached: &std::collections::BTreeMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let mut evidence = serde_json::Map::new();
        use std::{future::Future, task::Poll};
        let prepare = |unit| {
            Box::pin(extraction::prepare_checkpointed_unit(
                &self.runtime,
                workspace,
                model,
                Some(unit),
                checkpoint,
                cached,
            ))
        };
        let mut remaining = units.iter();
        let mut pending = remaining
            .by_ref()
            .take(extraction::EXTRACTION_PARALLELISM)
            .map(prepare)
            .collect::<Vec<_>>();
        let mut failure = None;
        while !pending.is_empty() {
            // The pool is bounded. Refill a completed slot immediately
            // instead of idling behind the slowest request in a fixed batch.
            let (index, result) = std::future::poll_fn(|cx| {
                for (index, future) in pending.iter_mut().enumerate() {
                    if let Poll::Ready(result) = future.as_mut().poll(cx) {
                        return Poll::Ready((index, result));
                    }
                }
                Poll::Pending
            })
            .await;
            drop(pending.swap_remove(index));
            match result {
                Ok(Some((code, graph))) => {
                    evidence.insert(code, graph);
                }
                Ok(None) => {}
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
            // Stop starting work after a terminal failure, but settle every paid
            // request already in flight: each peer saves its own checkpoint.
            if failure.is_none() {
                if let Some(unit) = remaining.next() {
                    pending.push(prepare(unit));
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(serde_json::Value::Object(evidence))
    }
}

#[derive(Clone, Debug)]
pub struct RagPreparedIndex {
    pub units: Vec<super::RagTextUnitWrite>,
    pub vectors: RagPreparedVectors,
    pub graph: Option<serde_json::Value>,
}

impl<R: Clone, E: RagEmbeddingRuntime + super::RagGraphRuntime> RagService<R, E> {
    /// Every source enters this executor after the public reader has normalized it.
    /// The checkpoint fences cancellation/configuration changes before each stage.
    pub async fn prepare_index(
        &self,
        workspace: &str,
        settings: &super::RagSettings,
        units: Vec<super::RagTextUnitWrite>,
        completed: RagPreparedVectors,
        structured_graph: Option<serde_json::Value>,
        checkpoint: &dyn RagIndexCheckpoint,
    ) -> Result<RagPreparedIndex, String> {
        self.prepare_index_with_content(
            workspace,
            settings,
            units,
            completed,
            structured_graph,
            checkpoint,
            None,
            &Default::default(),
            &Default::default(),
        )
        .await
    }

    pub async fn prepare_index_with_content(
        &self,
        workspace: &str,
        settings: &super::RagSettings,
        mut units: Vec<super::RagTextUnitWrite>,
        completed: RagPreparedVectors,
        structured_graph: Option<serde_json::Value>,
        checkpoint: &dyn RagIndexCheckpoint,
        inputs: Option<Vec<eliteforge_agent_framework::Content>>,
        cached: &std::collections::BTreeMap<String, Vec<f32>>,
        cached_graphs: &std::collections::BTreeMap<String, serde_json::Value>,
    ) -> Result<RagPreparedIndex, String> {
        settings.validate().map_err(|error| error.to_string())?;
        if units.is_empty() || units.iter().any(|unit| unit.text.trim().is_empty()) {
            return Err("没有可入库正文".into());
        }
        checkpoint.stage("INDEXING")?;
        let vectors = if settings.vector {
            checkpoint.stage("EMBEDDING")?;
            let contents = inputs.unwrap_or_else(|| {
                units
                    .iter()
                    .map(|unit| eliteforge_agent_framework::Content::from(unit.search_text.clone()))
                    .collect()
            });
            if contents.len() != units.len() {
                return Err("索引输入数量与分块不一致".into());
            }
            for (unit, input) in units.iter_mut().zip(&contents) {
                unit.metadata["embeddingInputHash"] =
                    serde_json::json!(embedding_input_hash(input));
            }
            self.prepare_content_vectors(
                workspace,
                settings.embedding_model_code.as_deref(),
                &contents,
                settings.embedding_dimension,
                completed,
                cached,
                Some(checkpoint),
            )
            .await?
        } else {
            RagPreparedVectors::default()
        };
        for (index, unit) in units.iter_mut().enumerate() {
            unit.fulltext_status = if settings.fulltext && unit.unit_type != "IMAGE" {
                "INDEXED"
            } else {
                "NOT_ENABLED"
            }
            .into();
            unit.embedding_model_code = vectors.model_code.clone();
            unit.embedding_dimension = settings
                .vector
                .then_some(settings.embedding_dimension as i64);
            unit.embedding_version = settings.vector.then(|| "v1".into());
            unit.embedding_text_hash = settings.vector.then(|| {
                use sha2::Digest;
                unit.metadata["embeddingInputHash"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        format!("{:x}", sha2::Sha256::digest(unit.search_text.as_bytes()))
                    })
            });
            unit.embedding = vectors
                .vectors
                .get(index)
                .map(|values| values.iter().copied().flat_map(f32::to_le_bytes).collect());
            unit.embedding_status = if settings.vector {
                "INDEXED"
            } else {
                "NOT_ENABLED"
            }
            .into();
        }
        let graph = if settings.graph {
            checkpoint.stage("GRAPH")?;
            if let Some(graph) = structured_graph {
                Some(structured_evidence(&units, &graph)?)
            } else {
                Some(
                    self.prepare_graph_with_cache(
                        workspace,
                        settings
                            .graph_model_code
                            .as_deref()
                            .ok_or("请先配置图谱抽取模型")?,
                        &units,
                        Some(checkpoint),
                        cached_graphs,
                    )
                    .await?,
                )
            }
        } else {
            None
        };
        checkpoint.stage("PUBLISHING")?;
        Ok(RagPreparedIndex {
            units,
            vectors,
            graph,
        })
    }
}

fn structured_evidence(
    units: &[super::RagTextUnitWrite],
    graph: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::{Value, json};
    let nodes = graph["nodes"].as_array().ok_or("结构化图谱实体缺失")?;
    let edges = graph["edges"].as_array().ok_or("结构化图谱关系缺失")?;
    let body = units
        .iter()
        .map(|unit| unit.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let refs = nodes
        .iter()
        .filter_map(|node| node["ref"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if refs.len() != nodes.len()
        || nodes.iter().chain(edges.iter()).any(|item| {
            item["evidence"]
                .as_str()
                .is_none_or(|quote| quote.is_empty() || !body.contains(quote))
        })
        || edges.iter().any(|edge| {
            !edge["from"]
                .as_str()
                .is_some_and(|value| refs.contains(value))
                || !edge["to"]
                    .as_str()
                    .is_some_and(|value| refs.contains(value))
        })
    {
        return Err("结构化实体或关系没有来源证据".into());
    }
    let mut evidence = serde_json::Map::new();
    for unit in units {
        let chunk_nodes = nodes
            .iter()
            .filter(|node| {
                node["evidence"]
                    .as_str()
                    .is_some_and(|quote| unit.text.contains(quote))
            })
            .cloned()
            .collect::<Vec<_>>();
        let chunk_edges = edges
            .iter()
            .filter(|edge| {
                edge["evidence"]
                    .as_str()
                    .is_some_and(|quote| unit.text.contains(quote))
            })
            .cloned()
            .collect::<Vec<_>>();
        // Structured source relations can name an entity declared in another
        // original chunk. Keep its actual declaration location rather than
        // copying a quote into a chunk where it never occurred.
        let local_refs = chunk_nodes
            .iter()
            .filter_map(|node| node["ref"].as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let mut references = serde_json::Map::new();
        for relation in &chunk_edges {
            for endpoint in ["from", "to"] {
                let reference = relation[endpoint].as_str().ok_or("结构化关系端点缺失")?;
                if local_refs.contains(reference) {
                    continue;
                }
                let declaration = nodes
                    .iter()
                    .find(|node| node["ref"].as_str() == Some(reference))
                    .ok_or("结构化关系端点不存在")?;
                let quote = declaration["evidence"]
                    .as_str()
                    .ok_or("结构化实体证据缺失")?;
                let original = units
                    .iter()
                    .find(|source| {
                        source.unit_type != "IMAGE"
                            && super::graph::graph_source_text(source).contains(quote)
                    })
                    .ok_or("结构化实体没有可定位的原文声明")?;
                references.insert(reference.into(), json!(original.text_unit_code));
            }
        }
        evidence.insert(
            unit.text_unit_code.clone(),
            json!({"nodes":chunk_nodes,"edges":chunk_edges,"references":references}),
        );
    }
    Ok(Value::Object(evidence))
}
