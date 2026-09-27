use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::{
    PromptCategory, PromptRuntimeProvider, PromptVariable, PromptVariableSourceKind,
    PromptVariableType,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinPromptDefinition {
    pub catalog_key: String,
    pub prompt_code: String,
    pub display_name: String,
    pub category: PromptCategory,
    #[serde(default = "default_builtin_prompt_version")]
    pub version: u64,
    #[serde(default)]
    pub runtime_required: bool,
    #[serde(default)]
    pub parameters: Vec<BuiltinPromptParameter>,
    pub translations: BTreeMap<String, BuiltinPromptTranslation>,
}

fn default_builtin_prompt_version() -> u64 {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinPromptParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: PromptVariableType,
    pub description: String,
    pub runtime_provider: PromptRuntimeProvider,
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

pub fn builtin_prompt_variables(catalog_key: &str) -> Option<Vec<PromptVariable>> {
    Some(
        builtin_prompt(catalog_key)?
            .parameters
            .iter()
            .map(|parameter| PromptVariable {
                name: parameter.name.clone(),
                variable_type: parameter.variable_type,
                description: parameter.description.clone(),
                source_kind: PromptVariableSourceKind::Runtime,
                managed_variable_key: None,
                runtime_provider: Some(parameter.runtime_provider),
            })
            .collect(),
    )
}

pub fn registered_runtime_prompt_variables() -> impl Iterator<Item = &'static BuiltinPromptParameter>
{
    builtin_prompts()
        .iter()
        .flat_map(|definition| definition.parameters.iter())
}

pub fn registered_runtime_prompt_variable(
    name: &str,
    provider: PromptRuntimeProvider,
) -> Option<&'static BuiltinPromptParameter> {
    registered_runtime_prompt_variables()
        .find(|parameter| parameter.name == name && parameter.runtime_provider == provider)
}
