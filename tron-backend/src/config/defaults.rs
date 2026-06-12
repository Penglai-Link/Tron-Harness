use crate::shared::event_bus::EventBusConfig;

use super::{
    AppConfig, AuthConfig, DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS, DEFAULT_AUTH_DEV_OAUTH2_NAME,
    DEFAULT_AUTH_JWT_SECRET, DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS, DEFAULT_BIND_ADDRESS,
    DEFAULT_PORT, DEFAULT_SYSTEM_AGENT_CODEX_COMMAND, DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND,
    HttpClientProfilesConfig, SystemAgentConfig, TEST_DATABASE_URL, VariableCryptoConfig,
};

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bind_address: DEFAULT_BIND_ADDRESS.into(),
            port: DEFAULT_PORT,
            database_url: TEST_DATABASE_URL.into(),
            graph_storage_path: ":memory:".into(),
            frontend_dist_dir: std::env::temp_dir()
                .join("tron-frontend-test")
                .display()
                .to_string(),
            project_doc_root: std::env::temp_dir().display().to_string(),
            skills_registry_root: std::env::temp_dir()
                .join("tron-skills-registry-test")
                .display()
                .to_string(),
            system_agents: SystemAgentConfig {
                codex_command: DEFAULT_SYSTEM_AGENT_CODEX_COMMAND.into(),
                opencode_command: DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND.into(),
            },
            variable_crypto: VariableCryptoConfig::test_default(),
            http_clients: HttpClientProfilesConfig::default_profiles(),
            auth: AuthConfig {
                jwt_secret: DEFAULT_AUTH_JWT_SECRET.into(),
                access_token_ttl_seconds: DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECONDS,
                refresh_token_ttl_seconds: DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECONDS,
                cookie_secure: false,
                dev_oauth2_enabled: true,
                dev_oauth2_name: DEFAULT_AUTH_DEV_OAUTH2_NAME.into(),
                github_oauth: None,
                ldap: None,
            },
            event_bus: EventBusConfig::default(),
        }
    }
}
