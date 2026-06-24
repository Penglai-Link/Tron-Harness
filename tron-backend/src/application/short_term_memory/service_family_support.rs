use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::domain::{
    agent::{AgentKind, AgentRunContext},
    short_term_memory::{
        ShortTermMemoryFamily, ShortTermMemoryFamilyMember, ShortTermMemoryFamilySnapshot,
    },
};

use crate::application::short_term_memory::ShortTermMemoryConfig;

#[derive(Debug, Clone)]
pub(super) struct FamilyBinding {
    pub(super) family_kind: &'static str,
    pub(super) family_key: String,
    pub(super) membership_role: &'static str,
    pub(super) load_policy: &'static str,
    pub(super) load_weight: f64,
    pub(super) member_kind: &'static str,
    pub(super) origin_kind: &'static str,
    pub(super) requirement_code: Option<String>,
    pub(super) task_code: Option<String>,
    pub(super) parent_family_code: Option<String>,
}

pub(super) struct FamilyContextEntry {
    pub(super) family: ShortTermMemoryFamily,
    pub(super) member: ShortTermMemoryFamilyMember,
    pub(super) active_session_codes: Vec<String>,
    pub(super) snapshots: Vec<ShortTermMemoryFamilySnapshot>,
    pub(super) weight: f64,
    pub(super) max_sessions: usize,
    pub(super) ttl_hours: u32,
    pub(super) goal_state_enabled: bool,
    pub(super) todo_state_enabled: bool,
}

