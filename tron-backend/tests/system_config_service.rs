mod common;

use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use tron_backend::adapters::outbound::config::YamlSystemConfigStore;
use tron_backend::application::system_config::{
    BackendRuntimeConfig, StoredSystemConfig, SystemConfigChange, SystemConfigError,
    SystemConfigService, SystemConfigStore, SystemConfigStoreError,
};
use tron_backend::config::AppConfig;

#[test]
fn exposes_structured_items_and_masks_sensitive_values() {
    let fixture = ConfigFixture::new("structured");
    let document = fixture.service.get().expect("configuration should load");

    assert!(
        document
            .items
            .iter()
            .any(|item| item.path == "port" && item.value_type == "number")
    );
    let secret = document
        .items
        .iter()
        .find(|item| item.path == "variableCrypto.keyBase64")
        .expect("secret item should exist");
    assert!(secret.sensitive);
    assert!(secret.value.is_none());
    assert!(
        !document
            .items
            .iter()
            .any(|item| item.path.starts_with("auth."))
    );
    assert!(
        !fs::read_to_string(&fixture.path)
            .expect("yaml should be readable")
            .contains("auth:")
    );
}

#[test]
fn updates_yaml_with_revision_control_and_reports_restart_effect() {
    let fixture = ConfigFixture::new("update");
    let document = fixture.service.get().expect("configuration should load");
    let result = fixture
        .service
        .update(
            &document.revision,
            vec![SystemConfigChange {
                path: "port".into(),
                value: 8181.into(),
            }],
            false,
        )
        .expect("configuration should update");

    assert_eq!(result.changed_paths, vec!["port"]);
    assert_eq!(result.restart_paths, vec!["port"]);
    assert!(
        fs::read_to_string(&fixture.path)
            .expect("yaml should be readable")
            .contains("port: 8181")
    );

    let conflict = fixture.service.update(
        &document.revision,
        vec![SystemConfigChange {
            path: "port".into(),
            value: 8282.into(),
        }],
        false,
    );
    assert!(matches!(conflict, Err(SystemConfigError::RevisionConflict)));
}

#[test]
fn migration_fields_require_explicit_confirmation() {
    let fixture = ConfigFixture::new("migration");
    let document = fixture.service.get().expect("configuration should load");
    let change = || SystemConfigChange {
        path: "databaseUrl".into(),
        value: "next.sqlite3".into(),
    };

    let rejected = fixture
        .service
        .update(&document.revision, vec![change()], false);
    assert!(
        matches!(rejected, Err(SystemConfigError::MigrationConfirmationRequired(paths)) if paths == vec!["databaseUrl"])
    );

    let result = fixture
        .service
        .update(&document.revision, vec![change()], true)
        .expect("confirmed migration change should persist");
    assert_eq!(result.migration_paths, vec!["databaseUrl"]);
    assert!(result.restart_paths.is_empty());
}

#[test]
fn classifies_all_setting_groups_and_omits_unused_legacy_limits() {
    let fixture = ConfigFixture::new("effects");
    let document = fixture.service.get().unwrap();
    for (path, effect) in [
        ("mineru.baseUrl", "hotUpdate"),
        ("mineru.imageDescription.enabled", "hotUpdate"),
        ("mineru.visualContextEnabled", "hotUpdate"),
        ("mineru.ingestion.pollMs", "hotUpdate"),
        (
            "shortTermMemory.context.blocks.toolSchema.maxRatio",
            "hotUpdate",
        ),
        ("shortTermMemory.compaction.autoEnabled", "hotUpdate"),
        ("resource.limits.maxBinaryBytes", "hotUpdate"),
        ("resource.connectionVerificationTimeoutMs", "hotUpdate"),
        ("navigation.maxDepth", "hotUpdate"),
        ("terminalWorkingDirectory", "hotUpdate"),
        ("httpClients.default.connectTimeoutMs", "restart"),
        ("eventBus.standardIngressCapacity", "restart"),
        ("systemAgents.codexCommand", "restart"),
        ("mineru.ingestion.parseCapacity", "restart"),
        ("port", "restart"),
        ("resource.storage.profiles.local-default.root", "migration"),
        ("variableCrypto.keyBase64", "migration"),
    ] {
        let item = document
            .items
            .iter()
            .find(|item| item.path == path)
            .unwrap();
        assert_eq!(item.effect, effect, "{path}");
        assert!(!item.effect_reason.is_empty());
    }
    assert!(!document.items.iter().any(|item| matches!(
        item.path.as_str(),
        "resource.limits.schemaMaxBytes"
            | "resource.limits.maxDepth"
            | "resource.limits.maxValidationErrors"
            | "resource.limits.maxBinaries"
            | "shortTermMemory.context.familyContextMaxChars"
    )));
}

