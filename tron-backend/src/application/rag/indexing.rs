//! Shared vector preparation. Publication happens only after all configured indexes succeed.
use super::{RagEmbeddingBatchCall, RagEmbeddingBatchResult, RagEmbeddingRuntime, RagService};

pub trait RagIndexCheckpoint: Send + Sync {
    fn validate(&self) -> Result<(), String>;
    fn save_batch(&self, result: &RagEmbeddingBatchResult) -> Result<(), String>;
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
impl RagEmbeddingBatchLimits {
    fn end(&self, inputs: &[String], start: usize) -> Result<usize, String> {
        if self.max_inputs == 0 || self.max_tokens == 0 {
            return Err("Embedding批次上限必须大于零".into());
        }
        let mut tokens = 0usize;
        let mut end = start;
        for input in inputs.iter().skip(start).take(self.max_inputs) {
            if input.trim().is_empty() {
                return Err("Embedding分块正文不能为空".into());
            }
            let cost = eliteforge_agent_framework::Content::from(input.clone()).estimated_tokens();
            if cost > self.max_tokens {
                return Err("单个分块超过Embedding批次token上限，请缩小分块后重建".into());
            }
            if cost > self.max_tokens - tokens {
                break;
            }
            tokens += cost;
            end += 1;
        }
        Ok(end)
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
        mut completed: RagPreparedVectors,
        checkpoint: Option<&dyn RagIndexCheckpoint>,
    ) -> Result<RagPreparedVectors, String> {
        if dimension == 0
            || completed.vectors.len() > inputs.len()
            || completed
                .vectors
                .iter()
                .any(|values| !valid(values, dimension))
        {
            return Err("向量检查点维度、数值或数量无效".into());
        }
        let fingerprint = self
            .runtime
            .embedding_fingerprint(workspace, model, dimension)?;
        if !completed.vectors.is_empty() && completed.fingerprint != fingerprint {
            return Err("检查点的Embedding模型配置已改变，请重建索引".into());
        }
        completed.fingerprint = fingerprint.clone();
        let mut start = completed.vectors.len();
        while start < inputs.len() {
            let limits = self
                .invocation_runtime_config
                .as_ref()
                .map(|runtime| {
                    let config = runtime.snapshot().model_invocation.embedding;
                    RagEmbeddingBatchLimits {
                        max_inputs: config.max_batch_inputs,
                        max_tokens: config.max_batch_tokens,
                    }
                })
                .unwrap_or(self.embedding_batch_limits);
            let end = limits.end(inputs, start)?;
            if let Some(checkpoint) = checkpoint {
                checkpoint.validate()?;
            }
            let mut result = self
                .runtime
                .embed_batch(RagEmbeddingBatchCall {
                    workspace_code: workspace.into(),
                    model_code: model.map(str::to_owned),
                    inputs: inputs[start..end].to_vec(),
                    dimensions: dimension,
                })
                .await?;
            // Validate the entire batch before any durable progress is advanced.
            let response = eliteforge_agent_framework::EmbeddingResponse {
                model: eliteforge_agent_framework::ModelRef::new(
                    "rag-embedding",
                    &result.model_code,
                ),
                provider_model: result.provider_model.clone(),
                vectors: result.vectors.clone(),
                input_tokens: None,
            };
            response
                .validate(end - start, Some(dimension))
                .map_err(|error| {
                    format!("Embedding批次返回的数量、索引或向量无效（要求{dimension}维）: {error}")
                })?;
            if model.is_some_and(|model| model != result.model_code) {
                return Err("Embedding模型不匹配，整份索引未发布".into());
            }
            if !completed.vectors.is_empty()
                && (completed.provider_model != result.provider_model
                    || completed
                        .model_code
                        .as_ref()
                        .is_some_and(|model| *model != result.model_code))
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
            result.vectors.sort_by_key(|vector| vector.index);
            if let Some(checkpoint) = checkpoint {
                checkpoint.validate()?;
                checkpoint.save_batch(&result)?;
            }
            completed.provider_model = result.provider_model;
            completed.model_code = Some(result.model_code);
            completed
                .vectors
                .extend(result.vectors.into_iter().map(|vector| vector.values));
            start = end;
        }
        if let Some(checkpoint) = checkpoint {
            checkpoint.validate()?;
        }
        Ok(completed)
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
        let mut evidence = serde_json::Map::new();
        for unit in units {
            if let Some(checkpoint) = checkpoint {
                checkpoint.validate()?;
            }
            let prompt = format!(
                "从下述正文抽取实体和关系。只输出JSON，格式为 {{\"nodes\":[{{\"ref\":\"实体标识\",\"name\":\"实体名\",\"evidence\":\"正文逐字摘录\"}}],\"edges\":[{{\"from\":\"标识\",\"to\":\"标识\",\"relationship\":\"关系\",\"evidence\":\"正文逐字摘录\"}}]}}。不得补充无证据的信息。正文：\n{}",
                unit.text
            );
            let response = self
                .runtime
                .extract_graph(super::RagGraphCall {
                    workspace_code: workspace.into(),
                    model_code: Some(model.into()),
                    prompt,
                })
                .await?;
            if let Some(checkpoint) = checkpoint {
                checkpoint.validate()?;
            }
            let raw = response
                .graph_json
                .trim()
                .strip_prefix("```json")
                .unwrap_or(response.graph_json.trim())
                .trim()
                .trim_end_matches("```")
                .trim();
            let graph: serde_json::Value = serde_json::from_str(raw)
                .map_err(|error| format!("图谱抽取返回格式错误: {error}"))?;
            let nodes = graph["nodes"].as_array().ok_or("图谱实体缺失")?;
            let edges = graph["edges"].as_array().ok_or("图谱关系缺失")?;
            let refs: std::collections::BTreeSet<_> = nodes
                .iter()
                .filter_map(|node| node["ref"].as_str())
                .collect();
            if refs.len() != nodes.len()
                || nodes.iter().chain(edges.iter()).any(|item| {
                    item["evidence"]
                        .as_str()
                        .is_none_or(|quote| quote.is_empty() || !unit.text.contains(quote))
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
                return Err("图谱实体或关系没有有效原文证据".into());
            }
            evidence.insert(unit.text_unit_code.clone(), graph);
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
        mut units: Vec<super::RagTextUnitWrite>,
        completed: RagPreparedVectors,
        structured_graph: Option<serde_json::Value>,
        checkpoint: &dyn RagIndexCheckpoint,
    ) -> Result<RagPreparedIndex, String> {
        settings.validate().map_err(|error| error.to_string())?;
        if units.is_empty() || units.iter().any(|unit| unit.text.trim().is_empty()) {
            return Err("没有可入库正文".into());
        }
        checkpoint.stage("INDEXING")?;
        let vectors = if settings.vector {
            checkpoint.stage("EMBEDDING")?;
            self.prepare_vectors(
                workspace,
                settings.embedding_model_code.as_deref(),
                &units
                    .iter()
                    .map(|unit| unit.text.clone())
                    .collect::<Vec<_>>(),
                settings.embedding_dimension,
                completed,
                Some(checkpoint),
            )
            .await?
        } else {
            RagPreparedVectors::default()
        };
        for (index, unit) in units.iter_mut().enumerate() {
            unit.fulltext_status = if settings.fulltext {
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
            unit.embedding_text_hash = settings.vector.then(|| unit.content_hash.clone());
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
                    self.prepare_graph(
                        workspace,
                        settings
                            .graph_model_code
                            .as_deref()
                            .ok_or("请先配置图谱抽取模型")?,
                        &units,
                        Some(checkpoint),
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
        if !chunk_nodes.is_empty() || !chunk_edges.is_empty() {
            evidence.insert(
                unit.text_unit_code.clone(),
                json!({"nodes":chunk_nodes,"edges":chunk_edges}),
            );
        }
    }
    Ok(Value::Object(evidence))
}

#[cfg(test)]
mod tests;
