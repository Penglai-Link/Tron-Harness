mod defaults;
mod error;
mod file;
mod mineru;
mod parsing;
mod paths;
mod variable_crypto;

use std::net::SocketAddr;

use crate::{
    application::short_term_memory::ShortTermMemoryConfig, shared::event_bus::EventBusConfig,
};

pub use self::error::ConfigError;
pub use self::mineru::MineruConfig;
use self::parsing::{env_port_override, ldap_env_override};
use self::paths::{
    default_database_url_from_home, default_graph_storage_path_from_home,
    default_skills_registry_root_from_home, env_database_url_override,
    env_graph_storage_path_override, normalize_storage_path,
};
pub use variable_crypto::VariableCryptoConfig;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_FRONTEND_DIST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/frontend");
const DEFAULT_PROJECT_DOC_ROOT: &str = ".";
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
const DEFAULT_AUTH_LDAP_NAME: &str = "LDAP";
const DEFAULT_SYSTEM_AGENT_CODEX_COMMAND: &str = "codex";
const DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND: &str = "opencode";
pub const DEFAULT_APPLICATION_CONFIG_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/application.yaml");
const ENV_BACKEND_PORT: &str = "TRON_BACKEND_PORT";
const ENV_LDAP_URL: &str = "LDAP_URL";
const ENV_LDAP_BIND_DN: &str = "LDAP_BIND_DN";
const ENV_LDAP_BIND_PASSWORD: &str = "LDAP_BIND_PASSWORD";
const ENV_LDAP_SEARCH_BASE: &str = "LDAP_SEARCH_BASE";
const ENV_LDAP_SEARCH_FILTER: &str = "LDAP_SEARCH_FILTER";

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
    pub skills_registry_root: String,
    pub system_agents: SystemAgentConfig,
    pub variable_crypto: VariableCryptoConfig,
    pub http_clients: HttpClientProfilesConfig,
    pub mineru: MineruConfig,
    pub short_term_memory: ShortTermMemoryConfig,
    pub auth: AuthConfig,
    pub event_bus: EventBusConfig,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub cookie_secure: bool,
    pub dev_oauth2_enabled: bool,
    pub dev_oauth2_name: String,
    pub github_oauth: Option<GitHubOAuthConfig>,
    pub ldap: Option<LdapConfig>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LdapConfig {
    pub name: String,
    pub url: String,
    pub bind_dn: String,
    pub bind_password: String,
    pub search_base: String,
    pub search_filter: String,
}

impl AppConfig {
    fn runtime_defaults() -> Self {
        Self {
            application_config_path: DEFAULT_APPLICATION_CONFIG_PATH.into(),
            bind_address: DEFAULT_BIND_ADDRESS.into(),
            port: DEFAULT_PORT,
            database_url: default_database_url_from_home(),
            graph_storage_path: default_graph_storage_path_from_home(),
            frontend_dist_dir: DEFAULT_FRONTEND_DIST_DIR.into(),
            project_doc_root: DEFAULT_PROJECT_DOC_ROOT.into(),
            skills_registry_root: default_skills_registry_root_from_home(),
            system_agents: SystemAgentConfig {
                codex_command: DEFAULT_SYSTEM_AGENT_CODEX_COMMAND.into(),
                opencode_command: DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND.into(),
            },
            variable_crypto: VariableCryptoConfig::default(),
            http_clients: HttpClientProfilesConfig::default_profiles(),
            mineru: MineruConfig::default(),
            short_term_memory: ShortTermMemoryConfig::default(),
            auth: AuthConfig {
                jwt_secret: DEFAULT_AUTH_JWT_SECRET.into(),
                access_token_ttl_seconds: DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS,
                refresh_token_ttl_seconds: DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS,
                cookie_secure: false,
                dev_oauth2_enabled: false,
                dev_oauth2_name: DEFAULT_AUTH_DEV_OAUTH2_NAME.into(),
                github_oauth: None,
                ldap: None,
            },
            event_bus: EventBusConfig::default(),
        }
    }

    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(value) = env_port_override()? {
            self.port = value;
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
        self.auth.github_oauth = parsing::github_oauth_env_override(self.auth.github_oauth.take())?;
        if let Some(value) = ldap_env_override().transpose()? {
            self.auth.ldap = Some(value);
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