#[test]
fn mixed_save_publishes_hot_settings_and_retains_boot_time_resources() {
    let fixture = ConfigFixture::new("mixed");
    let document = fixture.service.get().unwrap();
    let before = fixture.runtime.snapshot();
    let result = fixture
        .service
        .update(
            &document.revision,
            vec![
                change("navigation.maxDepth", 6.into()),
                change("resource.limits.maxBinaryBytes", 512.into()),
                change("resource.connectionVerificationTimeoutMs", 42.into()),
                change("shortTermMemory.goalLoop.maxRuns", 4.into()),
                change(
                    "shortTermMemory.context.blocks.toolSchema.maxRatio",
                    "25%".into(),
                ),
                change("mineru.visualContextEnabled", true.into()),
                change("mineru.imageDescription.enabled", true.into()),
                change("mineru.ingestion.pollMs", 1500.into()),
                change("mineru.ingestion.parseCapacity", 7.into()),
                change("httpClients.default.connectTimeoutMs", 5000.into()),
                change("eventBus.standardIngressCapacity", 32.into()),
                change("port", 8181.into()),
            ],
            false,
        )
        .unwrap();
    let active = fixture.runtime.snapshot();
    assert_eq!(active.navigation.max_depth, 6);
    assert_eq!(active.resource.limits.max_binary_bytes, 512);
    assert_eq!(active.resource.connection_verification_timeout_ms, 42);
    assert_eq!(active.short_term_memory.goal_loop.max_runs, 4);
    assert_eq!(
        active
            .short_term_memory
            .context
            .blocks
            .tool_schema
            .max_ratio_bps,
        Some(2500)
    );
    assert!(active.mineru.visual_context_enabled);
    assert!(active.mineru.image_description.enabled);
    assert_eq!(active.mineru.ingestion.poll_ms, 1500);
    assert_eq!(
        active.mineru.ingestion.parse_capacity,
        before.mineru.ingestion.parse_capacity
    );
    assert_eq!(active.http_clients, before.http_clients);
    assert_eq!(active.event_bus, before.event_bus);
    assert_eq!(active.port, before.port);
    assert_eq!(result.hot_update_paths.len(), 8);
    assert_eq!(result.restart_paths.len(), 4);
    assert!(
        fixture
            .service
            .get()
            .unwrap()
            .items
            .iter()
            .any(|item| item.path == "mineru.ingestion.parseCapacity"
                && item.value == Some(7.into()))
    );
    let stale = fixture.service.update(
        &document.revision,
        vec![change("navigation.maxDepth", 9.into())],
        false,
    );
    assert!(matches!(stale, Err(SystemConfigError::RevisionConflict)));
    assert_eq!(fixture.runtime.snapshot(), active);
}

#[test]
fn invalid_save_and_failed_persistence_leave_runtime_unchanged() {
    let fixture = ConfigFixture::new("failure");
    let document = fixture.service.get().unwrap();
    let before = fixture.runtime.snapshot();
    let before_yaml = fs::read_to_string(&fixture.path).unwrap();
    let invalid = fixture.service.update(
        &document.revision,
        vec![change("navigation.maxDepth", 0.into())],
        false,
    );
    assert!(matches!(invalid, Err(SystemConfigError::InvalidYaml(_))));
    assert_eq!(fixture.runtime.snapshot(), before);
    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), before_yaml);
    struct RejectWrites(YamlSystemConfigStore);
    impl SystemConfigStore for RejectWrites {
        fn read(&self) -> Result<StoredSystemConfig, SystemConfigStoreError> {
            self.0.read()
        }
        fn replace(&self, _: &str, _: &str) -> Result<StoredSystemConfig, SystemConfigStoreError> {
            Err(SystemConfigStoreError::WriteFailed)
        }
    }
    let service = SystemConfigService::new(Arc::new(RejectWrites(YamlSystemConfigStore::new(
        &fixture.path,
    ))))
    .with_runtime_config(fixture.runtime.clone());
    assert!(matches!(
        service.update(
            &document.revision,
            vec![change("navigation.maxDepth", 9.into())],
            false
        ),
        Err(SystemConfigError::WriteFailed)
    ));
    assert_eq!(fixture.runtime.snapshot(), before);
    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), before_yaml);
}

