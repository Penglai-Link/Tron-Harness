use serde::{Deserialize, Serialize};

const DEFAULT_MINERU_BASE_URL: &str = "http://127.0.0.1:8000";
const DEFAULT_MINERU_PARSE_ENDPOINT: &str = "/file_parse";
fn configured_default_timeout_seconds() -> u64 {
    static TIMEOUT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *TIMEOUT.get_or_init(|| {
        let config: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../../application.yaml"))
                .expect("packaged application configuration must be valid YAML");
        config["mineru"]["timeoutSeconds"]
            .as_u64()
            .filter(|value| *value > 0)
            .expect("packaged mineru.timeoutSeconds must be positive")
    })
}
const DEFAULT_MINERU_MAX_BYTES: usize = 52_428_800;
const DEFAULT_MINERU_BACKEND: &str = "pipeline";
const DEFAULT_MINERU_PARSE_METHOD: &str = "auto";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MineruConfig {
    pub nodes: Vec<MineruNode>,
    pub ocr_mode: String,
    pub parser_profile: String,
    pub expected_parser_version: String,
    pub ingestion: KnowledgeIngestionConfig,
    pub enabled: bool,
    pub base_url: String,
    pub parse_endpoint: String,
    pub timeout_seconds: u64,
    pub max_bytes: usize,
    pub backend: String,
    pub parse_method: String,
    pub return_content_list: bool,
    pub default_tier: crate::application::knowledge_base::parsing::ParsingTier,
    pub image_description: ImageDescriptionConfig,
    /// Explicit consent boundary: registered KB images may reach the answer model.
    pub visual_context_enabled: bool,
}

impl Default for MineruConfig {
    fn default() -> Self {
        Self {
            nodes: vec![],
            ocr_mode: "auto".into(),
            parser_profile: "mineru-v1".into(),
            expected_parser_version: "4.0.10".into(),
            ingestion: Default::default(),
            enabled: true,
            base_url: DEFAULT_MINERU_BASE_URL.into(),
            parse_endpoint: DEFAULT_MINERU_PARSE_ENDPOINT.into(),
            timeout_seconds: configured_default_timeout_seconds(),
            max_bytes: DEFAULT_MINERU_MAX_BYTES,
            backend: DEFAULT_MINERU_BACKEND.into(),
            parse_method: DEFAULT_MINERU_PARSE_METHOD.into(),
            return_content_list: false,
            default_tier: Default::default(),
            image_description: Default::default(),
            visual_context_enabled: false,
        }
    }
}

