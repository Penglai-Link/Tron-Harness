//! Validated extraction -> scoped entity identities and per-source supports.
use crate::{application::rag::RagTextUnitWrite, domain::rag::graph::*};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Debug)]
pub struct GraphNodeTextProjection {
    pub entity_id: String,
    pub generation_id: String,
    pub chunk_codes: Vec<String>,
    pub text: String,
}
fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("graph extraction field is missing: {key}"))
}
fn normalize_name(name: &str) -> String {
    name.nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn entity_id(
    partition: &GraphPartition,
    entity_type: &str,
    name: &str,
    disambiguation: &str,
) -> String {
    stable_id(
        "ENTITY",
        &[
            &partition.key(),
            entity_type,
            &normalize_name(name),
            &normalize_name(disambiguation),
        ],
    )
}
pub(super) fn support(
    g: &RagGraphGeneration,
    unit: &RagTextUnitWrite,
    quote: &str,
) -> RagGraphSupport {
    RagGraphSupport {
        generation_id: g.id.clone(),
        source_code: g.source_code.clone(),
        source_version: g.source_version.clone(),
        task_id: g.task_id.clone(),
        chunk_code: unit.text_unit_code.clone(),
        quote: quote.into(),
        location: unit
            .metadata
            .get("location")
            .cloned()
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({})),
    }
}
pub(super) fn edge(
    from: &str,
    to: &str,
    relationship: &str,
    structural: bool,
    support: RagGraphSupport,
) -> RagGraphEdge {
    let logical = stable_id("REL", &[from, relationship, to]);
    let id = stable_id(
        "SUPPORT",
        &[
            &logical,
            &support.generation_id,
            &support.chunk_code,
            &support.quote,
        ],
    );
    RagGraphEdge {
        id,
        logical_relation_id: logical,
        from: from.into(),
        to: to.into(),
        relationship: relationship.into(),
        structural,
        support,
    }
}

/// Generated visual descriptions aid retrieval but are not original-source business facts.
pub(crate) fn graph_source_text(unit: &RagTextUnitWrite) -> String {
    let location = &unit.metadata["location"];
    if unit.unit_type == "IMAGE" || location["blockKind"] == "generated_description" {
        return String::new();
    }
    source_text(&unit.text, location)
}
pub(crate) fn source_text(text: &str, location: &Value) -> String {
    if location["blockKind"] == "generated_description" {
        return String::new();
    }
    let annotations = location["imageAnnotations"].as_array();
    if location["blockKind"] == "image" {
        return annotations
            .into_iter()
            .flatten()
            .filter(|a| {
                matches!(
                    a["kind"].as_str(),
                    Some("source_caption" | "source_footnote" | "parser_body_text")
                )
            })
            .filter_map(|a| a["text"].as_str())
            .filter(|quote| !quote.is_empty() && text.contains(quote))
            .collect::<Vec<_>>()
            .join("\n");
    }
    let end = annotations
        .into_iter()
        .flatten()
        .filter(|a| a["kind"] == "generated_description")
        .filter_map(|a| a["text"].as_str())
        .filter(|s| !s.is_empty())
        .filter_map(|s| text.find(s))
        .min()
        .unwrap_or(text.len());
    let end = if location["imageResourceCodes"]
        .as_array()
        .is_some_and(|v| !v.is_empty())
    {
        text.find("\n[图片资源:")
            .map_or(end, |marker| marker.min(end))
    } else {
        end
    };
    text[..end].to_owned()
}

pub(crate) fn validate_graph_extraction(
    unit: &RagTextUnitWrite,
    value: &Value,
) -> Result<(), String> {
    let generation = RagGraphGeneration {
        partition: GraphPartition {
            corpus_id: "validation".into(),
            scope_type: "validation".into(),
            scope_code: "validation".into(),
        },
        id: "validation".into(),
        source_code: "validation".into(),
        source_version: "validation".into(),
        task_id: "validation".into(),
        nodes: vec![],
        edges: vec![],
    };
    let values = serde_json::Map::from_iter([(unit.text_unit_code.clone(), value.clone())]);
    build_graph_generation(
        generation,
        std::slice::from_ref(unit),
        &Value::Object(values),
    )
    .map(|_| ())
}

