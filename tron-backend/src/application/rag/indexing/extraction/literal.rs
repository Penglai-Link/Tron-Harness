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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_hint_keeps_original_unicode_newlines_and_punctuation() {
        assert_eq!(
            unique_original("说明：甲\u{3000}\n乙。其他内容", "甲乙。"),
            Some("甲\u{3000}\n乙。")
        );
        assert_eq!(
            unique_original("Alpha\n calls Beta.", "Alpha calls Beta."),
            Some("Alpha\n calls Beta.")
        );
        assert_eq!(unique_original("甲乙。", "甲乙!"), None);
        assert_eq!(unique_original("Alpha", "alpha"), None);
    }

    #[test]
    fn repeated_overlapping_empty_and_oversized_quotes_have_no_hint() {
        assert_eq!(unique_original("甲 乙，然后甲乙", "甲乙"), None);
        assert_eq!(unique_original("aaaa", "aaa"), None);
        assert_eq!(unique_original("甲乙", " \n"), None);
        assert_eq!(unique_original("甲乙", &"甲".repeat(4096)), None);
    }
}
