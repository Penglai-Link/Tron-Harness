use std::{
    env,
    error::Error,
    fmt, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::shared::event_bus::EventBusConfig;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DATABASE_URL: &str = "tron-backend/target/tron.sqlite3";
const DEFAULT_GRAPH_STORAGE_PATH: &str = "tron-backend/target/tron.graph.grafeo";
const TEST_DATABASE_URL: &str = ":memory:";
const DEFAULT_AUTH_JWT_SECRET: &str = "tron-dev-jwt-secret";
const DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS: i64 = 900;
const DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS: i64 = 604_800;
const DEFAULT_AUTH_DEV_OAUTH2_NAME: &str = "OAuth2";
const DEFAULT_AUTH_GITHUB_NAME: &str = "GitHub";
const DEFAULT_AUTH_GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const DEFAULT_AUTH_GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const DEFAULT_AUTH_GITHUB_USER_API_URL: &str = "https://api.github.com/user";
const DEFAULT_AUTH_GITHUB_EMAILS_API_URL: &str = "https://api.github.com/user/emails";
const DEFAULT_AUTH_GITHUB_SCOPE: &str = "read:user user:email";
const DEFAULT_SKILLS_REGISTRY_ROOT_SUFFIX: &str = ".tron/skills-registry";
const DEFAULT_SYSTEM_AGENT_CODEX_COMMAND: &str = "codex";
const DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND: &str = "opencode";
const DEFAULT_APPLICATION_CONFIG_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/application.yaml");

const DEFAULT_HTTP_CONNECT_TIMEOUT_MS: u64 = 1_500;
const DEFAULT_HTTP_POOL_IDLE_TIMEOUT_MS: u64 = 90_000;
const DEFAULT_HTTP_POOL_MAX_IDLE_PER_HOST: usize = 32;
const DEFAULT_HTTP_TCP_KEEPALIVE_MS: u64 = 30_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub bind_address: String,
    pub port: u16,
    pub database_url: String,
    pub graph_storage_path: String,
    pub skills_registry_root: String,
    pub system_agents: SystemAgentConfig,
    pub http_clients: HttpClientProfilesConfig,
    pub auth: AuthConfig,
    pub event_bus: EventBusConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpClientProfilesConfig {
    pub default: HttpClientProfileConfig,
    pub model_inference: HttpClientProfileConfig,
    pub runtime_control: HttpClientProfileConfig,
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
            runtime_control: default,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemAgentConfig {
    pub codex_command: String,
    pub opencode_command: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub cookie_secure: bool,
    pub dev_oauth2_enabled: bool,
    pub dev_oauth2_name: String,
    pub github_oauth: Option<GitHubOAuthConfig>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHubOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub name: String,
    pub scope: String,
    pub authorize_url: String,
    pub token_url: String,
    pub user_api_url: String,
    pub emails_api_url: String,
}

impl AppConfig {
    pub fn from_application_yaml() -> Result<Self, ConfigError> {
        Self::from_yaml_file(DEFAULT_APPLICATION_CONFIG_PATH)
    }

    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let file_config = AppConfigFile::from_file(path)?;
        let mut config = Self::runtime_defaults();
        config.apply_file_overrides(file_config)?;
        Ok(config)
    }

    fn runtime_defaults() -> Self {
        Self {
            bind_address: DEFAULT_BIND_ADDRESS.into(),
            port: DEFAULT_PORT,
            database_url: DEFAULT_DATABASE_URL.into(),
            graph_storage_path: DEFAULT_GRAPH_STORAGE_PATH.into(),
            skills_registry_root: default_skills_registry_root_from_home(),
            system_agents: SystemAgentConfig {
                codex_command: DEFAULT_SYSTEM_AGENT_CODEX_COMMAND.into(),
                opencode_command: DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND.into(),
            },
            http_clients: HttpClientProfilesConfig::default_profiles(),
            auth: AuthConfig {
                jwt_secret: DEFAULT_AUTH_JWT_SECRET.into(),
                access_token_ttl_seconds: DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS,
                refresh_token_ttl_seconds: DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS,
                cookie_secure: false,
                dev_oauth2_enabled: false,
                dev_oauth2_name: DEFAULT_AUTH_DEV_OAUTH2_NAME.into(),
                github_oauth: None,
            },
            event_bus: EventBusConfig::default(),
        }
    }

    fn apply_file_overrides(&mut self, overrides: AppConfigFile) -> Result<(), ConfigError> {
        if let Some(value) = overrides.bind_address {
            self.bind_address = value;
        }
        if let Some(value) = overrides.port {
            self.port = value;
        }
        if let Some(value) = overrides.database_url {
            self.database_url = value;
        }
        if let Some(value) = overrides.graph_storage_path {
            self.graph_storage_path = value;
        }
        if let Some(value) = overrides.skills_registry_root {
            self.skills_registry_root = value;
        }
        if let Some(value) = overrides.system_agents {
            self.system_agents.apply_overrides(value);
        }
        if let Some(value) = overrides.http_clients {
            self.http_clients.apply_overrides(value);
        }
        if let Some(value) = overrides.auth {
            self.auth.apply_overrides(value)?;
        }
        if let Some(value) = overrides.event_bus {
            self.event_bus.apply_overrides(value);
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
    skills_registry_root: Option<String>,
    system_agents: Option<SystemAgentFileConfig>,
    http_clients: Option<HttpClientProfilesFileConfig>,
    auth: Option<AuthFileConfig>,
    event_bus: Option<EventBusFileConfig>,
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
        serde_yaml::from_str::<Self>(&content).map_err(|source| ConfigError::ConfigFileInvalid {
            path: path_string,
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
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct HttpClientProfileFileConfig {
    connect_timeout_ms: Option<u64>,
    pool_idle_timeout_ms: Option<u64>,
    pool_max_idle_per_host: Option<usize>,
    tcp_keepalive_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AuthFileConfig {
    jwt_secret: Option<String>,
    access_token_ttl_seconds: Option<i64>,
    refresh_token_ttl_seconds: Option<i64>,
    cookie_secure: Option<bool>,
    dev_oauth2_enabled: Option<bool>,
    dev_oauth2_name: Option<String>,
    github_oauth: Option<GitHubOAuthFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GitHubOAuthFileConfig {
    enabled: Option<bool>,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
    name: Option<String>,
    scope: Option<String>,
    authorize_url: Option<String>,
    token_url: Option<String>,
    user_api_url: Option<String>,
    emails_api_url: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EventBusFileConfig {
    enabled: Option<bool>,
    critical_ingress_capacity: Option<usize>,
    standard_ingress_capacity: Option<usize>,
    default_sink_capacity: Option<usize>,
    max_name_bytes: Option<usize>,
    max_producer_bytes: Option<usize>,
    max_summary_bytes: Option<usize>,
    max_attrs: Option<usize>,
    max_attr_key_bytes: Option<usize>,
    max_attr_value_bytes: Option<usize>,
    max_payload_fields: Option<usize>,
    max_payload_bytes: Option<usize>,
    max_subject_type_bytes: Option<usize>,
    max_subject_code_bytes: Option<usize>,
    max_actor_user_code_bytes: Option<usize>,
    max_actor_username_bytes: Option<usize>,
    max_request_field_bytes: Option<usize>,
    agui_replay_events_per_stream: Option<usize>,
    agui_replay_bytes_per_stream: Option<usize>,
    agui_global_replay_bytes: Option<usize>,
    agui_stream_idle_ttl_seconds: Option<u64>,
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

impl AuthConfig {
    fn apply_overrides(&mut self, overrides: AuthFileConfig) -> Result<(), ConfigError> {
        if let Some(value) = overrides.jwt_secret {
            self.jwt_secret = value;
        }
        if let Some(value) = overrides.access_token_ttl_seconds {
            self.access_token_ttl_seconds = value;
        }
        if let Some(value) = overrides.refresh_token_ttl_seconds {
            self.refresh_token_ttl_seconds = value;
        }
        if let Some(value) = overrides.cookie_secure {
            self.cookie_secure = value;
        }
        if let Some(value) = overrides.dev_oauth2_enabled {
            self.dev_oauth2_enabled = value;
        }
        if let Some(value) = overrides.dev_oauth2_name {
            self.dev_oauth2_name = value;
        }
        if let Some(value) = overrides.github_oauth {
            self.github_oauth = value.into_runtime_config()?;
        }
        Ok(())
    }
}

impl GitHubOAuthFileConfig {
    fn into_runtime_config(self) -> Result<Option<GitHubOAuthConfig>, ConfigError> {
        let enabled = self.enabled.unwrap_or(false);
        if !enabled {
            return Ok(None);
        }

        Ok(Some(GitHubOAuthConfig {
            client_id: required_config_string(self.client_id, "auth.githubOauth.clientId")?,
            client_secret: required_config_string(
                self.client_secret,
                "auth.githubOauth.clientSecret",
            )?,
            redirect_uri: required_config_string(
                self.redirect_uri,
                "auth.githubOauth.redirectUri",
            )?,
            name: self
                .name
                .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_NAME.to_string()),
            scope: self
                .scope
                .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_SCOPE.to_string()),
            authorize_url: self
                .authorize_url
                .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_AUTHORIZE_URL.to_string()),
            token_url: self
                .token_url
                .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_TOKEN_URL.to_string()),
            user_api_url: self
                .user_api_url
                .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_USER_API_URL.to_string()),
            emails_api_url: self
                .emails_api_url
                .unwrap_or_else(|| DEFAULT_AUTH_GITHUB_EMAILS_API_URL.to_string()),
        }))
    }
}

impl EventBusConfig {
    fn apply_overrides(&mut self, overrides: EventBusFileConfig) {
        if let Some(value) = overrides.enabled {
            self.enabled = value;
        }
        if let Some(value) = overrides.critical_ingress_capacity {
            self.critical_ingress_capacity = value;
        }
        if let Some(value) = overrides.standard_ingress_capacity {
            self.standard_ingress_capacity = value;
        }
        if let Some(value) = overrides.default_sink_capacity {
            self.default_sink_capacity = value;
        }
        if let Some(value) = overrides.max_name_bytes {
            self.max_name_bytes = value;
        }
        if let Some(value) = overrides.max_producer_bytes {
            self.max_producer_bytes = value;
        }
        if let Some(value) = overrides.max_summary_bytes {
            self.max_summary_bytes = value;
        }
        if let Some(value) = overrides.max_attrs {
            self.max_attrs = value;
        }
        if let Some(value) = overrides.max_attr_key_bytes {
            self.max_attr_key_bytes = value;
        }
        if let Some(value) = overrides.max_attr_value_bytes {
            self.max_attr_value_bytes = value;
        }
        if let Some(value) = overrides.max_payload_fields {
            self.max_payload_fields = value;
        }
        if let Some(value) = overrides.max_payload_bytes {
            self.max_payload_bytes = value;
        }
        if let Some(value) = overrides.max_subject_type_bytes {
            self.max_subject_type_bytes = value;
        }
        if let Some(value) = overrides.max_subject_code_bytes {
            self.max_subject_code_bytes = value;
        }
        if let Some(value) = overrides.max_actor_user_code_bytes {
            self.max_actor_user_code_bytes = value;
        }
        if let Some(value) = overrides.max_actor_username_bytes {
            self.max_actor_username_bytes = value;
        }
        if let Some(value) = overrides.max_request_field_bytes {
            self.max_request_field_bytes = value;
        }
        if let Some(value) = overrides.agui_replay_events_per_stream {
            self.agui_replay_events_per_stream = value;
        }
        if let Some(value) = overrides.agui_replay_bytes_per_stream {
            self.agui_replay_bytes_per_stream = value;
        }
        if let Some(value) = overrides.agui_global_replay_bytes {
            self.agui_global_replay_bytes = value;
        }
        if let Some(value) = overrides.agui_stream_idle_ttl_seconds {
            self.agui_stream_idle_ttl_seconds = value;
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bind_address: DEFAULT_BIND_ADDRESS.into(),
            port: DEFAULT_PORT,
            database_url: TEST_DATABASE_URL.into(),
            graph_storage_path: ":memory:".into(),
            skills_registry_root: std::env::temp_dir()
                .join("tron-skills-registry-test")
                .display()
                .to_string(),
            system_agents: SystemAgentConfig {
                codex_command: DEFAULT_SYSTEM_AGENT_CODEX_COMMAND.into(),
                opencode_command: DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND.into(),
            },
            http_clients: HttpClientProfilesConfig::default_profiles(),
            auth: AuthConfig {
                jwt_secret: DEFAULT_AUTH_JWT_SECRET.into(),
                access_token_ttl_seconds: DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS,
                refresh_token_ttl_seconds: DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS,
                cookie_secure: false,
                dev_oauth2_enabled: true,
                dev_oauth2_name: DEFAULT_AUTH_DEV_OAUTH2_NAME.into(),
                github_oauth: None,
            },
            event_bus: EventBusConfig::default(),
        }
    }
}

fn default_skills_registry_root_from_home() -> String {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(DEFAULT_SKILLS_REGISTRY_ROOT_SUFFIX))
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SKILLS_REGISTRY_ROOT_SUFFIX))
        .display()
        .to_string()
}

#[derive(Debug)]
pub enum ConfigError {
    MissingConfigField {
        key: String,
    },
    InvalidSocketAddr {
        bind_address: String,
        port: u16,
        source: std::net::AddrParseError,
    },
    ConfigFileReadFailed {
        path: String,
        source: std::io::Error,
    },
    ConfigFileInvalid {
        path: String,
        source: serde_yaml::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingConfigField { key } => {
                write!(formatter, "missing required config field `{key}`")
            }
            Self::InvalidSocketAddr {
                bind_address, port, ..
            } => write!(
                formatter,
                "invalid backend bind socket address `{bind_address}:{port}`"
            ),
            Self::ConfigFileReadFailed { path, .. } => {
                write!(formatter, "failed to read backend config file `{path}`")
            }
            Self::ConfigFileInvalid { path, .. } => {
                write!(formatter, "invalid backend config file `{path}`")
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MissingConfigField { .. } => None,
            Self::InvalidSocketAddr { source, .. } => Some(source),
            Self::ConfigFileReadFailed { source, .. } => Some(source),
            Self::ConfigFileInvalid { source, .. } => Some(source),
        }
    }
}

fn required_config_string(value: Option<String>, key: &str) -> Result<String, ConfigError> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::MissingConfigField {
            key: key.to_string(),
        }),
    }
}
