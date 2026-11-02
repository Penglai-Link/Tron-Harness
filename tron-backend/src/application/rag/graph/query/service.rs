use super::*;
use crate::application::rag::{
    RagAccessResolver, RagIdentity, RagSearchRepository, RagSearchRuntime, RagService,
};
use crate::domain::rag::RagError;
use std::time::Instant;

#[derive(Clone, Debug)]
pub enum GraphQueryError {
    Invalid,
    Forbidden,
    Disabled,
    NodeNotFound,
    SnapshotChanged,
    Unavailable(String),
    Storage(String),
}
pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
impl GraphQueryError {
    pub fn descriptor(&self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::Invalid => ("101033004", "tron.rag.graph.invalid_request"),
            Self::Forbidden => ("101033005", "tron.rag.graph.scope_denied"),
            Self::NodeNotFound => ("101033006", "tron.rag.graph.node_not_found"),
            Self::SnapshotChanged => ("101033007", "tron.rag.graph.snapshot_changed"),
            Self::Unavailable(_) => ("101033008", "tron.rag.graph.channel_unavailable"),
            Self::Storage(_) => ("101033009", "tron.rag.graph.storage_failed"),
            Self::Disabled => ("101033010", "tron.rag.graph.disabled"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
    pub(super) fn from_rag(error: RagError) -> Self {
        match error {
            RagError::SearchScopeInvalid | RagError::IngestScopeInvalid => Self::Forbidden,
            RagError::SearchRequestInvalid | RagError::SearchFilterInvalid => Self::Invalid,
            RagError::CorpusNotFound | RagError::SourceNotFound => Self::SnapshotChanged,
            other => Self::Storage(other.to_string()),
        }
    }
}
impl From<GraphSnapshotError> for GraphQueryError {
    fn from(error: GraphSnapshotError) -> Self {
        match error {
            GraphSnapshotError::Invalid => Self::Invalid,
            GraphSnapshotError::Unavailable | GraphSnapshotError::PublicationChanged => {
                Self::SnapshotChanged
            }
            GraphSnapshotError::Storage(error) => Self::Storage(error),
        }
    }
}
impl std::fmt::Display for GraphQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.descriptor().message_key)
    }
}
impl std::error::Error for GraphQueryError {}

impl<R: RagSearchRepository + Clone, E: RagSearchRuntime> RagService<R, E> {
    pub async fn graph_search(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        input: GraphSearchInput,
        run: Option<&eliteforge_agent_framework::RunContext>,
    ) -> Result<GraphQueryOutput, GraphQueryError> {
        input.options.validate()?;
        if input.query.trim().is_empty()
            || input.query.len() > 16_000
            || input.options.node_limit
                < input.options.seed_count
                    + usize::from(
                        input.options.incoming_depth > 0 || input.options.outgoing_depth > 0,
                    ) * 2
        {
            return Err(GraphQueryError::Invalid);
        }
        let started = Instant::now();
        let ranges = self.graph_ranges(access, identity, &input.corpus_ids, &input.filters)?;
        let snapshot = self.graph_source_snapshot(ranges)?;
        let verify = || {
            self.verify_graph_snapshot(
                access,
                identity,
                &input.corpus_ids,
                &input.filters,
                &snapshot.stamp,
            )
        };
        let mut recalled = self
            .graph_node_seeds(
                &identity.workspace_code,
                &input.query,
                &snapshot,
                &mut Default::default(),
                run,
            )
            .await?;
        verify()?;
        if let Some(failed) = recalled.stages.iter().find(|s| s.status == "FAILED") {
            return Err(GraphQueryError::Unavailable(
                failed
                    .error
                    .clone()
                    .unwrap_or_else(|| failed.channel.clone()),
            ));
        }
        recalled.seeds.truncate(input.options.seed_count);
        let windows = self.graph_windows(&snapshot, &recalled.seeds, &input.options, false)?;
        let request = SavedGraphRequest {
            query: Some(input.query),
            corpus_ids: input.corpus_ids,
            filters: input.filters,
            options: input.options,
            stamp: snapshot.stamp.clone(),
        };
        let candidate_limited = recalled
            .warnings
            .iter()
            .any(|warning| warning == "NodeVectorCandidateBudget");
        let mut result = self.graph_page(
            access,
            identity,
            snapshot,
            request,
            SavedTraversal {
                windows,
                browse: None,
            },
            recalled.stages,
            chrono::Utc::now().timestamp_millis() + GRAPH_SNAPSHOT_TTL_MS,
            started,
        )?;
        if candidate_limited {
            result
                .truncated_by
                .insert("NodeVectorCandidateBudget".into());
        }
        Ok(result)
    }

