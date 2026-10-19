use super::*;
use crate::adapters::outbound::persistence::sqlite::StoredChunkContext;
use eliteforge_agent_core::ChunkSourceSpan;

mod source_context;

pub(crate) type ContextKey = (String, String, String);
pub(crate) const RETRIEVAL_CONTEXT_CHARS: usize = 16000;

impl KnowledgeBaseService {
    pub(crate) fn expand_context_evidence(
        &self,
        user: &str,
        result: &mut RagEvidenceResult,
    ) -> Result<(), KnowledgeBaseError> {
        let keys = result
            .hits
            .iter()
            .flat_map(|hit| {
                std::iter::once(hit)
                    .chain(hit.bundle.iter().flat_map(|bundle| bundle.members.iter()))
            })
            .filter_map(|hit| {
                hit.row.text_metadata["contextId"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .map(|id| {
                        (
                            hit.row.corpus_id.clone(),
                            hit.row.source_code.clone(),
                            id.into(),
                        )
                    })
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let contexts = self
            .repository
            .chunk_contexts(user, keys)?
            .into_iter()
            .map(|(kb, source, context)| ((kb, source, context.id.clone()), context))
            .collect();
        expand_result(result, &contexts);
        Ok(())
    }
}

/// Preserve the matched original blocks instead of replacing a hit with a whole
/// packing group. A packing group can contain unrelated paragraphs or notices.
pub(crate) fn expand_result(
    result: &mut RagEvidenceResult,
    contexts: &BTreeMap<ContextKey, StoredChunkContext>,
) {
    let mut remaining = RETRIEVAL_CONTEXT_CHARS;
    expand_with_budget(result, contexts, &mut remaining);
}
fn expand_with_budget(
    result: &mut RagEvidenceResult,
    contexts: &BTreeMap<ContextKey, StoredChunkContext>,
    remaining: &mut usize,
) {
    // Keep every original member for all-member authorization. Deduplicating the
    // display continuation must not erase its underlying publication anchors.
    if result.hits.iter().any(|hit| hit.bundle.is_some()) {
        for hit in &mut result.hits {
            let Some(mut bundle) = hit.bundle.take() else {
                let mut single = RagEvidenceResult {
                    hits: vec![hit.clone()],
                    ..Default::default()
                };
                expand_with_budget(&mut single, contexts, remaining);
                if let Some(expanded) = single.hits.pop() {
                    *hit = expanded;
                }
                result.warnings.extend(single.warnings);
                continue;
            };
            let mut group = RagEvidenceResult {
                hits: std::iter::once(hit.clone())
                    .chain(bundle.members.iter().cloned())
                    .collect(),
                ..Default::default()
            };
            expand_with_budget(&mut group, contexts, remaining);
            if group
                .warnings
                .iter()
                .any(|warning| warning == "SourceContextBudget")
            {
                bundle.truncated_by.insert("SourceContextBudget".into());
            }
            bundle.display = group.hits;
            result.warnings.extend(group.warnings);
            hit.bundle = Some(bundle);
        }
        return;
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut budget_limited = false;
    result.hits.retain_mut(|hit| {
        let key = hit.row.text_metadata["contextId"].as_str().map(|id| {
            (
                hit.row.corpus_id.clone(),
                hit.row.source_code.clone(),
                id.into(),
            )
        });
        let matched = crate::application::rag::selected_source_spans(
            &hit.row.text_metadata,
            &hit.selected_blocks,
        );
        let safe = key
            .as_ref()
            .and_then(|key| contexts.get_key_value(key))
            .zip(matched.as_deref())
            .and_then(|((key, context), spans)| {
                source_context::select(context, spans).map(|safe| (key, safe))
            });
        if let Some((key, context)) = safe {
            let identity = (key.clone(), context.blocks.clone());
            if seen.contains(&identity) {
                return false;
            }
            let count = context.text.chars().count();
            if count <= *remaining {
                if hit.row.text_metadata.get("matchedText").is_none() {
                    hit.row.text_metadata["matchedText"] = hit.row.text.clone().into();
                    hit.row.text_metadata["matchedLocation"] =
                        hit.row.text_metadata["location"].clone();
                }
                let source_ids = hit.row.text_metadata["sourceLocations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|location| {
                        hit.selected_blocks.is_empty()
                            || location["sourceBlockId"]
                                .as_str()
                                .is_some_and(|id| hit.selected_blocks.contains(id))
                    })
                    .filter_map(|location| location["sourceBlockId"].as_str())
                    .collect::<std::collections::BTreeSet<_>>();
                let locations = contexts[key]
                    .source_locations
                    .iter()
                    .filter(|location| {
                        location["sourceBlockId"]
                            .as_str()
                            .is_some_and(|id| source_ids.contains(id))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let pages = locations
                    .iter()
                    .flat_map(|location| {
                        location
                            .get("pages")
                            .and_then(|p| p.as_array())
                            .into_iter()
                            .flatten()
                            .filter_map(|p| p.as_u64())
                            .chain(location.get("page").and_then(|p| p.as_u64()))
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                if let Some(location) = hit.row.text_metadata["location"]
                    .as_object_mut()
                    .filter(|_| !locations.is_empty())
                {
                    location.insert("pages".into(), serde_json::json!(pages));
                }

                hit.row.text_metadata["contextSpans"] =
                    serde_json::to_value(&context.spans).unwrap_or_default();
                hit.row.text_metadata["contextLocations"] =
                    serde_json::to_value(&locations).unwrap_or_default();
                hit.row.text_metadata["contextAssembly"] = "OriginalBlockContinuations".into();
                hit.context_text = Some(context.text.clone());
                *remaining -= count;
                seen.insert(identity);
                return true;
            }
            budget_limited = true;
        }
        // Exhausting the expansion budget never removes an original candidate
        // before rerank/Top N. Keep the matched publication row as a fallback.
        true
    });
    if budget_limited
        && !result
            .warnings
            .iter()
            .any(|warning| warning == "SourceContextBudget")
    {
        result.warnings.push("SourceContextBudget".into());
    }
}

#[cfg(test)]
mod tests;
