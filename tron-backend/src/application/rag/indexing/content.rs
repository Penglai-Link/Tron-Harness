//! Ordered text/media embedding with lease-fenced checkpoints and content-addressed reuse.
use super::*;
use eliteforge_agent_framework::{Content, EmbeddingVector};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub fn embedding_input_hash(input: &Content) -> String {
    if input
        .0
        .iter()
        .all(|block| matches!(block, eliteforge_agent_framework::ContentBlock::Text { .. }))
    {
        format!("{:x}", Sha256::digest(input.text().as_bytes()))
    } else {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(input).expect("content is serializable"))
        )
    }
}
impl<R: Clone, E: RagEmbeddingRuntime> RagService<R, E> {
    pub async fn prepare_content_vectors(
        &self,
        workspace: &str,
        model: Option<&str>,
        inputs: &[Content],
        dimension: usize,
        mut completed: RagPreparedVectors,
        cached: &BTreeMap<String, Vec<f32>>,
        checkpoint: Option<&dyn RagIndexCheckpoint>,
    ) -> Result<RagPreparedVectors, String> {
        if dimension == 0
            || completed.vectors.len() > inputs.len()
            || completed.vectors.iter().any(|v| !valid(v, dimension))
        {
            return Err("向量检查点维度、数值或数量无效".into());
        }
        let resolved = if !completed.vectors.is_empty() && completed.model_code.is_some() {
            completed.model_code.clone()
        } else {
            self.runtime.resolve_index_model(workspace, model)?
        };
        let model = resolved.as_deref();
        let fingerprint = self
            .runtime
            .embedding_fingerprint(workspace, model, dimension)?;
        if !completed.vectors.is_empty() && completed.fingerprint != fingerprint {
            return Err("检查点的Embedding模型配置已改变，请重建索引".into());
        }
        completed.fingerprint = fingerprint.clone();
        let hashes = inputs.iter().map(embedding_input_hash).collect::<Vec<_>>();
        let mut start = completed.vectors.len();
        while start < inputs.len() {
            let mut limits = self
                .invocation_runtime_config
                .as_ref()
                .map(|runtime| {
                    let c = runtime.snapshot().model_invocation.embedding;
                    RagEmbeddingBatchLimits {
                        max_inputs: c.max_batch_inputs,
                        max_tokens: c.max_batch_tokens,
                    }
                })
                .unwrap_or(self.embedding_batch_limits);
            if limits.max_inputs == 0 || limits.max_tokens == 0 {
                return Err("Embedding批次上限必须大于零".into());
            }
            let reuse = cached
                .get(&hashes[start])
                .is_some_and(|v| valid(v, dimension));
            if !reuse {
                if let Some(maximum) = self.runtime.embedding_batch_input_limit(workspace, model)? {
                    if maximum == 0 {
                        return Err("供应商Embedding批次上限必须大于零".into());
                    }
                    limits.max_inputs = limits.max_inputs.min(maximum);
                }
            }
            let mut end = start;
            let mut tokens = 0usize;
            for (index, input) in inputs
                .iter()
                .enumerate()
                .skip(start)
                .take(limits.max_inputs)
            {
                if input.is_empty() || (!input.has_media() && input.text().trim().is_empty()) {
                    return Err("Embedding分块正文不能为空".into());
                }
                let cost = input.estimated_tokens();
                if cost > limits.max_tokens {
                    return Err("单个分块超过Embedding批次token上限，请缩小分块后重建".into());
                }
                if cost > limits.max_tokens - tokens
                    || cached
                        .get(&hashes[index])
                        .is_some_and(|v| valid(v, dimension))
                        != reuse
                {
                    break;
                }
                tokens += cost;
                end += 1;
            }
            if let Some(c) = checkpoint {
                c.validate()?;
            }
            let mut result = if reuse {
                RagEmbeddingBatchResult {
                    model_code: model.ok_or("缓存向量缺少模型身份")?.into(),
                    provider_model: completed.provider_model.clone(),
                    vectors: (start..end)
                        .enumerate()
                        .map(|(index, i)| EmbeddingVector {
                            index,
                            values: cached[&hashes[i]].clone(),
                        })
                        .collect(),
                }
            } else {
                self.runtime
                    .embed_content_batch(super::super::RagEmbeddingContentBatchCall {
                        workspace_code: workspace.into(),
                        model_code: model.map(str::to_owned),
                        inputs: inputs[start..end].to_vec(),
                        dimensions: dimension,
                    })
                    .await?
            };
            eliteforge_agent_framework::EmbeddingResponse {
                model: eliteforge_agent_framework::ModelRef::new(
                    "rag-embedding",
                    &result.model_code,
                ),
                provider_model: result.provider_model.clone(),
                vectors: result.vectors.clone(),
                input_tokens: None,
            }
            .validate(end - start, Some(dimension))
            .map_err(|error| {
                format!("Embedding批次返回的数量、索引或向量无效（要求{dimension}维）: {error}")
            })?;
            if model.is_some_and(|m| m != result.model_code) {
                return Err("Embedding模型不匹配，整份索引未发布".into());
            }
            if !completed.vectors.is_empty()
                && (completed.provider_model != result.provider_model
                    || completed
                        .model_code
                        .as_ref()
                        .is_some_and(|m| *m != result.model_code))
            {
                return Err("实际Embedding模型发生变化，整份索引未发布".into());
            }
            if self
                .runtime
                .embedding_fingerprint(workspace, model, dimension)?
                != fingerprint
            {
                return Err("Embedding模型配置已改变，整份索引未发布".into());
            }
            result.vectors.sort_by_key(|v| v.index);
            if let Some(c) = checkpoint {
                c.validate()?;
                c.save_batch(&result)?;
            }
            completed.provider_model = result.provider_model;
            completed.model_code = Some(result.model_code);
            completed
                .vectors
                .extend(result.vectors.into_iter().map(|v| v.values));
            start = end;
        }
        if let Some(c) = checkpoint {
            c.validate()?;
        }
        Ok(completed)
    }
}
