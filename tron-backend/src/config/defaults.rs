use crate::{
    application::short_term_memory::ShortTermMemoryConfig, shared::event_bus::EventBusConfig,
};

use super::{
    AppConfig, DEFAULT_BIND_ADDRESS, DEFAULT_PORT, DEFAULT_SYSTEM_AGENT_CODEX_COMMAND,
    DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND, HttpClientProfilesConfig, MineruConfig,
    NavigationConfig, ResourceConfig, SystemAgentConfig, TEST_DATABASE_URL, VariableCryptoConfig,
};

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            application_config_path: std::env::temp_dir()
                .join("tron-application-test.yaml")
                .display()
                .to_string(),
            bind_address: DEFAULT_BIND_ADDRESS.into(),
            port: DEFAULT_PORT,
            database_url: TEST_DATABASE_URL.into(),
            graph_storage_path: ":memory:".into(),
            frontend_dist_dir: std::env::temp_dir()
                .join("tron-frontend-test")
                .display()
                .to_string(),
            project_doc_root: std::env::temp_dir().display().to_string(),
            terminal_working_directory: std::env::temp_dir().display().to_string(),
            skills_registry_root: std::env::temp_dir()
                .join("tron-skills-registry-test")
                .display()
                .to_string(),
            resource: ResourceConfig::test_default(),
            system_agents: SystemAgentConfig {
                codex_command: DEFAULT_SYSTEM_AGENT_CODEX_COMMAND.into(),
                opencode_command: DEFAULT_SYSTEM_AGENT_OPENCODE_COMMAND.into(),
            },
            variable_crypto: VariableCryptoConfig::test_default(),
            http_clients: HttpClientProfilesConfig::default_profiles(),
            mineru: MineruConfig::default(),
            short_term_memory: ShortTermMemoryConfig::default(),
            event_bus: EventBusConfig::default(),
            navigation: NavigationConfig { max_depth: 3 },
        }
    }
}
