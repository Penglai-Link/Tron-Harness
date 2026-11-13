use std::{path::Path, sync::Arc};

use crate::{
    application::workspace::WorkspaceService,
    domain::directory::{Directory, DirectoryError, DirectoryView},
    shared::id::{IdGenerator, SnowflakedSnowflakeIdProvider},
};

pub trait DirectoryRepository: Send + Sync {
    fn list(&self, workspace_code: &str) -> Result<Vec<Directory>, DirectoryError>;
    fn get(&self, directory_code: &str) -> Result<Directory, DirectoryError>;
    /// Reuses and restores an existing registration for the same canonical path.
    fn register(&self, directory: &Directory) -> Result<Directory, DirectoryError>;
    fn rename(&self, directory_code: &str, name: &str) -> Result<(), DirectoryError>;
}

pub struct DirectoryBrowserEntry {
    pub name: String,
    pub path: String,
}
pub struct DirectoryBrowserPage {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<DirectoryBrowserEntry>,
}

#[async_trait::async_trait]
pub trait DirectoryPicker: Send + Sync {
    async fn pick(&self, locale: &str) -> Result<Option<String>, DirectoryError>;
}

#[derive(Clone)]
pub struct DirectoryService {
    repository: Arc<dyn DirectoryRepository>,
    workspaces: WorkspaceService,
    picker: Arc<dyn DirectoryPicker>,
    ids: Arc<dyn IdGenerator>,
}

