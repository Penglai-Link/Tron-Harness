//! Isolated browser acceptance host: real AppState, HTTP routes and PTY service.
//! No production profile, application database or filesystem content is reused.
#[path = "../tests/common/mod.rs"]
mod common;
use tron_backend::config::AppConfig;
#[tokio::main]
async fn main() {
    let root = std::path::PathBuf::from(
        std::env::var("TRON_ACCEPTANCE_DIRECTORY").expect("isolated acceptance directory"),
    );
    let port = std::env::var("TRON_ACCEPTANCE_PORT").expect("acceptance port");
    let config = AppConfig {
        database_url: root.join("tron.sqlite3").display().to_string(),
        graph_storage_path: root.join("graph.grafeo").display().to_string(),
        terminal_working_directory: root.display().to_string(),
        project_doc_root: root.join("docs").display().to_string(),
        skills_registry_root: root.join("skills").display().to_string(),
        ..AppConfig::default()
    };
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    axum::serve(listener, common::app_with_config(config))
        .await
        .unwrap();
}
