//! Business authorization supplies a source snapshot; the RAG service owns execution.
use super::*;
use crate::domain::rag::RagError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct RagIdentity {
    pub user_code: String,
    pub workspace_code: String,
    pub agent_code: String,
}
pub trait RagAccessResolver: Send + Sync {
    fn resolve(&self, identity: &RagIdentity) -> Result<Vec<AuthorizedRagRange>, RagError>;
    fn recheck(
        &self,
        identity: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError>;
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RagQueryInput {
    pub query: String,
    #[serde(default)]
    pub corpus_ids: Vec<String>,
    #[serde(default = "empty_filters")]
    pub filters: Value,
    #[serde(default = "default_limit")]
    pub limit: usize,
}
fn empty_filters() -> Value {
    json!({})
}
fn default_limit() -> usize {
    8
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagEvidenceView {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub chunk_code: String,
    pub title: Option<String>,
    pub text: String,
    pub location: Value,
    pub source: Value,
    pub channels: Vec<String>,
    pub score: f64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagQueryResult {
    pub query: String,
    pub authorized_corpus_ids: Vec<String>,
    pub stages: Vec<RagRetrievalStage>,
    pub warnings: Vec<String>,
    pub evidence: Vec<RagEvidenceView>,
    pub duration_ms: u64,
}
impl<R: RagSearchRepository + Clone, E: RagSearchRuntime> RagService<R, E> {
    pub async fn search(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        input: RagQueryInput,
        run: Option<&eliteforge_agent_framework::RunContext>,
    ) -> Result<RagQueryResult, RagError> {
        if input.query.trim().is_empty()
            || input.query.len() > 16000
            || !(1..=100).contains(&input.limit)
            || !input.filters.is_object()
        {
            return Err(RagError::SearchRequestInvalid);
        }
        let started = std::time::Instant::now();
        let mut ranges = access.resolve(identity)?;
        let authorized_corpus_ids = ranges
            .iter()
            .map(|range| range.corpus_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        // A requested corpus may narrow the source snapshot, never grant access.
        if input
            .corpus_ids
            .iter()
            .any(|id| !authorized_corpus_ids.contains(id))
        {
            return Err(RagError::SearchScopeInvalid);
        }
        ranges.retain(|range| {
            input.corpus_ids.is_empty() || input.corpus_ids.contains(&range.corpus_id)
        });
        let corpora = ranges
            .iter()
            .map(|range| {
                self.repository
                    .get_corpus(&range.corpus_id)?
                    .ok_or(RagError::CorpusNotFound)
            })
            .collect::<Result<Vec<_>, _>>()?;
        validate_request_filters_for_corpora(&corpora.iter().collect::<Vec<_>>(), &input.filters)?;
        let mut rerank = None;
        for (range, corpus) in ranges.iter_mut().zip(&corpora) {
            range.filters = merge_search_request_filters(corpus, &range.filters, &input.filters)?;
            let settings = RagSettings::from_capabilities(&corpus.capabilities)?;
            if rerank.is_none() {
                rerank = settings.rerank_model_code;
            }
        }
        let mut result = self
            .retrieve_authorized(
                &identity.workspace_code,
                &input.query,
                &ranges,
                input.limit,
                rerank.as_deref(),
                run,
            )
            .await?;
        self.recheck_published(&ranges, &mut result)?;
        access.recheck(identity, &mut result)?;
        if result.hits.is_empty() {
            result
                .warnings
                .push("当前授权及已发布范围内没有命中证据".into());
        }
        let evidence = result
            .hits
            .into_iter()
            .map(|hit| {
                let row = hit.row;
                let mut source = row.text_metadata.clone();
                source["publication"] = row.source_metadata.clone();
                let published_version = row.source_metadata["indexTaskId"]
                    .as_str()
                    .unwrap_or(&row.source_code)
                    .to_owned();
                RagEvidenceView {
                    corpus_id: row.corpus_id,
                    source_code: row.source_code,
                    source_version: published_version,
                    chunk_code: row.target_code,
                    title: row.title,
                    text: row.text,
                    location: row.text_metadata["location"].clone(),
                    source,
                    channels: hit.channels.into_iter().collect(),
                    score: hit.score,
                }
            })
            .collect();
        Ok(RagQueryResult {
            query: input.query,
            authorized_corpus_ids: ranges
                .iter()
                .map(|range| range.corpus_id.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
            stages: result.stages,
            warnings: result.warnings,
            evidence,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}
