//! 浏览器验收专用后端，由 `make be-ui-acceptance` 启动。
//! 终端、系统配置和聊天验收使用真实 AppState、HTTP 路由与 PTY 服务。
//! 测试调用方提供临时目录和端口，身份登记库使用内存中的测试数据。
#[path = "../tests/common/mod.rs"]
mod common;
use tron_backend::config::AppConfig;

fn seed_personai_fixture(state: &tron_backend::app::AppState, url: &str) {
    use rsa::{RsaPrivateKey, pkcs8::EncodePublicKey};
    use tron_backend::{
        adapters::outbound::crypto::Sm4GcmVariableCipher, application::variable::VariableCipher,
    };

    let parsed = reqwest::Url::parse(url).expect("fixture Person-AI URL");
    assert_eq!(
        parsed.host_str(),
        Some("127.0.0.1"),
        "fixture service must be local"
    );
    let public_key = RsaPrivateKey::new(&mut rand08::thread_rng(), 2048)
        .unwrap()
        .to_public_key()
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .unwrap();
    let payload = serde_json::json!({"token":"acceptance-token","refresh_token":"acceptance-refresh","client_id":"acceptance-device",
        "public_key":public_key,"server_base_url":url,"authorization_expires_at":4102444800000_i64}).to_string();
    let cipher = Sm4GcmVariableCipher::from_config(state.config.variable_crypto.clone()).unwrap();
    let encrypted = format!(
        "enc:v1:{}",
        serde_json::to_string(
            &cipher
                .encrypt(&payload, b"model_provider.credentials")
                .unwrap()
        )
        .unwrap()
    );
    let connection = rusqlite::Connection::open(&state.config.database_url).unwrap();
    connection.execute("INSERT INTO provider_connections(provider_code,server_base_url,status,credentials_encrypted,person_ai_user_id,person_ai_username,created_at,updated_at) VALUES ('PersonAi',?1,'Connected',?2,'USR-AUTH-0001','acceptance',1,1) ON CONFLICT(provider_code) DO UPDATE SET server_base_url=excluded.server_base_url,status=excluded.status,credentials_encrypted=excluded.credentials_encrypted,person_ai_user_id=excluded.person_ai_user_id,person_ai_username=excluded.person_ai_username", rusqlite::params![url,encrypted]).unwrap();
}

struct MockResourceConnectionVerifier;
impl tron_backend::application::resource::ResourceConnectionVerifier
    for MockResourceConnectionVerifier
{
    fn verify(
        &self,
        _: tron_backend::application::resource::ResourceConnectionTarget,
    ) -> tron_backend::application::resource::ResourceConnectionVerificationFuture<'_> {
        Box::pin(async { Ok(()) })
    }
}
#[tokio::main]
async fn main() {
    let root = std::path::PathBuf::from(
        std::env::var("TRON_ACCEPTANCE_DIRECTORY").expect("isolated acceptance directory"),
    );
    let port = std::env::var("TRON_ACCEPTANCE_PORT").expect("acceptance port");
    let config_path = root.join("application.yaml");
    let base = if config_path.exists() {
        AppConfig::from_yaml_file(&config_path).expect("isolated acceptance configuration")
    } else {
        AppConfig::default()
    };
    let config = AppConfig {
        application_config_path: config_path.display().to_string(),
        database_url: root.join("tron.sqlite3").display().to_string(),
        graph_storage_path: root.join("graph.grafeo").display().to_string(),
        terminal_working_directory: root.display().to_string(),
        skills_registry_root: root.join("skills").display().to_string(),
        ..base
    };
    tron_backend::adapters::outbound::observability::logs::init_application_logging(&config)
        .unwrap();
    tracing::info!("Browser acceptance backend starting");
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    // Opt-in only for isolated browser fixtures: never contact external databases or servers.
    let router = common::app_with_mut_state_setup(common::prepared_config(config), |state| {
        if let Ok(url) = std::env::var("TRON_ACCEPTANCE_PERSONAI_URL") {
            seed_personai_fixture(state, &url);
        }
        if std::env::var_os("TRON_ACCEPTANCE_MOCK_RESOURCE_CONNECTIONS").is_none() {
            return;
        }
        state.resources = state
            .resources
            .clone()
            .with_connection_verifier(std::sync::Arc::new(MockResourceConnectionVerifier));
        tron_backend::application::agent::register_default_agent_tools(
            state.agent_tool_registry.as_ref(),
            state.workspaces.clone(),
            state.apps.clone(),
            state.automations.clone(),
            state.tasks.clone(),
            state.skills.clone(),
            state.variables.clone(),
            state.resources.clone(),
            state.prompts.clone(),
            state.mcp.clone(),
            state.memory.clone(),
            state.short_term_memory.clone(),
        )
        .expect("isolated resource tools");
    });
    axum::serve(listener, router).await.unwrap();
}
