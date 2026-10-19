//! Append-only node-vector batches. The small manifest advances with each durable batch.
use super::super::RagPreparedVectors;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNodeVectorManifest {
    pub format_version: u8,
    pub storage_task_id: String,
    pub model_code: String,
    pub fingerprint: String,
    pub provider_model: Option<String>,
    pub dimension: usize,
    pub completed_units: usize,
    pub batch_count: usize,
}
impl GraphNodeVectorManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != 1
            || self.storage_task_id.is_empty()
            || self.model_code.is_empty()
            || self.fingerprint.is_empty()
            || !(1..=8192).contains(&self.dimension)
            || (self.batch_count == 0) != (self.completed_units == 0)
            || self.batch_count > self.completed_units
        {
            return Err("图谱节点向量检查点清单无效".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNodeVectorBatch {
    pub offset: usize,
    pub count: usize,
    pub data_base64: String,
}
impl GraphNodeVectorBatch {
    pub fn encode(offset: usize, values: &[Vec<f32>], dimension: usize) -> Result<Self, String> {
        if values.is_empty()
            || dimension == 0
            || values
                .iter()
                .any(|v| v.len() != dimension || v.iter().any(|x| !x.is_finite()))
        {
            return Err("图谱节点向量批次维度或数值无效".into());
        }
        let size = values
            .len()
            .checked_mul(dimension)
            .and_then(|n| n.checked_mul(4))
            .ok_or("图谱节点向量批次过大")?;
        let mut bytes = Vec::with_capacity(size);
        for value in values.iter().flatten() {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Ok(Self {
            offset,
            count: values.len(),
            data_base64: STANDARD.encode(bytes),
        })
    }
    pub fn append_decoded(
        self,
        manifest: &GraphNodeVectorManifest,
        vectors: &mut Vec<Vec<f32>>,
    ) -> Result<(), String> {
        if self.count == 0
            || self.offset != vectors.len()
            || self
                .offset
                .checked_add(self.count)
                .is_none_or(|end| end > manifest.completed_units)
        {
            return Err("图谱节点向量批次不连续或数量无效".into());
        }
        vectors.extend(self.decode(manifest.dimension)?);
        Ok(())
    }
    pub fn decode(&self, dimension: usize) -> Result<Vec<Vec<f32>>, String> {
        let size = self
            .count
            .checked_mul(dimension)
            .and_then(|n| n.checked_mul(4))
            .ok_or("图谱节点向量批次过大")?;
        if self.count == 0
            || !(1..=8192).contains(&dimension)
            || size > 16 * 1024 * 1024
            || self.data_base64.len()
                != size
                    .div_ceil(3)
                    .checked_mul(4)
                    .ok_or("图谱节点向量批次过大")?
        {
            return Err("图谱节点向量批次维度或字节数无效".into());
        }
        let bytes = STANDARD
            .decode(&self.data_base64)
            .map_err(|_| "图谱节点向量批次编码无效")?;
        if bytes.len() != size {
            return Err("图谱节点向量批次字节数无效".into());
        }
        let mut vectors = Vec::with_capacity(self.count);
        for row in bytes.chunks_exact(dimension * 4) {
            let values = row
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().expect("four bytes")))
                .collect::<Vec<_>>();
            if values.iter().any(|v| !v.is_finite()) {
                return Err("图谱节点向量批次包含非有限数值".into());
            }
            vectors.push(values);
        }
        Ok(vectors)
    }
}

#[derive(Clone, Debug)]
pub struct GraphNodeVectorCheckpoint {
    pub manifest: GraphNodeVectorManifest,
    pub batches: Vec<GraphNodeVectorBatch>,
}
impl GraphNodeVectorCheckpoint {
    pub fn restore(
        self,
        model: &str,
        fingerprint: &str,
        dimension: usize,
        input_count: usize,
    ) -> Result<(RagPreparedVectors, GraphNodeVectorManifest), String> {
        self.manifest.validate()?;
        let manifest = self.manifest;
        if manifest.model_code != model
            || manifest.fingerprint != fingerprint
            || manifest.dimension != dimension
            || manifest.completed_units > input_count
            || manifest.batch_count != self.batches.len()
        {
            return Err("图谱节点向量检查点的模型空间或数量不匹配".into());
        }
        let mut vectors = Vec::with_capacity(manifest.completed_units);
        for batch in self.batches {
            batch.append_decoded(&manifest, &mut vectors)?;
        }
        if vectors.len() != manifest.completed_units {
            return Err("图谱节点向量检查点不完整".into());
        }
        Ok((
            RagPreparedVectors {
                vectors,
                model_code: Some(manifest.model_code.clone()),
                fingerprint: Some(manifest.fingerprint.clone()),
                provider_model: manifest.provider_model.clone(),
            },
            manifest,
        ))
    }
}

pub fn graph_node_vector_batch_key(key: &str, index: usize) -> String {
    format!("{key}:Batch:{index:020}")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest() -> GraphNodeVectorManifest {
        GraphNodeVectorManifest {
            format_version: 1,
            storage_task_id: "task".into(),
            model_code: "model".into(),
            fingerprint: "space".into(),
            provider_model: Some("provider".into()),
            dimension: 2,
            completed_units: 3,
            batch_count: 2,
        }
    }
    #[test]
    fn node_vector_batches_preserve_float_bits_and_reject_incomplete_or_wrong_spaces() {
        let values = vec![
            vec![1., -0.],
            vec![f32::from_bits(1), -1.],
            vec![0.25, 0.75],
        ];
        let batches = vec![
            GraphNodeVectorBatch::encode(0, &values[..2], 2).unwrap(),
            GraphNodeVectorBatch::encode(2, &values[2..], 2).unwrap(),
        ];
        let saved = GraphNodeVectorCheckpoint {
            manifest: manifest(),
            batches,
        };
        let (restored, _) = saved.clone().restore("model", "space", 2, 3).unwrap();
        assert_eq!(
            restored
                .vectors
                .iter()
                .flatten()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>(),
            values
                .iter()
                .flatten()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>()
        );
        assert!(saved.clone().restore("model", "changed", 2, 3).is_err());
        assert!(saved.clone().restore("model", "space", 2, 2).is_err());
        let mut gap = saved.clone();
        gap.batches[1].offset = 1;
        assert!(gap.restore("model", "space", 2, 3).is_err());
        let mut missing = saved.clone();
        missing.batches.pop();
        assert!(missing.restore("model", "space", 2, 3).is_err());
        let mut corrupt = saved;
        corrupt.batches[0].data_base64 = "not-base64".into();
        assert!(corrupt.restore("model", "space", 2, 3).is_err());
        assert!(GraphNodeVectorBatch::encode(0, &[vec![f32::NAN, 0.]], 2).is_err());
    }

    #[tokio::test]
    async fn node_vector_retry_reuses_a_durable_prefix_before_publishing() {
        use crate::{
            adapters::outbound::persistence::{
                GrafeoAdapterConfig, GrafeoRagStore, GrafeoStoreCore, SqliteRagGraphIndexRepository,
            },
            application::rag::{graph::*, *},
            domain::rag::graph::{GraphPartition, RagGraphGeneration, RagGraphNode},
        };
        use std::{
            future::Future,
            pin::Pin,
            sync::{
                Arc, Mutex,
                atomic::{AtomicUsize, Ordering},
            },
        };
        #[derive(Clone, Default)]
        struct Runtime {
            attempts: Arc<AtomicUsize>,
            inputs: Arc<Mutex<Vec<Vec<String>>>>,
        }
        impl RagEmbeddingRuntime for Runtime {
            fn embedding_fingerprint(
                &self,
                _: &str,
                _: Option<&str>,
                _: usize,
            ) -> Result<Option<String>, String> {
                Ok(Some("space".into()))
            }
            fn embed<'a>(
                &'a self,
                _: RagEmbeddingCall,
            ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>>
            {
                Box::pin(async { panic!("batch embeddings only") })
            }
            fn embed_batch<'a>(
                &'a self,
                call: RagEmbeddingBatchCall,
            ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>>
            {
                Box::pin(async move {
                    self.inputs.lock().unwrap().push(call.inputs.clone());
                    if self.attempts.fetch_add(1, Ordering::SeqCst) == 1 {
                        return Err("temporary provider failure".into());
                    }
                    Ok(RagEmbeddingBatchResult {
                        model_code: call.model_code.unwrap(),
                        provider_model: Some("provider".into()),
                        vectors: call
                            .inputs
                            .iter()
                            .enumerate()
                            .map(|(index, _)| eliteforge_agent_framework::EmbeddingVector {
                                index,
                                values: vec![1., 0.],
                            })
                            .collect(),
                    })
                })
            }
        }
        struct Checkpoint;
        impl RagIndexCheckpoint for Checkpoint {
            fn validate(&self) -> Result<(), String> {
                Ok(())
            }
            fn save_batch(&self, _: &RagEmbeddingBatchResult) -> Result<(), String> {
                panic!("node batches use their own durable checkpoint")
            }
        }
        struct LeaseStore {
            inner: Arc<dyn RagGraphStore>,
            entered: Arc<tokio::sync::Notify>,
            heartbeat: Arc<std::sync::atomic::AtomicBool>,
        }
        impl RagGraphStore for LeaseStore {
            fn stage_generation(&self, facts: &RagGraphGeneration) -> Result<(), String> {
                self.entered.notify_one();
                for _ in 0..100 {
                    if self.heartbeat.load(Ordering::SeqCst) {
                        return self.inner.stage_generation(facts);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                Err("durable graph staging starved the ingestion heartbeat".into())
            }
            fn nodes(
                &self,
                p: &GraphPartition,
                ids: &[String],
                g: &std::collections::BTreeSet<String>,
            ) -> Result<Vec<RagGraphNode>, String> {
                self.inner.nodes(p, ids, g)
            }
            fn adjacent(&self, q: &GraphAdjacencyQuery) -> Result<GraphAdjacencyPage, String> {
                self.inner.adjacent(q)
            }
            fn structural_adjacent(
                &self,
                q: &GraphStructuralQuery,
            ) -> Result<GraphAdjacencyPage, String> {
                self.inner.structural_adjacent(q)
            }
            fn entity_evidence(
                &self,
                p: &GraphPartition,
                ids: &[String],
                g: &std::collections::BTreeSet<String>,
                after: Option<&str>,
                limit: usize,
            ) -> Result<GraphAdjacencyPage, String> {
                self.inner.entity_evidence(p, ids, g, after, limit)
            }
            fn delete_generation(&self, p: &GraphPartition, id: &str) -> Result<(), String> {
                self.inner.delete_generation(p, id)
            }
        }
        let index = Arc::new(SqliteRagGraphIndexRepository::open(":memory:").unwrap());
        let core = GrafeoStoreCore::connect(GrafeoAdapterConfig {
            storage_path: ":memory:".into(),
        })
        .unwrap();
        let runtime = Runtime::default();
        let entered = Arc::new(tokio::sync::Notify::new());
        let heartbeat = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let store = Arc::new(LeaseStore {
            inner: Arc::new(GrafeoRagStore::new(core)),
            entered: entered.clone(),
            heartbeat: heartbeat.clone(),
        });
        let service = RagService::new((), runtime.clone())
            .with_embedding_batch_limits(RagEmbeddingBatchLimits {
                max_inputs: 1,
                max_tokens: 6000,
            })
            .with_graph_infrastructure(GraphInfrastructure::new(store, index.clone()));
        let settings = RagSettings {
            fulltext: false,
            vector: false,
            graph: true,
            embedding_dimension: 2,
            embedding_model_code: Some("model".into()),
            graph_model_code: Some("extractor".into()),
            ..Default::default()
        };
        let context = GraphIndexContext {
            partition: GraphPartition {
                corpus_id: "corpus".into(),
                scope_type: "USER".into(),
                scope_code: "owner".into(),
            },
            logical_source_code: "source".into(),
            source_code: "source-version".into(),
            source_version: "v1".into(),
            task_id: "retry-task".into(),
            configuration_hash: "config".into(),
        };
        let text = "Alpha calls Beta.";
        let unit = RagTextUnitWrite {
            text_unit_code: "chunk".into(),
            unit_type: "LEAF".into(),
            text: text.into(),
            search_text: text.into(),
            heading_path: String::new(),
            ordinal: 0,
            token_count: 5,
            content_hash: "hash".into(),
            status: "ACTIVE".into(),
            fulltext_status: "NOT_ENABLED".into(),
            embedding_model_code: None,
            embedding_version: None,
            embedding_text_hash: None,
            embedding_dimension: None,
            embedding: None,
            embedding_status: "NOT_ENABLED".into(),
            metadata: serde_json::json!({"location":{"page":1}}),
        };
        let graph = serde_json::json!({"chunk":{"nodes":[{"ref":"a","name":"Alpha","type":"Service","evidence":text},
            {"ref":"b","name":"Beta","type":"Service","evidence":text}],"edges":[{"from":"a","to":"b","relationship":"CALLS","evidence":text}]}});
        let prepared = RagPreparedIndex {
            units: vec![unit],
            vectors: Default::default(),
            graph: Some(graph),
        };
        assert!(
            service
                .stage_graph_generation("space", &settings, &context, &prepared, &Checkpoint)
                .await
                .is_err()
        );
        assert!(
            index
                .active_generations(&[context.partition.clone()])
                .unwrap()
                .is_empty()
        );
        tokio::spawn(async move {
            entered.notified().await;
            heartbeat.store(true, Ordering::SeqCst);
        });
        let generation = service
            .stage_graph_generation("space", &settings, &context, &prepared, &Checkpoint)
            .await
            .unwrap()
            .unwrap();
        let (_, projections) = index.export_generation(&generation).unwrap().unwrap();
        assert_eq!(projections.len(), 2);
        let calls = runtime.inputs.lock().unwrap();
        assert_eq!(
            calls.len(),
            3,
            "one successful batch, one failed batch, then only the missing batch"
        );
        assert_ne!(calls[0], calls[1]);
        assert_eq!(
            calls[1], calls[2],
            "the successfully saved prefix must never be submitted again"
        );
    }
}
