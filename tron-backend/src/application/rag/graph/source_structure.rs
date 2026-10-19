//! Source structure is deterministic parser evidence, independent of business entity extraction.
use super::generation::{edge, support};
use crate::{application::rag::RagTextUnitWrite, domain::rag::graph::*};
use serde_json::{Value, json};
use std::collections::BTreeMap;

mod references;

struct Builder<'a> {
    generation: &'a RagGraphGeneration,
    nodes: BTreeMap<String, RagGraphNode>,
    edges: BTreeMap<String, RagGraphEdge>,
}
impl Builder<'_> {
    fn node(
        &mut self,
        id: String,
        kind: RagGraphNodeKind,
        name: String,
        properties: Value,
    ) -> Result<String, String> {
        let node = RagGraphNode {
            id: id.clone(),
            kind,
            generation_id: self.generation.id.clone(),
            name,
            entity_type: None,
            properties,
        };
        if self
            .nodes
            .get(&id)
            .is_some_and(|existing| existing != &node)
        {
            return Err("source structure has conflicting facts for one original block".into());
        }
        self.nodes.entry(id.clone()).or_insert(node);
        Ok(id)
    }
    fn link(
        &mut self,
        from: &str,
        to: &str,
        relationship: &str,
        unit: &RagTextUnitWrite,
        location: &Value,
    ) {
        let mut source = support(self.generation, unit, "");
        source.location = canonical(location);
        let link = edge(from, to, relationship, true, source);
        self.edges.entry(link.id.clone()).or_insert(link);
    }
    fn id(&self, kind: &str, parts: &[&str]) -> String {
        let mut identity = vec![self.generation.id.as_str()];
        identity.extend_from_slice(parts);
        stable_id(kind, &identity)
    }
}
fn canonical(value: &Value) -> Value {
    let mut location = if value.is_object() {
        value.clone()
    } else {
        json!({})
    };
    if let Some(start) = location.get("sourceBlockStart").cloned() {
        location["charStart"] = start;
    }
    if let Some(end) = location.get("sourceBlockEnd").cloned() {
        location["charEnd"] = end;
    }
    if let Some(object) = location.as_object_mut() {
        object.remove("sourceSections");
        object.remove("imageAnnotations");
    }
    location
}
fn facts(unit: &RagTextUnitWrite) -> Vec<Value> {
    match unit.metadata["sourceLocations"].as_array() {
        Some(locations) if !locations.is_empty() => locations
            .iter()
            .filter(|l| l.is_object())
            .cloned()
            .collect(),
        _ => vec![
            unit.metadata
                .get("location")
                .cloned()
                .filter(Value::is_object)
                .unwrap_or_else(|| json!({})),
        ],
    }
}
fn block_id(location: &Value, unit: &RagTextUnitWrite) -> String {
    location["sourceBlockId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("chunk/{}", unit.text_unit_code))
}
fn resources(location: &Value) -> Vec<&str> {
    location["imageResourceCodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|code| !code.is_empty())
        .collect()
}

// A container's coordinates locate the table/paragraph, not the embedded pixels.
fn embedded_location(location: &Value, original: &str) -> Value {
    let mut result = canonical(location);
    let object = result.as_object_mut().expect("canonical location object");
    for field in [
        "bbox",
        "charStart",
        "charEnd",
        "sourceBlockStart",
        "sourceBlockEnd",
        "rowStart",
        "rowEnd",
        "cellRanges",
        "sourceBlockId",
        "imageResourceCodes",
    ] {
        object.remove(field);
    }
    object.insert("parentSourceBlockId".into(), json!(original));
    object.insert("blockKind".into(), json!("image"));
    result
}

pub(super) fn build(
    g: &RagGraphGeneration,
    units: &[RagTextUnitWrite],
) -> Result<(Vec<RagGraphNode>, Vec<RagGraphEdge>), String> {
    let mut b = Builder {
        generation: g,
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
    };
    let metadata = units
        .first()
        .map(|u| &u.metadata)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let source=b.node(stable_id("SOURCE",&[&g.partition.key(),&g.id]),RagGraphNodeKind::SourceVersion,
        metadata["fileName"].as_str().unwrap_or(&g.source_code).into(),json!({"sourceCode":g.source_code,"sourceVersion":g.source_version,"taskId":g.task_id,"documentId":metadata["documentId"],"resourceCode":metadata["resourceCode"]}))?;
    let mut blocks = BTreeMap::<String, String>::new();
    let mut parents = Vec::new();
    for unit in units {
        let chunk=b.node(stable_id("CHUNK",&[&g.id,&unit.text_unit_code]),RagGraphNodeKind::Chunk,unit.text_unit_code.clone(),
            json!({"chunkCode":unit.text_unit_code,"unitType":unit.unit_type,"location":unit.metadata.get("location").cloned().unwrap_or_else(||json!({})),"sourceSpans":unit.metadata.get("sourceSpans").cloned().unwrap_or_else(||json!([]))}))?;
        b.link(
            &source,
            &chunk,
            "CONTAINS",
            unit,
            &unit.metadata["location"],
        );
        for location in facts(unit) {
            let original = block_id(&location, unit);
            let orphan = original.starts_with("unmapped-asset/");
            let mut parent = source.clone();
            if !orphan {
                if let Some(sheet) = location["sheet"]
                    .as_str()
                    .filter(|sheet| !sheet.trim().is_empty())
                {
                    let sheet_node = b.node(
                        b.id("SHEET", &[sheet]),
                        RagGraphNodeKind::Section,
                        sheet.into(),
                        json!({"sourceKind":"Sheet","location":{"sheet":sheet}}),
                    )?;
                    b.link(
                        &source,
                        &sheet_node,
                        "CONTAINS",
                        unit,
                        &json!({"sheet":sheet}),
                    );
                    parent = sheet_node;
                }
                for heading in location["sourceSections"].as_array().into_iter().flatten() {
                    let (Some(identity), Some(title)) =
                        (heading["id"].as_str(), heading["title"].as_str())
                    else {
                        return Err("original section identity or title is absent".into());
                    };
                    if identity.is_empty() || title.trim().is_empty() {
                        continue;
                    }
                    let section=b.node(b.id("SECTION",&[identity]),RagGraphNodeKind::Section,title.into(),
                        json!({"sourceBlockId":identity,"level":heading["level"],"location":canonical(&heading["location"])}))?;
                    b.link(&parent, &section, "CONTAINS", unit, &heading["location"]);
                    parent = section;
                }
            }
            if parent != source {
                b.link(&parent, &chunk, "CONTAINS", unit, &location);
            }
            let kind = location["blockKind"].as_str().unwrap_or_default();
            let codes = resources(&location);
            let single_image = matches!(kind, "image" | "image_vector") && codes.len() <= 1;
            let block = if kind == "table" {
                let table = b.node(
                    b.id("TABLE", &[&original]),
                    RagGraphNodeKind::Table,
                    "Table".into(),
                    json!({"sourceBlockId":original,"location":canonical(&location)}),
                )?;
                b.link(&parent, &table, "CONTAINS", unit, &location);
                b.link(&chunk, &table, "COVERS", unit, &location);
                Some(table)
            } else if single_image {
                let image=b.node(b.id("IMAGE",&[&original]),RagGraphNodeKind::Image,"Image".into(),json!({"sourceBlockId":original,"location":canonical(&location),"association":if orphan{"Unmapped"}else{"ParsedBlock"}}))?;
                b.link(&parent, &image, "CONTAINS", unit, &location);
                b.link(&chunk, &image, "COVERS", unit, &location);
                Some(image)
            } else {
                let original_block=b.node(b.id("BLOCK",&[&original]),RagGraphNodeKind::SourceBlock,"Original block".into(),
                    json!({"sourceBlockId":original,"blockKind":kind,"location":canonical(&location)}))?;
                b.link(&parent, &original_block, "CONTAINS", unit, &location);
                b.link(&chunk, &original_block, "COVERS", unit, &location);
                Some(original_block)
            };
            if let Some(block) = &block {
                if let Some(previous) = blocks.insert(original.clone(), block.clone()) {
                    if previous != *block {
                        return Err("one original block has conflicting structural kinds".into());
                    }
                }
                if let Some(parent_id) = location["parentSourceBlockId"].as_str() {
                    parents.push((parent_id.to_owned(), block.clone(), unit, location.clone()));
                }
                let mut image_nodes = BTreeMap::new();
                for code in codes {
                    let image = if !single_image {
                        let image=b.node(b.id("IMAGE",&[&original,code]),RagGraphNodeKind::Image,"Embedded image".into(),json!({"containerSourceBlockId":original,"resourceCode":code,"containerLocation":canonical(&location),"location":embedded_location(&location,&original),"association":"ParsedBlock"}))?;
                        b.link(block, &image, "HAS_IMAGE", unit, &location);
                        image
                    } else {
                        block.clone()
                    };
                    let asset = b.node(
                        b.id("ASSET", &[code]),
                        RagGraphNodeKind::Asset,
                        code.into(),
                        json!({"resourceCode":code}),
                    )?;
                    b.link(&image, &asset, "HAS_ASSET", unit, &location);
                    image_nodes.insert(code, image);
                }
                for annotation in location["imageAnnotations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    let Some(text) = annotation["text"]
                        .as_str()
                        .filter(|text| !text.trim().is_empty())
                    else {
                        continue;
                    };
                    let (node_kind, relationship) = match annotation["kind"].as_str() {
                        Some("generated_description") => {
                            (RagGraphNodeKind::GeneratedDescription, "HAS_DESCRIPTION")
                        }
                        Some("source_caption" | "source_footnote") => {
                            (RagGraphNodeKind::SourceCaption, "HAS_CAPTION")
                        }
                        Some("parser_body_text") => (RagGraphNodeKind::OcrText, "HAS_OCR"),
                        _ => continue,
                    };
                    let encoded = serde_json::to_string(annotation).map_err(|e| e.to_string())?;
                    let target = if let Some(code) = annotation["resourceCode"].as_str() {
                        image_nodes.get(code).ok_or(
                            "image annotation references an asset outside its original block",
                        )?
                    } else {
                        block
                    };
                    let description=b.node(b.id("ANNOTATION",&[&original,&encoded]),node_kind,text.chars().take(120).collect(),
                        json!({"text":text,"annotationKind":annotation["kind"],"source":annotation["source"],"modelCode":annotation["modelCode"],"artifactResourceCode":annotation["artifactResourceCode"],"resourceCode":annotation["resourceCode"],"location":canonical(&location)}))?;
                    b.link(target, &description, relationship, unit, &location);
                }
            }
        }
    }
    for (identity, child, unit, location) in parents {
        if let Some(parent) = blocks.get(&identity) {
            if parent != &child {
                b.link(parent, &child, "CONTAINS", unit, &location);
            }
        }
    }
    references::link(&mut b, units, &blocks)?;
    Ok((
        b.nodes.into_values().collect(),
        b.edges.into_values().collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::rag::graph::build_graph_generation;
    fn unit(id: &str, text: &str, locations: Vec<Value>) -> RagTextUnitWrite {
        RagTextUnitWrite {
            text_unit_code: id.into(),
            unit_type: "LEAF".into(),
            text: text.into(),
            search_text: text.into(),
            heading_path: "[]".into(),
            ordinal: 0,
            token_count: 1,
            content_hash: id.into(),
            status: "ACTIVE".into(),
            fulltext_status: "INDEXED".into(),
            embedding_model_code: None,
            embedding_version: None,
            embedding_text_hash: None,
            embedding_dimension: None,
            embedding: None,
            embedding_status: "NOT_ENABLED".into(),
            metadata: json!({"fileName":"manual.pdf","documentId":"doc","resourceCode":"original","location":locations.first().cloned().unwrap_or_else(||json!({})),"sourceLocations":locations}),
        }
    }
    fn generation() -> RagGraphGeneration {
        RagGraphGeneration {
            partition: GraphPartition {
                corpus_id: "corpus".into(),
                scope_type: "USER".into(),
                scope_code: "owner".into(),
            },
            id: "generation".into(),
            source_code: "source".into(),
            source_version: "v1".into(),
            task_id: "task".into(),
            nodes: vec![],
            edges: vec![],
        }
    }
    fn sections() -> Value {
        json!([{"id":"heading-1","title":"Operations","level":1,"location":{"page":1,"sourceBlockId":"heading-1"}},{"id":"heading-2","title":"Ratings","level":2,"location":{"page":2,"sourceBlockId":"heading-2"}}])
    }
    fn build_units(units: &[RagTextUnitWrite]) -> RagGraphGeneration {
        let extractions = Value::Object(
            units
                .iter()
                .map(|unit| (unit.text_unit_code.clone(), json!({"nodes":[],"edges":[]})))
                .collect(),
        );
        build_graph_generation(generation(), units, &extractions)
            .unwrap()
            .0
    }
    #[test]
    fn original_table_images_ocr_captions_and_generated_descriptions_have_distinct_real_links() {
        let mut location = json!({"sourceBlockId":"table-1","blockKind":"table","sourceSections":sections(),"sourceBlockStart":100,"sourceBlockEnd":200,"charStart":100,"charEnd":200,"page":4,"imageResourceCodes":["picture"],"imageAnnotations":[
            {"kind":"source_caption","source":"caption","text":"Table 1: Ratings","block":"table-1"},
            {"kind":"parser_body_text","source":"ocr","text":"A B C","block":"table-1"},
            {"kind":"generated_description","source":"vision","text":"Illustration of the ratings","block":"table-1","modelCode":"vision"}]});
        let first = unit("first", "| Name | Rating |", vec![location.clone()]);
        location["charStart"] = json!(150);
        let second = unit("second", "| Example | A |", vec![location.clone()]);
        let mut image = unit("image", "Illustration of the ratings", vec![location]);
        image.unit_type = "IMAGE".into();
        let facts = build_units(&[first, second, image]);
        let count = |kind| facts.nodes.iter().filter(|node| node.kind == kind).count();
        assert_eq!(count(RagGraphNodeKind::Section), 2);
        assert_eq!(count(RagGraphNodeKind::Table), 1);
        assert_eq!(count(RagGraphNodeKind::Image), 1);
        assert_eq!(count(RagGraphNodeKind::Asset), 1);
        assert_eq!(count(RagGraphNodeKind::SourceCaption), 1);
        assert_eq!(count(RagGraphNodeKind::OcrText), 1);
        assert_eq!(count(RagGraphNodeKind::GeneratedDescription), 1);
        assert_eq!(count(RagGraphNodeKind::Entity), 0);
        for relationship in [
            "COVERS",
            "HAS_IMAGE",
            "HAS_ASSET",
            "HAS_CAPTION",
            "HAS_OCR",
            "HAS_DESCRIPTION",
        ] {
            assert!(
                facts
                    .edges
                    .iter()
                    .any(|edge| edge.relationship == relationship),
                "{relationship}"
            );
        }
        let table = facts
            .nodes
            .iter()
            .find(|node| node.kind == RagGraphNodeKind::Table)
            .unwrap();
        assert_eq!(table.properties["location"]["charStart"], 100);
        assert_eq!(table.properties["location"]["charEnd"], 200);
        assert!(facts.edges.iter().all(|edge| edge.structural));
    }
    #[test]
    fn unmapped_image_has_source_ownership_but_never_a_neighbouring_section_parent() {
        let orphan = unit(
            "image",
            "description",
            vec![
                json!({"sourceBlockId":"unmapped-asset/picture","blockKind":"image","imageResourceCodes":["picture"],"sourceSections":sections()}),
            ],
        );
        let facts = build_units(&[orphan]);
        assert!(
            !facts
                .nodes
                .iter()
                .any(|node| node.kind == RagGraphNodeKind::Section)
        );
        let image = facts
            .nodes
            .iter()
            .find(|node| node.kind == RagGraphNodeKind::Image)
            .unwrap();
        assert_eq!(image.properties["association"], "Unmapped");
        assert!(image.properties["location"].get("page").is_none());
        for parent in facts
            .edges
            .iter()
            .filter(|edge| edge.to == image.id && edge.relationship == "CONTAINS")
        {
            assert_eq!(
                facts
                    .nodes
                    .iter()
                    .find(|node| node.id == parent.from)
                    .unwrap()
                    .kind,
                RagGraphNodeKind::SourceVersion
            );
        }
    }
    #[test]
    fn nested_image_uses_only_an_explicit_original_parent_and_shared_assets_keep_multiple_supports()
    {
        let units = [
            unit(
                "table",
                "table",
                vec![json!({"sourceBlockId":"table","blockKind":"table"})],
            ),
            unit(
                "figure-a",
                "caption",
                vec![
                    json!({"sourceBlockId":"figure-a","parentSourceBlockId":"table","blockKind":"image","imageResourceCodes":["shared"]}),
                ],
            ),
            unit(
                "figure-b",
                "caption",
                vec![
                    json!({"sourceBlockId":"figure-b","blockKind":"image","imageResourceCodes":["shared"]}),
                ],
            ),
        ];
        let facts = build_units(&units);
        let table = facts
            .nodes
            .iter()
            .find(|n| n.kind == RagGraphNodeKind::Table)
            .unwrap();
        let a = facts
            .nodes
            .iter()
            .find(|n| {
                n.kind == RagGraphNodeKind::Image && n.properties["sourceBlockId"] == "figure-a"
            })
            .unwrap();
        assert!(
            facts
                .edges
                .iter()
                .any(|e| e.from == table.id && e.to == a.id && e.relationship == "CONTAINS")
        );
        assert_eq!(
            facts
                .nodes
                .iter()
                .filter(|n| n.kind == RagGraphNodeKind::Asset)
                .count(),
            1
        );
        assert_eq!(
            facts
                .edges
                .iter()
                .filter(|e| e.relationship == "HAS_ASSET")
                .count(),
            2
        );
    }
    #[test]
    fn paragraph_pixels_keep_one_original_block_and_asset_annotations_bind_to_their_image() {
        let location = json!({"sourceBlockId":"paragraph","blockKind":"paragraph","page":2,"bbox":[0.1,0.2,0.8,0.9],"sourceBlockStart":10,"sourceBlockEnd":80,"imageResourceCodes":["a","b"],"imageAnnotations":[{"kind":"generated_description","text":"Only image A","resourceCode":"a"}]});
        let body = unit(
            "body",
            "Paragraph with two illustrations",
            vec![location.clone()],
        );
        let mut pixels = unit("pixels-a", "Only image A", vec![location]);
        pixels.unit_type = "IMAGE".into();
        let facts = build_units(&[body, pixels]);
        assert_eq!(
            facts
                .nodes
                .iter()
                .filter(|n| n.kind == RagGraphNodeKind::SourceBlock)
                .count(),
            1
        );
        assert_eq!(
            facts
                .nodes
                .iter()
                .filter(|n| n.kind == RagGraphNodeKind::Image)
                .count(),
            2
        );
        for image in facts
            .nodes
            .iter()
            .filter(|n| n.kind == RagGraphNodeKind::Image)
        {
            assert_eq!(image.properties["location"]["page"], 2);
            assert!(image.properties["location"].get("bbox").is_none());
            assert!(image.properties["location"].get("charStart").is_none());
            assert_eq!(
                image.properties["containerLocation"]["bbox"],
                json!([0.1, 0.2, 0.8, 0.9])
            );
        }
        for edge in facts
            .edges
            .iter()
            .filter(|e| e.relationship == "HAS_DESCRIPTION")
        {
            let image = facts.nodes.iter().find(|n| n.id == edge.from).unwrap();
            assert_eq!(image.kind, RagGraphNodeKind::Image);
            assert_eq!(image.properties["resourceCode"], "a");
        }
    }
    fn visual_location(id: &str, section: &str, caption: &str, kind: &str) -> Value {
        json!({"sourceBlockId":id,"blockKind":"image","page":4,"sourceSections":[{"id":section,"title":section,"level":1,"location":{"page":4}}],"imageResourceCodes":[format!("asset-{id}")],"imageAnnotations":[{"kind":kind,"text":caption}]})
    }
    #[test]
    fn explicit_figure_and_table_references_have_literal_source_supports() {
        let caption = visual_location("figure-1", "chapter", "图 1 评级", "source_caption");
        let mut table = visual_location("table-2", "chapter", "Table 2 Risks", "source_caption");
        table["blockKind"] = json!("table");
        let body = unit(
            "body",
            "参见图 1及 Table 2。",
            vec![
                json!({"sourceBlockId":"body","blockKind":"paragraph","sourceSections":[{"id":"chapter","title":"chapter","level":1,"location":{"page":4}}]}),
            ],
        );
        let facts = build_units(&[
            body,
            unit("caption", "图 1 评级", vec![caption.clone()]),
            unit("table", "Table 2 Risks", vec![table]),
        ]);
        let references = facts
            .edges
            .iter()
            .filter(|e| e.relationship.starts_with("REFERENCES_"))
            .collect::<Vec<_>>();
        assert_eq!(references.len(), 2);
        assert!(
            references
                .iter()
                .all(|e| e.structural && e.support.chunk_code == "body")
        );
        assert!(
            references
                .iter()
                .any(|e| e.relationship == "REFERENCES_FIGURE" && e.support.quote == "图 1")
        );
        assert!(
            references
                .iter()
                .any(|e| e.relationship == "REFERENCES_TABLE" && e.support.quote == "Table 2")
        );
        let visual = facts
            .nodes
            .iter()
            .find(|n| {
                n.kind == RagGraphNodeKind::Image && n.properties["sourceBlockId"] == "figure-1"
            })
            .unwrap();
        assert_eq!(
            visual.properties["referenceDeclarations"][0]["quote"],
            "图 1"
        );
        assert_eq!(
            visual.properties["referenceDeclarations"][0]["chunkCode"],
            "caption"
        );
    }
    #[test]
    fn repeated_figure_numbers_require_an_unambiguous_original_section() {
        let a = visual_location("figure-a", "chapter-a", "Figure 1 Alpha", "source_caption");
        let b = visual_location("figure-b", "chapter-b", "Figure 1 Beta", "source_caption");
        let body = unit(
            "body-a",
            "See Figure 1.",
            vec![
                json!({"sourceBlockId":"body-a","sourceSections":[{"id":"chapter-a","title":"chapter-a","level":1,"location":{"page":4}}]}),
            ],
        );
        let unknown = unit(
            "unknown",
            "See Figure 1.",
            vec![json!({"sourceBlockId":"unknown","page":4})],
        );
        let nearby = unit(
            "notice",
            "Unrelated notice",
            vec![
                json!({"sourceBlockId":"notice","page":4,"sourceSections":[{"id":"chapter-a","title":"chapter-a","level":1,"location":{"page":4}}]}),
            ],
        );
        let facts = build_units(&[
            body,
            unknown,
            nearby,
            unit("caption-a", "Figure 1 Alpha", vec![a]),
            unit("caption-b", "Figure 1 Beta", vec![b]),
        ]);
        let references = facts
            .edges
            .iter()
            .filter(|e| e.relationship.starts_with("REFERENCES_"))
            .collect::<Vec<_>>();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].support.chunk_code, "body-a");
        assert_eq!(
            facts
                .nodes
                .iter()
                .find(|n| n.id == references[0].to)
                .unwrap()
                .properties["sourceBlockId"],
            "figure-a"
        );
    }
    #[test]
    fn generated_descriptions_and_other_subfigures_never_declare_original_labels() {
        let generated = visual_location(
            "generated",
            "chapter",
            "Figure 9 imagined caption",
            "generated_description",
        );
        let original = visual_location(
            "original",
            "chapter",
            "Figure 1A actual panel",
            "source_caption",
        );
        let facts = build_units(&[
            unit(
                "body",
                "See Figure 9 and Figure 1B.",
                vec![json!({"sourceBlockId":"body"})],
            ),
            unit(
                "generated-caption",
                "Figure 9 imagined caption",
                vec![generated],
            ),
            unit("original-caption", "Figure 1A actual panel", vec![original]),
        ]);
        assert!(
            !facts
                .edges
                .iter()
                .any(|e| e.relationship.starts_with("REFERENCES_"))
        );
    }
}
