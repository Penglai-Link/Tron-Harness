use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tron_backend::config::AppConfig;

const APPLICATION_VARIABLE_CRYPTO_KEY_BASE64: &str = "sbxNdQm4XiwQyfXNEtjZCQ==";

#[test]
fn application_yaml_uses_home_storage_defaults() {
    let config = AppConfig::from_application_yaml().expect("application yaml should load");

    assert_eq!(
        PathBuf::from(config.database_url),
        database_url_expected_from_environment()
    );
    assert_eq!(
        PathBuf::from(config.graph_storage_path),
        graph_storage_home_default()
    );
    assert_eq!(
        config.variable_crypto.key_base64,
        APPLICATION_VARIABLE_CRYPTO_KEY_BASE64
    );
}

#[test]
fn graph_storage_path_yaml_override_still_wins() {
    let config_path = temp_config_path();
    fs::write(
        &config_path,
        "graphStoragePath: /tmp/custom-tron.graph.grafeo\n",
    )
    .expect("temp config should be written");

    let config = AppConfig::from_yaml_file(&config_path).expect("temp config should load");

    fs::remove_file(&config_path).expect("temp config should be removed");
    assert_eq!(config.graph_storage_path, "/tmp/custom-tron.graph.grafeo");
}

#[test]
fn mineru_and_document_parsing_config_load_from_yaml() {
    let config_path = temp_config_path();
    fs::write(
        &config_path,
        r#"
httpClients:
  documentParsing:
    connectTimeoutMs: 2500
    poolMaxIdlePerHost: 4
mineru:
  enabled: true
  baseUrl: http://127.0.0.1:18000
  parseEndpoint: /file_parse
  timeoutSeconds: 120
  maxBytes: 2097152
  backend: pipeline
  parseMethod: auto
  returnContentList: true
"#,
    )
    .expect("temp config should be written");

    let config = AppConfig::from_yaml_file(&config_path).expect("temp config should load");

    fs::remove_file(&config_path).expect("temp config should be removed");
    assert_eq!(
        config.http_clients.document_parsing.connect_timeout_ms,
        2500
    );
    assert_eq!(
        config.http_clients.document_parsing.pool_max_idle_per_host,
        4
    );
    assert_eq!(config.mineru.base_url, "http://127.0.0.1:18000");
    assert_eq!(config.mineru.timeout_seconds, 120);
    assert_eq!(config.mineru.max_bytes, 2_097_152);
    assert!(config.mineru.return_content_list);
}

fn database_url_expected_from_environment() -> PathBuf {
    env::var("TRON_BACKEND_DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(normalize_storage_path)
        .unwrap_or_else(database_home_default)
}

fn database_home_default() -> PathBuf {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".tron/tron.sqlite3"))
        .unwrap_or_else(|_| PathBuf::from(".tron/tron.sqlite3"))
}

fn graph_storage_home_default() -> PathBuf {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".tron/tron.graph.grafeo"))
        .unwrap_or_else(|_| PathBuf::from(".tron/tron.graph.grafeo"))
}

fn normalize_storage_path(value: String) -> PathBuf {
    if value == ":memory:" || value.starts_with("file:") {
        return PathBuf::from(value);
    }
    if value == "~" {
        return env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return env::var("HOME")
            .map(|home| PathBuf::from(home).join(rest))
            .unwrap_or_else(|_| PathBuf::from(rest));
    }
    PathBuf::from(value)
}

fn temp_config_path() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("tron-config-{suffix}.yaml"))
}
