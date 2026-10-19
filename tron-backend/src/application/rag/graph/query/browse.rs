//! Initial graph windows use published projections as seeds; no embedding call.
use super::*;
use crate::application::rag::{
    RagAccessResolver, RagIdentity, RagSearchRepository, RagSearchRuntime, RagService,
};
use std::time::Instant;

impl<R: RagSearchRepository + Clone, E: RagSearchRuntime> RagService<R, E> {
    pub async fn graph_browse(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        input: GraphBrowseInput,
    ) -> Result<GraphQueryOutput, GraphQueryError> {
        input.options.validate()?;
        if input.options.node_limit
            < input.options.seed_count
                + usize::from(input.options.incoming_depth > 0 || input.options.outgoing_depth > 0)
                    * 2
        {
            return Err(GraphQueryError::Invalid);
        }
        let started = Instant::now();
        let snapshot = self.graph_source_snapshot(self.graph_ranges(
            access,
            identity,
            &input.corpus_ids,
            &input.filters,
        )?)?;
        let infra = self
            .graph
            .as_ref()
            .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
        let phase = Instant::now();
        let page = infra
            .index
            .search_nodes(GraphNodeSearchQuery {
                generation_ids: snapshot.generation_ids().into_iter().collect(),
                matching: GraphNodeMatch::Browse,
                limit: input.options.seed_count,
            })
            .map_err(GraphQueryError::Storage)?;
        let seed_limited = page.has_more;
        let seeds = page
            .hits
            .into_iter()
            .map(|hit| GraphRecallSeed {
                id: hit.entity_id,
                score: hit.score,
                channels: BTreeSet::from(["browse".into()]),
            })
            .collect::<Vec<_>>();
        let stages = vec![seeds::stage(
            &snapshot,
            "graph.browse",
            phase,
            seeds.len(),
            None,
            false,
        )];
        let windows = self.graph_windows(&snapshot, &seeds, &input.options, false)?;
        let request = SavedGraphRequest {
            query: None,
            corpus_ids: input.corpus_ids,
            filters: input.filters,
            options: input.options,
            stamp: snapshot.stamp.clone(),
        };
        let mut output = self.graph_page(
            access,
            identity,
            snapshot,
            request,
            windows,
            stages,
            chrono::Utc::now().timestamp_millis() + GRAPH_SNAPSHOT_TTL_MS,
            started,
        )?;
        if seed_limited {
            output.truncated_by.insert("BrowseSeedBudget".into());
        }
        Ok(output)
    }
}
