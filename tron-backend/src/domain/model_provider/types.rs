use serde::{Deserialize, Serialize};

use crate::domain::{agent::AgentModelProtocol, model::ModelType};

/// A platform integration that owns a remote model catalogue and its credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ModelProviderCode {
    PersonAi,
}

impl ModelProviderCode {
    pub const fn storage_value(self) -> &'static str {
        match self {
            Self::PersonAi => "PersonAi",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "PersonAi" => Some(Self::PersonAi),
            _ => None,
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::PersonAi => "Person-AI",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProviderConnectionStatus {
    Disconnected,
    PendingAuthorization,
    Connected,
    Degraded,
    AuthRequired,
}

impl ProviderConnectionStatus {
    pub const fn storage_value(self) -> &'static str {
        match self {
            Self::Disconnected => "Disconnected",
            Self::PendingAuthorization => "PendingAuthorization",
            Self::Connected => "Connected",
            Self::Degraded => "Degraded",
            Self::AuthRequired => "AuthRequired",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "Disconnected" => Some(Self::Disconnected),
            "PendingAuthorization" => Some(Self::PendingAuthorization),
            "Connected" => Some(Self::Connected),
            "Degraded" => Some(Self::Degraded),
            "AuthRequired" => Some(Self::AuthRequired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProviderCatalogModelStatus {
    Available,
    Unavailable,
}

/// A remote catalogue entry is either a chat model that can be selected for a
/// conversation, or one of Person-AI's multimedia capabilities. Multimedia
/// entries are deliberately kept out of the chat model selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProviderCatalogModelType {
    Chat,
    Embedding,
    Image,
    Video,
    Speech,
    AigcTextDetection,
    AigcImageDetection,
}

impl ProviderCatalogModelType {
    pub const fn from_person_ai_media_type(media_type: i32) -> Option<Self> {
        match media_type {
            0 => Some(Self::Image),
            1 => Some(Self::Video),
            2 => Some(Self::Speech),
            3 => Some(Self::AigcTextDetection),
            4 => Some(Self::AigcImageDetection),
            5 => Some(Self::Embedding),
            _ => None,
        }
    }

    pub const fn person_ai_media_type(self) -> Option<i32> {
        match self {
            Self::Chat => None,
            Self::Embedding => Some(5),
            Self::Image => Some(0),
            Self::Video => Some(1),
            Self::Speech => Some(2),
            Self::AigcTextDetection => Some(3),
            Self::AigcImageDetection => Some(4),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProviderModelQuality {
    High,
    Middle,
    Low,
}

impl ProviderModelQuality {
    pub const fn storage_value(self) -> &'static str {
        match self {
            Self::High => "High",
            Self::Middle => "Middle",
            Self::Low => "Low",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "High" => Some(Self::High),
            "Middle" => Some(Self::Middle),
            "Low" => Some(Self::Low),
            _ => None,
        }
    }
}

impl ProviderCatalogModelStatus {
    pub const fn storage_value(self) -> &'static str {
        match self {
            Self::Available => "Available",
            Self::Unavailable => "Unavailable",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "Available" => Some(Self::Available),
            "Unavailable" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProviderSyncStatus {
    Running,
    Succeeded,
    Failed,
}

impl ProviderSyncStatus {
    pub const fn storage_value(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Succeeded => "Succeeded",
            Self::Failed => "Failed",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "Running" => Some(Self::Running),
            "Succeeded" => Some(Self::Succeeded),
            "Failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// A selection is stable even when a provider catalogue entry is renamed later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ModelSelectionKind {
    Fixed,
    ProviderRoute,
    Auto,
}

impl ModelSelectionKind {
    pub const fn storage_value(&self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::ProviderRoute => "ProviderRoute",
            Self::Auto => "Auto",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "Fixed" => Some(Self::Fixed),
            "ProviderRoute" => Some(Self::ProviderRoute),
            "Auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProviderRouteCode {
    ChatAuto,
    EmbeddingAuto,
    LlmFallback,
}

impl ProviderRouteCode {
    pub const fn storage_value(self) -> &'static str {
        match self {
            Self::ChatAuto => "ChatAuto",
            Self::EmbeddingAuto => "EmbeddingAuto",
            Self::LlmFallback => "LlmFallback",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "ChatAuto" => Some(Self::ChatAuto),
            "EmbeddingAuto" => Some(Self::EmbeddingAuto),
            "LlmFallback" => Some(Self::LlmFallback),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelection {
    pub kind: ModelSelectionKind,
    pub provider_code: Option<ModelProviderCode>,
    pub model_code: Option<String>,
    pub quality_type: Option<ProviderModelQuality>,
    pub route_code: Option<ProviderRouteCode>,
}

impl ModelSelection {
    pub fn auto() -> Self {
        Self {
            kind: ModelSelectionKind::Auto,
            provider_code: None,
            model_code: None,
            quality_type: None,
            route_code: None,
        }
    }

    pub fn fixed(model_code: impl Into<String>) -> Self {
        Self {
            kind: ModelSelectionKind::Fixed,
            provider_code: None,
            model_code: Some(model_code.into()),
            quality_type: None,
            route_code: None,
        }
    }

    pub fn person_ai_fixed(
        model_code: impl Into<String>,
        quality_type: ProviderModelQuality,
    ) -> Self {
        Self {
            kind: ModelSelectionKind::Fixed,
            provider_code: Some(ModelProviderCode::PersonAi),
            model_code: Some(model_code.into()),
            quality_type: Some(quality_type),
            route_code: None,
        }
    }

    pub fn person_ai_route(route_code: ProviderRouteCode) -> Self {
        Self {
            kind: ModelSelectionKind::ProviderRoute,
            provider_code: Some(ModelProviderCode::PersonAi),
            model_code: None,
            quality_type: None,
            route_code: Some(route_code),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnectionView {
    pub provider_code: ModelProviderCode,
    pub person_ai_user_id: Option<String>,
    pub person_ai_username: Option<String>,
    pub display_name: String,
    pub status: ProviderConnectionStatus,
    pub server_base_url: String,
    pub authorized_at: Option<i64>,
    pub authorization_expires_at: Option<i64>,
    pub last_sync_at: Option<i64>,
    pub last_error: Option<String>,
    pub catalog_model_count: usize,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogModelView {
    pub provider_code: ModelProviderCode,
    pub catalog_model_type: ProviderCatalogModelType,
    pub model_id: Option<String>,
    pub model_code: String,
    pub model_name: String,
    pub protocols: Vec<AgentModelProtocol>,
    pub upstream_vendor_code: Option<String>,
    pub upstream_vendor_name: Option<String>,
    pub context_window_tokens: Option<u32>,
    pub quality_type: Option<ProviderModelQuality>,
    pub input_coefficient: Option<f64>,
    pub cache_hit_coefficient: Option<f64>,
    pub output_coefficient: Option<f64>,
    pub status: ProviderCatalogModelStatus,
    pub first_seen_at: i64,
    pub last_seen_at: Option<i64>,
    pub updated_at: i64,
}

impl ProviderCatalogModelView {
    /// Person-AI accepts a vendor-qualified model code to bind a fixed selection
    /// to the upstream vendor. Older catalogues without a vendor keep bare codes.
    pub fn selection_model_code(&self) -> String {
        match self
            .upstream_vendor_code
            .as_deref()
            .filter(|code| !code.is_empty())
        {
            Some(vendor) => format!("{vendor}/{}", self.model_code),
            None => self.model_code.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSyncRunView {
    pub sync_run_code: String,
    pub provider_code: ModelProviderCode,
    pub status: ProviderSyncStatus,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub model_count: Option<usize>,
    pub error_summary: Option<String>,
}

/// Immutable model lineage attached to an agent run. The selected policy is
/// retained separately from Person-AI's actual route result, because an Auto
/// route can resolve to a different remote model over time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelRunSnapshotView {
    pub run_code: String,
    pub session_code: Option<String>,
    pub workspace_code: String,
    pub model_selection: ModelSelection,
    pub resolved_model_code: Option<String>,
    pub resolved_model_name: Option<String>,
    pub resolved_upstream_vendor_code: Option<String>,
    pub resolved_upstream_vendor_name: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelSelectOption {
    pub selection: ModelSelection,
    pub option_code: String,
    pub display_name: String,
    pub provider_code: ModelProviderCode,
    pub access_platform: String,
    pub upstream_vendor_name: Option<String>,
    pub model_type: ModelType,
    pub protocols: Vec<AgentModelProtocol>,
    pub enabled: bool,
    pub remote_status: ProviderCatalogModelStatus,
    pub context_window_tokens: Option<u32>,
    pub quality_type: Option<ProviderModelQuality>,
    pub input_coefficient: Option<f64>,
    pub cache_hit_coefficient: Option<f64>,
    pub output_coefficient: Option<f64>,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAccountBalanceView {
    pub username: Option<String>,
    pub vip_level: Option<i32>,
    pub remaining_points: Option<i64>,
}
