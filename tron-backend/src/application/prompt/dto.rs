use serde::Serialize;

use crate::domain::prompt::{PromptCategory, PromptView, PromptViewSourceKind};

#[derive(Debug, Clone)]
pub struct PromptSpecCommand {
    pub display_name: String,
    pub prompt_body: String,
    pub description: Option<String>,
    pub model_code: Option<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct CreatePromptCommand {
    pub category: PromptCategory,
    pub spec: PromptSpecCommand,
    pub allow_inheritance: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PromptPageQuery {
    pub current: usize,
    pub size: usize,
    pub search: Option<String>,
    pub category: Option<PromptCategory>,
    pub source_kind: Option<PromptViewSourceKind>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPage {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<PromptView>,
}

#[derive(Debug, Clone)]
pub struct UpdatePromptCommand {
    pub allow_inheritance: Option<bool>,
    pub prompt_code: String,
    pub expected_version: u64,
    pub spec: PromptSpecCommand,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDeleteResult {
    pub workspace_code: Option<String>,
    pub prompt_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestPreviewResult {
    pub rendered_prompt: String,
    pub model_code: Option<String>,
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAnalyzeResult {
    pub variables: Vec<crate::domain::prompt::PromptVariable>,
    pub invalid_variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptExportResult {
    pub file_name: String,
    pub content: String,
}
