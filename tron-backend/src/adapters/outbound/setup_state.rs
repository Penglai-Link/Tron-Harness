use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::application::setup_state::{SetupState, SetupStateError, SetupStateStore};
pub struct JsonSetupStateStore {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl JsonSetupStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            write_lock: Mutex::new(()),
        }
    }
}

impl SetupStateStore for JsonSetupStateStore {
    fn read(&self) -> Result<SetupState, SetupStateError> {
        if !self.path.exists() {
            return Ok(SetupState::default());
        }
        let content = fs::read_to_string(&self.path).map_err(|_| SetupStateError::ReadFailed)?;
        serde_json::from_str(&content).map_err(|_| SetupStateError::ReadFailed)
    }

    fn write(&self, state: &SetupState) -> Result<(), SetupStateError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| SetupStateError::WriteFailed)?;
        let content = serde_json::to_vec_pretty(state).map_err(|_| SetupStateError::WriteFailed)?;
        let parent = self.path.parent().ok_or(SetupStateError::WriteFailed)?;
        let temp = parent.join(format!(".initialization-state.{}.tmp", std::process::id()));
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)
            .map_err(|_| SetupStateError::WriteFailed)?;
        file.write_all(&content)
            .map_err(|_| SetupStateError::WriteFailed)?;
        file.sync_all().map_err(|_| SetupStateError::WriteFailed)?;
        drop(file);
        replace_file(&temp, &self.path)
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> Result<(), SetupStateError> {
    fs::rename(source, target).map_err(|_| SetupStateError::WriteFailed)
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<(), SetupStateError> {
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
        Err(SetupStateError::WriteFailed)
    } else {
        Ok(())
    }
}
