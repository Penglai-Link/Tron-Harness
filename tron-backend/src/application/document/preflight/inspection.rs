use super::*;
use crate::application::document::{
    LocalDocumentError, MAX_EXPANDED_BYTES, MAX_RAW_BYTES, MAX_TEXT_BYTES,
};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};

fn invalid(message: impl std::fmt::Display) -> LocalDocumentError {
    LocalDocumentError(message.to_string())
}

pub fn inspect_document(
    filename: &str,
    bytes: &[u8],
) -> Result<DocumentFeatures, LocalDocumentError> {
    if bytes.is_empty() || bytes.len() > MAX_RAW_BYTES {
        return Err(invalid("文件必须非空且不超过20 MiB"));
    }
    let format = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !PARSE_EXTENSIONS.contains(&format.as_str())
        && !DIRECT_READ_EXTENSIONS.contains(&format.as_str())
    {
        return Err(invalid("此格式不在MinerU支持范围内"));
    }
    let mut facts = DocumentFeatures {
        format: format.clone(),
        source_sha256: format!("{:x}", Sha256::digest(bytes)),
        ..Default::default()
    };
    match format.as_str() {
        "md" | "markdown" | "txt" | "rst" | "tex" => {
            let text = crate::application::document::decode_plain_text(bytes)?;
            facts.local_text = true;
            facts.text_characters = text.chars().count();
            facts.units = text.lines().count().max(1);
            facts.inspected_units = facts.units;
            facts.images = if format == "md" || format == "markdown" {
                text.matches("![").count()
            } else {
                0
            };
        }
        "docx" | "pptx" | "xlsx" => inspect_office(bytes, &mut facts)?,
        "pdf" => inspect_pdf(bytes, &mut facts)?,
        "doc" | "ppt" | "xls" => {
            if !bytes.starts_with(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1") {
                return Err(invalid("旧版Office文件签名无效"));
            }
            // Do not inspect or convert old binary Office content; the host's
            // fixed OCR policy forwards these exact original bytes.
            facts.image_only_units = 1;
            facts.units = 1;
        }
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "jp2" => {
            let signature = match format.as_str() {
                "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                "jpg" | "jpeg" => bytes.starts_with(b"\xff\xd8\xff"),
                "webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
                "gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
                "bmp" => bytes.starts_with(b"BM"),
                "tiff" => bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*"),
                _ => {
                    bytes.starts_with(b"\0\0\0\x0cjP  \r\n\x87\n")
                        || bytes.starts_with(b"\xff\x4f\xff\x51")
                }
            };
            if !signature {
                return Err(invalid("图片内容与格式不匹配"));
            }
            facts.images = 1;
            facts.image_only_units = 1;
            facts.units = 1;
            facts.inspected_units = 1;
        }
        "odt" | "ods" | "odp" | "epub" | "ofd" => {
            if !bytes.starts_with(b"PK") {
                return Err(invalid("压缩文档包签名无效"));
            }
            crate::application::document::validate_office(bytes, None)?;
        }
        "rtf" => {
            if !bytes.starts_with(b"{\\rtf") {
                return Err(invalid("RTF签名无效"));
            }
        }
        "html" | "htm" | "shtml" | "csv" | "tsv" => {
            facts.text_characters = crate::application::document::decode_plain_text(bytes)?
                .chars()
                .count();
        }
        _ => return Err(invalid("此格式不在MinerU支持范围内")),
    }
    Ok(facts)
}