#[tokio::test]
async fn mineru_requests_switch_servers_without_restart_and_task_snapshots_stay_pinned() {
    use axum::{Json, Router, routing::get};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tron_backend::{
        adapters::outbound::mineru_documents::RuntimeMineruDocumentParser,
        application::knowledge_base::parsing::KnowledgeDocumentParser,
    };
    async fn server() -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let hits = Arc::new(AtomicUsize::new(0));
        let counted = hits.clone();
        let router = Router::new()
            .route(
                "/v1/health",
                get(move || {
                    let hits = counted.clone();
                    async move {
                        hits.fetch_add(1, Ordering::SeqCst);
                        Json(json!({"status":"ok","version":"4.0.10"}))
                    }
                }),
            )
            .route(
                "/v1/tiers",
                get(|| async { Json(json!({"data":[{"id":"standard"}]})) }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (url, hits, task)
    }
    let fixture = ConfigFixture::new("http-switch");
    let (first_url, first_hits, first_server) = server().await;
    let (second_url, second_hits, second_server) = server().await;
    let doc = fixture.service.get().unwrap();
    let first = fixture
        .service
        .update(
            &doc.revision,
            vec![change("mineru.baseUrl", first_url.into())],
            false,
        )
        .unwrap();
    let parser = RuntimeMineruDocumentParser::new(fixture.runtime.clone());
    parser.capabilities().await.unwrap();
    let pinned = parser.snapshot().unwrap();
    let identity = pinned.profile_identity();
    let second = fixture
        .service
        .update(
            &first.revision,
            vec![
                change("mineru.baseUrl", second_url.clone().into()),
                change("mineru.timeoutSeconds", 2.into()),
            ],
            false,
        )
        .unwrap();
    assert!(second.restart_paths.is_empty());
    parser.capabilities().await.unwrap();
    pinned.capabilities().await.unwrap();
    parser
        .task_snapshot(&identity)
        .unwrap()
        .unwrap()
        .capabilities()
        .await
        .unwrap();
    assert_eq!(first_hits.load(Ordering::SeqCst), 3);
    assert_eq!(second_hits.load(Ordering::SeqCst), 1);
    assert_eq!(
        parser.profile_identity()["runtimeConfig"]["timeoutSeconds"],
        2
    );
    assert_ne!(identity["runtimeConfig"]["timeoutSeconds"], 2);
    let disabled = fixture
        .service
        .update(
            &second.revision,
            vec![change("mineru.enabled", false.into())],
            false,
        )
        .unwrap();
    assert_eq!(disabled.hot_update_paths, vec!["mineru.enabled"]);
    assert!(parser.capabilities().await.is_err());
    parser
        .task_snapshot(&identity)
        .unwrap()
        .unwrap()
        .capabilities()
        .await
        .unwrap();
    first_server.abort();
    second_server.abort();
}

#[test]
fn production_services_use_saved_limits_depth_goal_policy_and_new_terminal_directory() {
    use tron_backend::{
        app::AppState,
        application::{
            chat::CreateChatSessionCommand, short_term_memory::SetGoalCommand,
            terminal::IssueTerminalSocketTicketCommand,
        },
        config::ResourceConfig,
    };
    let fixture = ConfigFixture::new("production-services");
    let root = fixture.path.parent().unwrap();
    let old_dir = root.join("old-terminal");
    let new_dir = root.join("new-terminal");
    fs::create_dir(&old_dir).unwrap();
    fs::create_dir(&new_dir).unwrap();
    let mut config = fixture.runtime.snapshot();
    config.database_url = root.join("database.sqlite").display().to_string();
    config.graph_storage_path = root.join("graph.grafeo").display().to_string();
    config.skills_registry_root = root.join("skills").display().to_string();
    config.resource =
        ResourceConfig::with_default_root(root.join("resources").display().to_string());
    config.terminal_working_directory = old_dir.display().to_string();
    let state = AppState::new(common::prepared_config(config)).unwrap();
    let resources = state.resources.clone();
    let navigation = state.navigation.clone();
    let memory = state.short_term_memory.clone();
    let terminal = state.terminal.clone();
    let ticket = || {
        terminal
            .issue_socket_ticket(IssueTerminalSocketTicketCommand {
                workspace_code: "TRON_CORE".into(),
                app_code: None,
                user_code: "USR-AUTH-0001".into(),
            })
            .unwrap()
    };
    let old_ticket = ticket();
    let document = state.system_config.get().unwrap();
    let result = state
        .system_config
        .update(
            &document.revision,
            vec![
                change("resource.limits.maxBinaryBytes", 100.into()),
                change("resource.limits.maxCommitBytes", 200.into()),
                change("navigation.maxDepth", 8.into()),
                change("shortTermMemory.goalLoop.maxRuns", 3.into()),
                change(
                    "terminalWorkingDirectory",
                    new_dir.display().to_string().into(),
                ),
                change("mineru.visualContextEnabled", true.into()),
            ],
            false,
        )
        .unwrap();
    assert_eq!(result.hot_update_paths.len(), 6);
    assert!(result.restart_paths.is_empty());
    assert_eq!(resources.max_binary_bytes(), 100);
    assert_eq!(resources.max_commit_bytes(), 200);
    assert_eq!(navigation.max_depth(), 8);
    assert!(state.knowledge_bases.visual_context_enabled());
    assert_eq!(
        terminal
            .consume_socket_ticket(&old_ticket.ticket)
            .unwrap()
            .current_directory,
        old_dir.display().to_string()
    );
    assert_eq!(
        terminal
            .consume_socket_ticket(&ticket().ticket)
            .unwrap()
            .current_directory,
        new_dir.display().to_string()
    );
    let session = state
        .chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("hot settings".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USR-AUTH-0001".into()),
        )
        .unwrap();
    let goal = memory
        .set_goal(SetGoalCommand {
            session_code: session.session_code,
            objective: "verify policy".into(),
            max_runs: None,
        })
        .unwrap();
    assert_eq!(goal.goal.max_runs, 3);
}

fn change(path: &str, value: serde_json::Value) -> SystemConfigChange {
    SystemConfigChange {
        path: path.into(),
        value,
    }
}

struct ConfigFixture {
    path: PathBuf,
    service: SystemConfigService,
    runtime: BackendRuntimeConfig,
}

impl ConfigFixture {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "tron-system-config-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("fixture directory should be created");
        let path = directory.join("application.yaml");
        fs::write(
            &path,
            format!(
                "{}\ndatabaseUrl: tron.sqlite3\nauth:\n  jwtSecret: test-secret\n",
                include_str!("../application.yaml")
            ),
        )
        .expect("fixture should be written");
        let runtime = BackendRuntimeConfig::new(AppConfig::from_yaml_file(&path).unwrap());
        Self {
            service: SystemConfigService::new(Arc::new(YamlSystemConfigStore::new(path.clone())))
                .with_runtime_config(runtime.clone()),
            path,
            runtime,
        }
    }
}

