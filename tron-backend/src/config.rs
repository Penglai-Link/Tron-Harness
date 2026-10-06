mod defaults;
mod error;
mod file;
mod mineru;
mod parsing;
mod paths;
mod percentage;
mod resource;
mod variable_crypto;

use std::net::SocketAddr;

use crate::{
    application::short_term_memory::ShortTermMemoryConfig, shared::event_bus::EventBusConfig,
};

pub use self::error::ConfigError;
pub use self::mineru::{MineruConfig, MineruNode, KnowledgeIngestionConfig, ImageDescriptionConfig};
use self::parsing::env_port_override;
use self::paths::{
    default_application_config_path_from_home, default_database_url_from_home,
    default_graph_storage_path_from_home, default_resource_storage_root_from_home,
    default_skills_registry_root_from_home, env_database_url_override,
    env_graph_storage_path_override, normalize_storage_path,
};
pub use self::resource::{
    LocalResourceStorageProfile, ResourceConfig, ResourceLimits, ResourceStorageConfig,
    ResourceStorageProfile, S3ResourceStorageProfile,
};
pub use variable_crypto::VariableCryptoConfig;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_FRONTEND_DIST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/frontend");
const DEFAULT_PROJECT_DOC_ROOT: &str = ".";
const TEST_DATABASE_URL: &str = ":memory:";
const DEFAULT_SYSTEM_AGENT_CODEX_COMMAND: &str = "codex";
const DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND: &str = "opencode";

const DEFAULT_HTTP_CONNECT_TIMEOUT_MS: u64 = 1_500;
const DEFAULT_HTTP_POOL_IDLE_TIMEOUT_MS: u64 = 90_000;
const DEFAULT_HTTP_POOL_MAX_IDLE_PER_HOST: usize = 32;
const DEFAULT_HTTP_TCP_KEEPALIVE_MS: u64 = 30_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub application_config_path: String,
    pub bind_address: String,
    pub port: u16,
    pub database_url: String,
    pub graph_storage_path: String,
    pub frontend_dist_dir: String,
    pub project_doc_root: String,
    pub terminal_working_directory: String,
    pub skills_registry_root: String,
    pub resource: ResourceConfig,
    pub system_agents: SystemAgentConfig,
    pub variable_crypto: VariableCryptoConfig,
    pub http_clients: HttpClientProfilesConfig,
    pub mineru: MineruConfig,
    pub short_term_memory: ShortTermMemoryConfig,
    pub event_bus: EventBusConfig,
    pub navigation: NavigationConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationConfig {
    pub max_depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpClientProfilesConfig {
    pub default: HttpClientProfileConfig,
    pub model_inference: HttpClientProfileConfig,
    pub runtime_control: HttpClientProfileConfig,
    pub document_parsing: HttpClientProfileConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpClientProfileConfig {
    pub connect_timeout_ms: u64,
    pub pool_idle_timeout_ms: Option<u64>,
    pub pool_max_idle_per_host: usize,
    pub tcp_keepalive_ms: Option<u64>,
}

impl HttpClientProfilesConfig {
    pub fn default_profiles() -> Self {
        let default = HttpClientProfileConfig {
            connect_timeout_ms: DEFAULT_HTTP_CONNECT_TIMEOUT_MS,
            pool_idle_timeout_ms: Some(DEFAULT_HTTP_POOL_IDLE_TIMEOUT_MS),
            pool_max_idle_per_host: DEFAULT_HTTP_POOL_MAX_IDLE_PER_HOST,
            tcp_keepalive_ms: Some(DEFAULT_HTTP_TCP_KEEPALIVE_MS),
        };
        Self {
            default: default.clone(),
            model_inference: default.clone(),
            runtime_control: default.clone(),
            document_parsing: default,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemAgentConfig {
    pub codex_command: String,
    pub opencode_command: String,
}

impl AppConfig {
    fn runtime_defaults() -> Self {
        Self {
            application_config_path: default_application_config_path_from_home(),
            bind_address: DEFAULT_BIND_ADDRESS.into(),
            port: DEFAULT_PORT,
            database_url: default_database_url_from_home(),
            graph_storage_path: default_graph_storage_path_from_home(),
            frontend_dist_dir: DEFAULT_FRONTEND_DIST_DIR.into(),
            project_doc_root: DEFAULT_PROJECT_DOC_ROOT.into(),
            terminal_working_directory: DEFAULT_PROJECT_DOC_ROOT.into(),
            skills_registry_root: default_skills_registry_root_from_home(),
            resource: ResourceConfig::with_default_root(default_resource_storage_root_from_home()),
            system_agents: SystemAgentConfig {
                codex_command: DEFAULT_SYSTEM_AGENT_CODEX_COMMAND.into(),
                opencode_command: DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND.into(),
            },
            variable_crypto: VariableCryptoConfig::default(),
            http_clients: HttpClientProfilesConfig::default_profiles(),
            mineru: MineruConfig::default(),
            short_term_memory: ShortTermMemoryConfig::default(),
            event_bus: EventBusConfig::default(),
            navigation: NavigationConfig { max_depth: 3 },
        }
    }

    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(value) = env_port_override()? {
            self.port = value;
        }
        if let Ok(value) = std::env::var("TRON_BACKEND_BIND_ADDRESS")
            && !value.trim().is_empty()
        {
            self.bind_address = value.trim().to_owned();
        }
        if let Some(value) = env_database_url_override() {
            self.database_url = value;
        }
        if let Some(value) = env_graph_storage_path_override() {
            self.graph_storage_path = value;
        }
        if let Ok(value) = std::env::var("TRON_BACKEND_FRONTEND_DIST_DIR")
            && !value.trim().is_empty()
        {
            self.frontend_dist_dir = normalize_storage_path(value);
        }
        if let Ok(value) = std::env::var("TRON_PROJECT_DOC_ROOT")
            && !value.trim().is_empty()
        {
            self.project_doc_root = normalize_storage_path(value);
        }
        if let Ok(value) = std::env::var("TRON_TERMINAL_WORKING_DIRECTORY")
            && !value.trim().is_empty()
        {
            self.terminal_working_directory = normalize_storage_path(value);
        }
        Ok(())
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.bind_address, self.port)
            .parse()
            .map_err(|source| ConfigError::InvalidSocketAddr {
                bind_address: self.bind_address.clone(),
                port: self.port,
                source,
            })
    }
}
