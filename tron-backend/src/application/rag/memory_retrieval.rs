use super::*;
use crate::domain::rag::RagError;
use serde_json::json;

impl<R: RagSearchRepository + Clone, E: RagSearchRuntime> RagService<R, E> {
    pub async fn search_memory(
        &self,
        request: RagSearchRequest,
    ) -> Result<RagSearchResult, RagError> {
        if request.workspace_code != request.access_context.workspace_code
            || request.access_context.user_code.trim().is_empty()
            || request.access_context.agent_code.trim().is_empty()
        {
            return Err(RagError::SearchScopeInvalid);
        }
        let corpus = self
            .repository
            .get_corpus("memory-card")?
            .ok_or(RagError::CorpusNotFound)?;
        let filters = merge_search_request_filters(&corpus, &json!({}), &request.filters)?;
        let settings = RagSettings::from_capabilities(&corpus.capabilities)?;
        let mut scopes = vec![
            RagSearchScope {
                scope_type: "USER".into(),
                scope_code: request.access_context.user_code.clone(),
            },
            RagSearchScope {
                scope_type: "AGENT".into(),
                scope_code: request.access_context.agent_code.clone(),
            },
        ];
        if !request.workspace_code.trim().is_empty() {
            scopes.push(RagSearchScope {
                scope_type: "SPACE".into(),
                scope_code: request.workspace_code.clone(),
            });
        }
        let ranges = self.partition_ranges(AuthorizedRagRange {
            corpus_id: "memory-card".into(),
            source_version: "v1".into(),
            scopes,
            filters,
            embedding_model_code: settings.embedding_model_code.clone(),
            embedding_dimension: Some(
                crate::shared::sqlite_extensions::TRON_VECTOR_INDEX_DIMENSION,
            ),
            provider_model: None,
            embedding_fingerprint: None,
            return_fields: request.context.return_fields.clone(),
            fulltext: settings.fulltext || request.query.is_empty(),
            vector: settings.vector,
            graph: settings.graph,
        })?;
        let mut result = self
            .retrieve_authorized(
                super::MEMORY_MODEL_WORKSPACE,
                &request.query,
                &ranges,
                request.limit,
                settings.rerank_model_code.as_deref(),
                None,
            )
            .await?;
        self.recheck_published(&ranges, &mut result)?;
        let hits: Vec<CandidateHit> = result
            .hits
            .into_iter()
            .map(|evidence| {
                let row = evidence.row;
                CandidateHit {
                    corpus_id: row.corpus_id,
                    source_code: row.source_code,
                    source_version: row.source_version,
                    scope_type: row.scope_type,
                    scope_code: row.scope_code,
                    title: row.title,
                    score: evidence.score,
                    text: row.text,
                    metadata: row.source_metadata,
                    fields: row.field_values,
                }
            })
            .collect();
        let context_pack = if request.context.strategy == "RESULT_LIST" {
            None
        } else {
            let mut remaining = request.token_budget;
            let mut items = vec![];
            for hit in &hits {
                let tokens = hit.text.chars().count().div_ceil(4);
                if tokens > remaining {
                    continue;
                }
                remaining -= tokens;
                items.push(ContextPackItem {
                    source_code: hit.source_code.clone(),
                    text: hit.text.clone(),
                    tokens,
                    metadata: hit.metadata.clone(),
                });
            }
            Some(ContextPack {
                items,
                token_count: request.token_budget - remaining,
            })
        };
        Ok(RagSearchResult {
            query: request.query,
            context_strategy: request.context.strategy,
            hits,
            context_pack,
            warnings: result
                .warnings
                .into_iter()
                .map(|warning| json!({"code":"RAG_RETRIEVAL_PARTIAL","message":warning}))
                .collect(),
            trace_code: None,
        })
    }
}
