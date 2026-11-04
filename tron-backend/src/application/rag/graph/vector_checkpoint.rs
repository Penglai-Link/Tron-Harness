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
