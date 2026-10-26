mod common;
use axum::{
    Router,
    http::{Method, StatusCode},
};
use common::{app_with_config, authenticated_request, response_json};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use tron_backend::config::AppConfig;

struct Fixture {
    path: PathBuf,
    app: Router,
    // Keep each uniquely-created directory alive until the router has dropped.
    _root: tempfile::TempDir,
}
impl Fixture {
    fn new(extra: &str) -> Self {
        let root = tempfile::Builder::new()
            .prefix("tron-context-config-")
            .tempdir()
            .unwrap();
        let path = root.path().join("application.yaml");
        fs::write(
            &path,
            format!("port: 8080\nvariableCrypto:\n  keyBase64: dGVzdC1zZWNyZXQ=\n{extra}"),
        )
        .unwrap();
        let config = AppConfig {
            database_url: root.path().join("tron.sqlite3").display().to_string(),
            graph_storage_path: root.path().join("graph.grafeo").display().to_string(),
            skills_registry_root: root.path().join("skills").display().to_string(),
            ..AppConfig::from_yaml_file(&path).unwrap()
        };
        Self {
            path,
            app: app_with_config(config),
            _root: root,
        }
    }
    async fn get(&self) -> Value {
        let response =
            authenticated_request(&self.app, Method::GET, "/api/v1/system/config", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        response_json(response).await["data"].clone()
    }
    async fn put(&self, revision: &Value, changes: Value) -> (StatusCode, Value) {
        let response = authenticated_request(
            &self.app,
            Method::PUT,
            "/api/v1/system/config",
            Some(json!({
                "revision": revision, "changes": changes, "confirmMigration": false
            })),
        )
        .await;
        (response.status(), response_json(response).await)
    }
}
fn item<'a>(document: &'a Value, path: &str) -> &'a Value {
    document["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path"] == path)
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_policy_sections_expose_effective_defaults_without_writing_or_leaking_keys() {
    let f = Fixture::new(
        "shortTermMemory:\n  context:\n    blocks:\n      seedPromptPrefix:\n        maxTokens: 3210\n",
    );
    let before = fs::read(&f.path).unwrap();
    let d = f.get().await;
    let order = &item(&d, "shortTermMemory.context.providers.order")["value"];
    assert_eq!(order.as_array().unwrap().len(), 14);
    assert_eq!(item(&d, "shortTermMemory.context.providers.dataBlocks.agent_directory.strategy")["value"], "keep");
    assert_eq!(order[11], "knowledge_base");
    assert_eq!(order[8], "page_context");
    assert_eq!(order[4], "tool_domains");
    assert_eq!(order[5], "current_session_history");
    assert_eq!(order[0], "seed_prompt_prefix");
    assert_eq!(order[2], "runtime_identity");
    assert_eq!(order[3], "dude_onboarding");
    assert_eq!(
        item(&d, "shortTermMemory.context.providers.order")["valueType"],
        "array"
    );
    assert_eq!(
        item(&d, "shortTermMemory.context.providers.history.targetRatio")["value"],
        "20%"
    );
    assert_eq!(
        item(
            &d,
            "shortTermMemory.context.blocks.seedPromptPrefix.maxTokens"
        )["value"],
        3210
    );
    assert_eq!(
        item(&d, "shortTermMemory.context.providers.working.strategy")["value"],
        "bound"
    );
    assert!(item(&d, "variableCrypto.keyBase64")["value"].is_null());
    assert!(!d.to_string().contains("dGVzdC1zZWNyZXQ="));
    assert_eq!(fs::read(&f.path).unwrap(), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saves_defaulted_order_and_policies_to_yaml_then_reloads_runtime_values() {
    let f = Fixture::new(
        "shortTermMemory:\n  context:\n    blocks:\n      seedPromptPrefix:\n        maxTokens: 3210\n",
    );
    let d = f.get().await;
    let mut order = item(&d, "shortTermMemory.context.providers.order")["value"].clone();
    let entries = order.as_array_mut().unwrap();
    let application = entries.iter().position(|id| id == "application_context").unwrap();
    let working = entries.iter().position(|id| id == "current_session_working_state").unwrap();
    entries.swap(application, working);
    let (status, result) = f.put(&d["revision"], json!([
        {"path":"shortTermMemory.context.providers.order", "value":order},
        {"path":"shortTermMemory.context.providers.dataBlocks.runtime_identity.maxTokens", "value":4096},
        {"path":r"shortTermMemory.context.providers.dataBlocks.skills\.catalog.maxTokens", "value":2048},
        {"path":"shortTermMemory.context.providers.dataBlocks.agent_directory.strategy", "value":"keep"},
        {"path":"shortTermMemory.context.providers.working.strategy", "value":"semantic"},
        {"path":"shortTermMemory.context.providers.working.triggerRatio", "value":"75%"},
        {"path":"shortTermMemory.context.providers.working.targetRatio", "value":"25%"},
        {"path":"shortTermMemory.context.providers.maxAuxiliaryCallsPerStep", "value":2},
        {"path":"shortTermMemory.context.blocks.currentSessionCheckpoint.maxTokens", "value":700}
    ])).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["data"]["restartPaths"], json!([]));
    assert!(
        result["data"]["hotUpdatePaths"]
            .as_array()
            .unwrap()
            .contains(&json!("shortTermMemory.context.providers.order"))
    );
    let loaded = AppConfig::from_yaml_file(&f.path)
        .unwrap()
        .short_term_memory;
    assert_eq!(loaded.context.providers.data_blocks["runtime_identity"].max_tokens, Some(4096));
    assert_eq!(loaded.context.providers.data_blocks["skills.catalog"].max_tokens, Some(2048));
    assert_eq!(loaded.context.providers.data_blocks["agent_directory"].policy.strategy, tron_backend::application::short_term_memory::ContextReductionStrategy::Keep);
    assert_eq!(loaded.context.providers.order[application], "current_session_working_state");
    assert_eq!(
        loaded.context.providers.working.strategy,
        tron_backend::application::short_term_memory::ContextReductionStrategy::Semantic
    );
    assert_eq!(loaded.context.providers.working.trigger_ratio_bps, 7500);
    assert_eq!(loaded.context.providers.working.target_ratio_bps, 2500);
    assert_eq!(loaded.context.providers.max_auxiliary_calls_per_step, 2);
    assert_eq!(
        loaded.context.blocks.current_session_checkpoint.max_tokens,
        Some(700)
    );
    assert_eq!(
        loaded.context.blocks.seed_prompt_prefix.max_tokens,
        Some(3210)
    );
    let raw: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&f.path).unwrap()).unwrap();
    assert!(
        raw["shortTermMemory"]["context"]["providers"]["history"].is_null(),
        "unedited defaults must not be persisted"
    );
    let reloaded = f.get().await;
    assert_eq!(
        item(&reloaded, "shortTermMemory.context.providers.order")["value"],
        order
    );
    let (status, _) = f
        .put(&d["revision"], json!([{"path":"port", "value":9000}]))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_invalid_orders_strategies_and_hysteresis_without_writing() {
    let f = Fixture::new("");
    let d = f.get().await;
    let before = fs::read(&f.path).unwrap();
    let mut duplicate = item(&d, "shortTermMemory.context.providers.order")["value"].clone();
    duplicate[1] = duplicate[0].clone();
    for change in [
        json!({"path":"shortTermMemory.context.providers.order", "value":duplicate}),
        json!({"path":"shortTermMemory.context.providers.order", "value":[]}),
        json!({"path":"shortTermMemory.context.providers.history.strategy", "value":"bound"}),
        json!({"path":"shortTermMemory.context.providers.working.strategy", "value":"checkpoint"}),
        json!({"path":"shortTermMemory.context.providers.working.targetRatio", "value":"100%"}),
        json!({"path":"shortTermMemory.context.providers.unknown", "value":1}),
    ] {
        let (status, payload) = f.put(&d["revision"], json!([change])).await;
        assert!(
            !status.is_success(),
            "invalid configuration accepted: {payload}"
        );
        assert_eq!(fs::read(&f.path).unwrap(), before);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ratio_and_history_policy_updates_preserve_the_cache_friendly_layout() {
    let f = Fixture::new(
        "shortTermMemory:\n  context:\n    blocks:\n      seedPromptPrefix:\n        maxRatioBps: 2500\n    providers:\n      history:\n        strategy: keep\n",
    );
    let d = f.get().await;
    assert_eq!(
        item(&d, "shortTermMemory.context.providers.history.strategy")["value"],
        "keep"
    );
    assert_eq!(
        item(
            &d,
            "shortTermMemory.context.blocks.seedPromptPrefix.maxRatio"
        )["value"],
        "25%"
    );
    let (status, payload) = f
        .put(
            &d["revision"],
            json!([
                {"path":"shortTermMemory.context.blocks.seedPromptPrefix.maxRatio", "value":"40%"},
                {"path":"shortTermMemory.context.providers.history.strategy", "value":"checkpoint"},
                {"path":"shortTermMemory.context.providers.history.targetRatio", "value":"30%"}
            ]),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{payload}");
    let config = AppConfig::from_yaml_file(&f.path)
        .unwrap()
        .short_term_memory;
    assert_eq!(
        config.context.blocks.seed_prompt_prefix.max_ratio_bps,
        Some(4000)
    );
    assert_eq!(config.context.providers.history.target_ratio_bps, 3000);
    assert_eq!(
        config.context.providers.history.strategy,
        tron_backend::application::short_term_memory::ContextReductionStrategy::Checkpoint
    );
    assert!(!fs::read_to_string(&f.path).unwrap().contains("maxRatioBps"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compaction_switch_saves_and_reloads_without_materializing_other_defaults() {
    let f = Fixture::new("");
    let d = f.get().await;
    assert_eq!(
        item(&d, "shortTermMemory.compaction.autoEnabled")["value"],
        true
    );
    let (status, payload) = f
        .put(
            &d["revision"],
            json!([
                {"path":"shortTermMemory.compaction.autoEnabled", "value":false}
            ]),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{payload}");
    assert!(
        !AppConfig::from_yaml_file(&f.path)
            .unwrap()
            .short_term_memory
            .compaction
            .auto_enabled
    );
    let d = f.get().await;
    assert_eq!(
        item(&d, "shortTermMemory.compaction.autoEnabled")["value"],
        false
    );
    let before = fs::read(&f.path).unwrap();
    let (status, _) = f
        .put(
            &d["revision"],
            json!([
                {"path":"shortTermMemory.context.providers.working.strategy", "value":"semantic"},
                {"path":"shortTermMemory.context.providers.working.strategy", "value":"bound"}
            ]),
        )
        .await;
    assert!(!status.is_success());
    assert_eq!(fs::read(&f.path).unwrap(), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn null_legacy_context_sections_support_default_view_and_explicit_update() {
    for extra in [
        "shortTermMemory: null\n",
        "shortTermMemory:\n  context: null\n  compaction: null\n",
        "shortTermMemory:\n  context:\n    blocks: null\n    providers: null\n",
    ] {
        let f = Fixture::new(extra);
        let before = fs::read(&f.path).unwrap();
        let d = f.get().await;
        assert_eq!(
            item(&d, "shortTermMemory.context.providers.order")["value"]
                .as_array()
                .unwrap()
                .len(),
            14
        );
        assert_eq!(fs::read(&f.path).unwrap(), before);
        let (status, payload) = f
            .put(
                &d["revision"],
                json!([
                    {"path":"shortTermMemory.context.providers.working.minGainTokens", "value":50}
                ]),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{payload}");
        assert_eq!(
            AppConfig::from_yaml_file(&f.path)
                .unwrap()
                .short_term_memory
                .context
                .providers
                .working
                .min_gain_tokens,
            50
        );
    }
}

#[test]
fn templates_and_explicitly_selected_profile_parse_without_writing() {
    for source in [
        include_str!("../application.yaml"),
        include_str!("../../electron/application.yaml"),
    ] {
        AppConfig::validate_yaml(source, "template.yaml").expect("repository template must load");
    }
    if let Ok(path) = std::env::var("TRON_CONFIG_VALIDATION_PATH") {
        let before = fs::read(&path).unwrap();
        AppConfig::validate_yaml(std::str::from_utf8(&before).unwrap(), &path)
            .expect("selected profile must load");
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_null_token_cap_survives_api_save_and_runtime_reload() {
    let f = Fixture::new("");
    let before = f.get().await;
    let path = "shortTermMemory.context.blocks.currentSessionCheckpoint.maxTokens";
    assert!(item(&before, path)["value"].is_number());
    let untouched = item(
        &before,
        "shortTermMemory.context.blocks.seedPromptPrefix.maxTokens",
    )["value"]
        .clone();
    let (status, result) = f
        .put(
            &before["revision"],
            json!([{ "path": path, "value": null }]),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    let loaded = AppConfig::from_yaml_file(&f.path)
        .unwrap()
        .short_term_memory;
    assert_eq!(
        loaded.context.blocks.current_session_checkpoint.max_tokens,
        None
    );
    let after = f.get().await;
    assert!(item(&after, path)["value"].is_null());
    assert_eq!(
        item(
            &after,
            "shortTermMemory.context.blocks.seedPromptPrefix.maxTokens"
        )["value"],
        untouched
    );
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&f.path).unwrap()).unwrap();
    assert!(
        yaml["shortTermMemory"]["context"]["blocks"]["currentSessionCheckpoint"]["maxTokens"]
            .is_null()
    );
    assert!(yaml["shortTermMemory"]["context"]["blocks"]["seedPromptPrefix"].is_null());
}

#[test]
fn stable_blocks_are_uncapped_and_domains_precede_history_in_shipped_configs() {
    for file in ["application.yaml", "../electron/application.yaml"] {
        let config = AppConfig::from_yaml_file(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file)).unwrap();
        let context = config.short_term_memory.context;
        assert_eq!(context.blocks.seed_prompt_prefix.max_tokens, None);
        assert_eq!(context.blocks.seed_prompt_prefix.max_ratio_bps, Some(10_000));
        let domains = &context.providers.data_blocks["tool_domains"];
        assert_eq!(domains.max_tokens, None);
        assert_eq!(domains.policy.strategy, tron_backend::application::short_term_memory::ContextReductionStrategy::Keep);
        let position = |id| context.providers.order.iter().position(|value| value == id).unwrap();
        assert!(position("tool_domains") < position("current_session_history"));
        context.providers.validate().unwrap();
    }
}
