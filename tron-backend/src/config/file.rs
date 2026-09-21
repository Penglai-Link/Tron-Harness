use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use super::{
    AppConfig, ConfigError, HttpClientProfileConfig, HttpClientProfilesConfig,
    SystemAgentConfig, mineru::MineruFileConfig,
    paths::default_application_config_path_from_home, paths::normalize_storage_path,
    resource::ResourceFileConfig, variable_crypto,
};

#[path = "file_event_bus.rs"]
mod file_event_bus;
#[path = "file_short_term_memory.rs"]
mod file_short_term_memory;

use file_event_bus::EventBusFileConfig;
use file_short_term_memory::ShortTermMemoryFileConfig;

impl AppConfig {
    pub fn from_application_yaml() -> Result<Self, ConfigError> {
        let path = env::var("TRON_BACKEND_APPLICATION_CONFIG_PATH")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(default_application_config_path_from_home()));
        Self::from_yaml_file(path)
    }

    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let file_config = AppConfigFile::from_file(path)?;
        let mut config = Self::runtime_defaults();
        config.application_config_path = path.display().to_string();
        config.apply_file_overrides(file_config)?;
        config.apply_env_overrides()?;
        config.ensure_profile_crypto_key(path)?;
        Ok(config)
    }

    pub fn validate_yaml(content: &str, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let file_config = AppConfigFile::from_content(content, path)?;
        let mut config = Self::runtime_defaults();
        config.application_config_path = path.display().to_string();
        config.apply_file_overrides(file_config)?;
        config.apply_env_overrides()?;
        config.socket_addr()?;
        Ok(())
    }

    fn apply_file_overrides(&mut self, overrides: AppConfigFile) -> Result<(), ConfigError> {
        if let Some(value) = overrides.bind_address {
            self.bind_address = value;
        }
        if let Some(value) = overrides.port {
            self.port = value;
        }
        if let Some(value) = overrides.database_url {
            self.database_url = normalize_storage_path(value);
        }
        if let Some(value) = overrides.graph_storage_path {
            self.graph_storage_path = normalize_storage_path(value);
        }
        if let Some(value) = overrides.frontend_dist_dir {
            self.frontend_dist_dir = normalize_storage_path(value);
        }
        if let Some(value) = overrides.project_doc_root {
            self.project_doc_root = normalize_storage_path(value);
        }
        if let Some(value) = overrides.terminal_working_directory {
            self.terminal_working_directory = normalize_storage_path(value);
        }
        if let Some(value) = overrides.skills_registry_root {
            self.skills_registry_root = value;
        }
        if let Some(value) = overrides.resource {
            self.resource.apply_overrides(value)?;
        }
        if let Some(value) = overrides.system_agents {
            self.system_agents.apply_overrides(value);
        }
        if let Some(value) = overrides.variable_crypto {
            self.variable_crypto.apply_overrides(value);
        }
        if let Some(value) = overrides.http_clients {
            self.http_clients.apply_overrides(value);
        }
        if let Some(value) = overrides.mineru {
            self.mineru.apply_overrides(value);
        }
        if let Some(value) = overrides.short_term_memory {
            self.short_term_memory.apply_overrides(value);
        }
        if let Some(value) = overrides.event_bus {
            self.event_bus.apply_overrides(value);
        }
        if let Some(value) = overrides.navigation
            && let Some(max_depth) = value.max_depth
        {
            self.navigation.max_depth = max_depth;
        }
        if self.navigation.max_depth == 0 {
            return Err(ConfigError::InvalidConfigValue {
                key: "navigation.maxDepth".into(),
                value: "must be greater than zero".into(),
            });
        }
        Ok(())
    }

    /// Profile credentials are encrypted with a key owned by the backend.
    /// The desktop bootstrap deliberately starts a new profile with an empty
    /// key; provision it here and persist it next to that profile's database.
    /// No client-supplied key is accepted by the authorization API.
    fn ensure_profile_crypto_key(&mut self, path: &Path) -> Result<(), ConfigError> {
        if !self.variable_crypto.key_base64.trim().is_empty() {
            return Ok(());
        }

        let mut key = [0_u8; 16];
        rand::rng().fill_bytes(&mut key);
        let key_base64 = BASE64.encode(key);
        let content = fs::read_to_string(path).map_err(|source| ConfigError::ConfigFileReadFailed {
            path: path.display().to_string(),
            source,
        })?;
        let next_content = if content
            .lines()
            .any(|line| line.trim_start().starts_with("keyBase64:"))
        {
            content
                .lines()
                .map(|line| {
                    if line.trim_start().starts_with("keyBase64:") {
                        let indent = &line[..line.len() - line.trim_start().len()];
                        format!("{indent}keyBase64: {key_base64}")
                    } else {
                        line.to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        } else {
            format!("{content}\nvariableCrypto:\n  keyBase64: {key_base64}\n")
        };
        fs::write(path, next_content).map_err(|source| ConfigError::ConfigFileReadFailed {
            path: path.display().to_string(),
            source,
        })?;
        self.variable_crypto.key_base64 = key_base64;
        Ok(())
    }
}

impl HttpClientProfilesConfig {
    fn apply_overrides(&mut self, overrides: HttpClientProfilesFileConfig) {
        if let Some(profile) = overrides.default {
            self.default.apply_overrides(profile);
        }
        if let Some(profile) = overrides.model_inference {
            self.model_inference.apply_overrides(profile);
        }
        if let Some(profile) = overrides.runtime_control {
            self.runtime_control.apply_overrides(profile);
        }
        if let Some(profile) = overrides.document_parsing {
            self.document_parsing.apply_overrides(profile);
        }
    }
}

impl HttpClientProfileConfig {
    fn apply_overrides(&mut self, overrides: HttpClientProfileFileConfig) {
        if let Some(value) = overrides.connect_timeout_ms {
            self.connect_timeout_ms = value;
        }
        if let Some(value) = overrides.pool_idle_timeout_ms {
            self.pool_idle_timeout_ms = Some(value);
        }
        if let Some(value) = overrides.pool_max_idle_per_host {
            self.pool_max_idle_per_host = value;
        }
        if let Some(value) = overrides.tcp_keepalive_ms {
            self.tcp_keepalive_ms = Some(value);
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AppConfigFile {
    bind_address: Option<String>,
    port: Option<u16>,
    database_url: Option<String>,
    graph_storage_path: Option<String>,
    frontend_dist_dir: Option<String>,
    project_doc_root: Option<String>,
    terminal_working_directory: Option<String>,
    skills_registry_root: Option<String>,
    resource: Option<ResourceFileConfig>,
    system_agents: Option<SystemAgentFileConfig>,
    variable_crypto: Option<variable_crypto::VariableCryptoFileConfig>,
    http_clients: Option<HttpClientProfilesFileConfig>,
    mineru: Option<MineruFileConfig>,
    short_term_memory: Option<ShortTermMemoryFileConfig>,
    event_bus: Option<EventBusFileConfig>,
    navigation: Option<NavigationFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct NavigationFileConfig {
    max_depth: Option<usize>,
}

impl AppConfigFile {
    fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let path_string = path.display().to_string();
        let content =
            fs::read_to_string(path).map_err(|source| ConfigError::ConfigFileReadFailed {
                path: path_string.clone(),
                source,
            })?;
        Self::from_content(&content, path)
    }

    fn from_content(content: &str, path: &Path) -> Result<Self, ConfigError> {
        serde_yaml::from_str::<Self>(content).map_err(|source| ConfigError::ConfigFileInvalid {
            path: path.display().to_string(),
            source,
        })
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SystemAgentFileConfig {
    codex_command: Option<String>,
    opencode_command: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct HttpClientProfilesFileConfig {
    default: Option<HttpClientProfileFileConfig>,
    model_inference: Option<HttpClientProfileFileConfig>,
    runtime_control: Option<HttpClientProfileFileConfig>,
    document_parsing: Option<HttpClientProfileFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct HttpClientProfileFileConfig {
    connect_timeout_ms: Option<u64>,
    pool_idle_timeout_ms: Option<u64>,
    pool_max_idle_per_host: Option<usize>,
    tcp_keepalive_ms: Option<u64>,
}

impl SystemAgentConfig {
    fn apply_overrides(&mut self, overrides: SystemAgentFileConfig) {
        if let Some(value) = overrides.codex_command {
            self.codex_command = value;
        }
        if let Some(value) = overrides.opencode_command {
            self.opencode_command = value;
        }
    }
}
