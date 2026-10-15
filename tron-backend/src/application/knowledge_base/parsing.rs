//! Document parsing contract; parsing transports remain outbound adapters.
mod normalization;
use super::{KnowledgeBaseError, local_documents::ParsedDocument};
use async_trait::async_trait;
pub(crate) use normalization::normalize_middle;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParsingTier {
    Flash,
    Basic,
    #[default]
    Standard,
    Advanced,
}
impl ParsingTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flash => "flash",
            Self::Basic => "basic",
            Self::Standard => "standard",
            Self::Advanced => "advanced",
        }
    }
}
/// Document-level PDF text recognition; wire names belong to the transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrMode {
    Auto,
    Text,
    Ocr,
}
impl OcrMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Text => "txt",
            Self::Ocr => "ocr",
        }
    }
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "txt" => Some(Self::Text),
            "ocr" => Some(Self::Ocr),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsingCapabilities {
    pub default_tier: ParsingTier,
    pub default_ocr_mode: OcrMode,
    pub available_tiers: Vec<ParsingTier>,
    pub parser_version: String,
    pub office_tier: ParsingTier,
}
#[derive(Debug, Default)]
pub struct ParserRoutingStatus {
    pub endpoints: Vec<super::ingestion::ParseEndpointCapacity>,
    pub unavailable: Vec<(Option<String>, super::ingestion::IngestionWaitReason)>,
}
#[derive(Debug, Clone)]
pub struct ParsedAsset {
    pub path: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone)]
pub struct StructuredParse {
    pub document: ParsedDocument,
    pub middle: Value,
    pub structured: Value,
    pub markdown: String,
    pub assets: Vec<ParsedAsset>,
    pub effective_tier: ParsingTier,
    pub parser_version: String,
    pub vendor_job_id: String,
}
#[async_trait]
pub trait KnowledgeDocumentParser: Send + Sync {
    /// Pin a consistent configuration for a multi-step operation.
    fn snapshot(&self) -> Option<std::sync::Arc<dyn KnowledgeDocumentParser>> {
        None
    }
    /// Durable jobs retain the configuration authorized at enqueue time.
    fn task_snapshot(
        &self,
        _identity: &Value,
    ) -> Result<Option<std::sync::Arc<dyn KnowledgeDocumentParser>>, KnowledgeBaseError> {
        Ok(None)
    }
    fn default_tier(&self) -> ParsingTier;
    fn with_ocr_mode(
        &self,
        _mode: OcrMode,
    ) -> Result<std::sync::Arc<dyn KnowledgeDocumentParser>, KnowledgeBaseError> {
        Err(KnowledgeBaseError::Invalid(
            "解析服务不支持文档文字识别方式覆盖".into(),
        ))
    }
    fn profile_identity(&self) -> Value {
        serde_json::json!({})
    }
    fn endpoint_ids(&self) -> Vec<String> {
        vec!["default".into()]
    }
    async fn healthy_endpoints(
        &self,
    ) -> Result<Vec<super::ingestion::ParseEndpointCapacity>, KnowledgeBaseError> {
        Ok(vec![super::ingestion::ParseEndpointCapacity {
            supported_tiers: vec![],
            endpoint_id: "default".into(),
            capacity: 32,
            resource_group: "default-parser".into(),
            resource_group_capacity: 32,
            weight: 1,
        }])
    }
    async fn routing_status(&self) -> ParserRoutingStatus {
        match self.healthy_endpoints().await {
            Ok(endpoints) => ParserRoutingStatus {
                endpoints,
                unavailable: vec![],
            },
            Err(_) => ParserRoutingStatus {
                endpoints: vec![],
                unavailable: vec![(
                    None,
                    super::ingestion::IngestionWaitReason::new(
                        super::ingestion::WaitReasonCode::NodeUnreachable,
                    ),
                )],
            },
        }
    }
    async fn parse_durable(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
        _checkpoint: std::sync::Arc<dyn ParseCheckpoint>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        self.parse(filename, bytes, tier).await
    }
    async fn cancel_job(&self, _endpoint: &str, _job: &str) -> Result<(), KnowledgeBaseError> {
        Ok(())
    }
    async fn cancel_task(
        &self,
        task: &super::ingestion::IngestionTask,
    ) -> Result<Option<String>, KnowledgeBaseError> {
        if let (Some(endpoint), Some(job)) = (&task.endpoint_id, &task.remote_job_id) {
            self.cancel_job(endpoint, job).await?;
            return Ok(Some(job.clone()));
        }
        if task.reconciling {
            return Err(KnowledgeBaseError::Storage(
                "原解析任务仍在对账，取消尚未确认".into(),
            ));
        }
        Ok(None)
    }
    /// Reprocessing must remain on the explicitly configured loopback parser.
    fn supports_local_visual_reparse(&self) -> bool {
        false
    }
    async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError>;
    async fn parse(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError>;
    async fn parse_for_user(
        &self,
        _user: &str,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        self.parse(filename, bytes, tier).await
    }
}

/// The transport persists the upstream identity before polling or issuing a job POST.
pub trait ParseCheckpoint: Send + Sync {
    fn chunking(&self) -> Result<(), KnowledgeBaseError> {
        Ok(())
    }
    fn task(&self) -> Result<super::ingestion::IngestionTask, KnowledgeBaseError>;
    fn save(
        &self,
        file_id: Option<String>,
        job_id: Option<String>,
        reconciling: bool,
    ) -> Result<(), KnowledgeBaseError>;
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsingArtifact {
    pub signature: String,
    pub resource_code: String,
    pub effective_tier: ParsingTier,
    pub parser_version: String,
    pub vendor_job_id: String,
    pub image_resource_codes: Vec<String>,
    /// Current Office-rendered preview, retained with this parse generation.
    #[serde(default)]
    pub preview_resource_code: Option<String>,
    #[serde(default)]
    pub fidelity_checked: bool,
    #[serde(default)]
    pub fidelity_resolution: Option<super::fidelity::FidelityResolution>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fidelity_issues: Vec<super::fidelity::FidelityIssue>,
}

/// Convert only host-validated Office bytes locally; no model or third-party parser.
#[async_trait]
pub trait KnowledgeOfficeRenderer: Send + Sync {
    async fn render_pdf(&self, filename: &str, bytes: &[u8])
    -> Result<Vec<u8>, KnowledgeBaseError>;
}
