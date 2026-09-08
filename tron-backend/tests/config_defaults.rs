use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tron_backend::{application::short_term_memory::ShortTermMemoryConfig, config::AppConfig};

const APPLICATION_VARIABLE_CRYPTO_KEY_BASE64: &str = "sbxNdQm4XiwQyfXNEtjZCQ==";
static TEMP_CONFIG_COUNTER: AtomicU64 = AtomicU64::new(1);

#[test]
fn application_yaml_uses_home_storage_defaults() {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("application.yaml");
    let config = AppConfig::from_yaml_file(config_path).expect("application yaml should load");

    assert_eq!(
        PathBuf::from(config.database_url),
        database_url_expected_from_environment()
    );
    assert_eq!(
        PathBuf::from(config.graph_storage_path),
        graph_storage_home_default()
    );
    assert_eq!(
        config.variable_crypto.key_base64,
        APPLICATION_VARIABLE_CRYPTO_KEY_BASE64
    );
    assert_eq!(
        config
            .short_term_memory
            .context
            .blocks
            .seed_prompt_prefix
            .max_ratio_bps,
        Some(5000)
    );
    assert_eq!(
        config
            .short_term_memory
            .context
            .blocks
            .seed_prompt_prefix
            .max_tokens,
        Some(16000)
    );
    assert!(
        !config
            .short_term_memory
            .context
            .reasoning_summary_replay_enabled
    );
    assert_eq!(config.short_term_memory.event_log.mode, "full");
    assert_eq!(config.short_term_memory.event_log.retention_days, 5);
    assert_eq!(config.short_term_memory.event_log.max_payload_bytes, 8192);
    assert!(config.short_term_memory.event_log.cleanup_on_startup);
    assert_eq!(config.short_term_memory.compaction.trigger_ratio_bps, 8000);
    assert_eq!(
        config
            .short_term_memory
            .compaction
            .retained_history_ratio_bps,
        2000
    );
    assert_eq!(
        config.mineru.base_url,
        "https://pre-platform.cisdigital.cn/mineru"
    );
    assert_eq!(config.mineru.parse_endpoint, "/file_parse");
}

#[test]
fn graph_storage_path_yaml_override_still_wins() {
    let config_path = temp_config_path();
    fs::write(
        &config_path,
        "graphStoragePath: /tmp/custom-tron.graph.grafeo\n",
    )
    .expect("temp config should be written");

    let config = AppConfig::from_yaml_file(&config_path).expect("temp config should load");

    fs::remove_file(&config_path).expect("temp config should be removed");
    assert_eq!(config.graph_storage_path, "/tmp/custom-tron.graph.grafeo");
}

#[test]
fn mineru_and_document_parsing_config_load_from_yaml() {
    let config_path = temp_config_path();
    fs::write(
        &config_path,
        r#"
httpClients:
  documentParsing:
    connectTimeoutMs: 2500
    poolMaxIdlePerHost: 4
mineru:
  enabled: true
  baseUrl: http://127.0.0.1:18000
  parseEndpoint: /file_parse
  timeoutSeconds: 120
  maxBytes: 2097152
  backend: pipeline
  parseMethod: auto
  returnContentList: true
"#,
    )
    .expect("temp config should be written");

    let config = AppConfig::from_yaml_file(&config_path).expect("temp config should load");

    fs::remove_file(&config_path).expect("temp config should be removed");
    assert_eq!(
        config.http_clients.document_parsing.connect_timeout_ms,
        2500
    );
    assert_eq!(
        config.http_clients.document_parsing.pool_max_idle_per_host,
        4
    );
    assert_eq!(config.mineru.base_url, "http://127.0.0.1:18000");
    assert_eq!(config.mineru.timeout_seconds, 120);
    assert_eq!(config.mineru.max_bytes, 2_097_152);
    assert!(config.mineru.return_content_list);
}

