use crate::domain::agent::ToolError;
use std::{
    fs::Permissions,
    path::{Path, PathBuf},
};

/// Filesystem boundary for validated multi-file code editing.
pub trait CodeFileEditor: Send + Sync {
    fn resolve(&self, path: &Path) -> Result<PathBuf, ToolError>;
    fn read(&self, path: &Path) -> Result<Option<String>, ToolError>;
    fn permissions(&self, path: &Path) -> Result<Permissions, ToolError>;
    fn apply(&self, changes: &[CodeFileChange]) -> Result<(), ToolError>;
}
pub struct CodeFileChange {
    pub path: PathBuf,
    pub before: Option<String>,
    pub after: Option<String>,
    pub permissions: Option<Permissions>,
}
