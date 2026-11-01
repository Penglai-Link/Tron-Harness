use super::ModelType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoRouterKind {
    Chat,
    TextGeneration,
    TextEmbedding,
    MultimodalEmbedding,
    TextRerank,
    MultimodalRerank,
}
impl AutoRouterKind {
    pub const ALL: [Self; 6] = [
        Self::Chat,
        Self::TextGeneration,
        Self::TextEmbedding,
        Self::MultimodalEmbedding,
        Self::TextRerank,
        Self::MultimodalRerank,
    ];
    pub fn code(self) -> &'static str {
        match self {
            Self::Chat => "Chat",
            Self::TextGeneration => "TextGeneration",
            Self::TextEmbedding => "TextEmbedding",
            Self::MultimodalEmbedding => "MultimodalEmbedding",
            Self::TextRerank => "TextRerank",
            Self::MultimodalRerank => "MultimodalRerank",
        }
    }
    pub fn model_type(self) -> ModelType {
        match self {
            Self::Chat | Self::TextGeneration => ModelType::Text,
            Self::TextEmbedding => ModelType::TextEmbedding,
            Self::MultimodalEmbedding => ModelType::MultimodalEmbedding,
            Self::TextRerank => ModelType::TextRerank,
            Self::MultimodalRerank => ModelType::MultimodalRerank,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoRouteSourceMode {
    #[default]
    AllProviders,
    SelectedProviders,
    SelectedModels,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutoRouteSources {
    pub mode: AutoRouteSourceMode,
    pub provider_ids: Vec<String>,
    pub model_codes: Vec<String>,
}
impl AutoRouteSources {
    pub fn allows(&self, provider: &str, model: &str) -> bool {
        match self.mode {
            AutoRouteSourceMode::AllProviders => true,
            AutoRouteSourceMode::SelectedProviders => {
                self.provider_ids.iter().any(|p| p == provider)
            }
            AutoRouteSourceMode::SelectedModels => self.model_codes.iter().any(|m| m == model),
        }
    }
    pub fn valid(&self) -> bool {
        self.provider_ids.len() <= 256
            && self.model_codes.len() <= 4096
            && self
                .provider_ids
                .iter()
                .chain(&self.model_codes)
                .all(|s| !s.trim().is_empty() && s.len() <= 512)
            && match self.mode {
                AutoRouteSourceMode::AllProviders => {
                    self.provider_ids.is_empty() && self.model_codes.is_empty()
                }
                AutoRouteSourceMode::SelectedProviders => self.model_codes.is_empty(),
                AutoRouteSourceMode::SelectedModels => self.provider_ids.is_empty(),
            }
    }
}
