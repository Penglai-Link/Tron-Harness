use std::path::Path;

use crate::config::AppConfig;
use serde_yaml::Value;

use super::{SystemConfigError, SystemConfigItem};

const REMOVED_CONFIG_SECTIONS: &[&str] = &["auth"];
// Removed settings have no runtime consumer and must not reappear from old YAML.
const REMOVED_CONFIG_PATHS: &[&str] = &[
    "resource.limits.schemaMaxBytes",
    "resource.limits.maxDepth",
    "resource.limits.maxValidationErrors",
    "resource.limits.maxBinaries",
    "mineru.parseEndpoint",
    "mineru.backend",
    "mineru.parseMethod",
    "mineru.returnContentList",
];

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

pub(super) fn config_items(value: &Value, runtime_bound: bool) -> Vec<SystemConfigItem> {
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
                effect: effect(&path, runtime_bound),
                effect_reason: effect_reason(&path, runtime_bound),
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
    for path in REMOVED_CONFIG_PATHS {
        let (parent, key) = path.rsplit_once('.').expect("removed path has a parent");
        let parent = parent
            .split('.')
            .try_fold(&mut *value, |current, segment| current.get_mut(segment));
        if let Some(mapping) = parent.and_then(Value::as_mapping_mut) {
            removed |= mapping.remove(Value::String(key.to_owned())).is_some();
        }
    }
    removed
}

pub(super) fn is_removed_config_path(path: &str) -> bool {
    REMOVED_CONFIG_PATHS.contains(&path)
        || REMOVED_CONFIG_SECTIONS
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

fn effect(path: &str, runtime_bound: bool) -> &'static str {
    if is_migration(path) {
        "migration"
    } else if runtime_bound && super::runtime::supports_hot_update(path) {
        "hotUpdate"
    } else {
        "restart"
    }
}

fn effect_reason(path: &str, runtime_bound: bool) -> &'static str {
    match effect(path, runtime_bound) {
        "hotUpdate" if path.starts_with("shortTermMemory.") => "nextMemoryOperation",
        "hotUpdate" if path.starts_with("mineru.") => "newParsingTask",
        "hotUpdate" if path == "terminalWorkingDirectory" => "newTerminal",
        "hotUpdate" => "nextOperation",
        "migration" if path == "variableCrypto.keyBase64" => "encryptedData",
        "migration" => "storedData",
        _ if path.starts_with("httpClients.") => "httpPool",
        _ if path.starts_with("eventBus.") => "eventQueues",
        _ if path.starts_with("mineru.ingestion.") => "workerCount",
        _ if path.starts_with("systemAgents.") => "agentRegistry",
        _ if path.starts_with("remoteAccess.") => "listener",
        _ if matches!(path, "bindAddress" | "port") => "listener",
        _ if path.starts_with("resource.storage.") => "storageClients",
        _ => "startupResource",
    }
}

fn is_migration(path: &str) -> bool {
    if path.starts_with("resource.storage.profiles.")
        && matches!(
            path.rsplit('.').next(),
            Some("root" | "bucket" | "prefix" | "endpoint" | "type")
        )
    {
        return true;
    }
    matches!(
        path,
        "databaseUrl" | "graphStoragePath" | "variableCrypto.keyBase64"
    )
}

fn is_sensitive(path: &str) -> bool {
    if path == "remoteAccess.token" {
        return true;
    }
    let lowercase = path.to_ascii_lowercase();
    lowercase.contains("secret")
        || lowercase.contains("password")
        || lowercase.ends_with("keybase64")
}

fn is_environment_overridden(path: &str) -> bool {
    let variable = match path {
        "port" => Some("TRON_BACKEND_PORT"),
        "bindAddress" => Some("TRON_BACKEND_BIND_ADDRESS"),
        "databaseUrl" => Some("TRON_BACKEND_DATABASE_URL"),
        "graphStoragePath" => Some("TRON_BACKEND_GRAPH_STORAGE_PATH"),
        "frontendDistDir" => Some("TRON_BACKEND_FRONTEND_DIST_DIR"),
        "terminalWorkingDirectory" => Some("TRON_TERMINAL_WORKING_DIRECTORY"),
        _ => None,
    };
    variable.is_some_and(|name| std::env::var_os(name).is_some())
}

