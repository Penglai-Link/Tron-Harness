//! Resolve source grants before node recall; batch-check every returned original quote.
use super::*;
use crate::application::rag::{
    RagAccessResolver, RagEvidence, RagEvidenceResult, RagIdentity, RagRouteSearchQuery,
    RagSearchRepository, RagService, RagSettings, merge_search_request_filters,
    validate_request_filters_for_corpora,
};
use serde_json::json;
use std::collections::BTreeMap;

pub(crate) fn source_query(range: &AuthorizedRagRange) -> RagRouteSearchQuery {
    RagRouteSearchQuery {
        corpus_id: range.corpus_id.clone(),
        route_id: "graph-source".into(),
        retriever: "STRUCTURED_FILTER_SQLITE_V1".into(),
        workspace_code: String::new(),
        query: String::new(),
        query_embedding: None,
        query_embedding_model_code: None,
        source_version: range.source_version.clone(),
        scopes: range.scopes.clone(),
        filters: range.filters.clone(),
        policy: json!({}),
        return_fields: range.return_fields.clone(),
        limit: 100,
    }
}

impl<R: RagSearchRepository + Clone, E: Clone> RagService<R, E> {
    pub(super) fn graph_ranges(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        corpus_ids: &[String],
        filters: &Value,
    ) -> Result<Vec<AuthorizedRagRange>, GraphQueryError> {
        if corpus_ids.len() > 100 || !filters.is_object() {
            return Err(GraphQueryError::Invalid);
        }
        let mut ranges = access
            .resolve(identity)
            .map_err(GraphQueryError::from_rag)?;
        let permitted = ranges
            .iter()
            .map(|r| r.corpus_id.as_str())
            .collect::<BTreeSet<_>>();
        if corpus_ids.iter().any(|id| !permitted.contains(id.as_str())) {
            return Err(GraphQueryError::Forbidden);
        }
        ranges.retain(|r| corpus_ids.is_empty() || corpus_ids.contains(&r.corpus_id));
        let corpora = ranges
            .iter()
            .map(|range| {
                self.repository
                    .get_corpus(&range.corpus_id)
                    .map_err(GraphQueryError::from_rag)?
                    .ok_or(GraphQueryError::SnapshotChanged)
            })
            .collect::<Result<Vec<_>, _>>()?;
        validate_request_filters_for_corpora(&corpora.iter().collect::<Vec<_>>(), filters)
            .map_err(GraphQueryError::from_rag)?;
        for (range, corpus) in ranges.iter_mut().zip(corpora) {
            let enabled = RagSettings::from_capabilities(&corpus.capabilities)
                .map_err(GraphQueryError::from_rag)?
                .graph;
            if !enabled && corpus_ids.contains(&range.corpus_id) {
                return Err(GraphQueryError::Disabled);
            }
            range.graph &= enabled;
            range.filters = merge_search_request_filters(&corpus, &range.filters, filters)
                .map_err(GraphQueryError::from_rag)?;
        }
        ranges.retain(|r| r.graph && !r.scopes.is_empty());
        Ok(ranges)
    }

    pub(super) fn graph_source_snapshot(
        &self,
        ranges: Vec<AuthorizedRagRange>,
    ) -> Result<AuthorizedGraphSnapshot, GraphQueryError> {
        let mut permitted = BTreeSet::new();
        let mut partitions = BTreeSet::new();
        for range in &ranges {
            let corpus = self
                .repository
                .get_corpus(&range.corpus_id)
                .map_err(GraphQueryError::from_rag)?
                .ok_or(GraphQueryError::SnapshotChanged)?;
            if !range.graph
                || !RagSettings::from_capabilities(&corpus.capabilities)
                    .map_err(GraphQueryError::from_rag)?
                    .graph
            {
                continue;
            }
            permitted.extend(
                self.repository
                    .published_graph_generations(source_query(range))
                    .map_err(GraphQueryError::from_rag)?,
            );
            partitions.extend(range.scopes.iter().map(|s| GraphPartition {
                corpus_id: range.corpus_id.clone(),
                scope_type: s.scope_type.clone(),
                scope_code: s.scope_code.clone(),
            }));
        }
        let mut manifests = if permitted.is_empty() {
            vec![]
        } else {
            let infra = self
                .graph
                .as_ref()
                .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
            infra
                .index
                .active_generations(&partitions.into_iter().collect::<Vec<_>>())
                .map_err(GraphQueryError::Storage)?
                .into_iter()
                .filter(|g| permitted.contains(&g.id))
                .collect::<Vec<_>>()
        };
        manifests.sort_by(|a, b| a.id.cmp(&b.id));
        let stamp = stable_id(
            "GRAPH_SNAPSHOT",
            &[&serde_json::to_string(&manifests)
                .map_err(|e| GraphQueryError::Storage(e.to_string()))?],
        );
        Ok(AuthorizedGraphSnapshot {
            ranges,
            manifests,
            stamp,
        })
    }