impl DirectoryService {
    pub fn new(
        repository: impl DirectoryRepository + 'static,
        workspaces: WorkspaceService,
        picker: impl DirectoryPicker + 'static,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            workspaces,
            picker: Arc::new(picker),
            ids: Arc::new(SnowflakedSnowflakeIdProvider::default()),
        }
    }

    fn check_workspace(&self, workspace_code: &str) -> Result<(), DirectoryError> {
        match self.workspaces.workspace_is_active(workspace_code) {
            Ok(true) => Ok(()),
            _ => Err(DirectoryError::WorkspaceInvalid),
        }
    }

    pub fn list(&self, workspace_code: &str) -> Result<Vec<DirectoryView>, DirectoryError> {
        self.check_workspace(workspace_code)?;
        let directories = self.repository.list(workspace_code)?;
        let mut result: Vec<_> = directories
            .iter()
            .map(|directory| {
                let path = Path::new(&directory.canonical_path);
                let parent = directories
                    .iter()
                    .filter(|candidate| {
                        candidate.execution_host == directory.execution_host
                            && candidate.directory_code != directory.directory_code
                            && path.starts_with(Path::new(&candidate.canonical_path))
                    })
                    .max_by_key(|candidate| {
                        Path::new(&candidate.canonical_path).components().count()
                    });
                DirectoryView {
                    directory_code: directory.directory_code.clone(),
                    workspace_code: directory.workspace_code.clone(),
                    execution_host: directory.execution_host.clone(),
                    path: directory.canonical_path.clone(),
                    name: directory.display_name.clone(),
                    parent_directory_code: parent.map(|item| item.directory_code.clone()),
                    available: path.is_dir(),
                }
            })
            .collect();
        result.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.directory_code.cmp(&b.directory_code))
        });
        Ok(result)
    }

    pub fn resolve(&self, workspace_code: &str, code: &str) -> Result<Directory, DirectoryError> {
        self.check_workspace(workspace_code)?;
        let directory = self.repository.get(code)?;
        if directory.workspace_code != workspace_code {
            return Err(DirectoryError::NotFound);
        }
        Ok(directory)
    }

    pub fn browse(
        &self,
        workspace_code: &str,
        requested: Option<&str>,
    ) -> Result<DirectoryBrowserPage, DirectoryError> {
        self.check_workspace(workspace_code)?;
        let selected = requested
            .filter(|path| !path.trim().is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(std::path::PathBuf::from)
            })
            .ok_or(DirectoryError::PathInvalid)?;
        if !selected.is_absolute() {
            return Err(DirectoryError::PathInvalid);
        }
        let selected = std::fs::canonicalize(selected).map_err(|_| DirectoryError::PathInvalid)?;
        let path = selected
            .to_str()
            .ok_or(DirectoryError::PathInvalid)?
            .to_owned();
        let parent = selected
            .parent()
            .and_then(|parent| parent.to_str())
            .map(str::to_owned);
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&selected).map_err(|_| DirectoryError::PathInvalid)? {
            let entry = entry.map_err(|_| DirectoryError::PathInvalid)?;
            if !entry.path().is_dir() {
                continue;
            }
            let (Some(name), Some(path)) = (
                entry.file_name().to_str().map(str::to_owned),
                entry.path().to_str().map(str::to_owned),
            ) else {
                continue;
            };
            entries.push(DirectoryBrowserEntry { name, path });
        }
        entries.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(DirectoryBrowserPage {
            path,
            parent,
            entries,
        })
    }

    pub async fn pick_and_register(
        &self,
        workspace_code: &str,
        locale: &str,
    ) -> Result<Option<DirectoryView>, DirectoryError> {
        match self.pick_path(workspace_code, locale).await? {
            Some(path) => self.register(workspace_code, &path).map(Some),
            None => Ok(None),
        }
    }

    pub async fn pick_path(
        &self,
        workspace_code: &str,
        locale: &str,
    ) -> Result<Option<String>, DirectoryError> {
        self.check_workspace(workspace_code)?;
        self.picker.pick(locale).await
    }

    pub fn register(
        &self,
        workspace_code: &str,
        path: &str,
    ) -> Result<DirectoryView, DirectoryError> {
        self.check_workspace(workspace_code)?;
        let selected = Path::new(path);
        if !selected.is_absolute() {
            return Err(DirectoryError::PathInvalid);
        }
        let canonical = std::fs::canonicalize(selected).map_err(|_| DirectoryError::PathInvalid)?;
        if !canonical.is_dir() {
            return Err(DirectoryError::PathInvalid);
        }
        let canonical_path = canonical
            .to_str()
            .ok_or(DirectoryError::PathInvalid)?
            .to_owned();
        let name = selected
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or(path)
            .to_owned();
        let code = format!(
            "DIRECTORY-{}",
            self.ids
                .next_id()
                .map_err(|_| DirectoryError::StorageFailure)?
                .value()
        );
        let saved = self.repository.register(&Directory {
            directory_code: code,
            workspace_code: workspace_code.to_owned(),
            execution_host: "server".into(),
            canonical_path,
            display_name: name,
            archived: false,
        })?;
        self.list(workspace_code)?
            .into_iter()
            .find(|directory| directory.directory_code == saved.directory_code)
            .ok_or(DirectoryError::StorageFailure)
    }

    pub fn rename(&self, code: &str, name: &str) -> Result<(), DirectoryError> {
        let directory = self.repository.get(code)?;
        self.check_workspace(&directory.workspace_code)?;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 200 {
            return Err(DirectoryError::NameInvalid);
        }
        self.repository.rename(code, name)
    }

    pub fn archive(
        &self,
        code: &str,
        chats: &crate::application::chat::ChatService,
    ) -> Result<Vec<String>, DirectoryArchiveError> {
        let directory = self.repository.get(code)?;
        self.check_workspace(&directory.workspace_code)?;
        chats
            .archive_directory(&directory.workspace_code, code)
            .map_err(DirectoryArchiveError::Chat)
    }
    pub fn delete(
        &self,
        code: &str,
        chats: &crate::application::chat::ChatService,
    ) -> Result<Vec<String>, DirectoryArchiveError> {
        let directory = self.repository.get(code)?;
        self.check_workspace(&directory.workspace_code)?;
        chats
            .delete_directory(&directory.workspace_code, code)
            .map_err(DirectoryArchiveError::Chat)
    }
}

pub enum DirectoryArchiveError {
    Directory(DirectoryError),
    Chat(crate::domain::chat::ChatError),
}

impl From<DirectoryError> for DirectoryArchiveError {
    fn from(value: DirectoryError) -> Self {
        Self::Directory(value)
    }
}
