//! A complete source group is authorized as a unit before and after model work.
use super::*;
use crate::domain::rag::{RagError, graph::stable_id};
use std::collections::BTreeSet;

pub(crate) fn row_identity(row: &RagSearchRow) -> String {
    stable_id(
        "EVIDENCE",
        &[&serde_json::to_string(&(
            &row.corpus_id,
            &row.source_code,
            &row.source_version,
            &row.scope_type,
            &row.scope_code,
            &row.target_code,
            &row.text,
            &row.source_metadata,
        ))
        .expect("serializable source evidence")],
    )
}
impl<R: RagSearchRepository + Clone, E: Clone> RagService<R, E> {
    pub(crate) fn recheck_evidence_members(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        ranges: &[AuthorizedRagRange],
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        let mut seen = BTreeSet::new();
        let mut check = RagEvidenceResult {
            hits: result
                .hits
                .iter()
                .flat_map(|hit| {
                    std::iter::once(hit)
                        .chain(hit.bundle.iter().flat_map(|bundle| bundle.members.iter()))
                })
                .filter(|hit| seen.insert(row_identity(&hit.row)))
                .map(|hit| RagEvidence::associated(hit.row.clone()))
                .collect(),
            ..Default::default()
        };
        self.recheck_published(ranges, &mut check)?;
        access.recheck_sources(identity, &mut check)?;
        let visible = check
            .hits
            .iter()
            .map(|hit| row_identity(&hit.row))
            .collect::<BTreeSet<_>>();
        let generations = if result.hits.iter().any(|hit| hit.bundle.is_some()) {
            self.active_bundle_generations(ranges)
                .map_err(|_| RagError::StorageFailure)?
        } else {
            BTreeSet::new()
        };
        let before = result.hits.len();
        result.hits.retain(|hit| {
            visible.contains(&row_identity(&hit.row))
                && hit.bundle.as_ref().is_none_or(|bundle| {
                    bundle.generations.is_subset(&generations)
                        && bundle
                            .members
                            .iter()
                            .all(|member| visible.contains(&row_identity(&member.row)))
                })
        });
        if result.hits.len() != before {
            super::access::invalidate_graph_statistics(result);
            result.warnings.push(
                crate::shared::i18n_message::MessageText::new("rag.bundle.scope_changed")
                    .encode()
                    .into(),
            );
        }
        Ok(())
    }
}