fn collect_leaf_paths(value: &Value, prefix: &str, paths: &mut Vec<String>) {
    match value {
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let Some(key) = key.as_str() else { continue };
                let key = key.replace('\\', "\\\\").replace('.', "\\.");
                let path = if prefix.is_empty() {
                    key.to_owned()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_leaf_paths(value, &path, paths);
            }
        }
        Value::Sequence(_)
            if matches!(
                prefix,
                "shortTermMemory.context.providers.order" | "mineru.nodes"
            ) =>
        {
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
    for segment in path_segments(path) {
        let key = segment.split('[').next().unwrap_or(&segment);
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
    let segments = path_segments(path);
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

// Contributor IDs may contain dots (for example skills.catalog). Paths escape
// mapping-key dots so edits target the same leaf displayed by config_items.
fn path_segments(path: &str) -> Vec<String> {
    let mut segments = vec![String::new()];
    let mut escaped = false;
    for character in path.chars() {
        if escaped {
            segments.last_mut().unwrap().push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '.' {
            segments.push(String::new());
        } else {
            segments.last_mut().unwrap().push(character);
        }
    }
    if escaped {
        segments.last_mut().unwrap().push('\\');
    }
    segments
}

/// Display effective settings, but do not materialize defaults on GET.
pub(super) fn effective_context_value(
    content: &str,
    path: &Path,
    mut value: Value,
) -> Result<Value, SystemConfigError> {
    use crate::application::short_term_memory::ContextProviderPolicy;
    use serde_json::json;
    let effective = AppConfig::effective_config(content, path)
        .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
    set_value_at_path(
        &mut value,
        "modelInvocation",
        json!(effective.model_invocation),
    )?;
    set_value_at_path(&mut value, "remoteAccess", json!(effective.remote_access))?;
    set_value_at_path(&mut value, "ragRetrieval", json!(effective.rag_retrieval))?;
    for (field, command) in [
        (
            "systemAgents.codexCommand",
            effective.system_agents.codex_command,
        ),
        (
            "systemAgents.opencodeCommand",
            effective.system_agents.opencode_command,
        ),
        (
            "systemAgents.dshCommand",
            effective.system_agents.dsh_command,
        ),
    ] {
        set_value_at_path(&mut value, field, json!(command))?;
    }
    let config = AppConfig::effective_short_term_memory(content, path)
        .map_err(|error| SystemConfigError::InvalidYaml(error.to_string()))?;
    let ratio = |bps: u32| format!("{}%", f64::from(bps) / 100.0);
    let policy = |p: &ContextProviderPolicy| {
        json!({
            "strategy": p.strategy, "triggerRatio": ratio(p.trigger_ratio_bps),
            "targetRatio": ratio(p.target_ratio_bps), "minGainTokens": p.min_gain_tokens
        })
    };
    let providers = &config.context.providers;
    let data_blocks = providers
        .data_blocks
        .iter()
        .map(|(id, block)| {
            let value = policy(&block.policy);
            (id.clone(), value)
        })
        .collect::<serde_json::Map<_, _>>();
    if let Some(context) = value
        .get_mut("shortTermMemory")
        .and_then(|memory| memory.get_mut("context"))
        .and_then(Value::as_mapping_mut)
    {
        context.remove(Value::String("blocks".into()));
    }
    for (field, projected) in [
        (
            "shortTermMemory.context.providers",
            json!({
                "order": providers.order, "shares": providers.shares, "dataBlocks": data_blocks,
                "checkpoint": policy(&providers.checkpoint),
                "working": policy(&providers.working),
                "history": policy(&providers.history), "prices": providers.prices,
                "maxAuxiliaryCallsPerStep": providers.max_auxiliary_calls_per_step
            }),
        ),
        (
            "shortTermMemory.goalLoop.maxRuns",
            json!(config.goal_loop.max_runs),
        ),
        (
            "shortTermMemory.compaction.autoEnabled",
            json!(config.compaction.auto_enabled),
        ),
        (
            "shortTermMemory.compaction.triggerRatio",
            json!(ratio(config.compaction.trigger_ratio_bps)),
        ),
    ] {
        set_value_at_path(&mut value, field, projected)?;
    }
    let mineru = AppConfig::effective_mineru(content, path)
        .map_err(|e| SystemConfigError::InvalidYaml(e.to_string()))?;
    for (field, projected) in [
        ("mineru.ocrMode", json!(mineru.ocr_mode)),
        ("mineru.parserProfile", json!(mineru.parser_profile)),
        (
            "mineru.expectedParserVersion",
            json!(mineru.expected_parser_version),
        ),
        ("mineru.nodes", json!(mineru.nodes)),
        ("mineru.ingestion", json!(mineru.ingestion)),
        ("mineru.enabled", json!(mineru.enabled)),
        ("mineru.baseUrl", json!(mineru.base_url)),
        (
            "mineru.visualContextEnabled",
            json!(mineru.visual_context_enabled),
        ),
        (
            "mineru.imageDescription.enabled",
            json!(mineru.image_description.enabled),
        ),
        (
            "mineru.imageDescription.modelCode",
            json!(mineru.image_description.model_code),
        ),
        (
            "mineru.imageDescription.maxImagesPerDocument",
            json!(mineru.image_description.max_images_per_document),
        ),
        (
            "mineru.imageDescription.timeoutSeconds",
            json!(mineru.image_description.timeout_seconds),
        ),
        ("mineru.defaultTier", json!(mineru.default_tier)),
        ("mineru.timeoutSeconds", json!(mineru.timeout_seconds)),
        ("mineru.maxBytes", json!(mineru.max_bytes)),
    ] {
        set_value_at_path(&mut value, field, projected)?;
    }
    Ok(value)
}

pub(super) fn classify_paths(
    paths: Vec<String>,
    runtime_bound: bool,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut hot = Vec::new();
    let mut migration = Vec::new();
    let mut restart = Vec::new();
    for path in paths {
        if is_migration(&path) {
            migration.push(path);
        } else if runtime_bound && super::runtime::supports_hot_update(&path) {
            hot.push(path);
        } else {
            restart.push(path);
        }
    }
    (hot, restart, migration)
}