#[test]
fn short_term_memory_yaml_overrides_load_policy() {
    let config_path = temp_config_path();
    fs::write(
        &config_path,
        r#"
shortTermMemory:
  toolOutput:
    root: /tmp/stm-tools
    externalizeAboveBytes: 64
  eventLog:
    mode: lifecycleOnly
    retentionDays: 9
    maxPayloadBytes: 4096
    cleanupOnStartup: false
    cleanupBatchSize: 123
    cleanupMaxRowsPerStartup: 456
  context:
    defaultContextWindowTokens: 16000
    familyContextMaxRatioBps: 750
    familyContextHardCapTokens: 2000
    familyContextMaxChars: 1200
    reasoningSummaryReplayEnabled: true
    blocks:
      seedPromptPrefix:
        maxRatioBps: 2500
        maxTokens: 700
      currentSessionCheckpoint:
        maxRatioBps: 1250
        maxTokens: 900
      currentSessionWorkingState:
        maxTokens: 300
  familyLoad:
    defaultMaxSnapshots: 2
    defaultMaxSessions: 4
    defaultSnapshotMaxChars: 700
    defaultTtlHours: 24
    taskWeightBps: 7000
    requirementWeightBps: 2000
    workspaceChatWeightBps: 500
    workspaceHeartbeatWeightBps: 250
    otherWeightBps: 100
  compaction:
    triggerRatioBps: 7500
    retainedHistoryRatioBps: 1250
  goalLoop:
    maxRuns: 7
    maxConsecutiveJudgeFailures: 2
    judgeInputMaxChars: 2000
    judgeOutputMaxTokens: 512
"#,
    )
    .expect("temp config should be written");

    let config = AppConfig::from_yaml_file(&config_path).expect("temp config should load");

    fs::remove_file(&config_path).expect("temp config should be removed");
    assert_eq!(config.short_term_memory.tool_output.root, "/tmp/stm-tools");
    assert_eq!(
        config.short_term_memory.tool_output.externalize_above_bytes,
        64
    );
    assert_eq!(config.short_term_memory.event_log.mode, "lifecycleOnly");
    assert_eq!(config.short_term_memory.event_log.retention_days, 9);
    assert_eq!(config.short_term_memory.event_log.max_payload_bytes, 4096);
    assert!(!config.short_term_memory.event_log.cleanup_on_startup);
    assert_eq!(config.short_term_memory.event_log.cleanup_batch_size, 123);
    assert_eq!(
        config
            .short_term_memory
            .event_log
            .cleanup_max_rows_per_startup,
        456
    );
    assert_eq!(
        config
            .short_term_memory
            .context
            .blocks
            .seed_prompt_prefix
            .max_tokens,
        Some(700)
    );
    assert_eq!(
        config
            .short_term_memory
            .context
            .family_context_max_ratio_bps,
        750
    );
    assert!(
        config
            .short_term_memory
            .context
            .reasoning_summary_replay_enabled
    );
    assert_eq!(
        config.short_term_memory.family_load.default_max_snapshots,
        2
    );
    assert_eq!(config.short_term_memory.family_load.default_max_sessions, 4);
    assert_eq!(config.short_term_memory.family_load.task_weight_bps, 7000);
    assert_eq!(
        config
            .short_term_memory
            .context
            .blocks
            .current_session_checkpoint
            .max_tokens,
        Some(900)
    );
    assert!(config.short_term_memory.compaction.auto_enabled);
    assert_eq!(config.short_term_memory.compaction.trigger_ratio_bps, 7500);
    assert_eq!(
        config
            .short_term_memory
            .compaction
            .retained_history_ratio_bps,
        1250
    );
    assert_eq!(config.short_term_memory.goal_loop.max_runs, 7);
    assert_eq!(
        config.short_term_memory.goal_loop.judge_output_max_tokens,
        512
    );
}

#[test]
fn short_term_memory_block_ratios_use_effective_input_budget() {
    let config = ShortTermMemoryConfig::default();

    let plan = config.model_context_budget_plan(Some(2_000), Some(500));

    assert_eq!(plan.effective_input_budget, 1_500);
    assert_eq!(
        plan.block_budgets
            .current_session_checkpoint
            .effective_limit_tokens,
        300
    );
    assert_eq!(
        plan.block_budgets
            .session_family_context
            .effective_limit_tokens,
        150
    );
}

