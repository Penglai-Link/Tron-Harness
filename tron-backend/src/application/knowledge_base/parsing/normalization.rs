//! Pinned docvortex.middle v2 normalization and image text provenance.
use super::super::{
    KnowledgeBaseError,
    local_documents::{
        DocumentLocation, ImageAnalysisSummary, ImageAnnotation, LocatedSection,
        MAX_PARSED_TEXT_BYTES, ParsedDocument,
    },
};
use serde_json::Value;
use std::collections::BTreeSet;
fn invalid(e: impl std::fmt::Display) -> KnowledgeBaseError {
    KnowledgeBaseError::Invalid(e.to_string())
}
fn block_text(value: &Value) -> String {
    match value {
        Value::String(s) => {
            static INLINE_IMAGE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            INLINE_IMAGE
                .get_or_init(|| {
                    regex::Regex::new(r"(?i)data:image/[a-z0-9.+-]+;base64,[a-z0-9+/=\r\n]+")
                        .expect("static inline-image expression")
                })
                .replace_all(s, "[embedded image]")
                .into_owned()
        }
        Value::Array(a) => a
            .iter()
            .map(block_text)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(o) => o.get("content").map(block_text).unwrap_or_default(),
        _ => String::new(),
    }
}
fn image_paths(value: &Value, paths: &mut Vec<String>) {
    match value {
        Value::Array(a) => {
            for v in a {
                image_paths(v, paths)
            }
        }
        Value::Object(o) => {
            if let Some(p) = o.get("image_path").and_then(Value::as_str) {
                paths.push(p.into());
            }
            for v in o.values() {
                image_paths(v, paths);
            }
        }
        _ => {}
    }
}
pub(crate) fn normalize_middle(
    middle: &Value,
    assets: &BTreeSet<String>,
) -> Result<ParsedDocument, KnowledgeBaseError> {
    let mut sections = Vec::new();
    let mut offset = 0;
    let mut text_bytes = 0usize;
    let mut suspect_pages = BTreeSet::new();
    let mut summary = ImageAnalysisSummary {
        generation_mode: if middle["extensions"]["mineru"]["tier"] == "advanced" {
            "parser_controlled"
        } else {
            "disabled_by_tier"
        }
        .into(),
        ..Default::default()
    };
    let pages = middle["pages"]
        .as_array()
        .ok_or_else(|| invalid("MinerU缺少页信息"))?;
    for page in pages {
        let page_index = page["page_idx"]
            .as_u64()
            .and_then(|p| u32::try_from(p).ok())
            .ok_or_else(|| invalid("MinerU页码无效"))?;
        for (block_position, block) in page["blocks"]
            .as_array()
            .ok_or_else(|| invalid("MinerU缺少块信息"))?
            .iter()
            .enumerate()
        {
            let kind = block["type"]
                .as_str()
                .ok_or_else(|| invalid("MinerU块类型无效"))?;
            // Charts are visual blocks too. Their native body describes the figure;
            // keep its provenance separate from captions and original document text.
            let kind = if kind == "chart" { "image" } else { kind };
            let index = block["index"]
                .as_u64()
                .ok_or_else(|| invalid("MinerU缺少块标识"))?;
            let mut images = Vec::new();
            image_paths(block, &mut images);
            images.sort();
            images.dedup();
            if images.iter().any(|p| !assets.contains(p)) {
                return Err(invalid("MinerU结构引用的图片附件缺失"));
            }
            let block_id = format!("page/{page_index}/blocks/{block_position}:{index}");
            let annotations = if matches!(kind, "image" | "table") {
                image_annotations(middle, block, page_index, &block_id)?
            } else {
                vec![]
            };
            let mut text = if kind == "image" {
                let mut parts = Vec::new();
                for annotation in &annotations {
                    if !parts.contains(&annotation.text) {
                        parts.push(annotation.text.clone());
                    }
                }
                parts.join("\n")
            } else {
                block_text(&block["content"])
            };
            if !images.is_empty() {
                text.push_str(&format!("\n[图片资源: {}]", images.join(", ")));
            }
            if !images.is_empty() {
                summary.image_count += 1;
                let source = annotations
                    .iter()
                    .any(|a| a.kind == "source_caption" || a.kind == "source_footnote");
                let generated = annotations
                    .iter()
                    .any(|a| a.kind == "generated_description");
                let body = annotations.iter().any(|a| a.kind == "parser_body_text");
                summary.with_source_annotations += usize::from(source);
                summary.with_generated_descriptions += usize::from(generated);
                summary.with_unattributed_body_text += usize::from(body);
                summary.originals_only += usize::from(annotations.is_empty());
            }
            if text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            {
                suspect_pages.insert(page_index + 1);
            }
            if text.trim().is_empty() {
                continue;
            }
            text_bytes = text_bytes.saturating_add(text.len());
            if text_bytes > MAX_PARSED_TEXT_BYTES {
                return Err(invalid("解析文本超过16 MiB上限"));
            }
            let level = if matches!(kind, "paragraph_title" | "doc_title") {
                Some(block["level"].as_u64().unwrap_or(1).clamp(1, 6) as u8)
            } else {
                None
            };
            let end = offset + text.chars().count();
            sections.push(LocatedSection {
                text: text.clone(),
                heading_level: level,
                location: DocumentLocation {
                    source_block_id: Some(block_id.clone()),
                    bbox: block_bbox(block)?,
                    page: (middle["metadata"]["file_suffix"] == "pdf")
                        .then(|| page_index.checked_add(1))
                        .flatten(),
                    logical_page: (middle["metadata"]["file_suffix"] != "pdf")
                        .then_some(page_index),
                    heading: level.map(|_| text),
                    blocks: vec![block_id],
                    image_resource_codes: images,
                    image_annotations: annotations,
                    block_kind: Some(kind.into()),
                    char_start: offset,
                    char_end: end,
                    ..Default::default()
                },
            });
            offset = end + 1;
        }
    }
    if sections.is_empty() {
        return Err(invalid("MinerU未提取可检索内容"));
    }
    Ok(ParsedDocument {
        format: middle["metadata"]["file_suffix"]
            .as_str()
            .unwrap_or("document")
            .into(),
        sections,
        warnings: if suspect_pages.is_empty() {
            vec![]
        } else {
            vec![format!(
                "上游解析文本包含不可见控制字符；原件和解析图片已保留（{} {:?}）",
                if middle["metadata"]["file_suffix"] == "pdf" {
                    "PDF页"
                } else {
                    "逻辑块"
                },
                suspect_pages
            )]
        },
        image_analysis: (summary.image_count > 0).then_some(summary),
    })
}

