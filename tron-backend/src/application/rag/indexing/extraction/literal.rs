//! Suggest a unique original quote for model correction; never modify facts or validation.
use crate::application::rag::RagTextUnitWrite;
use serde_json::Value;

fn unique_original<'a>(source: &'a str, quote: &str) -> Option<&'a str> {
    if quote.len() > 4096 || source.len() > 256 * 1024 {
        return None;
    }
    let needle = quote
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    let first = needle.chars().next()?;
    let positions = source
        .char_indices()
        .filter(|(_, c)| !c.is_whitespace())
        .collect::<Vec<_>>();
    let compact = positions.iter().map(|(_, c)| c).collect::<String>();
    let offset = compact.find(&needle)?;
    // Include overlapping matches when checking ambiguity.
    if compact[offset + first.len_utf8()..].contains(&needle) {
        return None;
    }
    let start = compact[..offset].chars().count();
    let last = start + needle.chars().count() - 1;
    let (end, c) = positions.get(last)?;
    source.get(positions.get(start)?.0..end + c.len_utf8())
}

pub(super) fn hint(unit: &RagTextUnitWrite, source: &str, value: &Value) -> String {
    let original = value
        .as_str()
        .and_then(|quote| unique_original(source, quote))
        .filter(|original| unit.text.contains(*original));
    original
        .map(|original| {
            format!(
                "；仅空白不同且唯一匹配的原文候选（JSON字符串）：{}",
                Value::from(original)
            )
        })
        .unwrap_or_default()
}
