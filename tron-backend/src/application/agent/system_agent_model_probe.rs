use std::{path::Path, process::Command};

use serde_json::Value;

use crate::domain::agent::SystemAgentModelRecord;

pub(super) trait SystemCommandProbe: Send + Sync {
    fn is_available(&self, command_name: &str) -> Result<bool, String>;
    fn list_models(
        &self,
        agent_code: &str,
        command_name: &str,
    ) -> Result<Vec<SystemAgentModelRecord>, String>;
}

#[derive(Debug, Clone, Default)]
pub(super) struct ProcessSystemCommandProbe;

impl SystemCommandProbe for ProcessSystemCommandProbe {
    fn is_available(&self, command_name: &str) -> Result<bool, String> {
        let command_name = command_name.trim();
        if command_name.is_empty() {
            return Ok(false);
        }
        if command_name.contains(std::path::MAIN_SEPARATOR) {
            let path = Path::new(command_name);
            if !path.is_file() {
                return Ok(false);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                return path
                    .metadata()
                    .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
                    .map_err(|e| e.to_string());
            }
            #[cfg(not(unix))]
            return Ok(true);
        }
        let output = Command::new(command_discovery_command())
            .arg(command_name)
            .output()
            .map_err(|error| error.to_string())?;
        Ok(output.status.success())
    }

    fn list_models(
        &self,
        agent_code: &str,
        command_name: &str,
    ) -> Result<Vec<SystemAgentModelRecord>, String> {
        let agent_code = agent_code.trim().to_ascii_lowercase();
        let command_name = command_name.trim();
        if command_name.is_empty() {
            return Ok(Vec::new());
        }
        if agent_code == "dsh" {
            let catalog = eliteforge_agent_capability_external::DshModelProbe::discover_blocking(
                command_name,
                eliteforge_agent_capability_external::DshCliOptions {
                    reuse_stored_credentials: true,
                    provider_profile: Some("desktop".into()),
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
            if catalog.models.is_empty() && !catalog.errors.is_empty() {
                return Err(catalog.errors.join("; "));
            }
            return Ok(catalog
                .models
                .into_iter()
                .map(|m| SystemAgentModelRecord {
                    model_id: format!("{}/{}", m.provider, m.model),
                    display_name: m.display_name,
                    context_window_tokens: m.context_window_tokens,
                    supported_reasoning_efforts: crate::domain::reasoning::ReasoningEffort::ALL
                        .into_iter()
                        .filter(|effort| {
                            m.reasoning_efforts.iter().any(|id| {
                                id.eq_ignore_ascii_case(if effort.provider_value() == "none" {
                                    "off"
                                } else {
                                    effort.provider_value()
                                })
                            })
                        })
                        .collect(),
                })
                .collect());
        }
        let command_plans = command_plans_for_agent(&agent_code);
        if command_plans.is_empty() {
            return Ok(Vec::new());
        }

        let mut failures = Vec::new();
        for plan in command_plans {
            let plan_display = plan.join(" ");
            let output = Command::new(command_name)
                .args(&plan)
                .env("NO_COLOR", "1")
                .output()
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                failures.push(format!("`{plan_display}`"));
                continue;
            }
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let models = parse_system_agent_models_output(&stdout);
            if !models.is_empty() {
                return Ok(models);
            }
        }

        if failures.is_empty() {
            Ok(Vec::new())
        } else {
            Err(format!(
                "model listing commands returned no usable output: {}",
                failures.join(", ")
            ))
        }
    }
}

#[cfg(target_os = "windows")]
const fn command_discovery_command() -> &'static str {
    "where"
}

#[cfg(not(target_os = "windows"))]
const fn command_discovery_command() -> &'static str {
    "which"
}

fn command_plans_for_agent(agent_code: &str) -> Vec<Vec<&'static str>> {
    match agent_code {
        "codex" => vec![
            vec!["debug", "models"],
            vec!["models", "--json"],
            vec!["model", "list", "--json"],
            vec!["models"],
        ],
        "opencode" => vec![
            vec!["models", "--format", "json"],
            vec!["models", "--json"],
            vec!["models"],
        ],
        _ => Vec::new(),
    }
}

fn parse_system_agent_models_output(output: &str) -> Vec<SystemAgentModelRecord> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        let parsed = parse_models_from_json_value(&value);
        if !parsed.is_empty() {
            return deduplicate_models(parsed);
        }
    }

    let mut records = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('{') {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                records.extend(parse_models_from_json_value(&value));
            }
            continue;
        }
        if line.starts_with('[') || line.starts_with("model") || line.starts_with("id") {
            continue;
        }
        records.push(SystemAgentModelRecord {
            supported_reasoning_efforts: Vec::new(),
            model_id: line.to_string(),
            display_name: line.to_string(),
            context_window_tokens: None,
        });
    }
    deduplicate_models(records)
}

