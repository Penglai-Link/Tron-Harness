//! Fixed retrieval flow shared by product context and the RAG search tool.
//! Callers supply server-authorized source snapshots, never client scopes.
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::{Value, json};

use super::{
    RagEmbeddingCall, RagRerankCall, RagRouteSearchQuery, RagSearchRepository, RagSearchRow,
    RagSearchRuntime, RagSearchScope, RagService,
};
use crate::domain::rag::RagError;

#[derive(Clone, Debug)]
pub struct AuthorizedRagRange {
    pub corpus_id: String,
    pub scopes: Vec<RagSearchScope>,
    pub source_version: String,
    pub filters: Value,
    pub embedding_model_code: Option<String>,
    pub embedding_dimension: Option<usize>,
    pub provider_model: Option<String>,
    pub embedding_fingerprint: Option<String>,
    pub return_fields: Vec<String>,
    pub fulltext: bool,
    pub vector: bool,
    pub graph: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalStage {
    pub corpus_id: String,
    pub channel: String,
    pub status: String,
    pub hit_count: usize,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RagEvidence {
    pub row: RagSearchRow,
    pub channels: BTreeSet<String>,
    pub score: f64,
}

#[derive(Clone, Debug, Default)]
pub struct RagEvidenceResult {
    pub hits: Vec<RagEvidence>,
    pub stages: Vec<RagRetrievalStage>,
    pub warnings: Vec<String>,
}

impl<R, E> RagService<R, E>
where
    R: RagSearchRepository + Clone,
    E: RagSearchRuntime,
{
    pub async fn retrieve_authorized(
        &self,
        workspace: &str,
        query: &str,
        ranges: &[AuthorizedRagRange],
        limit: usize,
        rerank_model: Option<&str>,
        run: Option<&eliteforge_agent_framework::RunContext>,
    ) -> Result<RagEvidenceResult, RagError> {
        if !(1..=100).contains(&limit) {
            return Err(RagError::SearchRequestInvalid);
        }
        let mut result = RagEvidenceResult::default();
        let mut merged = BTreeMap::<String, RagEvidence>::new();
        let mut embeddings = BTreeMap::<
            (
                Option<String>,
                Option<usize>,
                Option<String>,
                Option<String>,
            ),
            Result<Vec<f32>, String>,
        >::new();
        for range in ranges {
            if range.scopes.is_empty() {
                continue;
            }
            let geometry = (
                range.embedding_model_code.clone(),
                range.embedding_dimension,
                range.provider_model.clone(),
                range.embedding_fingerprint.clone(),
            );
            if range.vector && !query.trim().is_empty() && !embeddings.contains_key(&geometry) {
                let before = self.runtime.embedding_fingerprint(
                    workspace,
                    geometry.0.as_deref(),
                    geometry.1.unwrap_or(1024),
                );
                let fingerprint_matches = |value: &Result<Option<String>, String>| {
                    value.as_ref().is_ok_and(|actual| {
                        geometry
                            .3
                            .as_ref()
                            .is_none_or(|expected| actual.as_ref() == Some(expected))
                    })
                };
                let vector = if !fingerprint_matches(&before) {
                    Err("查询模型配置与已发布向量空间不同，请重建索引".into())
                } else {
                    self.runtime
                        .embed_for_retrieval(
                            RagEmbeddingCall {
                                workspace_code: workspace.into(),
                                model_code: geometry.0.clone(),
                                input: query.into(),
                            },
                            geometry.1,
                            run,
                        )
                        .await
                        .and_then(|value| {
                            if geometry
                                .0
                                .as_ref()
                                .is_some_and(|expected| *expected != value.model_code)
                                || match (&geometry.2, &value.provider_model) {
                                    (Some(expected), actual) => actual.as_ref() != Some(expected),
                                    _ => false,
                                }
                            {
                                return Err("查询模型与已发布索引模型不一致".into());
                            }
                            let values = value.embedding;
                            if values.is_empty()
                                || values.iter().any(|value| !value.is_finite())
                                || geometry
                                    .1
                                    .is_some_and(|dimension| dimension != values.len())
                            {
                                Err("查询向量与已发布索引维度不一致".into())
                            } else {
                                Ok(values)
                            }
                        })
                        .and_then(|values| {
                            let after = self.runtime.embedding_fingerprint(
                                workspace,
                                geometry.0.as_deref(),
                                geometry.1.unwrap_or(1024),
                            );
                            if after != before || !fingerprint_matches(&after) {
                                Err("查询过程中Embedding模型配置改变，请重试或重建索引".into())
                            } else {
                                Ok(values)
                            }
                        })
                };
                embeddings.insert(geometry.clone(), vector);
            }
            for (channel, retriever, enabled) in [
                (
                    "fulltext",
                    if query.trim().is_empty() {
                        "STRUCTURED_FILTER_SQLITE_V1"
                    } else {
                        "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"
                    },
                    range.fulltext,
                ),
                ("vector", "VECTOR_SQLITE_VEC0_COSINE_V1", range.vector),
            ] {
                if !enabled || (channel == "vector" && query.trim().is_empty()) {
                    continue;
                }
                let started = std::time::Instant::now();
                let vector = if channel == "vector" {
                    match embeddings.get(&geometry) {
                        Some(Ok(values)) => Some(values.clone()),
                        Some(Err(error)) => {
                            result
                                .warnings
                                .push(format!("{}: {error}", range.corpus_id));
                            result.stages.push(stage(
                                range,
                                channel,
                                started,
                                0,
                                Some(error.clone()),
                            ));
                            continue;
                        }
                        None => return Err(RagError::SearchRequestInvalid),
                    }
                } else {
                    None
                };
                let rows = self.repository.search_route(RagRouteSearchQuery {
                    corpus_id: range.corpus_id.clone(), route_id: channel.into(),
                    retriever: retriever.into(), workspace_code: workspace.into(), query: query.into(),
                    query_embedding: vector, query_embedding_model_code: range.embedding_model_code.clone(),
                    source_version: range.source_version.clone(), scopes: range.scopes.clone(),
                    filters: range.filters.clone(), policy: json!({"providerModel":range.provider_model,"fingerprint":range.embedding_fingerprint}), return_fields: range.return_fields.clone(),
                    limit: limit.saturating_mul(3).min(100),
                });
                match rows {
                    Ok(rows) => {
                        result
                            .stages
                            .push(stage(range, channel, started, rows.len(), None));
                        merge_rrf(&mut merged, rows, channel);
                    }
                    Err(error) => {
                        result
                            .warnings
                            .push(format!("{} {channel}: {error}", range.corpus_id));
                        result.stages.push(stage(
                            range,
                            channel,
                            started,
                            0,
                            Some(error.to_string()),
                        ));
                    }
                }
            }
            if range.graph && !query.trim().is_empty() {
                // Graph evidence is implemented by the repository and must preserve
                // the same source/scope filters as the lexical/vector routes.
                let started = std::time::Instant::now();
                let rows = self.repository.search_route(RagRouteSearchQuery {
                    corpus_id: range.corpus_id.clone(),
                    route_id: "graph".into(),
                    retriever: "GRAPH_EVIDENCE_V1".into(),
                    workspace_code: workspace.into(),
                    query: query.into(),
                    query_embedding: None,
                    query_embedding_model_code: None,
                    source_version: range.source_version.clone(),
                    scopes: range.scopes.clone(),
                    filters: range.filters.clone(),
                    policy: json!({}),
                    return_fields: range.return_fields.clone(),
                    limit,
                });
                match rows {
                    Ok(rows) => {
                        result
                            .stages
                            .push(stage(range, "graph", started, rows.len(), None));
                        merge_rrf(&mut merged, rows, "graph");
                    }
                    Err(error) => {
                        result
                            .warnings
                            .push(format!("{} graph: {error}", range.corpus_id));
                        result.stages.push(stage(
                            range,
                            "graph",
                            started,
                            0,
                            Some(error.to_string()),
                        ));
                    }
                }
            }
        }
        let fusion_started = std::time::Instant::now();
        result.hits = merged.into_values().collect();
        result.hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.row.target_code.cmp(&b.row.target_code))
        });
        result.stages.push(RagRetrievalStage {
            corpus_id: String::new(),
            channel: "fusion".into(),
            status: "COMPLETED".into(),
            hit_count: result.hits.len(),
            duration_ms: fusion_started.elapsed().as_millis() as u64,
            error: None,
        });
        if let Some(model) = rerank_model.filter(|model| !model.trim().is_empty()) {
            let rerank_started = std::time::Instant::now();
            let order = self
                .runtime
                .rerank(RagRerankCall {
                    workspace_code: workspace.into(),
                    model_code: Some(model.into()),
                    query: query.into(),
                    documents: result.hits.iter().map(|hit| hit.row.text.clone()).collect(),
                    top_n: Some(limit),
                })
                .await;
            let mut rerank_error = None;
            match order {
                Ok(order)
                    if !order.is_empty()
                        && order.iter().all(|(index, score)| {
                            *index < result.hits.len() && score.is_finite()
                        })
                        && order
                            .iter()
                            .map(|(index, _)| index)
                            .collect::<BTreeSet<_>>()
                            .len()
                            == order.len() =>
                {
                    result.hits = order
                        .into_iter()
                        .map(|(index, score)| {
                            let mut hit = result.hits[index].clone();
                            hit.score = score;
                            hit
                        })
                        .collect();
                }
                Ok(_) => {
                    let error = "重排返回了无效结果，已保留融合排序".to_string();
                    result.warnings.push(error.clone());
                    rerank_error = Some(error);
                }
                Err(error) => {
                    result
                        .warnings
                        .push(format!("重排失败，已保留融合排序: {error}"));
                    rerank_error = Some(error);
                }
            }
            result.stages.push(RagRetrievalStage {
                corpus_id: String::new(),
                channel: "rerank".into(),
                status: if rerank_error.is_some() {
                    "FAILED"
                } else {
                    "COMPLETED"
                }
                .into(),
                hit_count: result.hits.len(),
                duration_ms: rerank_started.elapsed().as_millis() as u64,
                error: rerank_error,
            });
        }
        result.hits.truncate(limit);
        Ok(result)
    }
}

