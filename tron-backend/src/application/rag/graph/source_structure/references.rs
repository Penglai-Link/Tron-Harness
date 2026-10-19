//! Original figure/table labels provide explicit document-local reference edges.
//! Generated descriptions and neighboring positions never declare an association.
use super::*;
use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

static LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"(?i)(?:^|[^A-Za-z0-9_/])(?P<label>(?P<kind>figure|fig\.?|table|图|表)[ \t\u{3000}]*(?P<number>[0-9０-９]+(?:[.\-][0-9０-９]+)*[A-Za-z]?|[一二三四五六七八九十]+))"
).expect("static original-reference expression")
});

#[derive(Clone)]
struct Label {
    kind: &'static str,
    number: String,
    quote: String,
    start: usize,
}
fn labels(text: &str) -> Vec<Label> {
    LABEL
        .captures_iter(text)
        .map(|capture| {
            let raw = &capture["kind"];
            let kind = if raw == "表" || raw.eq_ignore_ascii_case("table") {
                "table"
            } else {
                "figure"
            };
            let number = capture["number"]
                .nfkc()
                .collect::<String>()
                .to_ascii_lowercase();
            let quote = capture.name("label").expect("label capture");
            Label {
                kind,
                number,
                quote: quote.as_str().into(),
                start: quote.start(),
            }
        })
        .collect()
}
fn section(location: &Value) -> Option<String> {
    location["sourceSections"].as_array()?.last()?["id"]
        .as_str()
        .map(str::to_owned)
}
#[derive(Clone)]
struct Declaration {
    node: String,
    block: String,
    section: Option<String>,
    quote: String,
    chunk: String,
    location: Value,
}

pub(super) fn link(
    builder: &mut Builder<'_>,
    units: &[RagTextUnitWrite],
    blocks: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut index = BTreeMap::<(&'static str, String), BTreeMap<String, Declaration>>::new();
    for unit in units.iter().filter(|unit| unit.unit_type != "IMAGE") {
        for location in facts(unit) {
            let original = block_id(&location, unit);
            let Some(node) = blocks.get(&original) else {
                continue;
            };
            let mut captions = location["imageAnnotations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|a| {
                    matches!(
                        a["kind"].as_str(),
                        Some("source_caption" | "source_footnote")
                    )
                })
                .filter_map(|a| a["text"].as_str())
                .collect::<Vec<_>>();
            if location["blockKind"] == "table" {
                captions.push(&unit.text);
            }
            for caption in captions {
                for label in labels(caption) {
                    // A declaration begins a caption line. A caption's later
                    // "see Figure 2" text is a reference, not a new label.
                    let line_start = caption[..label.start].rfind('\n').map_or(0, |i| i + 1);
                    let prefix = caption[line_start..label.start].trim();
                    if !prefix.is_empty() && !prefix.chars().all(|c| matches!(c, '*' | '#' | '_')) {
                        continue;
                    }
                    if !unit.text.contains(&label.quote) {
                        continue;
                    }
                    index
                        .entry((label.kind, label.number))
                        .or_default()
                        .entry(node.clone())
                        .or_insert(Declaration {
                            node: node.clone(),
                            block: original.clone(),
                            section: section(&location),
                            quote: label.quote,
                            chunk: unit.text_unit_code.clone(),
                            location: canonical(&location),
                        });
                }
            }
        }
    }
    for ((kind, number), declarations) in &index {
        for declaration in declarations.values() {
            let node = builder
                .nodes
                .get_mut(&declaration.node)
                .ok_or("declared visual block is absent")?;
            if !node.properties["referenceDeclarations"].is_array() {
                node.properties["referenceDeclarations"] = json!([]);
            }
            node.properties["referenceDeclarations"].as_array_mut().unwrap().push(json!({
                "kind":kind,"number":number,"quote":declaration.quote,"chunkCode":declaration.chunk,"location":declaration.location,
            }));
        }
    }
    for unit in units.iter().filter(|unit| unit.unit_type != "IMAGE") {
        let Some(parts) = crate::application::rag::source_parts::parts(
            &unit.metadata,
            &unit.text,
            &unit.text_unit_code,
        ) else {
            continue;
        };
        for part in parts {
            let Some(from) = blocks.get(&part.id) else {
                continue;
            };
            let source = super::super::generation::source_text(&part.text, &part.location);
            for label in labels(&source) {
                let Some(candidates) = index.get(&(label.kind, label.number)) else {
                    continue;
                };
                let local = candidates
                    .values()
                    .filter(|d| d.section.is_some() && d.section == section(&part.location))
                    .collect::<Vec<_>>();
                let candidate = if local.len() == 1 {
                    Some(local[0])
                } else if local.is_empty() && candidates.len() == 1 {
                    candidates.values().next()
                } else {
                    None
                };
                let Some(candidate) = candidate.filter(|d| d.block != part.id) else {
                    continue;
                };
                let relationship = if label.kind == "table" {
                    "REFERENCES_TABLE"
                } else {
                    "REFERENCES_FIGURE"
                };
                let mut proof = support(builder.generation, unit, &label.quote);
                proof.location = canonical(&part.location);
                let relation = edge(from, &candidate.node, relationship, true, proof);
                builder.edges.entry(relation.id.clone()).or_insert(relation);
            }
        }
    }
    Ok(())
}
