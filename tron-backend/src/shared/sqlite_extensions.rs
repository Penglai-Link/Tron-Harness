//! Tron 的 SQLite 扩展配置；资源嵌入、校验和加载由 Framework 本地存储插件负责。
use std::{
    env,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

pub use eliteforge_agent_official_plugin_storage_local::ExtensionMode as SqliteExtensionMode;
use eliteforge_agent_official_plugin_storage_local::{ExtensionConfig, ExtensionError};
use rusqlite::{Connection, OptionalExtension};

static DEFAULT_CONTEXT: OnceLock<SqliteExtensionContext> = OnceLock::new();
// sqlite-simple 使用进程级字典，保留连接避免业务连接关闭后卸载动态库。
static EXTENSION_KEEPALIVE: Mutex<Option<Connection>> = Mutex::new(None);

pub fn default_sqlite_extension_context() -> &'static SqliteExtensionContext {
    DEFAULT_CONTEXT.get_or_init(SqliteExtensionContext::from_env)
}

#[derive(Debug, Clone)]
pub struct SqliteExtensionContext {
    config: ExtensionConfig,
}

impl SqliteExtensionContext {
    pub fn from_env() -> Self {
        let cache_dir = env::var("TRON_SQLITE_EXTENSION_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::temp_dir().join("tron/sqlite-extensions"));
        let mode = match env::var("TRON_SQLITE_EXTENSION_MODE")
            .unwrap_or_else(|_| "preferred".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "disabled" => SqliteExtensionMode::Disabled,
            "required" => SqliteExtensionMode::Required,
            _ => SqliteExtensionMode::Preferred,
        };
        let mut config = ExtensionConfig::new(cache_dir);
        config.simple = mode;
        config.vector = mode;
        Self { config }
    }

    pub fn prepare_connection(&self, connection: &Connection) -> Result<(), ExtensionError> {
        if self.config.simple == SqliteExtensionMode::Disabled
            && self.config.vector == SqliteExtensionMode::Disabled
        {
            return Ok(());
        }
        let mut keepalive = EXTENSION_KEEPALIVE
            .lock()
            .map_err(|_| ExtensionError::Worker)?;
        if keepalive.is_none() {
            *keepalive =
                Some(
                    Connection::open_in_memory().map_err(|source| ExtensionError::Load {
                        operation: "keeper connection",
                        source,
                    })?,
                );
        }
        let keeper = keepalive.as_ref().ok_or(ExtensionError::Worker)?;
        self.config.prepare_connection(keeper)?;
        self.config.prepare_connection(connection)?;
        Ok(())
    }
}

pub fn sqlite_fts5_available(connection: &Connection) -> bool {
    probe_fts5(connection)
}

pub fn sqlite_simple_tokenizer_available(connection: &Connection) -> bool {
    probe_simple_tokenizer(connection) && probe_jieba_query(connection)
}

// Product embedding index space matches the configured singleton bge-m3 model.
pub const TRON_VECTOR_INDEX_DIMENSION: usize = 1024;

pub fn sqlite_vec_available(connection: &Connection) -> bool {
    probe_vec_version(connection).is_some()
}

fn probe_fts5(connection: &Connection) -> bool {
    connection
        .execute_batch(
            "CREATE VIRTUAL TABLE temp.tron_fts5_probe USING fts5(content);
             DROP TABLE temp.tron_fts5_probe;",
        )
        .is_ok()
}

fn probe_simple_tokenizer(connection: &Connection) -> bool {
    connection
        .execute_batch(
            "CREATE VIRTUAL TABLE temp.tron_simple_probe USING fts5(content, tokenize='simple');
             DROP TABLE temp.tron_simple_probe;",
        )
        .is_ok()
}

fn probe_jieba_query(connection: &Connection) -> bool {
    connection
        .query_row(
            "SELECT jieba_query('中华人民共和国国歌')",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map(|value| value.is_some_and(|value| !value.trim().is_empty()))
        .unwrap_or(false)
}

fn probe_vec_version(connection: &Connection) -> Option<String> {
    connection
        .query_row("SELECT vec_version()", [], |row| row.get::<_, String>(0))
        .optional()
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
}
