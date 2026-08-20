use serde::Deserialize;
use std::{fs, path::Path};

use super::{
    AppConfig, AuthConfig, ConfigError, DEFAULT_APPLICATION_CONFIG_PATH,
    DEFAULT_AUTH_GITHUB_AUTHORIZE_URL, DEFAULT_AUTH_GITHUB_EMAILS_API_URL,
    DEFAULT_AUTH_GITHUB_NAME, DEFAULT_AUTH_GITHUB_SCOPE, DEFAULT_AUTH_GITHUB_TOKEN_URL,
    DEFAULT_AUTH_GITHUB_USER_API_URL, DEFAULT_AUTH_LDAP_NAME, GitHubOAuthConfig,
    HttpClientProfileConfig, HttpClientProfilesConfig, LdapConfig, SystemAgentConfig,
    mineru::MineruFileConfig, parsing::required_config_string, paths::normalize_storage_path,
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
        Self::from_yaml_file(DEFAULT_APPLICATION_CONFIG_PATH)
    }

    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let file_config = AppConfigFile::from_file(path)?;
        let mut config = Self::runtime_defaults();
        config.application_config_path = path.display().to_string();
        config.apply_file_overrides(file_config)?;
        config.apply_env_overrides()?;
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
        if let Some(value) = overrides.auth {
            self.auth.apply_overrides(value)?;
        }
        if let Some(value) = overrides.event_bus {
            self.event_bus.apply_overrides(value);
        }
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
    ldap: Option<LdapFileConfig>,
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
struct LdapFileConfig {
    url: Option<String>,
    bind_dn: Option<String>,
    bind_password: Option<String>,
    search_base: Option<String>,
    search_filter: Option<String>,
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
        if let Some(value) = overrides.ldap {
            self.ldap = Some(value.into_runtime_config()?);
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

impl LdapFileConfig {
    fn into_runtime_config(self) -> Result<LdapConfig, ConfigError> {
        Ok(LdapConfig {
            name: DEFAULT_AUTH_LDAP_NAME.into(),
            url: required_config_string(self.url, "auth.ldap.url")?,
            bind_dn: required_config_string(self.bind_dn, "auth.ldap.bindDn")?,
            bind_password: required_config_string(self.bind_password, "auth.ldap.bindPassword")?,
            search_base: required_config_string(self.search_base, "auth.ldap.searchBase")?,
            search_filter: required_config_string(self.search_filter, "auth.ldap.searchFilter")?,
        })
    }
}