impl MineruConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.timeout_seconds == 0 {
            return Err("mineru.timeoutSeconds must be positive".into());
        }
        self.ingestion.validate()?;
        let mut ids = std::collections::BTreeSet::new();
        for node in &self.nodes {
            if node.id.is_empty()
                || node.id == "local-markdown"
                || !node
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
                || !ids.insert(&node.id)
            {
                return Err("MinerU 节点 ID 必须唯一，只能使用字母、数字、下划线或连字符".into());
            }
            if node.capacity == 0 || node.weight == 0 || node.resource_group_capacity == 0 {
                return Err(format!(
                    "MinerU 节点 {} 的并发、权重和共享组并发必须大于 0",
                    node.id
                ));
            }
            let url = reqwest::Url::parse(&node.base_url)
                .map_err(|_| format!("MinerU 节点 {} 的地址无效", node.id))?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(format!(
                    "MinerU 节点 {} 必须使用不含账号、查询参数或片段的 HTTP/HTTPS 地址",
                    node.id
                ));
            }
            let local = matches!(
                url.host_str(),
                Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
            );
            if node.enabled && !local && !node.allow_remote {
                return Err(format!(
                    "启用远程 MinerU 节点 {} 前，请允许向该节点传输文档",
                    node.id
                ));
            }
        }
        Ok(())
    }
    pub(super) fn apply_overrides(&mut self, overrides: MineruFileConfig) {
        if let Some(value) = overrides.nodes {
            self.nodes = value;
        }
        if let Some(value) = overrides.ingestion {
            self.ingestion = value;
        }
        if let Some(value) = non_blank(overrides.ocr_mode) {
            self.ocr_mode = value;
        }
        if let Some(value) = non_blank(overrides.parser_profile) {
            self.parser_profile = value;
        }
        if let Some(value) = non_blank(overrides.expected_parser_version) {
            self.expected_parser_version = value;
        }
        if let Some(value) = overrides.visual_context_enabled {
            self.visual_context_enabled = value;
        }
        if let Some(value) = overrides.image_description {
            self.image_description = value;
        }
        if let Some(value) = overrides.default_tier {
            self.default_tier = value;
        }
        if let Some(value) = overrides.enabled {
            self.enabled = value;
        }
        if let Some(value) = non_blank(overrides.base_url) {
            self.base_url = value;
        }
        if let Some(value) = non_blank(overrides.parse_endpoint) {
            self.parse_endpoint = value;
        }
        if let Some(value) = overrides.timeout_seconds {
            self.timeout_seconds = value;
        }
        if let Some(value) = overrides.max_bytes {
            self.max_bytes = value;
        }
        if let Some(value) = non_blank(overrides.backend) {
            self.backend = value;
        }
        if let Some(value) = non_blank(overrides.parse_method) {
            self.parse_method = value;
        }
        if let Some(value) = overrides.return_content_list {
            self.return_content_list = value;
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct MineruFileConfig {
    nodes: Option<Vec<MineruNode>>,
    ingestion: Option<KnowledgeIngestionConfig>,
    ocr_mode: Option<String>,
    parser_profile: Option<String>,
    expected_parser_version: Option<String>,
    image_description: Option<ImageDescriptionConfig>,
    visual_context_enabled: Option<bool>,
    default_tier: Option<crate::application::knowledge_base::parsing::ParsingTier>,
    enabled: Option<bool>,
    base_url: Option<String>,
    parse_endpoint: Option<String>,
    timeout_seconds: Option<u64>,
    max_bytes: Option<usize>,
    backend: Option<String>,
    parse_method: Option<String>,
    return_content_list: Option<bool>,
}

fn non_blank(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Optional host-side description supplement. Enabling this sends derived images to the selected model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageDescriptionConfig {
    pub enabled: bool,
    pub model_code: String,
    pub max_images_per_document: usize,
    pub timeout_seconds: u64,
}
impl Default for ImageDescriptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_code: String::new(),
            max_images_per_document: 256,
            timeout_seconds: 120,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct MineruNode {
    pub id: String,
    pub base_url: String,
    pub enabled: bool,
    pub allow_remote: bool,
    pub capacity: usize,
    pub weight: u32,
    pub resource_group: String,
    pub resource_group_capacity: usize,
}
impl Default for MineruNode {
    fn default() -> Self {
        Self {
            id: String::new(),
            base_url: String::new(),
            enabled: false,
            allow_remote: false,
            capacity: 1,
            weight: 1,
            resource_group: String::new(),
            resource_group_capacity: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeIngestionConfig {
    pub parse_capacity: usize,
    pub embedding_capacity: usize,
    pub embedding_resource_group: Option<String>,
    pub embedding_resource_group_capacity: usize,
    pub lease_ms: i64,
    pub poll_ms: u64,
    pub max_attempts: u32,
    pub retry_backoff_ms: i64,
}
impl Default for KnowledgeIngestionConfig {
    fn default() -> Self {
        Self {
            parse_capacity: 2,
            embedding_capacity: 2,
            embedding_resource_group: None,
            embedding_resource_group_capacity: 2,
            lease_ms: 30_000,
            poll_ms: 250,
            max_attempts: 3,
            retry_backoff_ms: 2_000,
        }
    }
}
impl KnowledgeIngestionConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=32).contains(&self.parse_capacity)
            || !(1..=32).contains(&self.embedding_capacity)
            || self.lease_ms < 300
            || self.poll_ms == 0
            || self.max_attempts == 0
            || self.retry_backoff_ms < 0
        {
            return Err("知识库并发、租约或重试配置无效".into());
        }
        if self.embedding_resource_group.as_ref().is_some_and(|g| {
            g.trim().is_empty() || !(1..=32).contains(&self.embedding_resource_group_capacity)
        }) {
            return Err("Embedding共享硬件组配置无效".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod image_description_tests {
    use super::*;
    #[test]
    fn parse_timeout_comes_from_configuration_and_explicit_overrides_win() {
        let packaged: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../../application.yaml")).unwrap();
        let mut config = MineruConfig::default();
        assert_eq!(
            config.timeout_seconds,
            packaged["mineru"]["timeoutSeconds"].as_u64().unwrap()
        );
        config.apply_overrides(
            serde_yaml::from_str::<MineruFileConfig>("timeoutSeconds: 7").unwrap(),
        );
        assert_eq!(config.timeout_seconds, 7);
        config.validate().unwrap();
        config.timeout_seconds = 0;
        assert!(config.validate().is_err());
    }
    #[test]
    fn default_is_off_and_partial_yaml_keeps_safe_defaults() {
        let config: ImageDescriptionConfig =
            serde_yaml::from_str("modelCode: PERSON_AI:FIXED:High:synthetic-vision").unwrap();
        assert!(!config.enabled);
        assert_eq!(config.timeout_seconds, 120);
        assert_eq!(config.max_images_per_document, 256);
        assert!(!MineruConfig::default().image_description.enabled);
        assert!(!MineruConfig::default().visual_context_enabled);
        assert_eq!(MineruConfig::default().default_tier.as_str(), "standard");
        assert!(serde_yaml::from_str::<ImageDescriptionConfig>("unknown: true").is_err());
    }
    #[test]
    fn ingestion_pool_partial_configuration_and_capacity_limits() {
        let config: KnowledgeIngestionConfig = serde_yaml::from_str("parseCapacity: 3\nembeddingCapacity: 1\nembeddingResourceGroup: gpu-a\nembeddingResourceGroupCapacity: 2").unwrap();
        config.validate().unwrap();
        assert_eq!(config.lease_ms, 30_000);
        assert_eq!(config.embedding_resource_group.as_deref(), Some("gpu-a"));
        let mut invalid = config.clone();
        invalid.parse_capacity = 0;
        assert!(invalid.validate().is_err());
        invalid = config.clone();
        invalid.embedding_resource_group_capacity = 0;
        assert!(invalid.validate().is_err());
        assert!(serde_yaml::from_str::<KnowledgeIngestionConfig>("parseCapcity: 2").is_err());
        let node: MineruNode =
            serde_yaml::from_str("id: candidate\nbaseUrl: http://192.0.2.1:8000").unwrap();
        assert!(!node.enabled);
        assert!(!node.allow_remote);
        let config: MineruFileConfig=serde_yaml::from_str("nodes:\n  - id: cpu\n    baseUrl: http://127.0.0.1:8100\n    enabled: true\nocrMode: ocr\nexpectedParserVersion: 4.0.10\ningestion:\n  parseCapacity: 4").unwrap();
        let mut runtime = MineruConfig::default();
        runtime.apply_overrides(config);
        assert_eq!(runtime.nodes[0].id, "cpu");
        assert_eq!(runtime.ingestion.parse_capacity, 4);
        assert_eq!(runtime.ocr_mode, "ocr");
    }
}
