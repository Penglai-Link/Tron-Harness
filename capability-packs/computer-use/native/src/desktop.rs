use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
};

/// One controller owns the user's desktop until its capability backend exits.
pub struct DesktopOwner {
    _file: File,
}

impl DesktopOwner {
    pub fn claim() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .ok_or("cannot resolve the desktop user's home directory")?;
        let directory = PathBuf::from(home).join(".tron/computer-use");
        fs::create_dir_all(&directory)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("desktop.lock"))?;
        file.try_lock_exclusive().map_err(|error| {
            format!("desktop_busy: another controller owns this desktop ({error})")
        })?;
        Ok(Self { _file: file })
    }
}
