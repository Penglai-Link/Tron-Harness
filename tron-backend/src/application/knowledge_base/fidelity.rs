//! Source fidelity screening. Findings quarantine data; they never rewrite glyphs/units.
use super::{
    KnowledgeBaseDocument, KnowledgeBaseError, KnowledgeBaseService, ResourceBinaryDownloadCommand,
    invalid_storage,
    local_documents::{MAX_EXPANDED_BYTES, MAX_RAW_BYTES, ParsedDocument},
};
mod reprocessing;
use serde::{Deserialize, Serialize};
use std::io;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FidelityIssue {
    #[serde(default)]
    pub source_region: Option<[f64; 4]>,
    #[serde(default)]
    pub candidate_region: Option<[f64; 4]>,
    #[serde(default)]
    pub candidate_excerpt: Option<String>,
    pub kind: String,
    pub page: Option<u32>,
    pub blocks: Vec<String>,
    pub source_excerpt: String,
    pub image_resource_codes: Vec<String>,
}
/// Bounded agreement evidence, never a manual pass or a guarantee of clinical truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FidelityResolution {
    pub method: String,
    pub original_sha256: String,
    pub native_text_sha256: String,
    pub candidate_text_sha256: String,
    pub candidate_resource_code: String,
    pub original_parse_resource_code: Option<String>,
    pub discrepancies: Vec<FidelityIssue>,
    pub coverage: String,
}
pub(super) struct FidelityAssessment {
    issues: Vec<FidelityIssue>,
    resolution: Option<FidelityResolution>,
}
pub(crate) fn blocked(doc: &KnowledgeBaseDocument) -> bool {
    doc.status == "REVIEW_REQUIRED"
        || doc
            .parsing_artifact
            .as_ref()
            .is_some_and(|p| !p.fidelity_issues.is_empty())
}
fn evidence(document: &ParsedDocument, page: Option<u32>) -> (Vec<String>, Vec<String>) {
    let sections = document.sections.iter().filter(|s| s.location.page == page);
    let mut blocks = Vec::new();
    let mut images = Vec::new();
    for s in sections {
        blocks.extend(s.location.blocks.iter().cloned());
        images.extend(s.location.image_resource_codes.iter().cloned());
    }
    blocks.sort();
    blocks.dedup();
    images.sort();
    images.dedup();
    (blocks, images)
}
fn resolve_object<'a>(
    pdf: &'a pdf_extract::Document,
    value: &'a pdf_extract::Object,
) -> Result<&'a pdf_extract::Object, pdf_extract::Error> {
    match value {
        pdf_extract::Object::Reference(id) => pdf.get_object(*id),
        other => Ok(other),
    }
}
/// Only inspect bytes actually shown by the page's direct text operations.
/// Nested form XObjects and multi-byte glyph codes are outside this screen's coverage.
fn page_glyph_usage(
    pdf: &pdf_extract::Document,
    page: pdf_extract::ObjectId,
) -> Result<std::collections::BTreeMap<Vec<u8>, std::collections::BTreeSet<u32>>, KnowledgeBaseError>
{
    let bytes = pdf.get_page_content(page).map_err(invalid_storage)?;
    if bytes.len() > MAX_EXPANDED_BYTES {
        return Err(KnowledgeBaseError::Invalid(
            "PDF字体检查内容超出上限".into(),
        ));
    }
    let content = pdf_extract::content::Content::decode(&bytes).map_err(invalid_storage)?;
    let mut usage = std::collections::BTreeMap::<Vec<u8>, std::collections::BTreeSet<u32>>::new();
    let mut font = Vec::new();
    let mut stack = Vec::new();
    for op in content.operations {
        match op.operator.as_str() {
            "q" => stack.push(font.clone()),
            "Q" => {
                if let Some(previous) = stack.pop() {
                    font = previous;
                }
            }
            "Tf" => {
                if let Some(pdf_extract::Object::Name(name)) = op.operands.first() {
                    font = name.clone();
                }
            }
            "Tj" | "TJ" | "'" | "\"" => {
                for value in op.operands {
                    let strings = match value {
                        pdf_extract::Object::Array(values) => values,
                        other => vec![other],
                    };
                    for value in strings {
                        if let pdf_extract::Object::String(bytes, _) = value {
                            usage
                                .entry(font.clone())
                                .or_default()
                                .extend(bytes.into_iter().map(u32::from));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(usage)
}
pub(super) fn assess(
    document: &ParsedDocument,
    original: &[u8],
) -> Result<Vec<FidelityIssue>, KnowledgeBaseError> {
    let mut issues = Vec::new();
    for section in &document.sections {
        // A combining accent before a base character can result from separately
        // positioned PDF glyphs. Flag ordering; never reorder or replace it.
        let mut previous = None;
        if section.text.chars().any(|c| {
            let detached = matches!(c, '\u{0300}'..='\u{036f}')
                && previous.is_none_or(|p: char| {
                    !p.is_alphabetic() && !matches!(p, '\u{0300}'..='\u{036f}')
                });
            previous = Some(c);
            detached
        }) {
            issues.push(FidelityIssue {
                source_region: section.location.bbox,
                candidate_region: None,
                candidate_excerpt: None,
                kind: "DETACHED_COMBINING_MARK".into(),
                page: section.location.page,
                blocks: section.location.blocks.clone(),
                source_excerpt: section.text.chars().take(160).collect(),
                image_resource_codes: section.location.image_resource_codes.clone(),
            });
        }
        if section
            .text
            .chars()
            .any(|c| (c.is_control() && !matches!(c, '\n' | '\r' | '\t')) || c == '\u{fffd}')
        {
            let chars: Vec<_> = section.text.chars().collect();
            let at = chars
                .iter()
                .position(|c| {
                    (c.is_control() && !matches!(c, '\n' | '\r' | '\t')) || *c == '\u{fffd}'
                })
                .unwrap_or(0);
            let fragment = chars[at.saturating_sub(60)..(at + 100).min(chars.len())]
                .iter()
                .collect::<String>();
            issues.push(FidelityIssue {
                source_region: section.location.bbox,
                candidate_region: None,
                candidate_excerpt: None,
                kind: "UNRESOLVED_TEXT_GLYPH".into(),
                page: section.location.page,
                blocks: section.location.blocks.clone(),
                source_excerpt: fragment.escape_debug().to_string(),
                image_resource_codes: section.location.image_resource_codes.clone(),
            });
        }
    }
    if document.format == "pdf" && original.starts_with(b"%PDF-") {
        let pdf = pdf_extract::Document::load_mem(original).map_err(invalid_storage)?;
        for (page, id) in pdf.get_pages() {
            let used = page_glyph_usage(&pdf, id)?;
            if used.is_empty()
                && !document.sections.iter().any(|s| {
                    s.location.page == Some(page)
                        && s.location.block_kind.as_deref() != Some("image")
                        && !s.text.trim().is_empty()
                })
            {
                let (blocks, images) = evidence(document, Some(page));
                issues.push(FidelityIssue { source_region:None, candidate_region:None, candidate_excerpt:None, kind: "PDF_PAGE_WITHOUT_EXTRACTED_TEXT".into(), page: Some(page), blocks,
                    source_excerpt: "No page text operations or independently extracted text; blank versus scanned content has not been established.".into(), image_resource_codes: images });
            }
            for (name, font) in pdf.get_page_fonts(id).map_err(invalid_storage)? {
                let Some(codes) = used.get(&name) else {
                    continue;
                };
                let obj = |value| resolve_object(&pdf, value);
                // lopdf prefers Encoding over ToUnicode when both exist. Inspect
                // the Unicode map explicitly without altering the source font.
                let mut unicode_font = font.clone();
                unicode_font.remove(b"Encoding");
                let encoding = unicode_font.get_font_encoding(&pdf).ok();
                let simple = font
                    .get(b"Subtype")
                    .and_then(|o| o.as_name())
                    .is_ok_and(|n| matches!(n, b"Type1" | b"TrueType" | b"MMType1"));
                if simple && font.get(b"ToUnicode").is_ok() {
                    for code in codes {
                        let mapped = match &encoding {
                            Some(pdf_extract::Encoding::UnicodeMapEncoding(map)) => {
                                map.get(*code, 1).is_some_and(|v| {
                                    !v.is_empty() && !v.iter().any(|n| *n < 32 || *n == 0xfffd)
                                })
                            }
                            _ => false,
                        };
                        if !mapped {
                            let (blocks, images) = evidence(document, Some(page));
                            issues.push(FidelityIssue { source_region:None, candidate_region:None, candidate_excerpt:None, kind: "INCOMPLETE_PDF_UNICODE_MAP".into(), page: Some(page), blocks,
                                source_excerpt: format!("Used simple-font code 0x{code:02x} in {} is absent from a usable ToUnicode map", String::from_utf8_lossy(&name)), image_resource_codes: images });
                        }
                    }
                }
                if let Ok(raw) = font.get(b"Encoding") {
                    if let Ok(dict) = obj(raw).and_then(|o| o.as_dict()) {
                        if let Ok(differences) = dict.get(b"Differences").and_then(|o| o.as_array())
                        {
                            let mut code = 0u32;
                            for item in differences {
                                match item {
                                    pdf_extract::Object::Integer(n) => {
                                        code = (*n).try_into().unwrap_or(256)
                                    }
                                    pdf_extract::Object::Name(glyph) => {
                                        let unresolved_name = glyph.first() == Some(&b'C')
                                            && glyph.len() > 1
                                            && glyph[1..].iter().all(u8::is_ascii_digit);
                                        let mapped = match &encoding {
                                            Some(pdf_extract::Encoding::UnicodeMapEncoding(
                                                map,
                                            )) => map.get(code, 1).is_some_and(|v| {
                                                !v.is_empty()
                                                    && !v.iter().any(|n| *n < 32 || *n == 0xfffd)
                                            }),
                                            _ => false,
                                        };
                                        if unresolved_name && !mapped && codes.contains(&code) {
                                            let (blocks, images) = evidence(document, Some(page));
                                            issues.push(FidelityIssue { source_region:None, candidate_region:None, candidate_excerpt:None,kind:"UNMAPPED_PDF_FONT_GLYPH".into(),page:Some(page),blocks,source_excerpt:format!("PDF font {} code 0x{code:02x} /{} has no verified Unicode mapping",String::from_utf8_lossy(&name),String::from_utf8_lossy(glyph)),image_resource_codes:images});
                                        }
                                        code += 1;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                if font.get(b"ToUnicode").is_err() {
                    if let Ok(descriptor) = font
                        .get(b"FontDescriptor")
                        .and_then(obj)
                        .and_then(|o| o.as_dict())
                    {
                        if let Ok(charset) = descriptor.get(b"CharSet").and_then(|o| o.as_str()) {
                            let charset = String::from_utf8_lossy(charset);
                            let names: Vec<_> =
                                charset.split('/').filter(|s| !s.is_empty()).collect();
                            let glyphs = names.len();
                            // Ordinary Adobe glyph names (quotes, greater/less, etc.) do not
                            // become a risk solely because ToUnicode is absent. Latin aliases
                            // in a tiny custom subset can silently stand for other symbols.
                            let has_alias = names
                                .iter()
                                .any(|n| n.len() == 1 && n.as_bytes()[0].is_ascii_alphabetic());
                            // Small custom subsets may alias alpha/beta/micro to Latin a/b/m. This is
                            // an ambiguity finding, not an inference that any particular unit is wrong.
                            let custom = font
                                .get(b"BaseFont")
                                .and_then(|o| o.as_name())
                                .is_ok_and(|n| n.contains(&b'+'));
                            if custom && has_alias && (1..=8).contains(&glyphs) {
                                let (blocks, images) = evidence(document, Some(page));
                                issues.push(FidelityIssue { source_region:None, candidate_region:None, candidate_excerpt:None,kind:"SMALL_SUBSET_WITHOUT_UNICODE_MAP".into(),page:Some(page),blocks,source_excerpt:format!("PDF font {} subset {charset} lacks ToUnicode; visible symbols/units need source verification",String::from_utf8_lossy(&name)),image_resource_codes:images});
                            }
                        }
                    }
                }
            }
        }
    }
    // The screening is bounded and conservatively blocks the whole document.
    issues.truncate(256);
    Ok(issues)
}
struct Bounded(Vec<u8>, usize);
impl io::Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > self.1 {
            return Err(io::Error::other("fidelity source exceeds bound"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl KnowledgeBaseService {
    pub(super) fn fidelity_bytes(
        &self,
        user: &str,
        workspace: &str,
        code: &str,
        limit: usize,
    ) -> Result<Vec<u8>, KnowledgeBaseError> {
        let (_, record) = self.resources.download_binary_to_file(
            user,
            ResourceBinaryDownloadCommand {
                workspace_code: workspace.into(),
                resource_code: code.into(),
            },
        )?;
        if record.byte_size > limit as u64 {
            return Err(KnowledgeBaseError::Invalid(
                "数据保真核验资源超出上限".into(),
            ));
        }
        let mut bytes = Bounded(Vec::new(), limit);
        self.resources
            .download_binary_storage(&record, &mut bytes)?;
        Ok(bytes.0)
    }
    /// Reprocessing changes the extraction method, not the trust of the source.
    /// Preserve original findings even when the raster PDF has no text fonts.
    pub(super) fn assess_document_fidelity(
        &self,
        user: &str,
        workspace: &str,
        doc: &KnowledgeBaseDocument,
        document: &ParsedDocument,
        input: &[u8],
    ) -> Result<FidelityAssessment, KnowledgeBaseError> {
        let mut issues = assess(document, input)?;
        let mut resolution = None;
        if let Some(source) = &doc.visual_source {
            let baseline = if let Some(code) = &source.original_parse_resource_code {
                let bytes = self.fidelity_bytes(user, workspace, code, MAX_EXPANDED_BYTES)?;
                Some(super::parsing_cache::cached_document_for_fidelity(&bytes)?)
            } else {
                None
            };
            let mut differences = reprocessing::compare(baseline.as_ref(), document);
            if let Some(baseline) = baseline.as_ref() {
                differences.extend(reprocessing::compare_regions(baseline, document));
            }
            if issues.is_empty()
                && source.original_findings.iter().all(|i| {
                    matches!(
                        i.kind.as_str(),
                        "UNRESOLVED_TEXT_GLYPH" | "DETACHED_COMBINING_MARK"
                    )
                })
                && !source.original_findings.is_empty()
            {
                let original =
                    self.fidelity_bytes(user, workspace, &doc.resource_code, MAX_RAW_BYTES)?;
                if let Some((native_hash, candidate_hash)) =
                    reprocessing::native_agreement(&original, document)?
                {
                    resolution = Some(FidelityResolution {
                        method: "complete-simple-font-native-ocr-agreement-v1".into(),
                        original_sha256: source.original_sha256.clone(), native_text_sha256: native_hash,
                        candidate_text_sha256: candidate_hash,
                        candidate_resource_code: doc.parsing_artifact.as_ref().map(|a|a.resource_code.clone()).unwrap_or_default(),
                        original_parse_resource_code: source.original_parse_resource_code.clone(),
                        discrepancies: differences.clone(),
                        coverage: "Direct simple-font text with complete ToUnicode coverage and ordered full-page text agreement; no font ambiguity, forms, tables, images, missing pages or clinical semantic guarantee.".into(),
                    });
                }
            }
            if resolution.is_none() {
                issues.extend(differences);
            }
            // Agreement between two extractors, absence of bad characters, and identical
            // page pixels cannot verify ambiguous source glyph semantics. No automatic
            // or sample-specific trust override exists; retain the review gate.
            if resolution.is_none() {
                issues.extend(source.original_findings.iter().cloned().map(|mut finding| {
                    finding.kind = "ORIGINAL_FIDELITY_UNRESOLVED".into();
                    finding
                }));
            }
            if source.original_findings.is_empty() {
                issues.push(FidelityIssue {
                    source_region: None,
                    candidate_region: None,
                    candidate_excerpt: None,
                    kind: "REPROCESSING_WITHOUT_SOURCE_FINDINGS".into(),
                    page: None,
                    blocks: vec![],
                    source_excerpt:
                        "Source risk evidence is missing; reprocessing does not establish fidelity."
                            .into(),
                    image_resource_codes: vec![],
                });
            }
        }
        issues.truncate(256);
        Ok(FidelityAssessment { issues, resolution })
    }
    pub(super) fn record_fidelity(
        &self,
        workspace: &str,
        kb: &str,
        doc: &KnowledgeBaseDocument,
        assessment: FidelityAssessment,
    ) -> Result<KnowledgeBaseDocument, KnowledgeBaseError> {
        let mut cached = doc.clone();
        let artifact = cached
            .parsing_artifact
            .as_mut()
            .ok_or_else(|| KnowledgeBaseError::Invalid("数据保真核验缺少解析来源".into()))?;
        artifact.fidelity_checked = true;
        artifact.fidelity_issues = assessment.issues;
        artifact.fidelity_resolution = assessment.resolution;
        if artifact.fidelity_issues.is_empty() && cached.status == "REVIEW_REQUIRED" {
            cached.status = if cached.active_source_code.is_some() {
                "NEEDS_REBUILD"
            } else {
                "UPLOADED"
            }
            .into();
            cached.error = None;
        }
        self.repository.save_fidelity(kb, &cached)?;
        if blocked(&cached) {
            // Persist quarantine before deactivation so reader checks fail closed even if
            // deactivation fails. Historical rows/resources remain available to the owner.
            if let Some(source) = &cached.active_source_code {
                self.deactivate(workspace, kb, source)?;
            }
            if !super::ingestion_worker::in_progress(&cached.status) {
                cached.status = "REVIEW_REQUIRED".into();
                cached.error =
                    Some("数据保真核验发现未解决字形或符号，禁止发布索引；请核对原件及图片".into());
            }
        }
        Ok(cached)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn subset_pdf(charset: &str, used: bool, unicode: bool) -> Vec<u8> {
        use pdf_extract::{
            Document, Object, Stream,
            content::{Content, Operation},
            dictionary,
        };
        let mut pdf = Document::with_version("1.7");
        let pages = pdf.new_object_id();
        let descriptor = pdf.add_object(
            dictionary! {"Type"=>"FontDescriptor","CharSet"=>Object::string_literal(charset)},
        );
        let mut font = dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"ABCDEF+Fixture","Encoding"=>"WinAnsiEncoding","FontDescriptor"=>descriptor};
        if unicode {
            let mapping=pdf.add_object(Stream::new(dictionary!{},b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Test) /Ordering (Unicode) /Supplement 0 >> def /CMapName /Test def /CMapType 2 def 1 begincodespacerange <00> <ff> endcodespacerange 1 beginbfchar <6d> <03bc> endbfchar endcmap CMapName currentdict /CMap defineresource pop end end".to_vec()));
            font.set("ToUnicode", mapping);
        }
        let font = pdf.add_object(font);
        let resources = pdf.add_object(dictionary! {"Font"=>dictionary!{"F1"=>font}});
        let mut operations = vec![];
        if used {
            operations = vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                Operation::new("Tj", vec![Object::string_literal("m")]),
                Operation::new("ET", vec![]),
            ];
        }
        let stream = pdf.add_object(Stream::new(
            dictionary! {},
            Content { operations }.encode().unwrap(),
        ));
        let page=pdf.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"Contents"=>stream,"Resources"=>resources,"MediaBox"=>vec![0.into(),0.into(),300.into(),300.into()]});
        pdf.objects.insert(
            pages,
            dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into(),
        );
        let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
        pdf.trailer.set("Root", catalog);
        let mut bytes = vec![];
        pdf.save_to(&mut bytes).unwrap();
        bytes
    }
    #[test]
    fn absent_unicode_does_not_quarantine_unused_or_standard_punctuation_subsets() {
        let parsed = ParsedDocument {
            format: "pdf".into(),
            sections: vec![],
            warnings: vec![],
            image_analysis: None,
        };
        assert!(
            assess(&parsed, &subset_pdf("/m", false, false))
                .unwrap()
                .iter()
                .all(|i| !i.kind.contains("FONT")
                    && !i.kind.contains("SUBSET")
                    && !i.kind.contains("UNICODE_MAP"))
        );
        assert!(
            assess(&parsed, &subset_pdf("/quoteright", true, false))
                .unwrap()
                .is_empty()
        );
        assert!(
            assess(&parsed, &subset_pdf("/m", true, true))
                .unwrap()
                .is_empty()
        );
        assert!(
            assess(&parsed, &subset_pdf("/m", true, false))
                .unwrap()
                .iter()
                .any(|i| i.kind == "SMALL_SUBSET_WITHOUT_UNICODE_MAP")
        );
    }
}

#[cfg(test)]
mod independent_pdf_fixtures {
    use super::*;
    use crate::application::knowledge_base::local_documents::{
        DocumentLocation, LocatedSection, parse_local_document,
    };
    use pdf_extract::{
        Document, Object, Stream,
        content::{Content, Operation},
        dictionary,
    };

    /// Each fixture defines its own font resources and used-code map; no real-file
    /// hashes, font identities, vocabulary, clinical units, or corrections are used.
    fn source(
        font_name: &str,
        base_font: &str,
        map: Option<&[(u8, u16)]>,
        overlay: bool,
        scan: bool,
    ) -> Vec<u8> {
        let mut pdf = Document::with_version("1.7");
        let pages = pdf.new_object_id();
        let descriptor = pdf.add_object(
            dictionary! {"Type"=>"FontDescriptor", "CharSet"=>Object::string_literal("/m/a")},
        );
        let mut font = dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>base_font, "Encoding"=>"WinAnsiEncoding", "FontDescriptor"=>descriptor};
        if let Some(entries) = map {
            let body = entries
                .iter()
                .map(|(code, value)| format!("<{code:02x}> <{value:04x}>"))
                .collect::<Vec<_>>()
                .join("\n");
            let cmap = format!(
                "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Fixture) /Ordering (Unicode) /Supplement 0 >> def /CMapName /Independent def /CMapType 2 def 1 begincodespacerange <00> <ff> endcodespacerange {} beginbfchar\n{body}\nendbfchar endcmap CMapName currentdict /CMap defineresource pop end end",
                entries.len()
            );
            let mapping = pdf.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
            font.set("ToUnicode", mapping);
        }
        let font = pdf.add_object(font);
        let resources = pdf.add_object(dictionary! {"Font"=>dictionary!{font_name=>font}});
        let mut operations = vec![
            Operation::new("BT", vec![]),
            Operation::new(
                "Tf",
                vec![Object::Name(font_name.as_bytes().to_vec()), 12.into()],
            ),
            Operation::new("Td", vec![20.into(), 100.into()]),
        ];
        if overlay {
            operations.push(Operation::new(
                "Tj",
                vec![Object::string_literal(vec![0x41])],
            ));
            operations.push(Operation::new("Td", vec![(-2).into(), 0.into()]));
            operations.push(Operation::new(
                "Tj",
                vec![Object::string_literal(vec![0x42])],
            ));
        } else {
            operations.push(Operation::new(
                "Tj",
                vec![Object::string_literal(vec![
                    0x6d, 0x20, 0x41, 0x20, 0x42, 0x20, 0x43, 0x20, 0x44, 0x20, 0x45, 0x20, 0x46,
                ])],
            ));
        }
        operations.push(Operation::new("ET", vec![]));
        let content = pdf.add_object(Stream::new(
            dictionary! {},
            Content { operations }.encode().unwrap(),
        ));
        let page=pdf.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"Contents"=>content,"Resources"=>resources,"MediaBox"=>vec![0.into(),0.into(),300.into(),200.into()]});
        let mut kids = vec![page.into()];
        if scan {
            let image=pdf.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Image","Width"=>1,"Height"=>1,"ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8},vec![255,255,255]));
            let resources = pdf.add_object(dictionary! {"XObject"=>dictionary!{"Scan"=>image}});
            let content = pdf.add_object(Stream::new(
                dictionary! {},
                Content {
                    operations: vec![
                        Operation::new("q", vec![]),
                        Operation::new(
                            "cm",
                            vec![
                                200.into(),
                                0.into(),
                                0.into(),
                                200.into(),
                                0.into(),
                                0.into(),
                            ],
                        ),
                        Operation::new("Do", vec![Object::Name(b"Scan".to_vec())]),
                        Operation::new("Q", vec![]),
                    ],
                }
                .encode()
                .unwrap(),
            ));
            let page=pdf.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"Contents"=>content,"Resources"=>resources,"MediaBox"=>vec![0.into(),0.into(),300.into(),200.into()]});
            kids.push(page.into());
        }
        let count = kids.len();
        pdf.objects.insert(
            pages,
            dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>count as i64}.into(),
        );
        let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
        pdf.trailer.set("Root", catalog);
        let mut bytes = vec![];
        pdf.save_to(&mut bytes).unwrap();
        bytes
    }
    fn parsed(text: &str) -> ParsedDocument {
        ParsedDocument {
            format: "pdf".into(),
            sections: vec![LocatedSection {
                text: text.into(),
                heading_level: None,
                location: DocumentLocation {
                    page: Some(1),
                    ..Default::default()
                },
            }],
            warnings: vec![],
            image_analysis: None,
        }
    }
    const FULL: &[(u8, u16)] = &[
        (0x20, 0x20),
        (0x6d, 0x6d),
        (0x41, 0x03bc),
        (0x42, 0x2265),
        (0x43, 0x2264),
        (0x44, 0x2212),
        (0x45, 0x00b2),
        (0x46, 0x2082),
    ];
    #[test]
    fn complete_normal_mapping_preserves_m_micro_comparators_minus_and_scripts() {
        let bytes = source("BodyNative", "Helvetica", Some(FULL), false, false);
        let document = parse_local_document("ordinary.pdf", &bytes).unwrap();
        let text = document
            .sections
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        for expected in ["m", "μ", "≥", "≤", "−", "²", "₂"] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(assess(&document, &bytes).unwrap().is_empty());
    }
    #[test]
    fn unrelated_custom_subset_without_mapping_is_a_risk_without_rewriting_m() {
        let bytes = source("AliasOther", "ZXCVBN+UnrelatedSubset", None, false, false);
        let document = parsed("m A B C D E F");
        assert!(
            assess(&document, &bytes)
                .unwrap()
                .iter()
                .any(|i| i.kind == "SMALL_SUBSET_WITHOUT_UNICODE_MAP")
        );
        assert_eq!(document.sections[0].text, "m A B C D E F");
    }
    #[test]
    fn partial_map_does_not_hide_unmapped_used_codes_behind_latin_glyph_names() {
        let bytes = source(
            "PartialSymbols",
            "QWERTY+PartialMap",
            Some(&[(0x20, 0x20), (0x6d, 0x6d)]),
            false,
            false,
        );
        let issues = assess(&parsed("m A B C D E F"), &bytes).unwrap();
        assert!(
            issues
                .iter()
                .any(|i| i.kind == "INCOMPLETE_PDF_UNICODE_MAP")
        );
        assert!(issues.iter().any(|i| i.source_excerpt.contains("0x41")));
        assert!(!issues.iter().any(|i| i.source_excerpt.contains("0x6d")));
    }
    #[test]
    fn positioned_combining_accent_is_flagged_without_automatic_ordering() {
        let bytes = source(
            "AccentOverlay",
            "ASDFGH+Accent",
            Some(&[(0x41, 0x0301), (0x42, 0x65)]),
            true,
            false,
        );
        let broken = parsed("\u{0301}e");
        assert!(
            assess(&broken, &bytes)
                .unwrap()
                .iter()
                .any(|i| i.kind == "DETACHED_COMBINING_MARK")
        );
        assert_eq!(broken.sections[0].text, "\u{0301}e");
        assert!(assess(&parsed("e\u{0301}"), &bytes).unwrap().is_empty());
    }
    #[test]
    fn mixed_native_and_scanned_pages_report_missing_text_at_the_correct_page() {
        let bytes = source("MixedNative", "Helvetica", Some(FULL), false, true);
        let document = parsed("m μ ≥ ≤ − ² ₂");
        let issues = assess(&document, &bytes).unwrap();
        assert!(
            issues
                .iter()
                .any(|i| i.kind == "PDF_PAGE_WITHOUT_EXTRACTED_TEXT" && i.page == Some(2))
        );
        assert!(!issues.iter().any(|i| i.page == Some(1)));
    }
    #[test]
    fn native_agreement_requires_complete_maps_and_exact_ordered_text() {
        let bytes = source("RecoveryNative", "Helvetica", Some(FULL), false, false);
        let native = parse_local_document("ordinary.pdf", &bytes).unwrap();
        assert!(
            reprocessing::native_agreement(&bytes, &native)
                .unwrap()
                .is_some()
        );
        let mut wrong = native.clone();
        wrong.sections[0].text = wrong.sections[0].text.replace("≥", "≤");
        assert!(
            reprocessing::native_agreement(&bytes, &wrong)
                .unwrap()
                .is_none()
        );
        let mut swapped = native.clone();
        swapped.sections[0].text = swapped.sections[0]
            .text
            .replace('m', "μ")
            .replacen("μ", "m", 1);
        // Explicitly swap the first two source words; token multisets may still agree.
        let words: Vec<_> = native.sections[0].text.split_whitespace().collect();
        if words.len() > 1 {
            let mut words = words;
            words.swap(0, 1);
            swapped.sections[0].text = words.join(" ");
        }
        assert!(
            reprocessing::native_agreement(&bytes, &swapped)
                .unwrap()
                .is_none()
        );
        assert!(
            reprocessing::native_agreement(
                &source(
                    "IncompleteOther",
                    "MNBVCX+Other",
                    Some(&[(0x6d, 0x6d)]),
                    false,
                    false
                ),
                &native
            )
            .unwrap()
            .is_none()
        );
        assert!(
            reprocessing::native_agreement(
                &source("MixedOther", "Helvetica", Some(FULL), false, true),
                &native
            )
            .unwrap()
            .is_none()
        );
    }
}