impl Drop for ConfigFixture {
    fn drop(&mut self) {
        if let Some(directory) = self.path.parent() {
            let _ = fs::remove_dir_all(directory);
        }
    }
}

#[test]
fn model_invocation_defaults_are_visible_and_partial_saves_are_hot_and_atomic() {
    let fixture = ConfigFixture::new("invocation");
    // Old installation without the new YAML section still exposes editable defaults.
    let old = fs::read_to_string(&fixture.path).unwrap();
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&old).unwrap();
    yaml.as_mapping_mut()
        .unwrap()
        .remove(serde_yaml::Value::String("modelInvocation".into()));
    yaml.as_mapping_mut()
        .unwrap()
        .remove(serde_yaml::Value::String("auth".into()));
    let sparse = serde_yaml::to_string(&yaml).unwrap();
    fs::write(&fixture.path, &sparse).unwrap();
    let doc = fixture.service.get().unwrap();
    assert_eq!(
        doc.items
            .iter()
            .find(|item| item.path == "modelInvocation.embedding.maxBatchInputs")
            .unwrap()
            .value,
        Some(32.into())
    );
    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), sparse);
    let saved = fixture
        .service
        .update(
            &doc.revision,
            vec![
                change("modelInvocation.embedding.maxBatchInputs", 8.into()),
                change("modelInvocation.embedding.maxBatchTokens", 8192.into()),
                change("modelInvocation.embedding.callTimeoutSeconds", 240.into()),
            ],
            false,
        )
        .unwrap();
    assert_eq!(saved.hot_update_paths.len(), 3);
    assert!(saved.restart_paths.is_empty());
    let config = fixture.runtime.snapshot().model_invocation;
    assert_eq!(config.embedding.max_batch_inputs, 8);
    assert_eq!(config.embedding.max_batch_tokens, 8192);
    assert_eq!(config.embedding.call_timeout_seconds, 240);
    assert_eq!(config.embedding.operation_timeout_seconds, 600);
    let before = fixture.runtime.snapshot();
    let before_yaml = fs::read_to_string(&fixture.path).unwrap();
    for (path, value) in [
        ("modelInvocation.embedding.maxBatchInputs", 0),
        ("modelInvocation.embedding.maxBatchTokens", 0),
        ("modelInvocation.embedding.callTimeoutSeconds", 0),
        ("modelInvocation.embedding.operationTimeoutSeconds", 100),
    ] {
        assert!(
            fixture
                .service
                .update(&saved.revision, vec![change(path, value.into())], false)
                .is_err()
        );
        assert_eq!(fixture.runtime.snapshot(), before);
        assert_eq!(fs::read_to_string(&fixture.path).unwrap(), before_yaml);
    }
}

