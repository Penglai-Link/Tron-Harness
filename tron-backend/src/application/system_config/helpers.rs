use std::path::Path;

use crate::config::AppConfig;
use serde_yaml::Value;

use super::{SystemConfigError, SystemConfigItem};

const REMOVED_CONFIG_SECTIONS: &[&str] = &["auth"];

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
        .filter(|path| !is_removed_config_path(path))
        .filter_map(|path| {
            let yaml_value = value_at_path(value, &path)?;
            let sensitive = is_sensitive(&path);
            Some(SystemConfigItem {
                value: if sensitive {
                    None
                } else {
                    serde_json::to_value(yaml_value).ok()
                },
                value_type: if yaml_value.is_null() && path.ends_with(".maxTokens") {
                    "number"
                } else {
                    value_type(yaml_value)
                },
                effect: effect(&path),
                overridden_by_environment: is_environment_overridden(&path),
                path,
                sensitive,
            })
        })
        .collect()
}

pub(super) fn remove_removed_config_sections(value: &mut Value) -> bool {
    let Some(mapping) = value.as_mapping_mut() else {
        return false;
    };

    let mut removed = false;
    for section in REMOVED_CONFIG_SECTIONS {
        removed |= mapping
            .remove(Value::String((*section).to_owned()))
            .is_some();
    }
    removed
}

fn is_removed_config_path(path: &str) -> bool {
    REMOVED_CONFIG_SECTIONS
        .iter()
        .any(|section| path == *section || path.starts_with(&format!("{section}.")))
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Sequence(_) => "array",
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
        Value::Sequence(_) if matches!(prefix, "shortTermMemory.context.providers.order" | "mineru.nodes") => {
            paths.push(prefix.to_owned());
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
    let mut current = value;
    let segments: Vec<_> = path.split('.').collect();
    for (position, segment) in segments.iter().enumerate() {
        if current.is_null() {
            *current = Value::Mapping(Default::default());
        }
        let key = segment.split('[').next().unwrap_or(segment);
        let mapping = current
            .as_mapping_mut()
            .ok_or_else(|| SystemConfigError::InvalidField(path.into()))?;
        if position + 1 == segments.len() && !segment.contains('[') {
            // Preserve supported legacy ratio aliases without writing duplicate keys.
            let alias = match key {
                "maxRatio" => Some("maxRatioBps"),
                "triggerRatio" => Some("triggerRatioBps"),
                "targetRatio" => Some("targetRatioBps"),
                "retainedHistoryRatio" => Some("retainedHistoryRatioBps"),
                _ => None,
            };
            if let Some(alias) = alias {
                mapping.remove(Value::String(alias.into()));
            }
            mapping.insert(
                Value::String(key.into()),
                serde_yaml::to_value(next)
                    .map_err(|_| SystemConfigError::InvalidField(path.into()))?,
            );
            return Ok(());
        }
        current = mapping
            .entry(Value::String(key.into()))
            .or_insert_with(|| Value::Mapping(Default::default()));
        for (start, _) in segment.match_indices('[') {
            let index = segment[start + 1..]
                .split(']')
                .next()
                .and_then(|index| index.parse::<usize>().ok())
                .ok_or_else(|| SystemConfigError::InvalidField(path.into()))?;
            current = current
                .as_sequence_mut()
                .and_then(|values| values.get_mut(index))
                .ok_or_else(|| SystemConfigError::InvalidField(path.into()))?;
        }
        if position + 1 == segments.len() {
            *current = serde_yaml::to_value(next)
                .map_err(|_| SystemConfigError::InvalidField(path.into()))?;
            return Ok(());
        }
    }
    Err(SystemConfigError::InvalidField(path.into()))
}

/// Display effective context settings, but do not materialize defaults on GET.
pub(super) fn effective_context_value(
    content: &str,
    path: &Path,
    mut value: Value,
) -> Result<Value, SystemConfigError> {
    use crate::application::short_term_memory::{ContextBlockBudgetConfig, ContextProviderPolicy};
    use serde_json::json;
    let config = AppConfig::effective_short_term_memory(content, path)
        .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
    let ratio = |bps: u32| format!("{}%", f64::from(bps) / 100.0);
    let budget = |block: &ContextBlockBudgetConfig| {
        json!({
            "maxRatio": block.max_ratio_bps.map(ratio), "maxTokens": block.max_tokens
        })
    };
    let policy = |p: &ContextProviderPolicy| {
        json!({
            "strategy": p.strategy, "triggerRatio": ratio(p.trigger_ratio_bps),
            "targetRatio": ratio(p.target_ratio_bps), "minGainTokens": p.min_gain_tokens
        })
    };
    let blocks = &config.context.blocks;
    let providers = &config.context.providers;
    for (field, projected) in [
        (
            "shortTermMemory.context.blocks",
            json!({
                "seedPromptPrefix": budget(&blocks.seed_prompt_prefix),
                "currentSessionCheckpoint": budget(&blocks.current_session_checkpoint),
                "sessionFamilyContext": budget(&blocks.session_family_context),
                "currentSessionWorkingState": budget(&blocks.current_session_working_state),
                "toolSchema": budget(&blocks.tool_schema)
            }),
        ),
        (
            "shortTermMemory.context.providers",
            json!({
                "order": providers.order, "checkpoint": policy(&providers.checkpoint),
                "family": policy(&providers.family), "working": policy(&providers.working),
                "history": policy(&providers.history), "prices": providers.prices,
                "maxAuxiliaryCallsPerStep": providers.max_auxiliary_calls_per_step
            }),
        ),
        (
            "shortTermMemory.compaction.autoEnabled",
            json!(config.compaction.auto_enabled),
        ),
        (
            "shortTermMemory.compaction.triggerRatio",
            json!(ratio(config.compaction.trigger_ratio_bps)),
        ),
        (
            "shortTermMemory.compaction.retainedHistoryRatio",
            json!(ratio(config.compaction.retained_history_ratio_bps)),
        ),
    ] {
        set_value_at_path(&mut value, field, projected)?;
    }
    let mineru=AppConfig::effective_mineru(content,path).map_err(|e|SystemConfigError::InvalidYaml(e.to_string()))?;
    for (field,projected) in [
        ("mineru.ocrMode",json!(mineru.ocr_mode)),("mineru.parserProfile",json!(mineru.parser_profile)),
        ("mineru.expectedParserVersion",json!(mineru.expected_parser_version)),("mineru.nodes",json!(mineru.nodes)),
        ("mineru.ingestion",json!(mineru.ingestion)),
        ("mineru.enabled",json!(mineru.enabled)),("mineru.baseUrl",json!(mineru.base_url)),
        ("mineru.visualContextEnabled",json!(mineru.visual_context_enabled)),
        ("mineru.imageDescription.enabled",json!(mineru.image_description.enabled)),
        ("mineru.imageDescription.modelCode",json!(mineru.image_description.model_code)),
        ("mineru.imageDescription.maxImagesPerDocument",json!(mineru.image_description.max_images_per_document)),
        ("mineru.imageDescription.timeoutSeconds",json!(mineru.image_description.timeout_seconds)),
        ("mineru.defaultTier",json!(mineru.default_tier)),("mineru.timeoutSeconds",json!(mineru.timeout_seconds)),("mineru.maxBytes",json!(mineru.max_bytes))
    ] {set_value_at_path(&mut value,field,projected)?;}
    Ok(value)
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
