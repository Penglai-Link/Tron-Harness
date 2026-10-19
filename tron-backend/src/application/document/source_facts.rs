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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_heading_titles_keep_occurrence_ids_and_unmapped_images_inherit_no_section() {
        let document = ParsedDocument {
            format: "markdown".into(),
            warnings: vec![],
            image_analysis: None,
            sections: vec![
                section(
                    "# Summary".into(),
                    Some(1),
                    DocumentLocation {
                        char_end: 9,
                        ..Default::default()
                    },
                ),
                section(
                    "First body".into(),
                    None,
                    DocumentLocation {
                        char_start: 10,
                        char_end: 20,
                        ..Default::default()
                    },
                ),
                section(
                    "# Summary".into(),
                    Some(1),
                    DocumentLocation {
                        char_start: 21,
                        char_end: 30,
                        ..Default::default()
                    },
                ),
                section(
                    "Second body".into(),
                    None,
                    DocumentLocation {
                        char_start: 31,
                        char_end: 42,
                        ..Default::default()
                    },
                ),
                section(
                    String::new(),
                    None,
                    DocumentLocation {
                        source_block_id: Some("unmapped-asset/picture".into()),
                        block_kind: Some("image".into()),
                        ..Default::default()
                    },
                ),
            ],
        };
        let facts = source_fact_locations(&document);
        assert_eq!(facts[1]["sourceSections"][0]["title"], "Summary");
        assert_eq!(facts[3]["sourceSections"][0]["title"], "Summary");
        assert_ne!(
            facts[1]["sourceSections"][0]["id"],
            facts[3]["sourceSections"][0]["id"]
        );
        assert!(facts[4]["sourceSections"].as_array().unwrap().is_empty());
        let chunks = chunk_document_input(&document, "source", "Example", 1);
        assert!(chunks.blocks[4].heading_path.is_empty());
    }
    #[test]
    fn chunking_keeps_original_block_bounds_and_full_hierarchy_independent_of_group_depth() {
        let document = ParsedDocument {
            format: "markdown".into(),
            warnings: vec![],
            image_analysis: None,
            sections: vec![
                section(
                    "# Parent".into(),
                    Some(1),
                    DocumentLocation {
                        source_block_id: Some("heading-a".into()),
                        char_end: 8,
                        ..Default::default()
                    },
                ),
                section(
                    "## Child".into(),
                    Some(2),
                    DocumentLocation {
                        source_block_id: Some("heading-b".into()),
                        char_start: 9,
                        char_end: 17,
                        ..Default::default()
                    },
                ),
                section(
                    "abcdefghij".repeat(40),
                    None,
                    DocumentLocation {
                        source_block_id: Some("paragraph".into()),
                        char_start: 18,
                        char_end: 418,
                        ..Default::default()
                    },
                ),
            ],
        };
        let config = ChunkConfig {
            max_chars: 200,
            overlap_chars: 16,
            heading_depth: 1,
            ..Default::default()
        };
        let chunks = split_document(&document, &config).unwrap();
        assert!(chunks.len() > 1);
        for location in chunks
            .iter()
            .flat_map(|chunk| &chunk.source_locations)
            .filter(|location| location["sourceBlockId"] == "paragraph")
        {
            assert_eq!(location["sourceSections"].as_array().unwrap().len(), 2);
            assert_eq!(location["sourceBlockStart"], 18);
            assert_eq!(location["sourceBlockEnd"], 418);
            assert!(location["charStart"].as_u64().unwrap() >= 18);
            assert!(location["charEnd"].as_u64().unwrap() <= 418);
        }
    }
}