#[tokio::test]
async fn embedding_timeout_changes_apply_to_the_next_http_call_without_rebuilding_client() {
    use axum::{Json, Router, routing::post};
    use tron_backend::{
        adapters::outbound::model::{
            HttpLlmRuntimeClient, HttpModelClient, HttpModelProviderClient,
        },
        application::{llm::LlmRuntimeClient, model::RuntimeModelConfig},
        domain::model::{ModelProtocol, ModelType},
    };
    let fixture = ConfigFixture::new("embedding-timeout");
    let doc = fixture.service.get().unwrap();
    let saved = fixture
        .service
        .update(
            &doc.revision,
            vec![
                change("modelInvocation.embedding.callTimeoutSeconds", 1.into()),
                change(
                    "modelInvocation.embedding.operationTimeoutSeconds",
                    3.into(),
                ),
            ],
            false,
        )
        .unwrap();
    let router = Router::new().route(
        "/embeddings",
        post(|Json(body): Json<serde_json::Value>| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
            let count = body["input"].as_array().map(Vec::len).unwrap_or(1);
            let vectors = (0..count)
                .map(|index| serde_json::json!({"index":index,"embedding":[1.0,2.0]}))
                .collect::<Vec<_>>();
            Json(serde_json::json!({"model":"fixture","data":vectors}))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = HttpLlmRuntimeClient::with_model_client(Arc::new(
        HttpModelClient::new(Arc::new(HttpModelProviderClient::new(
            eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
        )))
        .with_runtime_config(fixture.runtime.clone()),
    ));
    let config = RuntimeModelConfig {
        model_code: "fixture".into(),
        protocol: ModelProtocol::OpenAiCompatibleEmbedding,
        model_type: ModelType::Embedding,
        base_url: url,
        api_path: None,
        api_key: "fixture".into(),
        model_name: "fixture".into(),
        thinking_enabled: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: Some(0),
    };
    let inputs = vec!["fixture".into()];
    let failed = client
        .embed_batch_with_model(&config, &inputs, 2)
        .await
        .unwrap_err();
    assert!(failed.contains("timed out"), "{failed}");
    let saved = fixture
        .service
        .update(
            &saved.revision,
            vec![
                change("modelInvocation.embedding.callTimeoutSeconds", 2.into()),
                change("modelInvocation.embedding.maxBatchInputs", 1.into()),
            ],
            false,
        )
        .unwrap();
    let result = client
        .embed_batch_with_model(&config, &inputs, 2)
        .await
        .unwrap();
    assert_eq!(result.0.len(), 1);
    let inputs = vec!["first".into(), "second".into()];
    let error = client
        .embed_batch_with_model(&config, &inputs, 2)
        .await
        .unwrap_err();
    assert!(error.contains("batch size"), "{error}");
    fixture
        .service
        .update(
            &saved.revision,
            vec![change("modelInvocation.embedding.maxBatchInputs", 2.into())],
            false,
        )
        .unwrap();
    assert_eq!(
        client
            .embed_batch_with_model(&config, &inputs, 2)
            .await
            .unwrap()
            .0
            .len(),
        2
    );
    server.abort();
}
