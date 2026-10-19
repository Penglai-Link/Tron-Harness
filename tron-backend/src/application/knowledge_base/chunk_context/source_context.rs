//! Recover only ranges belonging to the matched original blocks from a packed context.
use super::{ChunkSourceSpan, StoredChunkContext};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct SourceContext {
    pub text: String,
    pub spans: Vec<ChunkSourceSpan>,
    pub blocks: Vec<String>,
}

pub(super) fn select(
    context: &StoredChunkContext,
    matched: &[ChunkSourceSpan],
) -> Option<SourceContext> {
    if matched.is_empty() || context.spans.len() > 10_000 {
        return None;
    }
    let selected = matched
        .iter()
        .map(|span| span.block_id.clone())
        .collect::<BTreeSet<_>>();
    let mut blocks = BTreeMap::<usize, (String, Vec<(usize, usize)>)>::new();
    for span in &context.spans {
        if span.start >= span.end {
            return None;
        }
        // These are Tron normalized-block coordinates, not supplier page numbers.
        // Unknown/custom span identities are not interpreted as document order.
        let index = span
            .block_id
            .strip_prefix("block-")?
            .parse::<usize>()
            .ok()?;
        let entry = blocks
            .entry(index)
            .or_insert_with(|| (span.block_id.clone(), vec![]));
        if entry.0 != span.block_id {
            return None;
        }
        entry.1.push((span.start, span.end));
    }
    let mut segments = Vec::new();
    for (_, (id, mut ranges)) in blocks {
        ranges.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in ranges {
            if let Some(previous) = merged.last_mut().filter(|p| start <= p.1) {
                previous.1 = previous.1.max(end);
            } else {
                merged.push((start, end));
            }
        }
        segments.extend(merged.into_iter().map(|(start, end)| ChunkSourceSpan {
            block_id: id.clone(),
            start,
            end,
        }));
    }
    if matched.iter().any(|span| {
        span.start >= span.end
            || !segments.iter().any(|original| {
                original.block_id == span.block_id
                    && original.start <= span.start
                    && original.end >= span.end
            })
    }) {
        return None;
    }
    // Framework renders these merged source ranges in original block order,
    // separated by one newline. Verify the complete representation before slicing.
    let expected = segments
        .iter()
        .try_fold(0usize, |n, span| n.checked_add(span.end - span.start))?
        .checked_add(segments.len().saturating_sub(1))?;
    if expected > 128_000 || context.text.chars().count() != expected {
        return None;
    }
    let chars = context.text.chars().collect::<Vec<_>>();
    let mut offset = 0usize;
    let mut text = Vec::new();
    let mut spans = Vec::new();
    for span in segments {
        let end = offset.checked_add(span.end - span.start)?;
        let segment = chars.get(offset..end)?;
        if selected.contains(&span.block_id) {
            text.push(segment.iter().collect::<String>());
            spans.push(span);
        }
        if end < chars.len() && chars.get(end) != Some(&'\n') {
            return None;
        }
        offset = end.checked_add(1)?;
    }
    if spans.is_empty() {
        return None;
    }
    Some(SourceContext {
        text: text.join("\n"),
        spans,
        blocks: selected.into_iter().collect(),
    })
}
