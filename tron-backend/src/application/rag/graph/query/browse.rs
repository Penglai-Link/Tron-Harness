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
        let mut browse = GraphBrowseState::default();
        let (windows, stage) = self.next_browse_windows(&snapshot, &input.options, &mut browse)?;
        let request = SavedGraphRequest {
            query: None,
            corpus_ids: input.corpus_ids,
            filters: input.filters,
            options: input.options,
            stamp: snapshot.stamp.clone(),
        };
        self.graph_page(
            access,
            identity,
            snapshot,
            request,
            SavedTraversal {
                windows,
                browse: Some(browse),
            },
            vec![stage],
            chrono::Utc::now().timestamp_millis() + GRAPH_SNAPSHOT_TTL_MS,
            started,
        )
    }

    pub(super) fn next_browse_windows(
        &self,
        snapshot: &AuthorizedGraphSnapshot,
        options: &GraphReadOptions,
        browse: &mut GraphBrowseState,
    ) -> Result<
        (
            Vec<GraphWindowState>,
            crate::application::rag::RagRetrievalStage,
        ),
        GraphQueryError,
    > {
        let infra = self
            .graph
            .as_ref()
            .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
        let phase = Instant::now();
        let page = infra
            .index
            .search_nodes(GraphNodeSearchQuery {
                generation_ids: snapshot.generation_ids().into_iter().collect(),
                matching: GraphNodeMatch::Browse {
                    excluded_ids: browse.seeded_ids.clone(),
                },
                limit: options.seed_count,
            })
            .map_err(GraphQueryError::Storage)?;
        let seeds = page
            .hits
            .into_iter()
            .map(|hit| GraphRecallSeed {
                id: hit.entity_id,
                score: hit.score,
                channels: BTreeSet::from(["browse".into()]),
            })
            .collect::<Vec<_>>();
        // Retain seed identities, rather than every traversed node: a later seed
        // can still reveal relationships beyond an earlier seed's depth bound.
        browse
            .seeded_ids
            .extend(seeds.iter().map(|seed| seed.id.clone()));
        let stage = seeds::stage(
            snapshot,
            "graph.browse",
            phase,
            seeds.len(),
            None,
            page.has_more,
        );
        let windows = self.graph_windows(snapshot, &seeds, options, false)?;
        Ok((windows, stage))
    }
}