fn inspect_office(bytes: &[u8], facts: &mut DocumentFeatures) -> Result<(), LocalDocumentError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(invalid)?;
    if zip.len() > 4096 {
        return Err(invalid("Office包条目超过4096上限"));
    }
    let required = match facts.format.as_str() {
        "docx" => "word/document.xml",
        "pptx" => "ppt/presentation.xml",
        _ => "xl/workbook.xml",
    };
    let mut found = false;
    let mut total = 0usize;
    let mut names = std::collections::BTreeSet::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(invalid)?;
        let name = entry.name().to_owned();
        if !names.insert(name.clone()) {
            return Err(invalid("Office包有重复条目"));
        }
        if entry.size() > MAX_EXPANDED_BYTES as u64 {
            return Err(invalid("Office解压内容超过100 MiB"));
        }
        found |= name == required;
        if name.contains("/media/") {
            facts.images += 1;
        }
        if name.contains("/charts/chart") && name.ends_with(".xml") {
            facts.charts += 1;
        }
        let relevant = name.ends_with(".xml")
            && (name == "word/document.xml"
                || name.starts_with("ppt/slides/slide")
                || name.starts_with("xl/worksheets/sheet")
                || name == "xl/sharedStrings.xml");
        // Validate real decompressed sizes for every entry without retaining images.
        let mut data = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let n = entry.read(&mut buffer).map_err(invalid)?;
            if n == 0 {
                break;
            }
            total = total
                .checked_add(n)
                .ok_or_else(|| invalid("Office包过大"))?;
            if total > MAX_EXPANDED_BYTES {
                return Err(invalid("Office解压内容超过100 MiB"));
            }
            if relevant {
                if data.len() + n > 8 * 1024 * 1024 {
                    return Err(invalid("Office结构XML超过8 MiB"));
                }
                data.extend_from_slice(&buffer[..n]);
            }
        }
        if !relevant {
            continue;
        }
        let xml = std::str::from_utf8(&data).map_err(invalid)?;
        let doc = roxmltree::Document::parse_with_options(
            xml,
            roxmltree::ParsingOptions {
                nodes_limit: 300_000,
                ..Default::default()
            },
        )
        .map_err(invalid)?;
        let mut text = 0;
        let mut image_refs = 0;
        for node in doc.descendants().filter(|n| n.is_element()) {
            match node.tag_name().name() {
                "t" | "v" => text += node.text().unwrap_or_default().chars().count(),
                "tbl" => facts.tables += 1,
                "f" | "oMath" => facts.formulas += 1,
                "txbxContent" => facts.text_boxes += 1,
                "cols" => {
                    facts.columns = facts.columns.max(
                        node.attributes()
                            .find(|a| a.name() == "num")
                            .and_then(|a| a.value().parse().ok())
                            .unwrap_or(1),
                    );
                }
                "blip" => image_refs += 1,
                _ => {}
            }
        }
        facts.text_characters += text;
        if facts.text_characters > MAX_TEXT_BYTES {
            return Err(invalid("文档文字超过2 MiB预检上限"));
        }
        if name != "xl/sharedStrings.xml" {
            facts.units += 1;
            facts.inspected_units += 1;
            if text < 20 && image_refs > 0 {
                facts.image_only_units += 1;
            }
        }
    }
    if !found {
        return Err(invalid("Office内容与文件格式不匹配"));
    }
    Ok(())
}

fn inspect_pdf(bytes: &[u8], facts: &mut DocumentFeatures) -> Result<(), LocalDocumentError> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(invalid("PDF签名无效"));
    }
    // Inspection must cost much less than parsing. Large image/font streams
    // are left to the supplier; unknown structure chooses layout once, without
    // decoding the document locally and again upstream. Page count stays unknown.
    if bytes.len() > 4 * 1024 * 1024 {
        facts.uncertain = true;
        return Ok(());
    }
    let doc = match pdf_extract::Document::load_mem(bytes) {
        Ok(doc) => doc,
        Err(error) => {
            // Preflight estimates extraction quality; it is not the supplier's
            // PDF validator. Real PDFs may have recoverable trailer/xref damage
            // that MinerU can read. Keep the exact original bytes and choose the
            // conservative layout path instead of preventing upload altogether.
            tracing::debug!(%error, "PDF preflight could not inspect structure; supplier will validate original");
            facts.uncertain = true;
            return Ok(());
        }
    };
    if doc.is_encrypted() {
        return Err(invalid("请先解锁加密PDF"));
    }
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err(invalid("PDF没有页面"));
    }
    facts.units = pages.len();
    let sample = pages
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            pages.len() <= 16
                || *index == 0
                || *index + 1 == pages.len()
                || *index % (pages.len() / 14).max(1) == 0
        })
        .take(16);
    for (_, (number, id)) in sample {
        facts.inspected_units += 1;
        let images = doc.get_page_images(*id).map_or(0, |images| images.len());
        facts.images += images;
        let fonts_supported = doc.get_page_fonts(*id).is_ok_and(|fonts| fonts.values().all(|font| {
            // pdf-extract panics for predefined non-Identity CID encodings.
            // They are valid PDFs; route conservatively without invoking that
            // decoder (and without producing a panic log for every inspection).
            if font.get(b"Subtype").and_then(pdf_extract::Object::as_name).ok() == Some(b"Type0") {
                match font.get(b"Encoding").and_then(pdf_extract::Object::as_name) {
                    Ok(b"Identity-H" | b"Identity-V") => {},
                    _ => return false,
                }
            }
            !matches!(font.get(b"ToUnicode").and_then(pdf_extract::Object::as_name), Ok(name) if name != b"Identity-H")
        }));
        if !fonts_supported {
            facts.uncertain = true;
            continue;
        }
        let mut geometry = super::pdf_geometry::PageGeometry::default();
        let extraction = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pdf_extract::output_doc_page(&doc, &mut geometry, *number)
        }));
        if !matches!(extraction, Ok(Ok(()))) {
            facts.uncertain = true;
        }
        let characters = geometry.text.chars().filter(|c| !c.is_whitespace()).count();
        facts.columns = facts.columns.max(geometry.columns());
        facts.tables += usize::from(geometry.has_grid());
        facts.text_characters += characters;
        if characters < 20 && images > 0 {
            facts.image_only_units += 1;
        }
        if geometry.text.contains('\u{fffd}') || (characters == 0 && images == 0) {
            facts.uncertain = true;
        }
    }
    // Uninspected pages may contain a different kind of content. A bounded
    // sample is a risk estimate, never a promise about the rest of the file.
    if facts.inspected_units < facts.units {
        facts.uncertain = true;
    }
    Ok(())
}