fn load_short_term_memory_yaml(yaml: &str) -> Result<AppConfig, tron_backend::config::ConfigError> {
    let path = temp_config_path();
    fs::write(&path, yaml).unwrap();
    let result = AppConfig::from_yaml_file(&path);
    fs::remove_file(path).unwrap();
    result
}

#[test]
fn short_term_memory_percentages_preserve_ratio_and_weight_meaning() {
    let config = load_short_term_memory_yaml(
        r#"
shortTermMemory:
  context:
    familyContextMaxRatio: "7.50%"
    blocks:
      seedPromptPrefix:
        maxRatio: "50%"
      currentSessionCheckpoint:
        maxRatio: "20%"
      currentSessionWorkingState:
        maxRatio: "0.01%"
      toolSchema:
        maxRatio: "20%"
        maxTokens: 6000
  familyLoad:
    taskWeight: "65%"
    requirementWeight: "25%"
    workspaceChatWeight: "10%"
    workspaceHeartbeatWeight: "10%"
    otherWeight: "150%"
  compaction:
    triggerRatio: "80%"
    retainedHistoryRatio: "12.34%"
"#,
    )
    .unwrap()
    .short_term_memory;
    assert_eq!(config.context.family_context_max_ratio_bps, 750);
    assert_eq!(
        config.context.blocks.session_family_context.max_ratio_bps,
        Some(750)
    );
    assert_eq!(
        config.context.blocks.seed_prompt_prefix.max_ratio_bps,
        Some(5000)
    );
    assert_eq!(
        config
            .context
            .blocks
            .current_session_checkpoint
            .max_ratio_bps,
        Some(2000)
    );
    assert_eq!(
        config
            .context
            .blocks
            .current_session_working_state
            .max_ratio_bps,
        Some(1)
    );
    assert_eq!(config.family_load.task_weight_bps, 6500);
    assert_eq!(config.family_load.requirement_weight_bps, 2500);
    assert_eq!(config.family_load.workspace_chat_weight_bps, 1000);
    assert_eq!(config.family_load.workspace_heartbeat_weight_bps, 1000);
    assert_eq!(
        config.family_load.other_weight_bps, 15000,
        "weights are relative, not capped ratios"
    );
    assert_eq!(config.compaction.trigger_ratio_bps, 8000);
    assert_eq!(config.compaction.retained_history_ratio_bps, 1234);
    assert_eq!(
        config
            .model_context_budget_plan(Some(32_000), Some(4_000))
            .block_budgets
            .tool_schema
            .effective_limit_tokens,
        5600
    );
    assert_eq!(
        config
            .model_context_budget_plan(Some(100_000), Some(0))
            .block_budgets
            .tool_schema
            .effective_limit_tokens,
        6000
    );
}

#[test]
fn short_term_memory_percentages_reject_conflicting_aliases_in_both_orders() {
    for (section, name, legacy) in [
        (
            "  context:\n",
            "familyContextMaxRatio",
            "familyContextMaxRatioBps",
        ),
        (
            "  context:\n    blocks:\n      toolSchema:\n",
            "maxRatio",
            "maxRatioBps",
        ),
        ("  familyLoad:\n", "taskWeight", "taskWeightBps"),
        (
            "  familyLoad:\n",
            "requirementWeight",
            "requirementWeightBps",
        ),
        (
            "  familyLoad:\n",
            "workspaceChatWeight",
            "workspaceChatWeightBps",
        ),
        (
            "  familyLoad:\n",
            "workspaceHeartbeatWeight",
            "workspaceHeartbeatWeightBps",
        ),
        ("  familyLoad:\n", "otherWeight", "otherWeightBps"),
        ("  compaction:\n", "triggerRatio", "triggerRatioBps"),
        (
            "  compaction:\n",
            "retainedHistoryRatio",
            "retainedHistoryRatioBps",
        ),
    ] {
        let indent = if name == "maxRatio" {
            "        "
        } else {
            "    "
        };
        for fields in [
            format!("{indent}{name}: \"20%\"\n{indent}{legacy}: 2000\n"),
            format!("{indent}{legacy}: 2000\n{indent}{name}: \"20%\"\n"),
            format!("{indent}{name}: null\n{indent}{legacy}: 2000\n"),
        ] {
            let error =
                load_short_term_memory_yaml(&format!("shortTermMemory:\n{section}{fields}"))
                    .unwrap_err();
            match error {
                tron_backend::config::ConfigError::ConfigFileInvalid { source, .. } => {
                    assert!(
                        source.to_string().contains("duplicate field"),
                        "{name}: {source}"
                    );
                }
                error => panic!("unexpected configuration error for {name}: {error:?}"),
            }
        }
    }
}

