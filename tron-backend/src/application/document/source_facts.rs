//! Stable source structure independent of the selected chunking strategy.
//! Repeated heading titles keep distinct original block identities.
use super::*;
use serde_json::{Value, json};

pub fn source_fact_locations(document: &ParsedDocument) -> Vec<Value> {
    let mut headings = Vec::<(u8, Value)>::new();
    document
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let mut location =
                serde_json::to_value(&section.location).expect("document location is serializable");
            let id = section
                .location
                .source_block_id
                .clone()
                .unwrap_or_else(|| format!("normalized/block/{index}"));
            location["sourceBlockId"] = json!(id);
            location["sourceBlockStart"] = json!(section.location.char_start);
            location["sourceBlockEnd"] = json!(section.location.char_end);
            let orphan = id.starts_with("unmapped-asset/");
            let page_furniture = matches!(
                section.location.block_kind.as_deref(),
                Some("page_header" | "page_footer" | "header" | "footer")
            );
            if !orphan && !page_furniture {
                if let Some(level) = section.heading_level {
                    headings.retain(|(previous, _)| *previous < level);
                    let title = section.location.heading.clone().unwrap_or_else(|| {
                        section
                            .text
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .trim_matches('#')
                            .trim()
                            .into()
                    });
                    let heading =
                        json!({"id":id,"title":title,"level":level,"location":location.clone()});
                    headings.push((level, heading));
                    location["blockKind"] = json!("heading");
                }
                location["sourceSections"] = json!(
                    headings
                        .iter()
                        .map(|(_, heading)| heading)
                        .collect::<Vec<_>>()
                );
            } else {
                location["sourceSections"] = json!([]);
            }
            location
        })
        .collect()
}
