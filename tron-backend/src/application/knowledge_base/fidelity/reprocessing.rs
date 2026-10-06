//! Comparing independent extractions detects disagreement, never decides which is true.
use super::{FidelityIssue, ParsedDocument, evidence};
use std::collections::BTreeMap;

type Counts = BTreeMap<String, usize>;
#[derive(Default)]
struct Fingerprint {
    numbers: Counts,
    words: Counts,
    symbols: Counts,
    rows: usize,
    cells: usize,
}
fn add(counts: &mut Counts, token: &mut String) {
    if !token.is_empty() {
        *counts.entry(std::mem::take(token)).or_default() += 1;
    }
}
fn fingerprint(text: &str) -> Fingerprint {
    let mut result = Fingerprint::default();
    let mut number = String::new();
    let mut word = String::new();
    let mut plain = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let candidate: String = chars.clone().take(512).take_while(|c| *c != '>').collect();
            let end_present = chars.clone().nth(candidate.chars().count()) == Some('>');
            let trimmed = candidate.trim_start();
            let closing = trimmed.starts_with('/');
            let name = trimmed
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if end_present
                && matches!(
                    name.as_str(),
                    "table"
                        | "tr"
                        | "td"
                        | "th"
                        | "thead"
                        | "tbody"
                        | "tfoot"
                        | "p"
                        | "span"
                        | "br"
                        | "div"
                        | "sup"
                        | "sub"
                        | "b"
                        | "i"
                        | "strong"
                        | "em"
                        | "a"
                )
            {
                if !closing && name == "tr" {
                    result.rows += 1;
                }
                if !closing && matches!(name.as_str(), "td" | "th") {
                    result.cells += 1;
                }
                for _ in 0..=candidate.chars().count() {
                    chars.next();
                }
                plain.push(' ');
                continue;
            }
        }
        plain.push(c);
    }
    for c in plain.chars() {
        if c.is_ascii_digit() {
            number.push(c);
        } else if matches!(c, '.' | ',') && !number.is_empty() {
            number.push(c);
        } else {
            add(&mut result.numbers, &mut number);
        }
        if c.is_alphabetic() || matches!(c, '\u{0300}'..='\u{036f}') {
            word.push(c);
        } else {
            add(&mut result.words, &mut word);
        }
        if matches!(
            c,
            '<' | '>' | '=' | '+' | '-' | '±' | '≥' | '≤' | '−' | '×' | '÷' | '%' | '\u{2070}'
                ..='\u{209f}' | '²' | '³' | '¹'
        ) {
            *result.symbols.entry(c.to_string()).or_default() += 1;
        }
    }
    add(&mut result.numbers, &mut number);
    add(&mut result.words, &mut word);
    result
}
fn pages(document: &ParsedDocument) -> BTreeMap<Option<u32>, String> {
    let mut pages = BTreeMap::<Option<u32>, String>::new();
    for section in &document.sections {
        // Resource IDs and generated image captions are not independent original text.
        if section.location.block_kind.as_deref() == Some("image") {
            continue;
        }
        let page = pages.entry(section.location.page).or_default();
        page.push_str(&section.text);
        page.push('\n');
    }
    pages
}
fn delta(before: &Counts, after: &Counts) -> String {
    let mut keys: Vec<_> = before.keys().chain(after.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|k| before.get(*k) != after.get(*k))
        .take(12)
        .map(|k| {
            format!(
                "{k:?}: original={} reprocessed={}",
                before.get(k).copied().unwrap_or(0),
                after.get(k).copied().unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
pub(super) fn compare(
    original: Option<&ParsedDocument>,
    candidate: &ParsedDocument,
) -> Vec<FidelityIssue> {
    let Some(original) = original else {
        return vec![FidelityIssue { source_region:None, candidate_region:None, candidate_excerpt:None, kind: "REPROCESSING_BASELINE_UNAVAILABLE".into(), page: None, blocks: vec![],
            source_excerpt: "The original parse is unavailable for a source-bound comparison; OCR fidelity is unresolved.".into(), image_resource_codes: vec![] }];
    };
    let before = pages(original);
    let after = pages(candidate);
    let mut page_keys: Vec<_> = before.keys().chain(after.keys()).copied().collect();
    page_keys.sort();
    page_keys.dedup();
    let mut issues = Vec::new();
    for page in page_keys {
        let a = fingerprint(before.get(&page).map(String::as_str).unwrap_or(""));
        let b = fingerprint(after.get(&page).map(String::as_str).unwrap_or(""));
        let mut differences = vec![];
        for (kind, left, right) in [
            ("NUMBER_TOKEN_CONFLICT", &a.numbers, &b.numbers),
            ("TEXT_TOKEN_CONFLICT", &a.words, &b.words),
            ("SYMBOL_TOKEN_CONFLICT", &a.symbols, &b.symbols),
        ] {
            if left != right {
                differences.push((kind, delta(left, right)));
            }
        }
        if (a.rows, a.cells) != (b.rows, b.cells) {
            differences.push((
                "TABLE_STRUCTURE_CONFLICT",
                format!(
                    "original rows/cells={}/{}; reprocessed rows/cells={}/{}",
                    a.rows, a.cells, b.rows, b.cells
                ),
            ));
        }
        for (kind, excerpt) in differences {
            let (blocks, images) = evidence(candidate, page);
            issues.push(FidelityIssue {
                source_region: None,
                candidate_region: None,
                candidate_excerpt: None,
                kind: kind.into(),
                page,
                blocks,
                source_excerpt: excerpt,
                image_resource_codes: images,
            });
        }
    }
    issues
}

/// Align only unique overlapping regions on the same original page and block kind.
/// Split/merged/moved regions are unresolved, not silently matched by text similarity.
pub(super) fn compare_regions(
    original: &ParsedDocument,
    candidate: &ParsedDocument,
) -> Vec<FidelityIssue> {
    fn overlap(a: [f64; 4], b: [f64; 4]) -> f64 {
        let intersection =
            (a[2].min(b[2]) - a[0].max(b[0])).max(0.0) * (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
        let union = (a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - intersection;
        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
    let mut issues = vec![];
    for before in original
        .sections
        .iter()
        .filter(|s| s.location.bbox.is_some())
        .take(2000)
    {
        let region = before.location.bbox.unwrap();
        let matches: Vec<_> = candidate
            .sections
            .iter()
            .filter(|s| {
                s.location.page == before.location.page
                    && s.location.block_kind == before.location.block_kind
                    && s.location.bbox.is_some_and(|b| overlap(region, b) >= 0.6)
            })
            .take(2)
            .collect();
        let after = (matches.len() == 1).then(|| matches[0]);
        let canonical = |text: &str| {
            text.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        };
        let changed = after.is_none_or(|after| canonical(&before.text) != canonical(&after.text));
        if changed {
            issues.push(FidelityIssue {
                source_region: Some(region),
                candidate_region: after.and_then(|s| s.location.bbox),
                candidate_excerpt: after.map(|s| s.text.chars().take(400).collect()),
                kind: if after.is_some() {
                    "REGION_TEXT_CONFLICT"
                } else {
                    "REGION_ALIGNMENT_UNRESOLVED"
                }
                .into(),
                page: before.location.page,
                blocks: before.location.blocks.clone(),
                source_excerpt: before.text.chars().take(400).collect(),
                image_resource_codes: before.location.image_resource_codes.clone(),
            });
        }
        if issues.len() >= 64 {
            break;
        }
    }
    issues
}

/// A restricted off-ramp for parser-only glyph errors. An unresolved font map is
/// never inferred from agreement. Scan/form/table/image documents remain reviewable.
pub(super) fn native_agreement(
    original: &[u8],
    candidate: &ParsedDocument,
) -> Result<Option<(String, String)>, super::KnowledgeBaseError> {
    use pdf_extract::{Encoding, Object};
    use sha2::{Digest, Sha256};
    let pdf = pdf_extract::Document::load_mem(original).map_err(super::invalid_storage)?;
    if candidate
        .sections
        .iter()
        .any(|s| matches!(s.location.block_kind.as_deref(), Some("image" | "table")))
    {
        return Ok(None);
    }
    let page_ids = pdf.get_pages();
    for id in page_ids.values() {
        let content = pdf_extract::content::Content::decode(
            &pdf.get_page_content(*id).map_err(super::invalid_storage)?,
        )
        .map_err(super::invalid_storage)?;
        if content.operations.iter().any(|op| op.operator == "Do") {
            return Ok(None);
        }
        let used = super::page_glyph_usage(&pdf, *id)?;
        if used.is_empty() {
            return Ok(None);
        }
        let fonts = pdf.get_page_fonts(*id).map_err(super::invalid_storage)?;
        for (name, codes) in used {
            let Some(font) = fonts.get(&name) else {
                return Ok(None);
            };
            if !font
                .get(b"Subtype")
                .and_then(Object::as_name)
                .is_ok_and(|n| matches!(n, b"Type1" | b"TrueType" | b"MMType1"))
                || font.get(b"ToUnicode").is_err()
            {
                return Ok(None);
            }
            let mut unicode_font = (*font).clone();
            unicode_font.remove(b"Encoding");
            let Ok(Encoding::UnicodeMapEncoding(map)) = unicode_font.get_font_encoding(&pdf) else {
                return Ok(None);
            };
            if codes.iter().any(|code| {
                !map.get(*code, 1)
                    .is_some_and(|v| !v.is_empty() && !v.iter().any(|n| *n < 32 || *n == 0xfffd))
            }) {
                return Ok(None);
            }
        }
    }
    let Ok(native) = crate::application::knowledge_base::local_documents::parse_local_document(
        "source.pdf",
        original,
    ) else {
        return Ok(None);
    };
    if !super::assess(&native, original)?.is_empty() {
        return Ok(None);
    }
    let before = pages(&native);
    let after = pages(candidate);
    if before.len() != page_ids.len() || before.keys().ne(after.keys()) {
        return Ok(None);
    }
    let canonical = |pages: &BTreeMap<Option<u32>, String>| -> String {
        pages
            .iter()
            .map(|(page, text)| {
                format!(
                    "{page:?}:{}",
                    text.chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let native_text = canonical(&before);
    let candidate_text = canonical(&after);
    if native_text != candidate_text {
        return Ok(None);
    }
    Ok(Some((
        format!("{:x}", Sha256::digest(native_text.as_bytes())),
        format!("{:x}", Sha256::digest(candidate_text.as_bytes())),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::knowledge_base::local_documents::{DocumentLocation, LocatedSection};
    fn document(text: &str) -> ParsedDocument {
        ParsedDocument {
            format: "pdf".into(),
            sections: vec![LocatedSection {
                text: text.into(),
                heading_level: None,
                location: DocumentLocation {
                    page: Some(2),
                    ..Default::default()
                },
            }],
            warnings: vec![],
            image_analysis: None,
        }
    }
    #[test]
    fn different_values_units_comparators_and_table_shape_remain_conflicts() {
        let original =
            document("m 8 μg ≥4 ≤7 −2 x² H₂ Société <table><tr><td>3.5</td></tr></table>");
        let candidate = document(
            "μ 8 mg >4 <7 -2 x2 H2 Societe <table><tr><td>3.8</td><td>0</td></tr></table>",
        );
        let issues = compare(Some(&original), &candidate);
        for kind in [
            "NUMBER_TOKEN_CONFLICT",
            "TEXT_TOKEN_CONFLICT",
            "SYMBOL_TOKEN_CONFLICT",
            "TABLE_STRUCTURE_CONFLICT",
        ] {
            assert!(issues.iter().any(|i| i.kind == kind), "{kind}");
        }
        assert_eq!(
            original.sections[0].text,
            "m 8 μg ≥4 ≤7 −2 x² H₂ Société <table><tr><td>3.5</td></tr></table>"
        );
    }
    #[test]
    fn whitespace_and_html_attributes_are_not_semantic_corrections() {
        let original = document("m μg ≥4 ≤7 −2 x² H₂ <table><tr><td>3.5</td></tr></table>");
        let candidate = document(
            "m\n μg  ≥4 ≤7 −2 x² H₂ <table class=\"a\"><tr><td colspan=\"1\">3.5</td></tr></table>",
        );
        assert!(compare(Some(&original), &candidate).is_empty());
        assert!(!compare(None, &candidate).is_empty());
    }
    #[test]
    fn region_alignment_catches_same_counts_with_swapped_cells_and_ambiguous_layout() {
        let mut before = document("12 34");
        before.sections[0].location.bbox = Some([0.1, 0.2, 0.5, 0.4]);
        before.sections[0].location.block_kind = Some("table".into());
        let mut after = before.clone();
        after.sections[0].text = "34 12".into();
        assert!(
            compare(Some(&before), &after).is_empty(),
            "multisets alone cannot detect swaps"
        );
        assert_eq!(
            compare_regions(&before, &after)[0].kind,
            "REGION_TEXT_CONFLICT"
        );
        after.sections[0].location.bbox = Some([0.6, 0.2, 0.9, 0.4]);
        assert_eq!(
            compare_regions(&before, &after)[0].kind,
            "REGION_ALIGNMENT_UNRESOLVED"
        );
        assert!(compare_regions(&before, &before).is_empty());
    }
}