fn stage(
    range: &AuthorizedRagRange,
    channel: &str,
    started: std::time::Instant,
    count: usize,
    error: Option<String>,
) -> RagRetrievalStage {
    RagRetrievalStage {
        corpus_id: range.corpus_id.clone(),
        channel: channel.into(),
        status: if error.is_some() {
            "FAILED"
        } else {
            "COMPLETED"
        }
        .into(),
        hit_count: count,
        duration_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        error,
    }
}

fn merge_rrf(merged: &mut BTreeMap<String, RagEvidence>, rows: Vec<RagSearchRow>, channel: &str) {
    let mut seen = BTreeSet::new();
    for (rank, row) in rows.into_iter().enumerate() {
        let key = format!(
            "{}:{}:{}:{}:{}:{}",
            row.corpus_id,
            row.source_code,
            row.source_version,
            row.scope_type,
            row.scope_code,
            row.target_code
        );
        if !seen.insert(key.clone()) {
            continue;
        }
        let evidence = merged.entry(key).or_insert_with(|| RagEvidence {
            row,
            channels: BTreeSet::new(),
            score: 0.0,
        });
        evidence.score += 1.0 / (60.0 + rank as f64 + 1.0);
        evidence.channels.insert(channel.into());
    }
}

impl<R: RagSearchRepository + Clone, E: Clone> RagService<R, E> {
    /// Build vector partitions from published metadata inside the authorized scope.
    pub fn partition_ranges(
        &self,
        mut range: AuthorizedRagRange,
    ) -> Result<Vec<AuthorizedRagRange>, RagError> {
        let corpus = self
            .repository
            .get_corpus(&range.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        range.filters = super::merge_search_request_filters(&corpus, &range.filters, &json!({}))?;
        let vector = range.vector;
        range.vector = false;
        let mut ranges = vec![range.clone()];
        if !vector {
            return Ok(ranges);
        }
        let rows = self.repository.embedding_partitions(RagRouteSearchQuery {
            corpus_id: range.corpus_id.clone(),
            route_id: "partition".into(),
            retriever: "STRUCTURED_FILTER_SQLITE_V1".into(),
            workspace_code: String::new(),
            query: String::new(),
            query_embedding: None,
            query_embedding_model_code: None,
            source_version: range.source_version.clone(),
            scopes: range.scopes.clone(),
            filters: range.filters.clone(),
            policy: json!({}),
            return_fields: vec![],
            limit: 100,
        })?;
        let mut groups =
            BTreeMap::<(String, usize, Option<String>, Option<String>), Vec<String>>::new();
        for row in rows {
            groups
                .entry((
                    row.model_code,
                    row.dimension,
                    row.provider_model,
                    row.fingerprint,
                ))
                .or_default()
                .push(row.source_code);
        }
        for ((model, dimension, provider, fingerprint), sources) in groups {
            let mut partition = range.clone();
            partition.fulltext = false;
            partition.graph = false;
            partition.vector = true;
            partition.embedding_model_code = Some(model);
            partition.embedding_dimension = Some(dimension);
            partition.provider_model = provider;
            partition.embedding_fingerprint = fingerprint;
            let corpus = self
                .repository
                .get_corpus(&range.corpus_id)?
                .ok_or(RagError::CorpusNotFound)?;
            partition.filters = super::merge_search_request_filters(
                &corpus,
                &range.filters,
                &json!({"sourceCode":{"op":"in","value":sources}}),
            )?;
            ranges.push(partition);
        }
        Ok(ranges)
    }
    pub fn recheck_published(
        &self,
        ranges: &[AuthorizedRagRange],
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        let mut visible = BTreeSet::new();
        for range in ranges {
            let sources = result
                .hits
                .iter()
                .filter(|hit| hit.row.corpus_id == range.corpus_id)
                .map(|hit| hit.row.source_code.clone())
                .collect::<BTreeSet<_>>();
            if sources.is_empty() {
                continue;
            }
            let corpus = self
                .repository
                .get_corpus(&range.corpus_id)?
                .ok_or(RagError::CorpusNotFound)?;
            let filters = super::merge_search_request_filters(
                &corpus,
                &range.filters,
                &json!({"sourceCode":{"op":"in","value":sources}}),
            )?;
            let rows = self.repository.search_route(RagRouteSearchQuery {
                corpus_id: range.corpus_id.clone(),
                route_id: "publication-check".into(),
                retriever: "STRUCTURED_FILTER_SQLITE_V1".into(),
                workspace_code: String::new(),
                query: String::new(),
                query_embedding: None,
                query_embedding_model_code: None,
                source_version: range.source_version.clone(),
                scopes: range.scopes.clone(),
                filters,
                policy: json!({"targetCodes":result.hits.iter().filter(|hit| hit.row.corpus_id==range.corpus_id).map(|hit|hit.row.target_code.clone()).collect::<BTreeSet<_>>()}),
                return_fields: vec![],
                limit: result.hits.len(),
            })?;
            for row in rows {
                visible.insert((
                    row.corpus_id,
                    row.target_code,
                    row.source_code,
                    row.source_version,
                    row.scope_type,
                    row.scope_code,
                    row.text,
                ));
            }
        }
        let count = result.hits.len();
        result.hits.retain(|hit| {
            visible.contains(&(
                hit.row.corpus_id.clone(),
                hit.row.target_code.clone(),
                hit.row.source_code.clone(),
                hit.row.source_version.clone(),
                hit.row.scope_type.clone(),
                hit.row.scope_code.clone(),
                hit.row.text.clone(),
            ))
        });
        if result.hits.len() != count {
            result
                .warnings
                .push("检索期间来源已删除、变更或撤销发布，相关证据已移除".into());
        }
        Ok(())
    }
}
