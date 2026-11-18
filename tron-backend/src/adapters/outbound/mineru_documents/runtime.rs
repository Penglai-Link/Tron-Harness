use std::sync::{Arc, Mutex};

use crate::application::{
    knowledge_base::{
        ingestion::{IngestionTask, ParseEndpointCapacity},
        parsing::{ParseCheckpoint, ParserRoutingStatus},
    },
    system_config::BackendRuntimeConfig,
};

use super::*;

/// Reuses a client until settings change; each operation owns a fixed parser.
pub struct RuntimeMineruDocumentParser {
    runtime: BackendRuntimeConfig,
    cached: Mutex<MineruKnowledgeDocumentParser>,
}

impl RuntimeMineruDocumentParser {
    pub fn new(runtime: BackendRuntimeConfig) -> Self {
        Self {
            cached: Mutex::new(MineruKnowledgeDocumentParser::new(
                runtime.snapshot().mineru,
            )),
            runtime,
        }
    }

    fn current(&self) -> MineruKnowledgeDocumentParser {
        let config = self.runtime.snapshot().mineru;
        let mut cached = self
            .cached
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if cached.config != config {
            *cached = MineruKnowledgeDocumentParser::new(config);
        }
        cached.clone()
    }

    fn for_task(
        &self,
        identity: &Value,
    ) -> Result<MineruKnowledgeDocumentParser, KnowledgeBaseError> {
        let config = if let Some(config) = identity.get("runtimeConfig") {
            serde_json::from_value(config.clone()).map_err(super::invalid_detail)?
        } else {
            // Tasks created before hot updates stored the transport identity only.
            let mut config = self.runtime.snapshot().mineru;
            for (key, field) in [
                ("baseUrl", &mut config.base_url),
                ("ocrMode", &mut config.ocr_mode),
                ("parserProfile", &mut config.parser_profile),
                ("expectedParserVersion", &mut config.expected_parser_version),
            ] {
                if let Some(value) = identity[key].as_str() {
                    *field = value.into();
                }
            }
            if let Some(nodes) = identity.get("nodes") {
                config.nodes =
                    serde_json::from_value(nodes.clone()).map_err(super::invalid_detail)?;
            }
            config.enabled = true;
            config
        };
        config.validate().map_err(super::invalid_detail)?;
        Ok(MineruKnowledgeDocumentParser::new(config))
    }
}

#[async_trait]
impl KnowledgeDocumentParser for RuntimeMineruDocumentParser {
    fn with_ocr_mode(
        &self,
        mode: OcrMode,
    ) -> Result<Arc<dyn KnowledgeDocumentParser>, KnowledgeBaseError> {
        self.current().with_ocr_mode(mode)
    }
    fn snapshot(&self) -> Option<Arc<dyn KnowledgeDocumentParser>> {
        Some(Arc::new(self.current()))
    }
    fn task_snapshot(
        &self,
        identity: &Value,
    ) -> Result<Option<Arc<dyn KnowledgeDocumentParser>>, KnowledgeBaseError> {
        Ok(Some(Arc::new(self.for_task(identity)?)))
    }
    fn default_tier(&self) -> ParsingTier {
        self.current().default_tier()
    }
    fn profile_identity(&self) -> Value {
        self.current().profile_identity()
    }
    fn endpoint_ids(&self) -> Vec<String> {
        self.current().endpoint_ids()
    }
    fn supports_local_visual_reparse(&self) -> bool {
        self.current().supports_local_visual_reparse()
    }
    async fn healthy_endpoints(&self) -> Result<Vec<ParseEndpointCapacity>, KnowledgeBaseError> {
        self.current().healthy_endpoints().await
    }
    async fn routing_status(&self) -> ParserRoutingStatus {
        self.current().routing_status().await
    }
    async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError> {
        self.current().capabilities().await
    }
    async fn parse(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        self.current().parse(filename, bytes, tier).await
    }
    async fn parse_durable(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
        checkpoint: Arc<dyn ParseCheckpoint>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        let task = checkpoint.task()?;
        self.for_task(&task.configuration["parser"])?
            .parse_durable(filename, bytes, tier, checkpoint)
            .await
    }
    async fn cancel_job(&self, endpoint: &str, job: &str) -> Result<(), KnowledgeBaseError> {
        self.current().cancel_job(endpoint, job).await
    }
    async fn cancel_task(
        &self,
        task: &IngestionTask,
    ) -> Result<Option<String>, KnowledgeBaseError> {
        self.for_task(&task.configuration["parser"])?
            .cancel_task(task)
            .await
    }
}
