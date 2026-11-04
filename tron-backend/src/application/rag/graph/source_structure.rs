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