    pub(super) fn graph_originals(
        &self,
        snapshot: &AuthorizedGraphSnapshot,
        supports: &[(GraphPartition, RagGraphSupport)],
    ) -> Result<Vec<RagSearchRow>, GraphQueryError> {
        let mut rows = BTreeMap::new();
        for range in &snapshot.ranges {
            let codes = supports
                .iter()
                .filter(|(p, _)| p.corpus_id == range.corpus_id)
                .map(|(_, s)| s.chunk_code.clone())
                .collect::<BTreeSet<_>>();
            if codes.is_empty() {
                continue;
            }
            let mut query = source_query(range);
            query.limit = codes.len();
            query.policy = json!({"targetCodes":codes});
            for row in self
                .repository
                .search_route(query)
                .map_err(GraphQueryError::from_rag)?
            {
                rows.insert(row_key(&row), row);
            }
        }
        Ok(rows.into_values().collect())
    }
}

pub(super) type OriginalKey = (String, String, String, String, String, String);
fn row_key(row: &RagSearchRow) -> OriginalKey {
    (
        row.corpus_id.clone(),
        row.source_code.clone(),
        row.source_version.clone(),
        row.scope_type.clone(),
        row.scope_code.clone(),
        row.target_code.clone(),
    )
}
fn support_key(partition: &GraphPartition, s: &RagGraphSupport) -> OriginalKey {
    (
        partition.corpus_id.clone(),
        s.source_code.clone(),
        s.source_version.clone(),
        partition.scope_type.clone(),
        partition.scope_code.clone(),
        s.chunk_code.clone(),
    )
}

pub(super) fn authorize_originals(
    access: &dyn RagAccessResolver,
    identity: &RagIdentity,
    rows: Vec<RagSearchRow>,
    supports: &[(GraphPartition, RagGraphSupport)],
) -> Result<Vec<GraphSourceReference>, GraphQueryError> {
    let mut check = RagEvidenceResult {
        hits: rows
            .into_iter()
            .map(|row| RagEvidence {
                row,
                score: 0.,
                context_text: None,
                recall_ranks: Default::default(),
                bundle: None,
                selected_blocks: Default::default(),
                channels: BTreeSet::from(["graph".into()]),
            })
            .collect(),
        ..Default::default()
    };
    access
        .recheck_sources(identity, &mut check)
        .map_err(GraphQueryError::from_rag)?;
    references_from_rows(
        &check
            .hits
            .into_iter()
            .map(|hit| hit.row)
            .collect::<Vec<_>>(),
        supports,
    )
}

pub(super) fn references_from_rows(
    rows: &[RagSearchRow],
    supports: &[(GraphPartition, RagGraphSupport)],
) -> Result<Vec<GraphSourceReference>, GraphQueryError> {
    let rows = rows
        .iter()
        .map(|row| (row_key(row), row))
        .collect::<BTreeMap<_, _>>();
    let mut references = BTreeMap::new();
    for (partition, support) in supports {
        let row = rows
            .get(&support_key(partition, support))
            .ok_or(GraphQueryError::SnapshotChanged)?;
        if !support.quote.is_empty() && !row.text.contains(&support.quote) {
            return Err(GraphQueryError::SnapshotChanged);
        }
        let id = graph_reference_id(partition, support);
        references
            .entry(id.clone())
            .or_insert_with(|| GraphSourceReference {
                id,
                corpus_id: partition.corpus_id.clone(),
                source_code: support.source_code.clone(),
                source_version: support.source_version.clone(),
                generation_id: support.generation_id.clone(),
                chunk_code: support.chunk_code.clone(),
                file_name: row.title.clone(),
                document_id: row.text_metadata["documentId"].as_str().map(str::to_owned),
                resource_code: row.text_metadata["resourceCode"]
                    .as_str()
                    .map(str::to_owned),
                workspace_code: partition.scope_code.clone(),
                quote: support.quote.clone(),
                location: support.location.clone(),
            });
    }
    Ok(references.into_values().collect())
}
