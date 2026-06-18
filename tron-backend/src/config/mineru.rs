use serde::Deserialize;

const DEFAULT_MINERU_BASE_URL: &str = "http://127.0.0.1:8000";
const DEFAULT_MINERU_PARSE_ENDPOINT: &str = "/file_parse";
const DEFAULT_MINERU_TIMEOUT_SECONDS: u64 = 300;
const DEFAULT_MINERU_MAX_BYTES: usize = 52_428_800;
const DEFAULT_MINERU_BACKEND: &str = "pipeline";
const DEFAULT_MINERU_PARSE_METHOD: &str = "auto";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MineruConfig {
    pub enabled: bool,
    pub base_url: String,
    pub parse_endpoint: String,
    pub timeout_seconds: u64,
    pub max_bytes: usize,
    pub backend: String,
    pub parse_method: String,
    pub return_content_list: bool,
}

impl Default for MineruConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: DEFAULT_MINERU_BASE_URL.into(),
            parse_endpoint: DEFAULT_MINERU_PARSE_ENDPOINT.into(),
            timeout_seconds: DEFAULT_MINERU_TIMEOUT_SECONDS,
            max_bytes: DEFAULT_MINERU_MAX_BYTES,
            backend: DEFAULT_MINERU_BACKEND.into(),
            parse_method: DEFAULT_MINERU_PARSE_METHOD.into(),
            return_content_list: false,
        }
    }
}

impl MineruConfig {
    pub(super) fn apply_overrides(&mut self, overrides: MineruFileConfig) {
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
