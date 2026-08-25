use std::path::Path;

use crate::config::AppConfig;
use serde_yaml::Value;

use super::{SystemConfigError, SystemConfigItem};

pub(super) fn parse_yaml(content: &str, path: &Path) -> Result<Value, SystemConfigError> {
    AppConfig::validate_yaml(content, path)
        .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
    serde_yaml::from_str(content).map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))
}

pub(super) fn leaf_paths(value: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    collect_leaf_paths(value, "", &mut paths);
    paths.sort();
    paths.dedup();
    paths
}

pub(super) fn config_items(value: &Value) -> Vec<SystemConfigItem> {
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
    matches!(
        path,
        "databaseUrl" | "graphStoragePath" | "variableCrypto.keyBase64"
    )
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

pub(super) fn changed_leaf_paths(current: &Value, next: &Value) -> Vec<String> {
    let mut candidates = leaf_paths(current);
    candidates.extend(leaf_paths(next));
    candidates.sort();
    candidates.dedup();
    candidates
        .into_iter()
        .filter(|path| value_at_path(current, path) != value_at_path(next, path))
        .collect()
}

pub(super) fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
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

pub(super) fn set_value_at_path(
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

pub(super) fn classify_paths(paths: Vec<String>) -> (Vec<String>, Vec<String>, Vec<String>) {
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
