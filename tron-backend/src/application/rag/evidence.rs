//! Public source evidence never includes vectors or native node properties.
use super::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagEvidencePartView {
    pub chunk_code: String,
    pub text: String,
    pub image_vector: bool,
    pub file_name: Option<String>,
    pub location: Value,
    pub source_locations: Vec<Value>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagImageAnnotationView {
    pub kind: graph::GraphNodeKindView,
    pub text: String,
    pub source: Option<String>,
    pub chunk_codes: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagEvidenceImageView {
    pub id: String,
    pub resource_code: String,
    pub workspace_code: String,
    pub file_name: Option<String>,
    pub location: Value,
    pub container_location: Option<Value>,
    pub annotations: Vec<RagImageAnnotationView>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagEvidenceAssociationView {
    pub from: String,
    pub to: String,
    pub relationship: String,
    pub chunk_code: String,
    pub quote: String,
    pub location: Value,
}

pub(crate) fn public_evidence(
    hit: &RagEvidence,
) -> (
    Vec<RagEvidencePartView>,
    Vec<RagEvidenceImageView>,
    Vec<RagEvidenceAssociationView>,
    Vec<String>,
) {
    let rows = hit
        .bundle
        .as_ref()
        .filter(|bundle| !bundle.display.is_empty())
        .map(|bundle| bundle.display.iter().collect::<Vec<_>>())
        .unwrap_or_else(|| {
            std::iter::once(hit)
                .chain(hit.bundle.iter().flat_map(|bundle| bundle.members.iter()))
                .collect()
        });
    let parts = rows
        .into_iter()
        .map(|member| {
            let locations = member
                .row
                .text_metadata
                .get("contextLocations")
                .or_else(|| member.row.text_metadata.get("sourceLocations"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|location| {
                    member.selected_blocks.is_empty()
                        || location["sourceBlockId"]
                            .as_str()
                            .is_some_and(|id| member.selected_blocks.contains(id))
                })
                .cloned()
                .collect::<Vec<_>>();
            RagEvidencePartView {
                chunk_code: member.row.target_code.clone(),
                text: member.text().to_owned(),
                image_vector: member.row.text_metadata["location"]["blockKind"] == "image_vector",
                file_name: member.row.text_metadata["fileName"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| member.row.title.clone()),
                location: locations
                    .first()
                    .cloned()
                    .unwrap_or_else(|| member.row.text_metadata["location"].clone()),
                source_locations: locations,
            }
        })
        .collect();
    let Some(bundle) = &hit.bundle else {
        return (parts, vec![], vec![], vec![]);
    };
    let nodes = bundle
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut images = Vec::new();
    use crate::domain::rag::graph::RagGraphNodeKind as Kind;
    for image in bundle.nodes.iter().filter(|node| node.kind == Kind::Image) {
        let assets = bundle
            .edges
            .iter()
            .filter(|edge| edge.from == image.id && edge.relationship == "HAS_ASSET")
            .filter_map(|edge| nodes.get(edge.to.as_str()))
            .filter_map(|node| node.properties["resourceCode"].as_str())
            .collect::<BTreeSet<_>>();
        for asset in assets {
            let mut annotations = Vec::new();
            let mut seen = BTreeSet::new();
            for edge in bundle.edges.iter().filter(|edge| {
                edge.from == image.id
                    && matches!(
                        edge.relationship.as_str(),
                        "HAS_CAPTION" | "HAS_OCR" | "HAS_DESCRIPTION"
                    )
            }) {
                let Some(node) = nodes.get(edge.to.as_str()) else {
                    continue;
                };
                let kind = match node.kind {
                    Kind::SourceCaption => graph::GraphNodeKindView::SourceCaption,
                    Kind::OcrText => graph::GraphNodeKindView::OcrText,
                    Kind::GeneratedDescription => graph::GraphNodeKindView::GeneratedDescription,
                    _ => continue,
                };
                let Some(text) = node.properties["text"].as_str() else {
                    continue;
                };
                if !seen.insert((format!("{:?}", node.kind), text)) {
                    continue;
                }
                annotations.push(RagImageAnnotationView {
                    kind,
                    text: text.into(),
                    source: node.properties["source"].as_str().map(str::to_owned),
                    chunk_codes: bundle
                        .edges
                        .iter()
                        .filter(|e| e.to == node.id)
                        .map(|e| e.support.chunk_code.clone())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                });
            }
            images.push(RagEvidenceImageView {
                id: image.id.clone(),
                resource_code: asset.into(),
                workspace_code: bundle.partition.scope_code.clone(),
                file_name: hit.row.text_metadata["fileName"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| hit.row.title.clone()),
                location: image
                    .properties
                    .get("location")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
                container_location: image.properties.get("containerLocation").cloned(),
                annotations,
            });
        }
    }
    let associations = bundle
        .edges
        .iter()
        .map(|edge| RagEvidenceAssociationView {
            from: edge.from.clone(),
            to: edge.to.clone(),
            relationship: edge.relationship.clone(),
            chunk_code: edge.support.chunk_code.clone(),
            quote: edge.support.quote.clone(),
            location: edge.support.location.clone(),
        })
        .collect();
    (
        parts,
        images,
        associations,
        bundle.truncated_by.iter().cloned().collect(),
    )
}