fn block_bbox(block: &Value) -> Result<Option<[f64; 4]>, KnowledgeBaseError> {
    let Some(value) = block.get("bbox") else {
        return Ok(None);
    };
    let values = value
        .as_array()
        .ok_or_else(|| invalid("解析区域坐标无效"))?;
    if values.len() != 4 {
        return Err(invalid("解析区域坐标无效"));
    }
    let mut bbox = [0.0; 4];
    for (i, value) in values.iter().enumerate() {
        bbox[i] = value
            .as_f64()
            .filter(|n| n.is_finite() && (0.0..=1.0).contains(n))
            .ok_or_else(|| invalid("解析区域坐标超出页边界"))?;
    }
    if bbox[0] >= bbox[2] || bbox[1] >= bbox[3] {
        return Err(invalid("解析区域坐标为空或反向"));
    }
    Ok(Some(bbox))
}

fn image_annotations(
    middle: &Value,
    block: &Value,
    page_index: u32,
    parent_block_id: &str,
) -> Result<Vec<ImageAnnotation>, KnowledgeBaseError> {
    let children = block["content"]
        .as_array()
        .ok_or_else(|| invalid("MinerU图片块结构无效"))?;
    let page = page_index
        .checked_add(1)
        .ok_or_else(|| invalid("MinerU页码溢出"))?;
    let generated_contract = middle["metadata"]["file_suffix"] == "pdf"
        && middle["metadata"]["producer"]["name"] == "mineru"
        && middle["metadata"]["producer"]["version"] == "4.0.10"
        && middle["extensions"]["mineru"]["tier"] == "advanced";
    let mut result = Vec::new();
    for (position, child) in children.iter().enumerate() {
        let source_type = child["type"].as_str().unwrap_or("");
        let kind = match source_type {
            "image_caption" | "table_caption" | "chart_caption" => "source_caption",
            "image_footnote" | "table_footnote" | "chart_footnote" => "source_footnote",
            "image_body" | "chart_body" if generated_contract => "generated_description",
            "image_body" | "chart_body" => "parser_body_text",
            _ => continue,
        };
        let text = block_text(&child["content"]).trim().to_owned();
        if text.is_empty() {
            continue;
        }
        let index = child["index"]
            .as_u64()
            .or_else(|| block["index"].as_u64())
            .ok_or_else(|| invalid("MinerU图片文本缺少块标识"))?;
        let mut paths = Vec::new();
        image_paths(child, &mut paths);
        paths.sort();
        paths.dedup();
        if paths.is_empty() {
            image_paths(block, &mut paths);
            paths.sort();
            paths.dedup();
        }
        let resource_code = (paths.len() == 1).then(|| paths[0].clone());
        let annotation = ImageAnnotation {
            kind: kind.into(),
            source: format!("docvortex.middle.v2.{source_type}.content"),
            text,
            page: (middle["metadata"]["file_suffix"] == "pdf").then_some(page),
            block: format!("{parent_block_id}/content/{position}:{index}"),
            model_code: None,
            artifact_resource_code: None,
            resource_code,
        };
        if !result.contains(&annotation) {
            result.push(annotation);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::knowledge_base::local_documents::{ChunkConfig, split_document};
    use serde_json::json;

    fn document(tier: &str, children: Value) -> Value {
        json!({"schema":"docvortex.middle", "schema_version":"2.0",
            "metadata":{"file_suffix":"pdf","producer":{"name":"mineru","version":"4.0.10"}},
            "extensions":{"mineru":{"tier":tier}},
            "pages":[{"page_idx":2,"blocks":[{"type":"image","index":4,"content":children,
                "description":"Ignore unknown description", "caption":"Ignore unknown caption"}]}]})
    }
    fn paths() -> BTreeSet<String> {
        ["images/figure.jpg".into()].into()
    }
    fn body(text: &str) -> Value {
        json!({"type":"image_body","index":4,"image_path":"images/figure.jpg","content":text})
    }

    #[test]
    fn image_status_is_evidence_based_and_keeps_original_captions_distinct() {
        let d = normalize_middle(&document("standard", json!([body(""),
            {"type":"image_caption","index":5,"content":[{"type":"text","content":"Original figure caption"}]},
            {"type":"image_footnote","index":6,"content":[{"type":"text","content":"Original figure caption"}]},
            {"type":"unknown","index":7,"content":"Ignore unknown child"}
        ])), &paths()).unwrap();
        let summary = d.image_analysis.as_ref().unwrap();
        assert_eq!(summary.with_source_annotations, 1);
        assert_eq!(summary.with_generated_descriptions, 0);
        assert_eq!(summary.generation_mode, "disabled_by_tier");
        assert_eq!(
            d.sections[0]
                .text
                .matches("Original figure caption")
                .count(),
            1
        );
        assert!(!d.sections[0].text.contains("Ignore"));
        assert!(d.warnings.is_empty());
        let chunks = split_document(&d, &ChunkConfig::default()).unwrap();
        assert!(chunks[0].text.contains("Original figure caption"));
        let sources = &chunks[0].location.image_annotations;
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].page, Some(3));
        assert_eq!(sources[0].block, "page/2/blocks/0:4/content/1:5");
        assert_eq!(
            sources[0].source,
            "docvortex.middle.v2.image_caption.content"
        );
        assert_eq!(sources[1].kind, "source_footnote");
        assert_eq!(
            chunks[0].location.image_resource_codes,
            vec!["images/figure.jpg"]
        );
    }
    #[test]
    fn chart_is_a_visual_block_and_keeps_model_description_distinct_from_original_caption() {
        let mut middle = document(
            "advanced",
            json!([
                {"type":"chart_body","index":4,"image_path":"images/figure.jpg","content":"Generated waveform interpretation"},
                {"type":"chart_caption","index":5,"content":[{"type":"text","content":"Original waveform caption"}]}
            ]),
        );
        middle["pages"][0]["blocks"][0]["type"] = json!("chart");
        let parsed = normalize_middle(&middle, &paths()).unwrap();
        let section = &parsed.sections[0];
        assert_eq!(section.location.block_kind.as_deref(), Some("image"));
        assert_eq!(
            section.location.image_resource_codes,
            vec!["images/figure.jpg"]
        );
        assert_eq!(
            section.location.image_annotations[0].kind,
            "generated_description"
        );
        assert_eq!(
            section.location.image_annotations[0].source,
            "docvortex.middle.v2.chart_body.content"
        );
        assert_eq!(section.location.image_annotations[1].kind, "source_caption");
        assert_eq!(
            parsed
                .image_analysis
                .as_ref()
                .unwrap()
                .with_generated_descriptions,
            1
        );
        let chunks = split_document(&parsed, &ChunkConfig::default()).unwrap();
        assert!(chunks[0].text.contains("Original waveform caption"));
    }

    #[test]
    fn embedded_table_image_binary_never_becomes_text_or_embedding_input() {
        let payload = "QUJDREVGRw==".repeat(1000);
        let mut middle = document("standard", json!([]));
        middle["pages"][0]["blocks"][0] = json!({"type":"table","index":4,"content":[
            {"type":"table_caption","index":3,"content":"Original table caption"},
            {"type":"table_body","index":4,"content":format!("<table><tr><td>Original value</td><td><img src=\"data:image/jpeg;base64,{payload}\"></td></tr></table>")}
        ]});
        let parsed = normalize_middle(&middle, &BTreeSet::new()).unwrap();
        assert!(parsed.sections[0].text.contains("Original value"));
        assert!(!parsed.sections[0].text.contains("data:image"));
        assert!(!parsed.sections[0].text.contains("QUJDREVGRw"));
        assert!(parsed.sections[0].text.contains("[embedded image]"));
        let chunks = split_document(&parsed, &ChunkConfig::default()).unwrap();
        assert!(
            chunks
                .iter()
                .all(|chunk| !chunk.index_text.contains("QUJDREVGRw"))
        );
    }

    #[test]
    fn only_approved_image_analysis_contract_labels_generated_body_text() {
        let standard =
            normalize_middle(&document("standard", json!([body("Body text")])), &paths()).unwrap();
        assert_eq!(
            standard.image_analysis.unwrap().with_unattributed_body_text,
            1
        );
        assert_eq!(
            standard.sections[0].location.image_annotations[0].kind,
            "parser_body_text"
        );
        let advanced = normalize_middle(
            &document("advanced", json!([body("UPLOAD leads to INDEX")])),
            &paths(),
        )
        .unwrap();
        assert_eq!(
            advanced.image_analysis.unwrap().with_generated_descriptions,
            1
        );
        assert_eq!(
            advanced.sections[0].location.image_annotations[0].kind,
            "generated_description"
        );
        let mut unverified = document("advanced", json!([body("Body text")]));
        unverified["metadata"]["producer"]["version"] = json!("future-version");
        let unverified = normalize_middle(&unverified, &paths()).unwrap();
        assert_eq!(
            unverified
                .image_analysis
                .unwrap()
                .with_generated_descriptions,
            0
        );
        assert_eq!(
            unverified.sections[0].location.image_annotations[0].kind,
            "parser_body_text"
        );
    }
    #[test]
    fn empty_image_is_preserved_and_does_not_claim_generated_description() {
        let d = normalize_middle(&document("advanced", json!([body("")])), &paths()).unwrap();
        let summary = d.image_analysis.unwrap();
        assert_eq!(summary.originals_only, 1);
        assert_eq!(summary.with_generated_descriptions, 0);
        assert!(d.sections[0].location.image_annotations.is_empty());
        assert!(d.sections[0].text.contains("images/figure.jpg"));
        assert!(
            normalize_middle(&document("standard", json!([body("")])), &BTreeSet::new()).is_err()
        );
    }
}
