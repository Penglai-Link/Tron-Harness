//! 浏览器验收专用后端，由 `make be-ui-acceptance` 启动。
//! 终端、系统配置和聊天验收使用真实 AppState、HTTP 路由与 PTY 服务。
//! 测试调用方提供临时目录和端口，身份登记库使用内存中的测试数据。
#[path = "../tests/common/mod.rs"]
mod common;
use tron_backend::config::AppConfig;
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
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    axum::serve(listener, common::app_with_config(config))
        .await
        .unwrap();
}