pub(super) fn resolve_bindings(
    context: &AgentRunContext,
    agent_code: &str,
    config: &ShortTermMemoryConfig,
) -> Vec<FamilyBinding> {
    let user_key = context.user_code.as_deref().unwrap_or("system");
    let input_kind = context
        .short_term_memory
        .as_ref()
        .map(|memory| memory.input_kind.as_str())
        .unwrap_or("USER_PROMPT");
    let input_metadata = context
        .short_term_memory
        .as_ref()
        .and_then(|memory| memory.input_metadata.as_ref());
    let origin_kind = match input_kind {
        "HEARTBEAT_TICK" => "HEARTBEAT_TICK",
        "TASK_TRIGGER" => "TASK_TRIGGER",
        "CONTINUATION" => "CONTINUATION",
        _ => "USER_PROMPT",
    };
    let mut bindings = vec![FamilyBinding {
        family_kind: "WORKSPACE_CHAT",
        family_key: format!("{}|{}|{}", context.workspace_code, agent_code, user_key),
        membership_role: "WORKSPACE_SCOPE",
        load_policy: "RECENT_FAMILY_SESSIONS",
        load_weight: weight(config.family_load.workspace_chat_weight_bps),
        member_kind: if context.execution_scope.task_code.is_some() {
            "TASK_CHAT"
        } else {
            "USER_CHAT"
        },
        origin_kind,
        requirement_code: None,
        task_code: context.execution_scope.task_code.clone(),
        parent_family_code: None,
    }];
    if input_kind == "HEARTBEAT_TICK" {
        bindings.push(FamilyBinding {
            family_kind: "WORKSPACE_HEARTBEAT",
            family_key: format!("{}|{}", context.workspace_code, agent_code),
            membership_role: "HEARTBEAT_SCOPE",
            load_policy: "RECENT_FAMILY_SESSIONS",
            load_weight: weight(config.family_load.workspace_heartbeat_weight_bps),
            member_kind: "HEARTBEAT_SESSION",
            origin_kind: "HEARTBEAT_TICK",
            requirement_code: None,
            task_code: None,
            parent_family_code: None,
        });
    }
    if let Some(requirement_code) = requirement_code(input_metadata) {
        bindings.push(FamilyBinding {
            family_kind: "REQUIREMENT",
            family_key: format!(
                "{}|{}|{}",
                context.workspace_code, agent_code, requirement_code
            ),
            membership_role: "REQUIREMENT_SCOPE",
            load_policy: "RECENT_FAMILY_SESSIONS",
            load_weight: weight(config.family_load.requirement_weight_bps),
            member_kind: if context.execution_scope.task_code.is_some() {
                "TASK_CHAT"
            } else {
                "USER_CHAT"
            },
            origin_kind,
            requirement_code: Some(requirement_code),
            task_code: context.execution_scope.task_code.clone(),
            parent_family_code: None,
        });
    }
    if let Some(task_code) = context.execution_scope.task_code.as_deref() {
        bindings.push(FamilyBinding {
            family_kind: "TASK",
            family_key: format!("{}|{}|{}", context.workspace_code, agent_code, task_code),
            membership_role: "PRIMARY_TASK",
            load_policy: "RECENT_FAMILY_SESSIONS",
            load_weight: weight(config.family_load.task_weight_bps),
            member_kind: "TASK_CHAT",
            origin_kind,
            requirement_code: requirement_code(input_metadata),
            task_code: Some(task_code.to_string()),
            parent_family_code: None,
        });
    }
    if let Some(fork_key) = input_metadata
        .and_then(|value| value.get("forkKey"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
    {
        bindings.push(FamilyBinding {
            family_kind: "FORK",
            family_key: format!(
                "{}|{}|{}",
                context.workspace_code,
                agent_code,
                fork_key.trim()
            ),
            membership_role: "IMPORTED_PARENT",
            load_policy: "IMPORT_PARENT_ONCE",
            load_weight: weight(config.family_load.other_weight_bps),
            member_kind: "FORK_SESSION",
            origin_kind,
            requirement_code: requirement_code(input_metadata),
            task_code: context.execution_scope.task_code.clone(),
            parent_family_code: input_metadata
                .and_then(|value| value.get("parentFamilyCode"))
                .and_then(|value| value.as_str())
                .map(str::to_string),
        });
    }
    bindings
}

pub(super) fn agent_code(context: &AgentRunContext) -> String {
    context
        .short_term_memory
        .as_ref()
        .and_then(|memory| memory.agent_code.clone())
        .or_else(|| context.execution_scope.app_code.clone())
        .unwrap_or_else(|| match context.agent_kind {
            AgentKind::React => "react-agent".into(),
            AgentKind::SystemCli => "system-cli-agent".into(),
        })
}

pub(super) fn family_code(kind: &str, key: &str) -> String {
    format!("ASF-{}", stable_hash(&[kind, key]))
}

pub(super) fn snapshot_code(family_code: &str, session_code: &str, run_code: &str) -> String {
    format!(
        "AFS-{}",
        stable_hash(&[family_code, session_code, run_code])
    )
}

pub(super) fn stable_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hex::encode(hasher.finalize())[..24].to_string()
}

pub(super) fn default_weight(config: &ShortTermMemoryConfig, family_kind: &str) -> f64 {
    match family_kind {
        "TASK" => weight(config.family_load.task_weight_bps),
        "REQUIREMENT" => weight(config.family_load.requirement_weight_bps),
        "WORKSPACE_CHAT" => weight(config.family_load.workspace_chat_weight_bps),
        "WORKSPACE_HEARTBEAT" => weight(config.family_load.workspace_heartbeat_weight_bps),
        _ => weight(config.family_load.other_weight_bps),
    }
}

fn weight(basis_points: u32) -> f64 {
    basis_points as f64 / 10_000.0
}

pub(super) fn load_limit_usize(value: Option<&Value>, key: &str, default: usize) -> usize {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub(super) fn load_limit_u32(value: Option<&Value>, key: &str, default: u32) -> u32 {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub(super) fn load_flag_bool(value: Option<&Value>, key: &str, default: bool) -> bool {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn requirement_code(input_metadata: Option<&Value>) -> Option<String> {
    input_metadata
        .and_then(|value| {
            value
                .get("requirementCode")
                .or_else(|| value.get("parentTaskCode"))
        })
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn truncate_chars(value: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let suffix = "\n...[snapshot truncated]";
    if max_chars <= suffix.chars().count() {
        return value.chars().take(max_chars).collect();
    }
    let mut text = value
        .chars()
        .take(max_chars.saturating_sub(suffix.chars().count()))
        .collect::<String>();
    text.push_str(suffix);
    text
}
