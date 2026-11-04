//! Normalize supplier segment envelopes before chunking; never append images to the final paragraph.
use super::{KnowledgeBaseError, ParsedDocument, normalize_middle};
use crate::application::document::{
    DocumentLocation, ImageAnalysisSummary, LocatedSection, MAX_PARSED_TEXT_BYTES,
};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn has_structured_middle(value: &Value) -> bool {
    value["schema"] == "docvortex.middle"
        || value.as_array().is_some_and(|items| {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| item["data"]["schema"] == "docvortex.middle")
        })
}
fn invalid(message: &str) -> KnowledgeBaseError {
    KnowledgeBaseError::Invalid(message.into())
}

/// Segment-local coordinates remain explicit when the platform did not return the
/// original page range. A segment size or filename is never used as a page offset.
pub(crate) fn normalize_parse_middle(
    value: &Value,
    assets: &BTreeSet<String>,
) -> Result<ParsedDocument, KnowledgeBaseError> {
    if value["schema"] == "docvortex.middle" {
        return normalize_middle(value, assets);
    }
    let segments = value
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| invalid("解析结构化分段缺失"))?;
    let mut result = ParsedDocument {
        format: String::new(),
        sections: vec![],
        warnings: vec![],
        image_analysis: None,
    };
    let mut seen = BTreeSet::new();
    let mut offset = 0;
    let mut bytes = 0usize;
    for (position, segment) in segments.iter().enumerate() {
        let seq = segment["seq"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| invalid("解析分段标识无效"))?;
        if !seen.insert(seq) {
            return Err(invalid("解析分段标识重复"));
        }
        if seq as usize != position {
            return Err(invalid("解析分段顺序不完整"));
        }
        if segment["data"]["schema"] != "docvortex.middle" {
            return Err(invalid("解析分段结构不受支持"));
        }
        let mut parsed = normalize_middle(&segment["data"], assets)?;
        if result.format.is_empty() {
            result.format = parsed.format.clone();
        }
        if result.format != parsed.format {
            return Err(invalid("解析分段格式不一致"));
        }
        // Newer platform manifests can supply the actual zero-based original page start.
        let page_start = segment
            .get("source_page_start")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok());
        // Host-verified per-page coordinates can be sparse. Unmapped pages keep
        // their explicit segment coordinates; never interpolate a source offset.
        let mut source_pages = std::collections::BTreeMap::<u32, u32>::new();
        if let Some(mapping) = segment.get("source_page_indices") {
            let mapping = mapping
                .as_object()
                .ok_or_else(|| invalid("原始页定位映射无效"))?;
            let local_pages = segment["data"]["pages"]
                .as_array()
                .ok_or_else(|| invalid("解析分段页信息缺失"))?;
            let mut seen_original = std::collections::BTreeSet::new();
            for (local, original) in mapping {
                let local: u32 = local.parse().map_err(|_| invalid("分段页定位无效"))?;
                let original = original
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .filter(|v| *v < u32::MAX)
                    .ok_or_else(|| invalid("原始页定位无效"))?;
                if !local_pages
                    .iter()
                    .any(|page| page["page_idx"].as_u64() == Some(local as u64))
                    || !seen_original.insert(original)
                    || page_start.is_some_and(|start| start.checked_add(local) != Some(original))
                {
                    return Err(invalid("原始页定位与分段结构不一致"));
                }
                source_pages.insert(local, original);
            }
        }
        for section in &mut parsed.sections {
            section.location.source_segment = Some(seq);
            section.location.segment_page = section.location.page;
            if let Some(page) = section.location.page {
                section.location.page = source_pages
                    .get(&(page - 1))
                    .and_then(|original| original.checked_add(1))
                    .or_else(|| page_start.and_then(|start| start.checked_add(page)));
            }
            section.location.source_block_id = section
                .location
                .source_block_id
                .as_ref()
                .map(|id| format!("segment/{seq}/{id}"));
            section.location.parent_source_block_id = section
                .location
                .parent_source_block_id
                .as_ref()
                .map(|id| format!("segment/{seq}/{id}"));
            section.location.blocks = section
                .location
                .blocks
                .iter()
                .map(|id| format!("segment/{seq}/{id}"))
                .collect();
            for annotation in &mut section.location.image_annotations {
                annotation.page = section.location.page;
                annotation.block = format!("segment/{seq}/{}", annotation.block);
            }
            section.location.char_start = offset;
            offset += section.text.chars().count();
            section.location.char_end = offset;
            offset += 1;
            bytes = bytes.saturating_add(section.text.len());
            if bytes > MAX_PARSED_TEXT_BYTES {
                return Err(invalid("解析文本超过16 MiB上限"));
            }
        }
        result.sections.extend(parsed.sections);
        result.warnings.extend(parsed.warnings);
        if parsed.format == "pdf"
            && page_start.is_none()
            && source_pages.len() < segment["data"]["pages"].as_array().map_or(0, Vec::len)
        {
            result.warnings.push(format!(
                "解析分段{seq}未提供原始页范围；保留分段页码，未推测PDF页码"
            ));
        }
        if let Some(summary) = parsed.image_analysis {
            let total = result
                .image_analysis
                .get_or_insert_with(ImageAnalysisSummary::default);
            total.image_count += summary.image_count;
            total.originals_only += summary.originals_only;
            total.with_source_annotations += summary.with_source_annotations;
            total.with_generated_descriptions += summary.with_generated_descriptions;
            total.with_unattributed_body_text += summary.with_unattributed_body_text;
            total.generation_mode = summary.generation_mode;
        }
    }
    let linked: BTreeSet<_> = result
        .sections
        .iter()
        .flat_map(|s| s.location.image_resource_codes.iter().cloned())
        .collect();
    for asset in assets.difference(&linked) {
        // Unmapped assets stay retrievable, but do not claim an unrelated paragraph as parent.
        result.sections.push(LocatedSection {
            text: String::new(),
            heading_level: None,
            location: DocumentLocation {
                source_block_id: Some(format!("unmapped-asset/{asset}")),
                block_kind: Some("image".into()),
                image_resource_codes: vec![asset.clone()],
                blocks: vec![format!("unmapped-asset/{asset}")],
                char_start: offset,
                char_end: offset,
                ..Default::default()
            },
        });
        let total = result
            .image_analysis
            .get_or_insert_with(ImageAnalysisSummary::default);
        total.image_count += 1;
        total.originals_only += 1;
    }
    if linked.len() < assets.len() {
        result
            .warnings
            .push("部分图片缺少上游来源块映射；原图保留，未推测关联正文".into());
    }
    Ok(result)
}
