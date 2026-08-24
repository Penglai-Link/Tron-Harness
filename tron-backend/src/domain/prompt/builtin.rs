use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::{PromptCategory, PromptVariableType};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinPromptDefinition {
    pub catalog_key: String,
    pub prompt_code: String,
    pub display_name: String,
    pub category: PromptCategory,
    #[serde(default)]
    pub runtime_required: bool,
    #[serde(default)]
    pub parameters: Vec<BuiltinPromptParameter>,
    pub translations: BTreeMap<String, BuiltinPromptTranslation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinPromptParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: PromptVariableType,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinPromptTranslation {
    pub display_name: String,
    pub description: String,
    pub prompt_body: String,
}

static BUILTIN_PROMPTS: OnceLock<Vec<BuiltinPromptDefinition>> = OnceLock::new();

pub fn builtin_prompts() -> &'static [BuiltinPromptDefinition] {
    BUILTIN_PROMPTS
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../resources/prompts/system-prompts.json"
            ))
            .expect("bundled prompt catalog must be valid JSON")
        })
        .as_slice()
}

pub fn builtin_prompt(catalog_key: &str) -> Option<&'static BuiltinPromptDefinition> {
    builtin_prompts()
        .iter()
        .find(|prompt| prompt.catalog_key == catalog_key)
}

pub fn builtin_prompt_text(catalog_key: &str, locale: &str) -> Option<&'static str> {
    let prompt = builtin_prompt(catalog_key)?;
    prompt
        .translations
        .get(locale)
        .or_else(|| prompt.translations.get("en-US"))
        .map(|translation| translation.prompt_body.as_str())
}
