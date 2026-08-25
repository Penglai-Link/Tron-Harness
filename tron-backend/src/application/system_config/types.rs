use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct SystemConfigService {
    pub(super) store: Arc<dyn SystemConfigStore>,
}

#[derive(Debug)]
pub struct StoredSystemConfig {
    pub content: String,
    pub path: String,
    pub revision: String,
}

#[derive(Debug)]
pub enum SystemConfigStoreError {
    ReadFailed,
    RevisionConflict,
    WriteFailed,
}

pub trait SystemConfigStore: Send + Sync {
    fn read(&self) -> Result<StoredSystemConfig, SystemConfigStoreError>;
    fn replace(
        &self,
        expected_revision: &str,
        content: &str,
    ) -> Result<StoredSystemConfig, SystemConfigStoreError>;
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfigDocument {
    pub revision: String,
    pub path: String,
    pub items: Vec<SystemConfigItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfigItem {
    pub path: String,
    pub value: Option<serde_json::Value>,
    pub value_type: &'static str,
    pub effect: &'static str,
    pub sensitive: bool,
    pub overridden_by_environment: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfigChange {
    pub path: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfigUpdateResult {
    pub revision: String,
    pub changed_paths: Vec<String>,
    pub hot_update_paths: Vec<String>,
    pub restart_paths: Vec<String>,
    pub migration_paths: Vec<String>,
}

#[derive(Debug)]
pub enum SystemConfigError {
    ReadFailed,
    InvalidYaml(String),
    InvalidField(String),
    RevisionConflict,
    MigrationConfirmationRequired(Vec<String>),
    WriteFailed,
}

impl SystemConfigError {
    pub(super) fn from_store(error: SystemConfigStoreError) -> Self {
        match error {
            SystemConfigStoreError::ReadFailed => Self::ReadFailed,
            SystemConfigStoreError::RevisionConflict => Self::RevisionConflict,
            SystemConfigStoreError::WriteFailed => Self::WriteFailed,
        }
    }
}
