//! Product provenance and fail-closed Office validation; generic rasterization lives in the SDK.
use super::{KnowledgeBaseError, local_documents::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};
use zip::ZipArchive;
fn invalid(message: impl std::fmt::Display) -> KnowledgeBaseError {
    KnowledgeBaseError::Invalid(format!("Office图表预览：{message}"))
}
#[derive(Debug, Clone)]
pub struct OfficeChartPlan {
    pub format: String,
    pub sheets: Vec<String>,
    pub chart_sources: BTreeMap<String, Vec<String>>,
}
impl OfficeChartPlan {
    pub fn has_charts(&self) -> bool {
        self.chart_sources.values().any(|r| !r.is_empty())
    }
}
fn xml(text: &str) -> Result<roxmltree::Document<'_>, KnowledgeBaseError> {
    roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            nodes_limit: 300_000,
            ..Default::default()
        },
    )
    .map_err(invalid)
}
fn package(bytes: &[u8], depth: usize) -> Result<BTreeMap<String, String>, KnowledgeBaseError> {
    if bytes.len() > MAX_RAW_BYTES || depth > 1 {
        return Err(invalid("包大小或嵌套层数超过限制"));
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(invalid)?;
    if zip.len() > 4096 {
        return Err(invalid("包文件数量超过限制"));
    }
    let mut parts = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut total = 0usize;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(invalid)?;
        let name = entry.name().to_owned();
        if !seen.insert(name.clone())
            || entry.enclosed_name().is_none()
            || name.contains('\\')
            || name.starts_with('/')
        {
            return Err(invalid("非法或重复包路径"));
        }
        let lower = name.to_ascii_lowercase();
        if [
            "vbaproject",
            "activex",
            "macros",
            "scripts",
            "connections",
            "externallinks",
            "oleobject",
        ]
        .iter()
        .any(|bad| lower.contains(bad))
            || lower.ends_with(".bin")
        {
            return Err(invalid("宏、OLE对象和外部数据连接不支持安全预览"));
        }
        if entry.size() > MAX_EXPANDED_BYTES as u64 {
            return Err(invalid("解压内容超过限制"));
        }
        let mut data = Vec::new();
        entry
            .by_ref()
            .take(MAX_EXPANDED_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(invalid)?;
        total = total.saturating_add(data.len());
        if total > MAX_EXPANDED_BYTES {
            return Err(invalid("解压内容超过100 MiB"));
        }
        if lower.contains("/embeddings/") {
            if !lower.ends_with(".xlsx") {
                return Err(invalid("仅支持图表内嵌的安全xlsx数据"));
            }
            package(&data, depth + 1)?;
        }
        if !(lower.ends_with(".xml") || lower.ends_with(".rels")) {
            continue;
        }
        if data.len() > 5 * 1024 * 1024 {
            return Err(invalid("XML大小超过限制"));
        }
        let text = String::from_utf8(data).map_err(invalid)?;
        let tree = xml(&text)?; // DTD disabled by default; no external entity expansion.
        for node in tree.descendants().filter(|n| n.is_element()) {
            let tag = node.tag_name().name();
            if matches!(
                tag,
                "oleObject" | "altChunk" | "externalLink" | "connection"
            ) {
                return Err(invalid("OLE、外部内容及数据连接不支持"));
            }
            if node.attributes().any(|a| {
                a.value().to_ascii_lowercase().contains("macroenabled")
                    || a.value().to_ascii_lowercase().contains("vbaproject")
            }) {
                return Err(invalid("含宏的包不支持"));
            }
            if tag == "Relationship"
                && node
                    .attribute("TargetMode")
                    .is_some_and(|v| v.eq_ignore_ascii_case("external"))
            {
                return Err(invalid("外部链接不支持；请先在原文件移除"));
            }
            if matches!(tag, "f" | "instrText" | "fldSimple") {
                let code = node
                    .text()
                    .or_else(|| node.attributes().find(|a|a.name()=="instr").map(|a|a.value()))
                    .unwrap_or("")
                    .to_ascii_uppercase()
                    .replace(char::is_whitespace, "");
                if [
                    "WEBSERVICE(",
                    "HYPERLINK(",
                    "IMAGE(",
                    "RTD(",
                    "DDE",
                    "HTTP:",
                    "HTTPS:",
                    "FILE:",
                    "[",
                    "|",
                    "INCLUDETEXT",
                    "INCLUDEPICTURE",
                    "LINK",
                ]
                .iter()
                .any(|bad| code.contains(bad))
                {
                    return Err(invalid("外部引用或主动公式不支持"));
                }
            }
        }
        parts.insert(name, text);
    }
    Ok(parts)
}
fn resolve(base: &str, target: &str) -> Result<String, KnowledgeBaseError> {
    if target.contains(':')
        || target.contains('\\')
        || target.starts_with("//")
        || target.contains('%')
    {
        return Err(invalid("非法关系目标"));
    }
    let mut segments: Vec<_> = base
        .rsplit_once('/')
        .map(|(p, _)| p.split('/').collect())
        .unwrap_or_default();
    // OPC permits package-root absolute relationship targets; these are ZIP
    // part names, never paths opened on the host filesystem.
    if target.starts_with('/') { segments.clear(); }
    for segment in target.split('/') {
        match segment {
            ".." => {
                if segments.pop().is_none() {
                    return Err(invalid("关系目标越界"));
                }
            }
            "." | "" => {}
            s => segments.push(s),
        }
    }
    Ok(segments.join("/"))
}
fn relationships(
    parts: &BTreeMap<String, String>,
    base: &str,
) -> Result<BTreeMap<String, String>, KnowledgeBaseError> {
    let (dir, name) = base.rsplit_once('/').unwrap_or(("", base));
    let path = format!("{dir}/_rels/{name}.rels");
    let Some(text) = parts.get(&path) else {
        return Ok(BTreeMap::new());
    };
    let tree = xml(text)?;
    let mut result = BTreeMap::new();
    for n in tree
        .descendants()
        .filter(|n| n.tag_name().name() == "Relationship")
    {
        let id = n.attribute("Id").ok_or_else(|| invalid("关系标识缺失"))?;
        let target = n
            .attribute("Target")
            .ok_or_else(|| invalid("关系目标缺失"))?;
        if result.insert(id.into(), resolve(base, target)?).is_some() {
            return Err(invalid("重复关系标识"));
        }
    }
    Ok(result)
}
fn relation_id<'a, 'input>(n: roxmltree::Node<'a, 'input>) -> Option<&'a str> {
    n.attribute((
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
        "id",
    ))
}
fn chart_reference(
    parts: &BTreeMap<String, String>,
    path: &str,
) -> Result<String, KnowledgeBaseError> {
    let tree = xml(parts.get(path).ok_or_else(|| invalid("图表原件缺失"))?)?;
    let formulas: Vec<_> = tree
        .descendants()
        .filter(|n| n.tag_name().name() == "f")
        .filter_map(|n| n.text())
        .collect();
    let title: Vec<_> = tree
        .descendants()
        .filter(|n| n.tag_name().name() == "t")
        .filter_map(|n| n.text())
        .collect();
    let text = format!("{path} [{}] {}", formulas.join(", "), title.join(" "));
    if text.len() > 8192 {
        return Err(invalid("图表来源过长"));
    }
    Ok(text)
}
/// Validate before any local converter or parser transport receives an Office chart document.
pub fn validate_office_chart_input(
    filename: &str,
    bytes: &[u8],
) -> Result<OfficeChartPlan, KnowledgeBaseError> {
    let format = if filename.to_ascii_lowercase().ends_with(".docx") {
        "docx"
    } else if filename.to_ascii_lowercase().ends_with(".xlsx") {
        "xlsx"
    } else {
        return Err(invalid("仅支持docx/xlsx"));
    };
    let parts = package(bytes, 0)?;
    let mut plan = OfficeChartPlan {
        format: format.into(),
        sheets: vec![],
        chart_sources: BTreeMap::new(),
    };
    if format == "docx" {
        let tree = xml(parts
            .get("word/document.xml")
            .ok_or_else(|| invalid("正文缺失"))?)?;
        let rel = relationships(&parts, "word/document.xml")?;
        let mut refs = Vec::new();
        for (i, p) in tree
            .descendants()
            .filter(|n| {
                n.has_tag_name((
                    "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                    "p",
                ))
            })
            .enumerate()
        {
            for chart in p.descendants().filter(|n| {
                n.has_tag_name((
                    "http://schemas.openxmlformats.org/drawingml/2006/chart",
                    "chart",
                ))
            }) {
                let path = relation_id(chart)
                    .and_then(|id| rel.get(id))
                    .ok_or_else(|| invalid("Word图表关联缺失"))?;
                refs.push(format!(
                    "Word段{} {}",
                    i + 1,
                    chart_reference(&parts, path)?
                ));
            }
        }
        plan.chart_sources.insert("document".into(), refs);
    } else {
        let tree = xml(parts
            .get("xl/workbook.xml")
            .ok_or_else(|| invalid("工作簿缺失"))?)?;
        let rel = relationships(&parts, "xl/workbook.xml")?;
        for sheet in tree
            .descendants()
            .filter(|n| n.tag_name().name() == "sheet")
        {
            if sheet.attribute("state").is_some_and(|v| v != "visible") {
                return Err(invalid("隐藏Sheet无法完整预览；请先取消隐藏"));
            }
            let name = sheet
                .attribute("name")
                .ok_or_else(|| invalid("Sheet名称缺失"))?
                .to_owned();
            let path = relation_id(sheet)
                .and_then(|id| rel.get(id))
                .ok_or_else(|| invalid("Sheet关联缺失"))?;
            let sheet_xml = xml(parts.get(path).ok_or_else(|| invalid("Sheet原件缺失"))?)?;
            let sheet_rel = relationships(&parts, path)?;
            let mut refs = Vec::new();
            for drawing in sheet_xml
                .descendants()
                .filter(|n| n.tag_name().name() == "drawing")
            {
                let draw_path = relation_id(drawing)
                    .and_then(|id| sheet_rel.get(id))
                    .ok_or_else(|| invalid("绘图关联缺失"))?;
                let draw_xml = xml(parts
                    .get(draw_path)
                    .ok_or_else(|| invalid("绘图原件缺失"))?)?;
                let draw_rel = relationships(&parts, draw_path)?;
                for chart in draw_xml.descendants().filter(|n| {
                    n.has_tag_name((
                        "http://schemas.openxmlformats.org/drawingml/2006/chart",
                        "chart",
                    ))
                }) {
                    let chart_path = relation_id(chart)
                        .and_then(|id| draw_rel.get(id))
                        .ok_or_else(|| invalid("Excel图表关联缺失"))?;
                    refs.push(format!(
                        "Sheet {name} {}",
                        chart_reference(&parts, chart_path)?
                    ));
                }
            }
            if plan.chart_sources.insert(name.clone(), refs).is_some() {
                return Err(invalid("Sheet名称重复"));
            }
            plan.sheets.push(name);
        }
        if plan.sheets.len() > 16 {
            return Err(invalid("图表预览最多16个Sheet"));
        }
    }
    Ok(plan)
}
/// Keep conversion page identity separate from the original source coordinates.
pub(super) fn append_rendered_sections(
    document: &mut ParsedDocument,
    sections: Vec<LocatedSection>,
) -> Result<(), KnowledgeBaseError> {
    let count = sections.len();
    document.sections.extend(sections);
    if count > 0 {
        let summary = document.image_analysis.get_or_insert_with(Default::default);
        summary.image_count += count;
        summary.originals_only += count;
        document.warnings.push(
            "Office图表已生成本地视觉预览；转换页不等于原文页码，布局以预览及原件为准".into(),
        );
    }
    let mut offset = 0;
    let mut bytes = 0usize;
    for section in &mut document.sections {
        bytes += section.text.len();
        if bytes > MAX_TEXT_BYTES {
            return Err(invalid("文本超过2 MiB"));
        }
        section.location.char_start = offset;
        offset += section.text.chars().count();
        section.location.char_end = offset;
        offset += 1;
    }
    Ok(())
}
