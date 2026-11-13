//! Validate the supported static document format before installation.
use scraper::{Html, Selector};
use std::collections::BTreeMap;
fn reference<'a>(
    files: &'a BTreeMap<String, Vec<u8>>,
    from: &str,
    value: &str,
) -> Result<(String, &'a [u8]), String> {
    if value.is_empty() || value.starts_with('/') || value.contains([':', '?', '#', '%', '\\']) {
        return Err(format!("unsupported page resource: {value}"));
    }
    let base =
        reqwest::Url::parse(&format!("https://pack.invalid/{from}")).map_err(|e| e.to_string())?;
    let name = base
        .join(value)
        .map_err(|e| e.to_string())?
        .path()
        .trim_start_matches('/')
        .to_owned();
    files
        .get(&name)
        .map(|bytes| (name, bytes.as_slice()))
        .ok_or_else(|| format!("missing page resource: {value}"))
}
fn css(files: &BTreeMap<String, Vec<u8>>, from: &str, value: &str) -> Result<(), String> {
    if value.to_ascii_lowercase().contains("@import")
        || value.to_ascii_lowercase().contains("</style")
    {
        return Err("CSS imports are unsupported".into());
    }
    let urls = regex::Regex::new(r#"(?i)url\(\s*([^)]*)\)"#).map_err(|e| e.to_string())?;
    for capture in urls.captures_iter(value) {
        let value = capture[1].trim().trim_matches(['\'', '"']);
        reference(files, from, value)?;
    }
    Ok(())
}
pub(super) fn validate(files: &BTreeMap<String, Vec<u8>>, entry: &str) -> Result<(), String> {
    let source = std::str::from_utf8(files.get(entry).ok_or("missing page entry")?)
        .map_err(|e| e.to_string())?;
    let document = Html::parse_document(source);
    let mut scripts = 0;
    for element in document.select(&Selector::parse("*").map_err(|e| e.to_string())?) {
        let tag = element.value().name();
        if matches!(
            tag,
            "base"
                | "iframe"
                | "frame"
                | "object"
                | "embed"
                | "form"
                | "audio"
                | "video"
                | "source"
        ) || (tag == "meta" && element.value().attr("http-equiv").is_some())
        {
            return Err(format!("unsupported page element: {tag}"));
        }
        for (name, value) in element.value().attrs() {
            if name.starts_with("on")
                || matches!(
                    name,
                    "srcdoc" | "srcset" | "action" | "formaction" | "ping" | "background"
                )
            {
                return Err(format!("unsupported page attribute: {name}"));
            }
            if name == "style" {
                css(files, entry, value)?;
            }
        }
        if tag == "script" {
            scripts += 1;
            if element
                .value()
                .attr("src")
                .is_some_and(|src| !src.ends_with(".js"))
            {
                return Err("script entry must be JavaScript".into());
            }
            if scripts > 1 {
                return Err("only one script entry is supported".into());
            }
            let code = if let Some(src) = element.value().attr("src") {
                String::from_utf8(reference(files, entry, src)?.1.to_vec())
                    .map_err(|e| e.to_string())?
            } else {
                element.inner_html()
            };
            let imports = regex::Regex::new(r#"\bimport\s*(?:\(|[{'"*])|\bexport\s|(?i:</script)"#)
                .map_err(|e| e.to_string())?;
            if imports.is_match(&code) {
                return Err("use a single IIFE bundle without imports".into());
            }
        } else if tag == "link" {
            if element.value().attr("rel") != Some("stylesheet") {
                return Err("only stylesheet links are supported".into());
            }
            let (name, bytes) = reference(
                files,
                entry,
                element.value().attr("href").unwrap_or_default(),
            )?;
            css(
                files,
                &name,
                std::str::from_utf8(bytes).map_err(|e| e.to_string())?,
            )?;
        } else {
            if let Some(src) = element.value().attr("src") {
                reference(files, entry, src)?;
            }
            if element
                .value()
                .attr("href")
                .is_some_and(|v| !v.starts_with('#'))
            {
                return Err("only in-page links are supported".into());
            }
            if tag == "style" {
                css(files, entry, &element.inner_html())?;
            }
        }
    }
    Ok(())
}
