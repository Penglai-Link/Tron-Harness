use std::{path::Path, sync::Arc};

use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Clone)]
pub struct SystemConfigService {
    store: Arc<dyn SystemConfigStore>,
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

impl SystemConfigService {
    pub fn new(store: Arc<dyn SystemConfigStore>) -> Self {
        Self { store }
    }

    pub fn get(&self) -> Result<SystemConfigDocument, SystemConfigError> {
        let stored = self.store.read().map_err(SystemConfigError::from_store)?;
        let value = parse_yaml(&stored.content, Path::new(&stored.path))?;
        Ok(SystemConfigDocument {
            revision: stored.revision,
            path: stored.path,
            items: config_items(&value),
        })
    }

    pub fn update(
        &self,
        expected_revision: &str,
        changes: Vec<SystemConfigChange>,
        confirm_migration: bool,
    ) -> Result<SystemConfigUpdateResult, SystemConfigError> {
        let stored = self.store.read().map_err(SystemConfigError::from_store)?;
        if stored.revision != expected_revision {
            return Err(SystemConfigError::RevisionConflict);
        }
        let path = Path::new(&stored.path);
        let mut next_value = parse_yaml(&stored.content, path)?;
        let available_paths = leaf_paths(&next_value);
        for change in changes {
            if !available_paths.contains(&change.path) {
                return Err(SystemConfigError::InvalidField(change.path));
            }
            set_value_at_path(&mut next_value, &change.path, change.value)?;
        }
        let next_yaml = serde_yaml::to_string(&next_value)
            .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
        parse_yaml(&next_yaml, path)?;
        let current_value = parse_yaml(&stored.content, path)?;
        let changed_paths = changed_leaf_paths(&current_value, &next_value);
        let (hot_update_paths, restart_paths, migration_paths) =
            classify_paths(changed_paths.clone());
        if !confirm_migration && !migration_paths.is_empty() {
            return Err(SystemConfigError::MigrationConfirmationRequired(
                migration_paths,
            ));
        }

        if changed_paths.is_empty() {
            return Ok(SystemConfigUpdateResult {
                revision: stored.revision,
                changed_paths,
                hot_update_paths,
                restart_paths,
                migration_paths,
            });
        }

        let persisted = self
            .store
            .replace(expected_revision, &next_yaml)
            .map_err(SystemConfigError::from_store)?;

        Ok(SystemConfigUpdateResult {
            revision: persisted.revision,
            changed_paths,
            hot_update_paths,
            restart_paths,
            migration_paths,
        })
    }
}

impl SystemConfigError {
    fn from_store(error: SystemConfigStoreError) -> Self {
        match error {
            SystemConfigStoreError::ReadFailed => Self::ReadFailed,
            SystemConfigStoreError::RevisionConflict => Self::RevisionConflict,
            SystemConfigStoreError::WriteFailed => Self::WriteFailed,
        }
    }
}

fn parse_yaml(content: &str, path: &Path) -> Result<Value, SystemConfigError> {
    AppConfig::validate_yaml(content, path)
        .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
    serde_yaml::from_str(content).map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))
}

fn leaf_paths(value: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    collect_leaf_paths(value, "", &mut paths);
    paths.sort();
    paths.dedup();
    paths
}

fn config_items(value: &Value) -> Vec<SystemConfigItem> {
    leaf_paths(value)
        .into_iter()
        .filter_map(|path| {
            let yaml_value = value_at_path(value, &path)?;
            let sensitive = is_sensitive(&path);
            Some(SystemConfigItem {
                value: if sensitive {
                    None
                } else {
                    serde_json::to_value(yaml_value).ok()
                },
                value_type: value_type(yaml_value),
                effect: effect(&path),
                overridden_by_environment: is_environment_overridden(&path),
                path,
                sensitive,
            })
        })
        .collect()
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        _ => "string",
    }
}

fn effect(path: &str) -> &'static str {
    if is_migration(path) {
        "migration"
    } else {
        "restart"
    }
}

