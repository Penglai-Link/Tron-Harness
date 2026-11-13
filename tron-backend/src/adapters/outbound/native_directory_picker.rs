use crate::{application::directory::DirectoryPicker, domain::directory::DirectoryError};
use std::{process::Stdio, time::Duration};
use tokio::{process::Command, sync::Semaphore};

pub struct NativeDirectoryPicker {
    gate: Semaphore,
}

impl Default for NativeDirectoryPicker {
    fn default() -> Self {
        Self {
            gate: Semaphore::new(1),
        }
    }
}

#[async_trait::async_trait]
impl DirectoryPicker for NativeDirectoryPicker {
    async fn pick(&self, locale: &str) -> Result<Option<String>, DirectoryError> {
        let _permit = self
            .gate
            .try_acquire()
            .map_err(|_| DirectoryError::PickerBusy)?;
        #[cfg(target_os = "linux")]
        if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
            return Err(DirectoryError::PickerUnavailable);
        }
        let executable = std::env::current_exe().map_err(|_| DirectoryError::PickerUnavailable)?;
        let mut command = Command::new(executable);
        command
            .args(["--pick-directory", locale])
            .stdin(Stdio::null())
            .kill_on_drop(true);
        let output = tokio::time::timeout(Duration::from_secs(300), command.output())
            .await
            .map_err(|_| DirectoryError::PickerUnavailable)?
            .map_err(|_| DirectoryError::PickerUnavailable)?;
        if !output.status.success() {
            return Err(DirectoryError::PickerUnavailable);
        }
        serde_json::from_slice(&output.stdout).map_err(|_| DirectoryError::PickerUnavailable)
    }
}
