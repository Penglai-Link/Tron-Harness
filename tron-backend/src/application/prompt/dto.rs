use serde::Serialize;

use crate::domain::prompt::{
    PromptCategory, PromptRuntimeProvider, PromptTranslationStatus, PromptVariableSourceKind,
    PromptVariableType, PromptView, PromptViewSourceKind,
};

#[derive(Debug, Clone)]
pub struct PromptVariableSpecCommand {
    pub name: String,
    pub variable_type: PromptVariableType,
    pub description: String,
    pub source_kind: PromptVariableSourceKind,
    pub managed_variable_key: Option<String>,
    pub runtime_provider: Option<PromptRuntimeProvider>,
}

#[derive(Debug, Clone)]
pub struct PromptSpecCommand {
    pub display_name: String,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub variables: Vec<PromptVariableSpecCommand>,
    pub translations: Vec<PromptTranslationSpecCommand>,
}

#[derive(Debug, Clone)]
pub struct PromptTranslationSpecCommand {
    pub locale: String,
    pub display_name: String,
    pub prompt_body: String,
    pub description: Option<String>,
    pub status: PromptTranslationStatus,
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
pub struct PromptAnalyzeResult {
    pub variables: Vec<PromptAnalyzedVariable>,
    pub invalid_variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAnalyzedVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: PromptVariableType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptExportResult {
    pub file_name: String,
    pub content: String,
}