fn is_migration(path: &str) -> bool {
    path == "databaseUrl" || path == "graphStoragePath" || path == "variableCrypto.keyBase64"
}

fn is_sensitive(path: &str) -> bool {
    let lowercase = path.to_ascii_lowercase();
    lowercase.contains("secret")
        || lowercase.contains("password")
        || lowercase.ends_with("keybase64")
}

fn is_environment_overridden(path: &str) -> bool {
    let variable = match path {
        "port" => Some("TRON_BACKEND_PORT"),
        "databaseUrl" => Some("TRON_BACKEND_DATABASE_URL"),
        "graphStoragePath" => Some("TRON_BACKEND_GRAPH_STORAGE_PATH"),
        "frontendDistDir" => Some("TRON_BACKEND_FRONTEND_DIST_DIR"),
        "projectDocRoot" => Some("TRON_PROJECT_DOC_ROOT"),
        "auth.ldap.url" => Some("LDAP_URL"),
        "auth.ldap.bindDn" => Some("LDAP_BIND_DN"),
        "auth.ldap.bindPassword" => Some("LDAP_BIND_PASSWORD"),
        "auth.ldap.searchBase" => Some("LDAP_SEARCH_BASE"),
        "auth.ldap.searchFilter" => Some("LDAP_SEARCH_FILTER"),
        _ => None,
    };
    variable.is_some_and(|name| std::env::var_os(name).is_some())
}

fn collect_leaf_paths(value: &Value, prefix: &str, paths: &mut Vec<String>) {
    match value {
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let Some(key) = key.as_str() else { continue };
                let path = if prefix.is_empty() {
                    key.to_owned()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_leaf_paths(value, &path, paths);
            }
        }
        Value::Sequence(sequence) => {
            for (index, value) in sequence.iter().enumerate() {
                collect_leaf_paths(value, &format!("{prefix}[{index}]"), paths);
            }
        }
        _ => paths.push(prefix.to_owned()),
    }
}

fn changed_leaf_paths(current: &Value, next: &Value) -> Vec<String> {
    let current_paths = leaf_paths(current);
    let next_paths = leaf_paths(next);
    let mut candidates = current_paths;
    candidates.extend(next_paths);
    candidates.sort();
    candidates.dedup();
    candidates
        .into_iter()
        .filter(|path| value_at_path(current, path) != value_at_path(next, path))
        .collect()
}

fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        let key = segment.split('[').next().unwrap_or(segment);
        current = current.as_mapping()?.get(Value::String(key.to_owned()))?;
        for index in segment.match_indices('[').filter_map(|(start, _)| {
            segment[start + 1..]
                .split(']')
                .next()?
                .parse::<usize>()
                .ok()
        }) {
            current = current.as_sequence()?.get(index)?;
        }
    }
    Some(current)
}

fn set_value_at_path(
    value: &mut Value,
    path: &str,
    next: serde_json::Value,
) -> Result<(), SystemConfigError> {
    let segments: Vec<&str> = path.split('.').collect();
    let (leaf, parents) = segments
        .split_last()
        .ok_or_else(|| SystemConfigError::InvalidField(path.into()))?;
    let mut current = value;
    for segment in parents {
        current = current
            .as_mapping_mut()
            .and_then(|mapping| mapping.get_mut(Value::String((*segment).to_owned())))
            .ok_or_else(|| SystemConfigError::InvalidField(path.into()))?;
    }
    let mapping = current
        .as_mapping_mut()
        .ok_or_else(|| SystemConfigError::InvalidField(path.into()))?;
    let yaml_value =
        serde_yaml::to_value(next).map_err(|_| SystemConfigError::InvalidField(path.into()))?;
    mapping.insert(Value::String((*leaf).to_owned()), yaml_value);
    Ok(())
}

fn classify_paths(paths: Vec<String>) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut migration = Vec::new();
    let mut restart = Vec::new();
    for path in paths {
        if is_migration(&path) {
            migration.push(path);
        } else {
            restart.push(path);
        }
    }
    (Vec::new(), restart, migration)
}
