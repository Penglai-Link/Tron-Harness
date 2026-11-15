use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use sha2::{Digest, Sha256};

use crate::application::system_config::{
    StoredSystemConfig, SystemConfigStore, SystemConfigStoreError,
};

pub struct YamlSystemConfigStore {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl YamlSystemConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            write_lock: Mutex::new(()),
        }
    }

    fn read_current(&self) -> Result<StoredSystemConfig, SystemConfigStoreError> {
        let content =
            fs::read_to_string(&self.path).map_err(|_| SystemConfigStoreError::ReadFailed)?;
        Ok(StoredSystemConfig {
            revision: revision(&content),
            content,
            path: self.path.display().to_string(),
        })
    }
}

impl SystemConfigStore for YamlSystemConfigStore {
    fn read(&self) -> Result<StoredSystemConfig, SystemConfigStoreError> {
        self.read_current()
    }

    fn replace(
        &self,
        expected_revision: &str,
        content: &str,
    ) -> Result<StoredSystemConfig, SystemConfigStoreError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| SystemConfigStoreError::WriteFailed)?;
        if self.read_current()?.revision != expected_revision {
            return Err(SystemConfigStoreError::RevisionConflict);
        }
        atomic_write(&self.path, content.as_bytes())?;
        let persisted = self
            .read_current()
            .map_err(|_| SystemConfigStoreError::WriteFailed)?;
        if persisted.content != content {
            return Err(SystemConfigStoreError::WriteFailed);
        }
        Ok(persisted)
    }
}

fn revision(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), SystemConfigStoreError> {
    let parent = path.parent().ok_or(SystemConfigStoreError::WriteFailed)?;
    let temp_path = parent.join(format!(".application.yaml.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp_path)
        .map_err(|_| SystemConfigStoreError::WriteFailed)?;
    file.write_all(content)
        .map_err(|_| SystemConfigStoreError::WriteFailed)?;
    file.sync_all()
        .map_err(|_| SystemConfigStoreError::WriteFailed)?;
    drop(file);
    replace_file(&temp_path, path)?;
    restrict_config_permissions(path);
    let _ = File::open(parent).and_then(|directory| directory.sync_all());
    Ok(())
}

#[cfg(unix)]
fn restrict_config_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_config_permissions(_path: &Path) {}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> Result<(), SystemConfigStoreError> {
    fs::rename(source, target).map_err(|_| SystemConfigStoreError::WriteFailed)
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<(), SystemConfigStoreError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(SystemConfigStoreError::WriteFailed)
    } else {
        Ok(())
    }
}
