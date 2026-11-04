//! Tron-owned, local-only document inspection and parsing decisions.
use serde::{Deserialize, Serialize};
mod inspection;
mod pdf_geometry;
pub use inspection::inspect_document;
/// MinerU 4's documented input contract. Plain text is read directly, as in MinerU read.
pub const PARSE_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "jp2", "doc", "docx", "ppt", "pptx",
    "xls", "xlsx", "rtf", "odt", "ods", "odp", "epub", "ofd", "html", "htm", "shtml", "csv", "tsv",
];
pub const DIRECT_READ_EXTENSIONS: &[&str] = &["txt", "md", "markdown", "rst", "tex"];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentParsePreference {
    #[default]
    Auto,
    Native,
    Recognition,
    Layout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentExtractionMethod {
    Native,
    Recognition,
    Layout,
}

/// Facts supplied by a format adapter. No provider modes or model names belong here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFeatures {
    pub format: String,
    pub source_sha256: String,
    pub units: usize,
    pub inspected_units: usize,
    pub text_characters: usize,
    pub image_only_units: usize,
    pub images: usize,
    pub tables: usize,
    pub formulas: usize,
    pub charts: usize,
    pub text_boxes: usize,
    pub columns: usize,
    pub uncertain: bool,
    pub local_text: bool,
}

/// Persist before any supplier task is created; retries must retain this decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentParsePlan {
    pub preference: DocumentParsePreference,
    pub method: DocumentExtractionMethod,
    pub reason: String,
    pub rules_version: String,
    pub features: DocumentFeatures,
}

pub trait DocumentParsingRouter: Send + Sync {
    fn plan(
        &self,
        features: DocumentFeatures,
        preference: DocumentParsePreference,
    ) -> DocumentParsePlan;
}

/// Deterministic interpretation of host-supplied facts, independent of suppliers.
pub struct FeatureBasedDocumentRouter;
impl DocumentParsingRouter for FeatureBasedDocumentRouter {
    fn plan(
        &self,
        features: DocumentFeatures,
        preference: DocumentParsePreference,
    ) -> DocumentParsePlan {
        use DocumentExtractionMethod::*;
        let (method, reason) = if features.local_text {
            (Native, "local_text")
        } else if matches!(features.format.as_str(), "doc" | "ppt" | "xls") {
            (Recognition, "legacy_office")
        } else {
            match preference {
                DocumentParsePreference::Native => (Native, "user_choice"),
                DocumentParsePreference::Recognition => (Recognition, "user_choice"),
                DocumentParsePreference::Layout => (Layout, "user_choice"),
                // MinerU normalizes native structured formats to its native
                // path, independently of PDF/image quality. Avoid upselling
                // a mode that cannot improve native Office extraction.
                DocumentParsePreference::Auto
                    if !matches!(
                        features.format.as_str(),
                        "pdf" | "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "jp2"
                    ) =>
                {
                    (Native, "native_format")
                }
                DocumentParsePreference::Auto if features.uncertain => {
                    (Layout, "uncertain_structure")
                }
                DocumentParsePreference::Auto
                    if features.image_only_units > 0 && features.text_characters < 40 =>
                {
                    (Recognition, "image_only")
                }
                DocumentParsePreference::Auto if features.image_only_units > 0 => {
                    (Layout, "mixed_content")
                }
                DocumentParsePreference::Auto
                    if features.charts > 0
                        || features.columns > 1
                        || features.text_boxes > 0
                        || (features.format == "pdf" && features.tables > 0)
                        || (features.format != "xlsx" && features.formulas > 0) =>
                {
                    (Layout, "complex_layout")
                }
                DocumentParsePreference::Auto => (Native, "native_content"),
            }
        };
        DocumentParsePlan {
            preference,
            method,
            reason: reason.into(),
            rules_version: "document-features-v2".into(),
            features,
        }
    }
}
