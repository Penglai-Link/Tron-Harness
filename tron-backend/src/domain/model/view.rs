use super::{ModelProtocol, ModelType, ModelViewSourceKind};
use crate::domain::model_provider::{
    ModelProviderCode, ModelSelection, ProviderCatalogModelStatus,
};
use crate::domain::reasoning::ReasoningEffort;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelectOption {
    pub option_code: String,
    pub model_code: String,
    pub display_name: String,
    pub model_name: String,
    pub source_kind: ModelViewSourceKind,
    pub source_label: String,
    pub vendor: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub enabled: bool,
    pub model_selection: ModelSelection,
    pub provider_code: Option<ModelProviderCode>,
    pub access_platform: Option<String>,
    pub upstream_vendor_name: Option<String>,
    pub remote_status: Option<ProviderCatalogModelStatus>,
    pub context_window_tokens: Option<u32>,
    pub quality_type: Option<crate::domain::model_provider::ProviderModelQuality>,
    pub input_coefficient: Option<f64>,
    pub cache_hit_coefficient: Option<f64>,
    pub output_coefficient: Option<f64>,
    pub supported_reasoning_efforts: Vec<ReasoningEffort>,
    pub default_reasoning_effort: Option<ReasoningEffort>,
    pub is_default: bool,
}