/// The graph JSON is transient validated model output, never the graph database.
/// Entity projections are source-specific so merged identities cannot leak other sources.
pub fn build_graph_generation(
    mut generation: RagGraphGeneration,
    units: &[RagTextUnitWrite],
    extractions: &Value,
) -> Result<(RagGraphGeneration, Vec<GraphNodeTextProjection>), String> {
    generation.nodes.clear();
    generation.edges.clear();
    let (source_nodes, source_edges) = super::source_structure::build(&generation, units)?;
    generation.nodes.extend(source_nodes);
    generation.edges.extend(source_edges);
    let mut entities = BTreeMap::<String, RagGraphNode>::new();
    let mut projections = BTreeMap::<String, (BTreeSet<String>, BTreeSet<String>)>::new();
    for unit in units {
        let chunk_id = stable_id("CHUNK", &[&generation.id, &unit.text_unit_code]);
        if unit.unit_type == "IMAGE" {
            continue;
        }
        let source_text = graph_source_text(unit);
        let extraction = extractions
            .get(&unit.text_unit_code)
            .ok_or("graph extraction chunk is missing")?;
        let nodes = extraction["nodes"]
            .as_array()
            .ok_or("graph entities are missing")?;
        let relations = extraction["edges"]
            .as_array()
            .ok_or("graph relationships are missing")?;
        let mut refs = BTreeMap::new();
        for node in nodes {
            let reference = required(node, "ref")?;
            let name = required(node, "name")?;
            let entity_type = required(node, "type")?;
            let quote = required(node, "evidence")?;
            let disambiguation = node["disambiguation"].as_str().unwrap_or("");
            if !source_text.contains(quote)
                || !unit.text.contains(quote)
                || !quote.contains(name)
                || (!disambiguation.is_empty() && !quote.contains(disambiguation))
            {
                return Err("entity name or disambiguation has no exact source evidence".into());
            }
            let id = entity_id(&generation.partition, entity_type, name, disambiguation);
            if refs.insert(reference, id.clone()).is_some() {
                return Err("duplicate entity reference".into());
            }
            let identity_name = normalize_name(name);
            entities.entry(id.clone()).or_insert_with(|| RagGraphNode {
                id: id.clone(),
                kind: RagGraphNodeKind::Entity,
                generation_id: String::new(),
                name: identity_name,
                entity_type: Some(entity_type.into()),
                properties: json!({}),
            });
            let mut aliases = Vec::new();
            if let Some(items) = node["aliases"].as_array() {
                for alias in items {
                    let alias_name = required(alias, "name")?;
                    let evidence = required(alias, "evidence")?;
                    if !source_text.contains(evidence)
                        || !unit.text.contains(evidence)
                        || !evidence.contains(alias_name)
                        || !evidence.contains(name)
                    {
                        return Err("alias declaration has no exact source evidence".into());
                    }
                    aliases.push(json!({"name":alias_name,"evidence":evidence}));
                }
            }
            let contribution_id = stable_id(
                "CONTRIBUTION",
                &[&id, &generation.id, &unit.text_unit_code, quote],
            );
            generation.nodes.push(RagGraphNode { id: contribution_id.clone(), kind: RagGraphNodeKind::EntityContribution,
                generation_id: generation.id.clone(), name: name.into(), entity_type: Some(entity_type.into()),
                properties: json!({"entityId":id,"description":quote,"aliases":aliases,"disambiguation":disambiguation,
                    "chunkCode":unit.text_unit_code,"location":unit.metadata.get("location").cloned().unwrap_or_else(||json!({}))}) });
            generation.edges.push(edge(
                &chunk_id,
                &id,
                "MENTIONS",
                true,
                support(&generation, unit, quote),
            ));
            generation.edges.push(edge(
                &contribution_id,
                &id,
                "DESCRIBES",
                true,
                support(&generation, unit, quote),
            ));
            let projection = projections.entry(id).or_default();
            projection.0.insert(unit.text_unit_code.clone());
            projection
                .1
                .insert(format!("{name}\n{entity_type}\n{quote}"));
            for alias in aliases {
                projection.1.insert(alias["name"].as_str().unwrap().into());
            }
        }
        for relation in relations {
            let resolve = |reference: &str| -> Result<String, String> {
                if let Some(id) = refs.get(reference) {
                    return Ok(id.clone());
                }
                let declaring_chunk = extraction["references"][reference]
                    .as_str()
                    .ok_or("relationship entity declaration is missing")?;
                let source = units
                    .iter()
                    .find(|source| {
                        source.unit_type != "IMAGE" && source.text_unit_code == declaring_chunk
                    })
                    .ok_or("relationship declaration references another source")?;
                let declarations = extractions[declaring_chunk]["nodes"]
                    .as_array()
                    .ok_or("relationship declaration chunk is missing")?;
                let matching = declarations
                    .iter()
                    .filter(|node| node["ref"].as_str() == Some(reference))
                    .collect::<Vec<_>>();
                if matching.len() != 1 {
                    return Err("relationship declaration is absent or ambiguous".into());
                }
                let node = matching[0];
                let name = required(node, "name")?;
                let kind = required(node, "type")?;
                let quote = required(node, "evidence")?;
                let disambiguation = node["disambiguation"].as_str().unwrap_or("");
                if !graph_source_text(source).contains(quote)
                    || !quote.contains(name)
                    || (!disambiguation.is_empty() && !quote.contains(disambiguation))
                {
                    return Err("relationship entity declaration has no original evidence".into());
                }
                Ok(entity_id(&generation.partition, kind, name, disambiguation))
            };
            let from = resolve(required(relation, "from")?)?;
            let to = resolve(required(relation, "to")?)?;
            let relationship = required(relation, "relationship")?;
            let quote = required(relation, "evidence")?;
            if !source_text.contains(quote) || !unit.text.contains(quote) {
                return Err("relationship has no exact source evidence".into());
            }
            generation.edges.push(edge(
                &from,
                &to,
                relationship,
                false,
                support(&generation, unit, quote),
            ));
        }
    }
    generation.nodes.extend(entities.into_values());
    // Duplicate declarations are a single support, while distinct sources stay separate.
    generation.nodes.sort_by(|a, b| a.id.cmp(&b.id));
    generation.nodes.dedup_by(|a, b| a.id == b.id);
    generation.edges.sort_by(|a, b| a.id.cmp(&b.id));
    generation.edges.dedup_by(|a, b| a.id == b.id);
    generation.validate(
        &units
            .iter()
            .map(|u| (u.text_unit_code.clone(), graph_source_text(u)))
            .collect(),
    )?;
    let projections = projections
        .into_iter()
        .map(|(entity_id, (chunks, texts))| GraphNodeTextProjection {
            entity_id,
            generation_id: generation.id.clone(),
            chunk_codes: chunks.into_iter().collect(),
            text: texts.into_iter().collect::<Vec<_>>().join("\n"),
        })
        .collect();
    Ok((generation, projections))
}
