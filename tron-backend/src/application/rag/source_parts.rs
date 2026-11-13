//! Recover parser-block fragments from verified span order, never from proximity.
use eliteforge_agent_core::ChunkSourceSpan;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) struct SourcePart {
    pub id: String,
    pub text: String,
    pub location: Value,
}
pub(crate) fn parts(metadata: &Value, text: &str, fallback: &str) -> Option<Vec<SourcePart>> {
    let locations = metadata["sourceLocations"]
        .as_array()
        .filter(|v| !v.is_empty())
        .cloned()
        .unwrap_or_else(|| {
            vec![
                metadata
                    .get("location")
                    .cloned()
                    .filter(Value::is_object)
                    .unwrap_or_else(|| json!({})),
            ]
        });
    let identity = |location: &Value| {
        location["sourceBlockId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("chunk/{fallback}"))
    };
    let ids = locations.iter().map(identity).collect::<BTreeSet<_>>();
    if ids.len() == 1 {
        return Some(vec![SourcePart {
            id: ids.into_iter().next()?,
            text: text.into(),
            location: locations[0].clone(),
        }]);
    }
    let spans: Vec<ChunkSourceSpan> =
        serde_json::from_value(metadata["sourceSpans"].clone()).ok()?;
    if spans.len() != locations.len() || spans.iter().any(|s| s.start >= s.end) {
        return None;
    }
    let expected = spans
        .iter()
        .try_fold(0usize, |n, s| n.checked_add(s.end - s.start))?
        .checked_add(spans.len().saturating_sub(1))?;
    let raw = text.chars().collect::<Vec<_>>();
    let separators_match = |chars: &[char]| {
        let mut offset = 0;
        for span in spans.iter().take(spans.len() - 1) {
            offset += span.end - span.start;
            if chars.get(offset) != Some(&'\n') {
                return false;
            }
            offset += 1;
        }
        true
    };
    // Prefer the exact original text. Some packers add one framing newline;
    // accept it only when there is exactly one verifiable interpretation.
    // Trimming all newlines would corrupt legitimate original block content.
    let chars = if raw.len() == expected {
        raw.as_slice()
    } else {
        let mut candidates = Vec::new();
        for start in 0..=usize::from(raw.first() == Some(&'\n')) {
            for end in 0..=usize::from(raw.last() == Some(&'\n')) {
                if start + end > raw.len() || raw.len() - start - end != expected {
                    continue;
                }
                let slice = &raw[start..raw.len() - end];
                if separators_match(slice) {
                    candidates.push(slice);
                }
            }
        }
        if candidates.len() != 1 {
            return None;
        }
        candidates[0]
    };
    if !separators_match(chars) {
        return None;
    }
    let mut offset = 0;
    let mut parts = Vec::new();
    for (i, (span, location)) in spans.iter().zip(locations).enumerate() {
        if let (Some(base), Some(start), Some(end)) = (
            location["sourceBlockStart"].as_u64(),
            location["charStart"].as_u64(),
            location["charEnd"].as_u64(),
        ) {
            if start.checked_sub(base) != Some(span.start as u64)
                || end.checked_sub(base) != Some(span.end as u64)
            {
                return None;
            }
        }
        let end = offset + span.end - span.start;
        parts.push(SourcePart {
            id: identity(&location),
            text: chars.get(offset..end)?.iter().collect(),
            location,
        });
        if i + 1 < spans.len() && chars.get(end) != Some(&'\n') {
            return None;
        }
        offset = end + 1;
    }
    Some(parts)
}

pub(crate) fn select(
    metadata: &Value,
    text: &str,
    fallback: &str,
    selected: &BTreeSet<String>,
) -> Option<String> {
    let selected = parts(metadata, text, fallback)?
        .into_iter()
        .filter(|part| selected.contains(&part.id))
        .map(|part| part.text)
        .collect::<Vec<_>>();
    (!selected.is_empty()).then(|| selected.join("\n"))
}

pub(crate) fn selected_spans(
    metadata: &Value,
    selected: &BTreeSet<String>,
) -> Option<Vec<ChunkSourceSpan>> {
    let spans: Vec<ChunkSourceSpan> =
        serde_json::from_value(metadata["sourceSpans"].clone()).ok()?;
    if selected.is_empty() {
        return Some(spans);
    }
    let locations = metadata["sourceLocations"].as_array()?;
    if locations.len() != spans.len() {
        return None;
    }
    Some(
        spans
            .into_iter()
            .zip(locations)
            .filter(|(_, location)| {
                location["sourceBlockId"]
                    .as_str()
                    .is_some_and(|id| selected.contains(id))
            })
            .map(|(span, _)| span)
            .collect(),
    )
}