fn parse_models_from_json_value(value: &Value) -> Vec<SystemAgentModelRecord> {
    match value {
        Value::Array(items) => items
            .iter()
            .flat_map(parse_models_from_json_value)
            .collect(),
        Value::Object(object) => {
            if let Some(list) = object
                .get("models")
                .or_else(|| object.get("data"))
                .or_else(|| object.get("items"))
                .or_else(|| object.get("records"))
            {
                return parse_models_from_json_value(list);
            }

            let model_id = object
                .get("id")
                .or_else(|| object.get("model"))
                .or_else(|| object.get("modelId"))
                .or_else(|| object.get("model_id"))
                .or_else(|| object.get("slug"))
                .or_else(|| object.get("name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());

            let display_name = object
                .get("displayName")
                .or_else(|| object.get("display_name"))
                .or_else(|| object.get("display"))
                .or_else(|| object.get("title"))
                .or_else(|| object.get("name"))
                .or_else(|| object.get("model"))
                .or_else(|| object.get("id"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let context_window_tokens = parse_context_window_tokens_from_object(object);

            match model_id {
                Some(model_id) => vec![SystemAgentModelRecord {
                    supported_reasoning_efforts: parse_reasoning_efforts(object),
                    model_id: model_id.to_string(),
                    display_name: display_name.unwrap_or(model_id).to_string(),
                    context_window_tokens,
                }],
                None => Vec::new(),
            }
        }
        Value::String(value) => {
            let model_id = value.trim();
            if model_id.is_empty() {
                Vec::new()
            } else {
                vec![SystemAgentModelRecord {
                    supported_reasoning_efforts: Vec::new(),
                    model_id: model_id.to_string(),
                    display_name: model_id.to_string(),
                    context_window_tokens: None,
                }]
            }
        }
        _ => Vec::new(),
    }
}

fn parse_context_window_tokens_from_object(object: &serde_json::Map<String, Value>) -> Option<u32> {
    let top_level = [
        "contextWindowTokens",
        "context_window_tokens",
        "contextWindow",
        "context_window",
        "maxContextTokens",
        "max_context_tokens",
        "maxInputTokens",
        "max_input_tokens",
    ];
    for key in top_level {
        if let Some(value) = object.get(key)
            && let Some(tokens) = parse_u32_value(value)
        {
            return Some(tokens);
        }
    }
    if let Some(context) = object
        .get("context")
        .or_else(|| object.get("limits"))
        .or_else(|| object.get("window"))
        .and_then(Value::as_object)
    {
        for key in [
            "tokens",
            "maxTokens",
            "max_tokens",
            "maxInputTokens",
            "max_input_tokens",
        ] {
            if let Some(value) = context.get(key)
                && let Some(tokens) = parse_u32_value(value)
            {
                return Some(tokens);
            }
        }
    }
    None
}

fn parse_u32_value(value: &Value) -> Option<u32> {
    match value {
        Value::Number(number) => number.as_u64().and_then(|v| u32::try_from(v).ok()),
        Value::String(value) => value.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn deduplicate_models(models: Vec<SystemAgentModelRecord>) -> Vec<SystemAgentModelRecord> {
    let mut seen = std::collections::BTreeSet::<String>::new();
    let mut deduplicated = Vec::new();
    for model in models {
        let key = model.model_id.trim().to_ascii_lowercase();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        deduplicated.push(model);
    }
    deduplicated
}

fn parse_reasoning_efforts(
    object: &serde_json::Map<String, Value>,
) -> Vec<crate::domain::reasoning::ReasoningEffort> {
    let values = object
        .get("supportedReasoningEfforts")
        .or_else(|| object.get("supportedReasoningEffort"))
        .or_else(|| object.get("supported_reasoning_levels"))
        .or_else(|| object.get("variants"));
    let names: Vec<&str> = match values {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .or_else(|| item.get("effort").and_then(Value::as_str))
            })
            .collect(),
        Some(Value::Object(items)) => items.keys().map(String::as_str).collect(),
        _ => Vec::new(),
    };
    crate::domain::reasoning::ReasoningEffort::ALL
        .into_iter()
        .filter(|effort| {
            names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(effort.provider_value()))
        })
        .collect()
}
