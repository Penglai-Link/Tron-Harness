//! The ordinary RAG route uses the same candidates, traversal and batch originals.
//! Unlike standalone graph queries it may retain usable channels after a failure.
use super::*;
use crate::application::rag::graph::recall::{GraphRangeRecall, QueryEmbeddingCache};
use crate::application::rag::{RagSearchRepository, RagSearchRuntime, RagService};
use std::{collections::BTreeMap, time::Instant};

impl<R: RagSearchRepository + Clone, E: RagSearchRuntime> RagService<R, E> {
    pub(crate) async fn recall_graph_range(
        &self,
        workspace: &str,
        query: &str,
        range: &AuthorizedRagRange,
        run: Option<&eliteforge_agent_framework::RunContext>,
        cache: &mut QueryEmbeddingCache,
    ) -> Result<GraphRangeRecall, String> {
        if !range.graph || range.scopes.is_empty() {
            return Ok(Default::default());
        }
        self.ordinary_graph_range(workspace, query, range, run, cache)
            .await
            .map_err(|error| match error {
                GraphQueryError::Storage(cause) | GraphQueryError::Unavailable(cause) => cause,
                other => other.to_string(),
            })
    }
    async fn ordinary_graph_range(
        &self,
        workspace: &str,
        query: &str,
        range: &AuthorizedRagRange,
        run: Option<&eliteforge_agent_framework::RunContext>,
        cache: &mut QueryEmbeddingCache,
    ) -> Result<GraphRangeRecall, GraphQueryError> {
        let snapshot = self.graph_source_snapshot(vec![range.clone()])?;
        let mut recalled = self
            .graph_node_seeds(workspace, query, &snapshot, cache, run)
            .await?;
        let options = GraphReadOptions::default();
        recalled.seeds.truncate(options.seed_count);
        let verify = || {
            let current = self.graph_source_snapshot(vec![range.clone()])?;
            if current.stamp != snapshot.stamp {
                Err(GraphQueryError::SnapshotChanged)
            } else {
                Ok(())
            }
        };
        verify()?;
        let windows = self.graph_windows(&snapshot, &recalled.seeds, &options, false)?;
        let phase = Instant::now();
        let window = self.read_graph_windows(windows, &options, &verify)?;
        recalled.stages.push(seeds::stage(
            &snapshot,
            "graph.expand",
            phase,
            window.contexts.iter().map(|c| c.edges.len()).sum(),
            None,
            !window.pending.is_empty(),
        ));
        let phase = Instant::now();
        let mut rows = self.graph_originals(&snapshot, &window.supports)?;
        sources::references_from_rows(&rows, &window.supports)?;
        rank_originals(&window.contexts, &mut rows);
        recalled.stages.push(seeds::stage(
            &snapshot,
            "graph.evidence",
            phase,
            rows.len(),
            None,
            false,
        ));
        verify()?;
        Ok(GraphRangeRecall {
            rows,
            contexts: window.contexts,
            stages: recalled.stages,
            warnings: recalled.warnings,
        })
    }
}

fn rank_originals(contexts: &[GraphRecallContext], rows: &mut [RagSearchRow]) {
    let mut scores = BTreeMap::new();
    for context in contexts {
        let seed_scores = context
            .seeds
            .iter()
            .map(|s| (s.id.as_str(), s.score))
            .collect::<BTreeMap<_, _>>();
        let mut node_scores = BTreeMap::<&str, f64>::new();
        for hop in &context.hops {
            let score = seed_scores
                .get(hop.seed_id.as_str())
                .copied()
                .unwrap_or_default()
                / (1. + f64::from(hop.hops));
            node_scores
                .entry(&hop.node_id)
                .and_modify(|s| *s = s.max(score))
                .or_insert(score);
        }
        for edge in context.edges.iter().chain(&context.provenance) {
            let score = node_scores
                .get(edge.from.as_str())
                .copied()
                .unwrap_or_default()
                .max(
                    node_scores
                        .get(edge.to.as_str())
                        .copied()
                        .unwrap_or_default(),
                );
            let key = (
                context.partition.corpus_id.clone(),
                context.partition.scope_type.clone(),
                context.partition.scope_code.clone(),
                edge.support.source_code.clone(),
                edge.support.source_version.clone(),
                edge.support.chunk_code.clone(),
            );
            scores
                .entry(key)
                .and_modify(|s: &mut f64| *s = s.max(score))
                .or_insert(score);
        }
    }
    let score = |row: &RagSearchRow| {
        scores
            .get(&(
                row.corpus_id.clone(),
                row.scope_type.clone(),
                row.scope_code.clone(),
                row.source_code.clone(),
                row.source_version.clone(),
                row.target_code.clone(),
            ))
            .copied()
            .unwrap_or_default()
    };
    rows.sort_by(|a, b| {
        score(b)
            .total_cmp(&score(a))
            .then(a.target_code.cmp(&b.target_code))
    });
}
