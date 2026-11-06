use serde::Serialize;

/// A registered execution directory, independent of any topic or session.
#[derive(Clone, Debug)]
pub struct Directory {
    pub directory_code: String,
    pub workspace_code: String,
    pub execution_host: String,
    pub canonical_path: String,
    pub display_name: String,
    pub archived: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryView {
    pub directory_code: String,
    pub workspace_code: String,
    pub execution_host: String,
    pub path: String,
    pub name: String,
    pub parent_directory_code: Option<String>,
    pub available: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum DirectoryError {
    WorkspaceInvalid,
    PathInvalid,
    NameInvalid,
    NotFound,
    StorageFailure,
    PickerUnavailable,
    PickerBusy,
}