    pub async fn graph_expand(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        input: GraphExpandInput,
    ) -> Result<GraphQueryOutput, GraphQueryError> {
        let started = Instant::now();
        let now = chrono::Utc::now().timestamp_millis();
        if let Some(cursor) = input.cursor {
            if !input.node_ids.is_empty()
                || !input.corpus_ids.is_empty()
                || input.filters != serde_json::json!({})
                || input.options != GraphReadOptions::default()
            {
                return Err(GraphQueryError::Invalid);
            }
            let infra = self
                .graph
                .as_ref()
                .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
            let saved = infra
                .index
                .load_snapshot(&identity_hash(identity), &cursor, now)?;
            let request: SavedGraphRequest = serde_json::from_value(saved.request)
                .map_err(|e| GraphQueryError::Storage(e.to_string()))?;
            let mut traversal: SavedTraversal = serde_json::from_value(saved.traversal)
                .map_err(|e| GraphQueryError::Storage(e.to_string()))?;
            let snapshot = self.verify_graph_snapshot(
                access,
                identity,
                &request.corpus_ids,
                &request.filters,
                &request.stamp,
            )?;
            if snapshot.generation_ids() != saved.generation_ids {
                return Err(GraphQueryError::SnapshotChanged);
            }
            let mut stages = vec![];
            if traversal.windows.is_empty()
                && let Some(browse) = &mut traversal.browse
            {
                let (windows, stage) =
                    self.next_browse_windows(&snapshot, &request.options, browse)?;
                traversal.windows = windows;
                stages.push(stage);
            }
            return self.graph_page(
                access,
                identity,
                snapshot,
                request,
                traversal,
                stages,
                saved.expires_at,
                started,
            );
        }
        input.options.validate()?;
        let ids = input.node_ids.iter().cloned().collect::<BTreeSet<_>>();
        if ids.is_empty()
            || ids.len() > 100
            || ids.len() != input.node_ids.len()
            || ids.iter().any(|id| id.is_empty() || id.len() > 256)
        {
            return Err(GraphQueryError::Invalid);
        }
        let ranges = self.graph_ranges(access, identity, &input.corpus_ids, &input.filters)?;
        let snapshot = self.graph_source_snapshot(ranges)?;
        let seeds = input
            .node_ids
            .into_iter()
            .map(|id| GraphRecallSeed {
                id,
                score: 0.,
                channels: BTreeSet::from(["selected".into()]),
            })
            .collect::<Vec<_>>();
        let windows = self.graph_windows(&snapshot, &seeds, &input.options, true)?;
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
                browse: None,
            },
            vec![],
            now + GRAPH_SNAPSHOT_TTL_MS,
            started,
        )
    }

    fn verify_graph_snapshot(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        corpus_ids: &[String],
        filters: &Value,
        expected: &str,
    ) -> Result<AuthorizedGraphSnapshot, GraphQueryError> {
        let ranges = self
            .graph_ranges(access, identity, corpus_ids, filters)
            .map_err(|error| {
                if matches!(error, GraphQueryError::Disabled) {
                    GraphQueryError::SnapshotChanged
                } else {
                    error
                }
            })?;
        let snapshot = self.graph_source_snapshot(ranges)?;
        if snapshot.stamp != expected {
            return Err(GraphQueryError::SnapshotChanged);
        }
        Ok(snapshot)
    }

    pub(super) fn graph_page(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        snapshot: AuthorizedGraphSnapshot,
        request: SavedGraphRequest,
        traversal: SavedTraversal,
        mut stages: Vec<RagRetrievalStage>,
        expires_at: i64,
        started: Instant,
    ) -> Result<GraphQueryOutput, GraphQueryError> {
        let verify = || {
            self.verify_graph_snapshot(
                access,
                identity,
                &request.corpus_ids,
                &request.filters,
                &request.stamp,
            )
            .map(|_| ())
        };
        let phase = Instant::now();
        let window = self.read_graph_windows(traversal.windows, &request.options, &verify)?;
        let browse = if let Some(browse) = traversal.browse {
            let infra = self
                .graph
                .as_ref()
                .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
            let remaining = infra
                .index
                .search_nodes(GraphNodeSearchQuery {
                    generation_ids: snapshot.generation_ids().into_iter().collect(),
                    matching: GraphNodeMatch::Browse {
                        excluded_ids: browse.seeded_ids.clone(),
                    },
                    limit: 1,
                })
                .map_err(GraphQueryError::Storage)?;
            (!remaining.hits.is_empty()).then_some(browse)
        } else {
            None
        };
        stages.push(seeds::stage(
            &snapshot,
            "graph.expand",
            phase,
            window.contexts.iter().map(|c| c.edges.len()).sum(),
            None,
            !window.pending.is_empty(),
        ));
        let phase = Instant::now();
        let rows = self.graph_originals(&snapshot, &window.supports)?;
        let references = sources::authorize_originals(access, identity, rows, &window.supports)?;
        stages.push(seeds::stage(
            &snapshot,
            "graph.evidence",
            phase,
            references.len(),
            None,
            false,
        ));
        verify()?;
        let unfinished = window::unfinished(&window.pending);
        let more_browse = browse.is_some();
        let next_cursor = if window.pending.is_empty() && browse.is_none() {
            None
        } else {
            let infra = self
                .graph
                .as_ref()
                .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
            let draft = GraphSnapshotDraft {
                owner_hash: stable_id("GRAPH_OWNER", &[&identity.user_code]),
                identity_hash: identity_hash(identity),
                request: serde_json::to_value(&request)
                    .map_err(|e| GraphQueryError::Storage(e.to_string()))?,
                traversal: serde_json::to_value(SavedTraversal {
                    windows: window.pending,
                    browse,
                })
                .map_err(|e| GraphQueryError::Storage(e.to_string()))?,
                generation_ids: snapshot.generation_ids(),
                expires_at,
            };
            Some(
                infra
                    .index
                    .save_snapshot(draft, chrono::Utc::now().timestamp_millis())?
                    .cursor,
            )
        };
        // A grant change while saving the cursor must not publish even a partial page.
        verify()?;
        let mut truncated_by = window.truncated_by;
        if more_browse {
            truncated_by.insert("BrowseSeedBudget".into());
        }
        Ok(GraphQueryOutput {
            snapshot_id: snapshot.stamp.clone(),
            contexts: window.contexts,
            references,
            stages,
            expires_at: next_cursor.as_ref().map(|_| expires_at),
            next_cursor,
            unfinished,
            truncated_by,
            duration_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        })
    }
}
fn identity_hash(identity: &RagIdentity) -> String {
    stable_id(
        "GRAPH_IDENTITY",
        &[
            &identity.user_code,
            &identity.workspace_code,
            &identity.agent_code,
        ],
    )
}
