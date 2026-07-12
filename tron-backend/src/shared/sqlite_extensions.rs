use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use rusqlite::{Connection, LoadExtensionGuard, OptionalExtension, params};

const SQLITE_SIMPLE_VERSION: &str = "v0.7.1";
const SQLITE_VEC_VERSION: &str = "v0.1.9";
const SIMPLE_RESOURCE_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/native/sqlite-simple/",
    "v0.7.1"
);
const VEC_RESOURCE_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/native/sqlite-vec/",
    "v0.1.9"
);
const DEFAULT_RUNTIME_ROOT_SUFFIX: &str = "tron/sqlite-extensions";

static DEFAULT_CONTEXT: OnceLock<SqliteExtensionContext> = OnceLock::new();

pub fn default_sqlite_extension_context() -> &'static SqliteExtensionContext {
    DEFAULT_CONTEXT.get_or_init(SqliteExtensionContext::from_env)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteSystemInfo {
    pub os: String,
    pub arch: String,
    pub simple_platform: Option<String>,
    pub vec_platform: Option<String>,
}

impl SqliteSystemInfo {
    pub fn detect() -> Self {
        let os = env::consts::OS.to_string();
        let arch = env::consts::ARCH.to_string();
        Self {
            simple_platform: sqlite_simple_platform(&os, &arch),
            vec_platform: sqlite_vec_platform(&os, &arch),
            os,
            arch,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteExtensionMode {
    Disabled,
    Preferred,
    Required,
}

impl SqliteExtensionMode {
    fn from_env() -> Self {
        match env::var("TRON_SQLITE_EXTENSION_MODE")
            .unwrap_or_else(|_| "preferred".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "disabled" => Self::Disabled,
            "required" => Self::Required,
            _ => Self::Preferred,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SqliteExtensionContext {
    mode: SqliteExtensionMode,
    runtime_root: PathBuf,
    system: SqliteSystemInfo,
}

impl SqliteExtensionContext {
    pub fn from_env() -> Self {
        let runtime_root = env::var("TRON_SQLITE_EXTENSION_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::temp_dir().join(DEFAULT_RUNTIME_ROOT_SUFFIX));
        Self {
            mode: SqliteExtensionMode::from_env(),
            runtime_root,
            system: SqliteSystemInfo::detect(),
        }
    }

    pub fn mode(&self) -> SqliteExtensionMode {
        self.mode
    }

    pub fn system(&self) -> &SqliteSystemInfo {
        &self.system
    }

    pub fn prepare_connection(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        if self.mode == SqliteExtensionMode::Disabled {
            return Ok(());
        }

        if let Err(error) = self.load_simple(connection) {
            if self.mode == SqliteExtensionMode::Required {
                return Err(error);
            }
        }
        if let Err(error) = self.load_vec(connection) {
            if self.mode == SqliteExtensionMode::Required {
                return Err(error);
            }
        }

        Ok(())
    }

    fn load_simple(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        if probe_simple_tokenizer(connection) && probe_jieba_query(connection) {
            return Ok(());
        }
        let Some(platform) = &self.system.simple_platform else {
            return Err(rusqlite::Error::InvalidPath(PathBuf::from(format!(
                "sqlite-simple unsupported platform {}-{}",
                self.system.os, self.system.arch
            ))));
        };
        let extension_path = self.extract_extension(
            Path::new(SIMPLE_RESOURCE_ROOT),
            SQLITE_SIMPLE_VERSION,
            platform,
            simple_file_name(platform),
        )?;
        load_extension(connection, &extension_path)?;
        let dict_directory = self.extract_simple_dict()?;
        let dict_path = dict_directory.to_string_lossy().to_string();
        let _ = connection.execute("SELECT jieba_dict(?1)", params![dict_path]);
        Ok(())
    }

    fn load_vec(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        if probe_vec_version(connection).is_some() {
            return Ok(());
        }
        let Some(platform) = &self.system.vec_platform else {
            return Err(rusqlite::Error::InvalidPath(PathBuf::from(format!(
                "sqlite-vec unsupported platform {}-{}",
                self.system.os, self.system.arch
            ))));
        };
        let extension_path = self.extract_extension(
            Path::new(VEC_RESOURCE_ROOT),
            SQLITE_VEC_VERSION,
            platform,
            vec_file_name(platform),
        )?;
        load_extension(connection, &extension_path)
    }

    fn extract_extension(
        &self,
        resource_root: &Path,
        version: &str,
        platform: &str,
        file_name: &str,
    ) -> Result<PathBuf, rusqlite::Error> {
        let source = resource_root.join(platform).join(file_name);
        if !source.is_file() {
            return Err(rusqlite::Error::InvalidPath(source));
        }
        let target = self
            .runtime_root
            .join(version)
            .join(platform)
            .join(file_name)
            .to_absolute_path();
        copy_file_once(&source, &target)?;
        Ok(target)
    }

    fn extract_simple_dict(&self) -> Result<PathBuf, rusqlite::Error> {
        let source_root = Path::new(SIMPLE_RESOURCE_ROOT).join("dict");
        let target_root = self.runtime_root.join(SQLITE_SIMPLE_VERSION).join("dict");
        copy_dir_once(&source_root, &target_root)?;
        Ok(target_root)
    }
}

trait AbsolutePath {
    fn to_absolute_path(&self) -> PathBuf;
}

impl AbsolutePath for PathBuf {
    fn to_absolute_path(&self) -> PathBuf {
        if self.is_absolute() {
            self.clone()
        } else {
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(self)
        }
    }
}

fn load_extension(connection: &Connection, path: &Path) -> Result<(), rusqlite::Error> {
    unsafe {
        let _guard = LoadExtensionGuard::new(connection)?;
        connection.load_extension(path, None)
    }
}

pub fn sqlite_fts5_available(connection: &Connection) -> bool {
    probe_fts5(connection)
}

pub fn sqlite_simple_tokenizer_available(connection: &Connection) -> bool {
    probe_simple_tokenizer(connection)
}

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

fn sqlite_simple_platform(os: &str, arch: &str) -> Option<String> {
    let arch = normalized_arch(arch)?;
    match os {
        "macos" => Some(format!("macos-{arch}")),
        "linux" => Some(format!("linux-{arch}")),
        "windows" => Some(format!("windows-{arch}")),
        _ => None,
    }
}

fn sqlite_vec_platform(os: &str, arch: &str) -> Option<String> {
    let arch = normalized_arch(arch)?;
    match os {
        "macos" => Some(format!("macos-{arch}")),
        "linux" => Some(format!("linux-{arch}")),
        "windows" if arch == "x86_64" => Some("windows-x86_64".into()),
        _ => None,
    }
}

fn normalized_arch(arch: &str) -> Option<&'static str> {
    match arch {
        "aarch64" | "arm64" => Some("aarch64"),
        "x86_64" | "amd64" => Some("x86_64"),
        "x86" | "i386" | "i686" => Some("i686"),
        _ => None,
    }
}

fn simple_file_name(platform: &str) -> &'static str {
    if platform.starts_with("windows-") {
        "simple.dll"
    } else if platform.starts_with("macos-") {
        "libsimple.dylib"
    } else {
        "libsimple.so"
    }
}

fn vec_file_name(platform: &str) -> &'static str {
    if platform.starts_with("windows-") {
        "vec0.dll"
    } else if platform.starts_with("macos-") || platform.starts_with("ios") {
        "vec0.dylib"
    } else {
        "vec0.so"
    }
}

fn copy_file_once(source: &Path, target: &Path) -> Result<(), rusqlite::Error> {
    if target.is_file() {
        let source_size = fs::metadata(source)
            .map_err(|_| rusqlite::Error::InvalidPath(source.into()))?
            .len();
        let target_size = fs::metadata(target)
            .map_err(|_| rusqlite::Error::InvalidPath(target.into()))?
            .len();
        if source_size == target_size {
            return Ok(());
        }
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|_| rusqlite::Error::InvalidPath(parent.into()))?;
    }
    fs::copy(source, target).map_err(|_| rusqlite::Error::InvalidPath(source.into()))?;
    Ok(())
}

fn copy_dir_once(source_root: &Path, target_root: &Path) -> Result<(), rusqlite::Error> {
    if !source_root.is_dir() {
        return Err(rusqlite::Error::InvalidPath(source_root.into()));
    }
    for entry in
        fs::read_dir(source_root).map_err(|_| rusqlite::Error::InvalidPath(source_root.into()))?
    {
        let entry = entry.map_err(|_| rusqlite::Error::InvalidPath(source_root.into()))?;
        let source = entry.path();
        let target = target_root.join(entry.file_name());
        if source.is_dir() {
            copy_dir_once(&source, &target)?;
        } else {
            copy_file_once(&source, &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_current_platform_mapping() {
        let system = SqliteSystemInfo::detect();

        assert!(!system.os.is_empty());
        assert!(!system.arch.is_empty());
        if matches!(system.os.as_str(), "macos" | "linux") {
            assert!(system.vec_platform.is_some());
            assert!(system.simple_platform.is_some());
        }
    }

    #[test]
    fn preferred_context_prepares_connection_without_requiring_extensions() {
        let connection = Connection::open_in_memory().unwrap();
        let context = SqliteExtensionContext {
            mode: SqliteExtensionMode::Preferred,
            runtime_root: env::temp_dir()
                .join(format!("tron-sqlite-extension-test-{}", std::process::id())),
            system: SqliteSystemInfo::detect(),
        };

        context.prepare_connection(&connection).unwrap();

        assert!(sqlite_fts5_available(&connection));
    }

    #[test]
    fn refreshes_stale_extracted_files() {
        let root = env::temp_dir().join(format!(
            "tron-sqlite-extension-copy-test-{}",
            std::process::id()
        ));
        let source = root.join("source.bin");
        let target = root.join("nested").join("target.bin");

        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, b"new extension").unwrap();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"old pointer").unwrap();

        copy_file_once(&source, &target).unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new extension");
        fs::remove_dir_all(root).unwrap();
    }
}