#[test]
fn short_term_memory_percentages_reject_invalid_or_rounded_values() {
    for value in [
        "\"101%\"",
        "\"-1%\"",
        "\"NaN%\"",
        "\"20\"",
        "\"1.234%\"",
        "\"1e1%\"",
        "\"1.%\"",
        "\"%\"",
        "\"999999999999999999999%\"",
        "-1",
        "1.5",
    ] {
        let yaml = format!(
            "shortTermMemory:\n  context:\n    blocks:\n      toolSchema:\n        maxRatio: {value}\n"
        );
        assert!(
            load_short_term_memory_yaml(&yaml).is_err(),
            "accepted {value}"
        );
    }
    let zero = load_short_term_memory_yaml(
        "shortTermMemory:\n  context:\n    blocks:\n      toolSchema:\n        maxRatio: \"0%\"\n",
    )
    .unwrap();
    assert_eq!(
        zero.short_term_memory
            .model_context_budget_plan(Some(32_000), None)
            .block_budgets
            .tool_schema
            .effective_limit_tokens,
        0
    );
}

#[test]
fn short_term_memory_tool_schema_legacy_bps_and_large_windows_remain_supported() {
    let config = load_short_term_memory_yaml(
        "shortTermMemory:\n  context:\n    blocks:\n      toolSchema:\n        maxRatioBps: 5000\n",
    )
    .unwrap();
    assert_eq!(
        config
            .short_term_memory
            .context
            .blocks
            .tool_schema
            .max_ratio_bps,
        Some(5000)
    );
    assert_eq!(
        config
            .short_term_memory
            .model_context_budget_plan(Some(2_000_000), Some(0))
            .block_budgets
            .tool_schema
            .effective_limit_tokens,
        1_000_000
    );
}

#[test]
fn short_term_memory_yaml_rejects_removed_compaction_fields() {
    let config_path = temp_config_path();
    fs::write(
        &config_path,
        r#"
shortTermMemory:
  compaction:
    maxRecentTurns: 12
"#,
    )
    .expect("temp config should be written");

    let result = AppConfig::from_yaml_file(&config_path);

    fs::remove_file(&config_path).expect("temp config should be removed");
    assert!(result.is_err());
}

fn database_url_expected_from_environment() -> PathBuf {
    env::var("TRON_BACKEND_DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(normalize_storage_path)
        .unwrap_or_else(database_home_default)
}

fn database_home_default() -> PathBuf {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".tron/tron.sqlite3"))
        .unwrap_or_else(|_| PathBuf::from(".tron/tron.sqlite3"))
}

fn graph_storage_home_default() -> PathBuf {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".tron/tron.graph.grafeo"))
        .unwrap_or_else(|_| PathBuf::from(".tron/tron.graph.grafeo"))
}

fn normalize_storage_path(value: String) -> PathBuf {
    if value == ":memory:" || value.starts_with("file:") {
        return PathBuf::from(value);
    }
    if value == "~" {
        return env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return env::var("HOME")
            .map(|home| PathBuf::from(home).join(rest))
            .unwrap_or_else(|_| PathBuf::from(rest));
    }
    PathBuf::from(value)
}

fn temp_config_path() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let counter = TEMP_CONFIG_COUNTER.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "tron-config-{}-{suffix}-{counter}.yaml",
        std::process::id()
    ))
}
