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
