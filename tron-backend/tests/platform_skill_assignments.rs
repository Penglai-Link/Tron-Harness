mod common;
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{
    app::AppState,
    application::{
        agent::CreateWorkspaceInternalAgentCommand,
        capability::{CapabilityResource, CapabilityResources},
    },
    config::AppConfig,
    domain::skill::{RegistrySkillListQuery, SkillError},
};
struct Fixture {
    _dir: tempfile::TempDir,
    state: AppState,
    app: Router,
    workspace: String,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut config = AppConfig::default();
        config.database_url = dir.path().join("db.sqlite").display().to_string();
        config.graph_storage_path = dir.path().join("graph").display().to_string();
        config.skills_registry_root = dir.path().join("skills").display().to_string();
        let solo =
            std::path::Path::new(&config.skills_registry_root).join(".agents/skills/independent");
        std::fs::create_dir_all(&solo).unwrap();
        std::fs::write(
            solo.join("SKILL.md"),
            "---\nname: independent\ndescription: Independent skill\n---\nBusiness instructions",
        )
        .unwrap();
        let config = common::prepared_config(config);
        let mut state = None;
        let app = common::app_with_state_setup(config, |s| state = Some(s.clone()));
        let state = state.unwrap();
        let workspace = state
            .workspaces
            .ensure_default_workspace("USR-AUTH-0001", "profile.operator")
            .unwrap()
            .workspace_code;
        state.knowledge_bases.stop_ingestion_workers();
        let mut settings = state.rag.settings_for_corpus("skill").unwrap();
        settings.vector = false;
        settings.graph = false;
        state.rag.configure_corpus("skill", settings).unwrap();
        Self {
            _dir: dir,
            state,
            app,
            workspace,
        }
    }
    fn agent(&self, code: &str) {
        if self
            .state
            .agent_management
            .list_loop_designs()
            .unwrap()
            .is_empty()
        {
            self.state.agent_management.save_loop_design(serde_json::from_value(json!({"loopCode":"fixture-react","name":"Fixture React","kind":"React","version":0,"updatedAt":0,"nodes":[],"edges":[],"maxParallelNodes":1})).unwrap(),0).unwrap();
        }

        self.state
            .agent_management
            .create_platform_internal_agent(CreateWorkspaceInternalAgentCommand {
                candidate_model_codes: Vec::new(),
                agent_code: code.into(),
                name: code.into(),
                role_prompt: "Use assigned capabilities".into(),
                avatar_id: None,
                loop_code: Some(
                    self.state.agent_management.list_loop_designs().unwrap()[0]
                        .loop_code
                        .clone(),
                ),
                group_name: None,
            })
            .unwrap();
        self.state
            .agent_management
            .assign_agent_workspaces(code, vec![self.workspace.clone()])
            .unwrap();
    }
    fn assigned(&self, code: &str) -> Vec<String> {
        self.state
            .skills
            .agent_skills(code)
            .unwrap()
            .into_iter()
            .filter(|s| s.assigned)
            .map(|s| s.skill_name)
            .collect()
    }
    async fn api(&self, method: Method, path: &str, body: Option<Value>) -> Value {
        let response = common::authenticated_request(&self.app, method, path, body).await;
        assert_eq!(response.status(), StatusCode::OK);
        common::response_json(response).await
    }
}
#[tokio::test]
async fn builtin_defaults_are_once_only_and_bidirectional_replacements_are_atomic() {
    let f = Fixture::new();
    f.agent("child-reader");
    let builtin = f
        .state
        .skills
        .list_registry(RegistrySkillListQuery { search: None })
        .unwrap()
        .into_iter()
        .find(|s| s.provider_pack_id.is_some())
        .unwrap()
        .skill_name;
    assert!(f.assigned("main-agent").contains(&builtin));
    assert!(f.assigned("child-reader").contains(&builtin));
    assert!(!f.assigned("main-agent").contains(&"independent".into()));
    assert!(
        f.state
            .skills
            .effective_agent_skills("legacy-workspace-agent")
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.state
            .skills
            .replace_agent_skills("legacy-workspace-agent", &[]),
        Err(SkillError::InvalidAgent)
    );
    f.state.skills.replace_skill_agents(&builtin, &[]).unwrap();
    f.state.skills.rescan_registry().unwrap();
    assert!(!f.assigned("main-agent").contains(&builtin));
    assert!(!f.assigned("child-reader").contains(&builtin));
    f.state
        .skills
        .replace_skill_agents(&builtin, &["child-reader".into()])
        .unwrap();
    assert!(!f.assigned("main-agent").contains(&builtin));
    assert!(f.assigned("child-reader").contains(&builtin));
    f.state
        .skills
        .replace_agent_skills("main-agent", &["independent".into()])
        .unwrap();
    assert_eq!(f.assigned("main-agent"), vec!["independent"]);
    assert_eq!(
        f.state
            .skills
            .replace_agent_skills("main-agent", &[builtin.clone(), "missing".into()]),
        Err(SkillError::Unavailable)
    );
    assert_eq!(f.assigned("main-agent"), vec!["independent"]);
    let c = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    c.execute_batch("CREATE TRIGGER reject_skill_grant BEFORE UPDATE ON agent_skill_assignments BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(
        f.state
            .skills
            .replace_agent_skills("main-agent", &[])
            .is_err()
    );
    assert_eq!(f.assigned("main-agent"), vec!["independent"]);
    c.execute_batch("DROP TRIGGER reject_skill_grant").unwrap();
    f.state
        .agent_management
        .delete_platform_internal_agent("child-reader")
        .unwrap();
    assert!(
        !f.state
            .skills
            .skill_agents(&builtin)
            .unwrap()
            .contains(&"child-reader".into())
    );
}
#[tokio::test]
async fn platform_api_and_agent_api_share_grants_and_reject_stale_resources() {
    let f = Fixture::new();
    let path = "/api/v1/agents/registry/internal/main-agent/skills";
    let body = f
        .api(
            Method::PUT,
            path,
            Some(json!({"skillNames":["independent"]})),
        )
        .await;
    assert_eq!(body["code"], "0");
    let grants = f
        .api(
            Method::GET,
            "/api/v1/skills/registry/independent/agent-grants",
            None,
        )
        .await;
    assert_eq!(grants["data"]["agentCodes"], json!(["main-agent"]));
    f.api(
        Method::PUT,
        "/api/v1/skills/registry/independent/agent-grants",
        Some(json!({"agentCodes":[]})),
    )
    .await;
    assert!(f.assigned("main-agent").is_empty());
    for (locale, expected) in [("zh-CN", "技能不可分配"), ("en-US", "skill is unavailable")] {
        let request = Request::builder()
            .method(Method::PUT)
            .uri(path)
            .header("authorization", "Bearer profile-test-token")
            .header("accept-language", locale)
            .header("content-type", "application/json")
            .body(Body::from(json!({"skillNames":["missing"]}).to_string()))
            .unwrap();
        let response = f.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = common::response_json(response).await;
        assert!(body["message"].as_str().unwrap().contains(expected));
        assert_eq!(body["data"], json!({}));
    }
    let response = common::authenticated_request(
        &f.app,
        Method::GET,
        "/api/v1/skills/workspaces/TRON_CORE/bindings",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn capability_transition_defaults_only_on_enable_and_clears_all_owned_resources() {
    let f = Fixture::new();
    f.agent("pack-reader");
    let resources = CapabilityResources::new(
        tron_backend::adapters::outbound::persistence::sqlite::capability_repository::SqliteCapabilityRepository::connect(
            &f.state.config.database_url,
        )
        .unwrap(),
    );
    let pack = "fixture.pack";
    let root = f._dir.path().join("pack/owned");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("SKILL.md"),
        "---\nname: owned\ndescription: Pack skill\n---\nInstructions",
    )
    .unwrap();
    f.state
        .skills
        .register_pack_skill(pack, "owned", &root, false)
        .unwrap();
    resources
        .register(&[
            CapabilityResource {
                pack_id: pack.into(),
                kind: "skill".into(),
                code: "owned".into(),
                available: false,
            },
            CapabilityResource {
                pack_id: pack.into(),
                kind: "tool".into(),
                code: "fixture.tool".into(),
                available: false,
            },
            CapabilityResource {
                pack_id: pack.into(),
                kind: "mcp".into(),
                code: "fixture-mcp".into(),
                available: false,
            },
        ])
        .unwrap();
    f.state
        .agent_management
        .persist_internal_agent_defaults()
        .unwrap();
    let codes = f.state.skills.internal_agent_codes().unwrap();
    resources.transition(pack, true, &codes).unwrap();
    assert!(f.assigned("main-agent").contains(&"owned".into()));
    assert!(f.assigned("pack-reader").contains(&"owned".into()));
    f.state
        .skills
        .replace_agent_skills("main-agent", &["independent".into()])
        .unwrap();
    resources.transition(pack, true, &codes).unwrap();
    assert_eq!(f.assigned("main-agent"), vec!["independent"]);
    resources.transition(pack, false, &codes).unwrap();
    assert_eq!(
        f.state
            .skills
            .replace_agent_skills("main-agent", &["owned".into()]),
        Err(SkillError::Unavailable)
    );
    let option = f
        .state
        .skills
        .agent_skills("pack-reader")
        .unwrap()
        .into_iter()
        .find(|s| s.skill_name == "owned")
        .unwrap();
    assert!(!option.assigned && !option.available);
    assert!(option.disabled_reason.is_some());
    let c = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    let configs = c
        .prepare("SELECT config_json FROM platform_internal_agent_configs")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for config in configs {
        let config: Value = serde_json::from_str(&config).unwrap();
        assert!(
            !config["selectedToolCodes"]
                .as_array()
                .unwrap()
                .contains(&json!("fixture.tool"))
        );
        assert!(
            !config["selectedServerCodes"]
                .as_array()
                .unwrap()
                .contains(&json!("fixture-mcp"))
        );
    }
    resources.transition(pack, true, &codes).unwrap();
    assert!(f.assigned("main-agent").contains(&"owned".into()));
    f.state.skills.delete_pack_skills(pack).unwrap();
    resources.remove(pack).unwrap();
    assert!(
        !f.state
            .skills
            .agent_skills("main-agent")
            .unwrap()
            .iter()
            .any(|s| s.skill_name == "owned")
    );
    assert!(f.state.skills.get_registry_skill("independent").is_ok());
}

fn archive() -> Vec<u8> {
    archive_with("lifecycle-review", "lifecycle-mcp", "1.0.0")
}
fn archive_with(skill_name: &str, mcp_id: &str, version: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    use std::io::Write;
    let mut files=std::collections::BTreeMap::from([
        ("manifest.json",serde_json::to_vec(&json!({"manifest_version":1,"id":"fixture.lifecycle","name":"Lifecycle fixture","version":version,"framework":"^0.1","skills":["skills"],"mcp":[{"id":mcp_id,"transport":"http","url":"http://127.0.0.1:1/mcp"}]})).unwrap()),
        ("skills/review/SKILL.md",format!("---\nname: {skill_name}\ndescription: Lifecycle review skill\n---\nReview instructions").into_bytes()),
    ]);
    let checksums = files
        .iter()
        .map(|(p, b)| (*p, format!("{:x}", Sha256::digest(b))))
        .collect::<std::collections::BTreeMap<_, _>>();
    files.insert("checksums.json", serde_json::to_vec(&checksums).unwrap());
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (p, b) in files {
        zip.start_file(p, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&b).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
#[tokio::test]
async fn imported_pack_lifecycle_is_live_and_retryable_without_restoring_deselections() {
    let f = Fixture::new();
    f.agent("lifecycle-child");
    let host = &f.state.agent_framework;
    host.import_pack(archive()).await.unwrap();
    let name = "lifecycle-review";
    assert!(!f.assigned("main-agent").contains(&name.into()));
    host.set_pack_enabled("fixture.lifecycle", true)
        .await
        .unwrap();
    assert!(f.assigned("main-agent").contains(&name.into()));
    assert!(f.assigned("lifecycle-child").contains(&name.into()));
    f.state.skills.replace_skill_agents(name, &[]).unwrap();
    host.set_pack_enabled("fixture.lifecycle", true)
        .await
        .unwrap();
    host.sync_capability_resources().unwrap();
    assert!(!f.assigned("main-agent").contains(&name.into()));
    host.set_pack_enabled("fixture.lifecycle", false)
        .await
        .unwrap();
    assert_eq!(
        f.state
            .skills
            .replace_skill_agents(name, &["main-agent".into()]),
        Err(SkillError::Unavailable)
    );
    host.set_pack_enabled("fixture.lifecycle", true)
        .await
        .unwrap();
    assert!(f.assigned("main-agent").contains(&name.into()));
    let indexer = tron_backend::application::skill::SkillRagIndexService::new(
        f.state.skills.clone(),
        f.state.rag.clone(),
    );
    let (removed, catalog, indexed) = tokio::join!(
        host.uninstall_pack("fixture.lifecycle"),
        host.ensure_capability_catalog(),
        indexer.index_all(),
    );
    removed.unwrap();
    catalog.unwrap();
    indexed.unwrap();
    assert!(indexer.state_summary(name).unwrap().is_none());
    host.uninstall_pack("fixture.lifecycle").await.unwrap();
    assert!(matches!(
        f.state.skills.get_registry_skill(name),
        Err(SkillError::NotFound)
    ));
    assert!(f.state.skills.get_registry_skill("independent").is_ok());
    assert!(!f.assigned("lifecycle-child").contains(&name.into()));
}
#[tokio::test]
async fn standalone_provider_enforces_each_agent_and_live_revocation() {
    use eliteforge_agent_core::{self as fw, SkillProvider};
    use tron_backend::application::skill::SkillAgentIdentity;
    let f = Fixture::new();
    f.agent("provider-child");
    let provider = f.state.skills.provider();
    let mut options = fw::ModelInvocationOptions::for_call(&fw::RunLimits::default()).unwrap();
    let run = options.run.as_mut().unwrap();
    run.extensions
        .insert(SkillAgentIdentity("main-agent".into()));
    assert!(provider.catalog(run).await.unwrap().skills.is_empty());
    assert!(provider.load(run, "independent").await.is_err());
    f.state
        .skills
        .replace_agent_skills("main-agent", &["independent".into()])
        .unwrap();
    assert_eq!(
        provider.catalog(run).await.unwrap().skills[0].name,
        "independent"
    );
    assert!(provider.load(run, "independent").await.is_ok());
    run.extensions
        .insert(SkillAgentIdentity("provider-child".into()));
    assert!(provider.load(run, "independent").await.is_err());
    run.extensions
        .insert(SkillAgentIdentity("main-agent".into()));
    f.state
        .skills
        .replace_agent_skills("main-agent", &[])
        .unwrap();
    assert!(provider.load(run, "independent").await.is_err());
    assert!(
        provider
            .read_resource(run, "independent", "SKILL.md")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn uninstall_preflight_preserves_state_and_failed_cleanup_can_retry() {
    use eliteforge_agent_capability_runtime::{
        BackendInstance, CapabilityPackManager, DurablePackPin, PackVersionRequirement,
    };
    let f = Fixture::new();
    let host = &f.state.agent_framework;
    host.import_pack(archive()).await.unwrap();
    host.set_pack_enabled("fixture.lifecycle", true)
        .await
        .unwrap();
    let manager = CapabilityPackManager::new(host.pack_root()).unwrap();
    let pack = manager.installed().unwrap().remove(0);
    manager
        .pin(DurablePackPin {
            id: "pending-run".into(),
            scope: "main-agent".into(),
            versions: vec![PackVersionRequirement {
                package: pack.manifest.id.clone(),
                version: pack.manifest.version.clone(),
                digest: pack.digest.clone(),
            }],
        })
        .unwrap();
    assert!(host.uninstall_pack("fixture.lifecycle").await.is_err());
    assert!(
        host.set_pack_enabled("fixture.lifecycle", false)
            .await
            .is_err()
    );
    assert!(
        f.assigned("main-agent")
            .contains(&"lifecycle-review".into())
    );
    assert_eq!(
        host.capability_state().0.get("fixture.lifecycle"),
        Some(&true)
    );
    manager.unpin("pending-run").unwrap();
    let external = BackendInstance::new(
        manager.clone(),
        pack,
        "other-process",
        "config",
        Value::Null,
    )
    .unwrap();
    assert!(host.uninstall_pack("fixture.lifecycle").await.is_err());
    assert_eq!(
        host.capability_state().0.get("fixture.lifecycle"),
        Some(&true)
    );
    assert!(
        f.assigned("main-agent")
            .contains(&"lifecycle-review".into())
    );
    external.shutdown().await.unwrap();
    let c = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    c.execute_batch("CREATE TRIGGER reject_pack_cleanup BEFORE DELETE ON registry_skills WHEN OLD.provider_pack_id='fixture.lifecycle' BEGIN SELECT RAISE(ABORT,'cleanup fixture'); END;").unwrap();
    let failed = common::authenticated_request(
        &f.app,
        Method::DELETE,
        "/api/v1/capabilities/packs/fixture.lifecycle",
        None,
    )
    .await;
    assert_eq!(failed.status(), StatusCode::CONFLICT);
    assert!(
        !f.assigned("main-agent")
            .contains(&"lifecycle-review".into())
    );
    assert!(
        !f.state
            .skills
            .list_registry(RegistrySkillListQuery { search: None })
            .unwrap()
            .into_iter()
            .find(|s| s.skill_name == "lifecycle-review")
            .unwrap()
            .available
    );
    c.execute_batch("DROP TRIGGER reject_pack_cleanup").unwrap();
    let result = f
        .api(
            Method::DELETE,
            "/api/v1/capabilities/packs/fixture.lifecycle",
            None,
        )
        .await;
    assert_eq!(result["code"], "0");
    assert!(f.state.mcp.get_platform_server("lifecycle-mcp").is_err());
    assert!(
        !f.assigned("main-agent")
            .contains(&"lifecycle-review".into())
    );
}
#[tokio::test]
async fn shared_skill_rag_filters_each_agent_and_rechecks_revocation() {
    use tron_backend::application::{
        rag::{RagAccessResolver, RagIdentity},
        skill::SkillRagIndexService,
    };
    let f = Fixture::new();
    f.agent("rag-child");
    let indexer = SkillRagIndexService::new(f.state.skills.clone(), f.state.rag.clone());
    let state = indexer.index_registry_skill("independent").await.unwrap();
    assert_ne!(
        state.vector_status,
        tron_backend::domain::skill::SkillRagIndexStatus::Failed
    );
    f.state
        .skills
        .replace_agent_skills("main-agent", &["independent".into()])
        .unwrap();
    let identity = RagIdentity {
        user_code: "USR-AUTH-0001".into(),
        workspace_code: f.workspace.clone(),
        agent_code: "main-agent".into(),
    };
    let ranges = f.state.knowledge_bases.resolve(&identity).unwrap();
    let range = ranges.iter().find(|r| r.corpus_id == "skill").unwrap();
    assert_eq!(range.scopes[0].scope_code, "PLATFORM");
    assert!(
        range
            .filters
            .to_string()
            .contains("skill:PLATFORM:independent")
    );
    let child = RagIdentity {
        agent_code: "rag-child".into(),
        ..identity.clone()
    };
    assert!(
        !f.state
            .knowledge_bases
            .resolve(&child)
            .unwrap()
            .iter()
            .filter(|r| r.corpus_id == "skill")
            .any(|r| r.filters.to_string().contains("skill:PLATFORM:independent"))
    );
    let mut evidence = f
        .state
        .rag
        .retrieve_authorized(&f.workspace, "Independent", &ranges, 10, None, None)
        .await
        .unwrap();
    assert!(
        evidence
            .hits
            .iter()
            .any(|h| h.row.source_code == "skill:PLATFORM:independent")
    );
    f.state
        .skills
        .replace_agent_skills("main-agent", &[])
        .unwrap();
    f.state
        .knowledge_bases
        .recheck(&identity, &mut evidence)
        .unwrap();
    assert!(!evidence.hits.iter().any(|h| h.row.corpus_id == "skill"));
    let c = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    let scopes:String=c.query_row("SELECT group_concat(DISTINCT scope_code) FROM rag_skill_text_units WHERE source_code='skill:PLATFORM:independent'",[],|r|r.get(0)).unwrap();
    assert_eq!(scopes, "PLATFORM");
}

#[tokio::test]
async fn tool_and_mcp_grants_reject_disabled_and_removed_resources_and_preserve_modes() {
    let f = Fixture::new();
    f.agent("grant-child");
    let host = &f.state.agent_framework;
    host.import_pack(archive()).await.unwrap();
    assert!(
        f.state
            .agent_management
            .replace_platform_internal_agent_mcp_server_assignments(
                "main-agent",
                vec!["lifecycle-mcp".into()]
            )
            .is_err()
    );
    assert!(
        f.state
            .agent_management
            .replace_platform_internal_agent_mcp_server_assignments(
                "main-agent",
                vec!["missing-server".into()]
            )
            .is_err()
    );
    host.set_pack_enabled("fixture.lifecycle", true)
        .await
        .unwrap();
    let before = f
        .state
        .agent_management
        .list_platform_internal_agents()
        .unwrap()
        .into_iter()
        .find(|a| a.agent_code == "main-agent")
        .unwrap();
    assert!(
        before
            .selected_server_codes
            .contains(&"lifecycle-mcp".into())
    );
    host.set_pack_enabled("fixture.lifecycle", false)
        .await
        .unwrap();
    let after = f
        .state
        .agent_management
        .list_platform_internal_agents()
        .unwrap()
        .into_iter()
        .find(|a| a.agent_code == "main-agent")
        .unwrap();
    assert_eq!(before.mcp_access_mode, after.mcp_access_mode);
    assert_eq!(before.tool_assignment_mode, after.tool_assignment_mode);
    assert!(
        !after
            .selected_server_codes
            .contains(&"lifecycle-mcp".into())
    );
    host.uninstall_pack("fixture.lifecycle").await.unwrap();
    assert!(
        f.state
            .agent_management
            .replace_platform_internal_agent_mcp_server_assignments(
                "main-agent",
                vec!["lifecycle-mcp".into()]
            )
            .is_err()
    );
    let tool = f
        .state
        .agent_management
        .list_tools(None, None, "en-US")
        .into_iter()
        .find(|t| t.tool_code == "bash.exec")
        .unwrap();
    f.state
        .agent_management
        .replace_platform_internal_agent_tool_assignments(
            "main-agent",
            vec![tool.tool_code.clone()],
        )
        .unwrap();
    host.set_pack_enabled("framework.local.scoped", false)
        .await
        .unwrap();
    assert!(
        f.state
            .agent_management
            .replace_platform_internal_agent_tool_assignments(
                "main-agent",
                vec![tool.tool_code.clone()]
            )
            .is_err()
    );
    assert!(
        !f.state
            .agent_management
            .list_platform_internal_agents()
            .unwrap()
            .into_iter()
            .find(|a| a.agent_code == "main-agent")
            .unwrap()
            .selected_tool_codes
            .contains(&tool.tool_code)
    );
    host.set_pack_enabled("framework.local.scoped", true)
        .await
        .unwrap();
    f.state
        .agent_management
        .replace_internal_agent_tool_assignments(
            &f.workspace,
            "main-agent",
            vec![tool.tool_code.clone()],
        )
        .unwrap();
    let c = rusqlite::Connection::open(&f.state.config.database_url).unwrap();
    c.execute_batch("CREATE TRIGGER reject_workspace_grant BEFORE INSERT ON workspace_internal_agent_tool_assignments BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(
        f.state
            .agent_management
            .replace_internal_agent_tool_assignments(
                &f.workspace,
                "main-agent",
                vec![tool.tool_code.clone()]
            )
            .is_err()
    );
    let count:i64=c.query_row("SELECT COUNT(*) FROM workspace_internal_agent_tool_assignments WHERE workspace_code=?1 AND agent_code='main-agent' AND tool_code='bash.exec'",[&f.workspace],|r|r.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn conflicting_imports_preserve_the_installed_catalog_and_restart() {
    for scenario in ["skill", "mcp", "upgrade"] {
        let f = Fixture::new();
        if scenario == "mcp" {
            let response = common::authenticated_request(&f.app, Method::POST, "/api/v1/mcp/servers",
                Some(json!({"serverCode":"lifecycle-mcp","name":"Independent MCP","url":"http://127.0.0.1:9/mcp","authKind":"none","headers":{}}))).await;
            assert_eq!(response.status(), StatusCode::CREATED);
        }
        if scenario == "upgrade" {
            f.state
                .agent_framework
                .import_pack(archive())
                .await
                .unwrap();
            f.state
                .agent_framework
                .set_pack_enabled("fixture.lifecycle", true)
                .await
                .unwrap();
        }
        let before = f.assigned("main-agent");
        let candidate = match scenario {
            "mcp" => archive(),
            "upgrade" => archive_with("independent", "lifecycle-mcp", "2.0.0"),
            _ => archive_with("independent", "lifecycle-mcp", "1.0.0"),
        };
        assert!(
            f.state
                .agent_framework
                .import_pack(candidate)
                .await
                .is_err()
        );
        assert_eq!(f.assigned("main-agent"), before);
        let manager = eliteforge_agent_capability_runtime::CapabilityPackManager::new(
            f.state.agent_framework.pack_root(),
        )
        .unwrap();
        let installed = manager.installed().unwrap();
        if scenario == "upgrade" {
            assert_eq!(installed.len(), 1);
            assert_eq!(installed[0].manifest.version.to_string(), "1.0.0");
            assert_eq!(
                f.state
                    .agent_framework
                    .capability_state()
                    .0
                    .get("fixture.lifecycle"),
                Some(&true)
            );
        } else {
            assert!(installed.is_empty());
            assert!(
                !f.state
                    .agent_framework
                    .capability_state()
                    .0
                    .contains_key("fixture.lifecycle")
            );
        }
        if scenario == "mcp" {
            assert_eq!(
                f.state
                    .mcp
                    .get_platform_server("lifecycle-mcp")
                    .unwrap()
                    .url,
                "http://127.0.0.1:9/mcp"
            );
        }
        let Fixture {
            _dir, state, app, ..
        } = f;
        let config = state.config.clone();
        assert!(state.agent_framework.close().await.is_success());
        drop(app);
        drop(state);
        let restarted = common::app_with_state_setup(config, |state| {
            state.knowledge_bases.stop_ingestion_workers();
            assert!(state.skills.get_registry_skill("independent").is_ok());
        });
        drop(restarted);
    }
}


fn composer_session(f: &Fixture) -> tron_backend::domain::chat::ChatSession {
    f.state
        .chats
        .create_session(
            tron_backend::application::chat::CreateChatSessionCommand {
                agent_code: Some("main-agent".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                title: Some("Composer acceptance".into()),
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
            },
            Some("USR-AUTH-0001".into()),
        )
        .unwrap()
}

#[tokio::test]
async fn composer_suggestions_follow_effective_grants_and_exclude_external_agents() {
    let f = Fixture::new();
    f.agent("composer-child");
    f.state
        .skills
        .replace_agent_skills("main-agent", &["independent".into()])
        .unwrap();
    f.state
        .skills
        .replace_agent_skills("composer-child", &[])
        .unwrap();
    let data = f
        .api(
            Method::GET,
            "/api/v1/chats/input-suggestions?agentCode=main-agent",
            None,
        )
        .await;
    assert_eq!(data["code"], "0");
    assert_eq!(data["data"]["skills"][0]["name"], "independent");
    assert_eq!(data["data"]["commands"], json!(["goal"]));
    let child = f
        .api(
            Method::GET,
            "/api/v1/chats/input-suggestions?agentCode=composer-child",
            None,
        )
        .await;
    assert_eq!(child["data"]["skills"], json!([]));
    let external = f
        .api(
            Method::GET,
            "/api/v1/chats/input-suggestions?agentCode=external-provider",
            None,
        )
        .await;
    assert_eq!(external["data"]["skills"], json!([]));
    assert_eq!(external["data"]["commands"], json!([]));
    f.state
        .skills
        .replace_agent_skills("main-agent", &[])
        .unwrap();
    let revoked = f
        .api(
            Method::GET,
            "/api/v1/chats/input-suggestions?agentCode=main-agent",
            None,
        )
        .await;
    assert_eq!(revoked["data"]["skills"], json!([]));
}

#[tokio::test]
async fn composer_steer_persists_selection_and_rejects_revocation_before_admission() {
    let f = Fixture::new();
    f.state
        .skills
        .replace_agent_skills("main-agent", &["independent".into()])
        .unwrap();
    let session = composer_session(&f);
    let _active = f
        .state
        .active_runs
        .register(&session.session_code, "composer-run", "REACT", true)
        .unwrap();
    let payload = json!({"sessionCode":session.session_code,"expectedRunCode":"composer-run","content":"$independent adjust now","attachments":[],"requestedSkills":["independent","independent"]});
    let accepted = f
        .api(
            Method::POST,
            "/api/v1/chats/sessions/steer-message",
            Some(payload.clone()),
        )
        .await;
    assert_eq!(
        accepted["data"]["message"]["requestedSkills"],
        json!(["independent"])
    );
    let messages = f.state.chats.list_messages(&session.session_code).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].requested_skills, vec!["independent"]);
    let pending = f
        .state
        .active_runs
        .control(&session.session_code, "composer-run")
        .unwrap()
        .take_pending()
        .unwrap();
    assert_eq!(pending[0].requested_skills, vec!["independent"]);
    f.state
        .skills
        .replace_agent_skills("main-agent", &[])
        .unwrap();
    let rejected = common::authenticated_request(
        &f.app,
        Method::POST,
        "/api/v1/chats/sessions/steer-message",
        Some(payload),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);
    assert_eq!(common::response_json(rejected).await["code"], "101032002");
    assert_eq!(
        f.state
            .chats
            .list_messages(&session.session_code)
            .unwrap()
            .len(),
        1
    );
    for suffix in ["messages", "messages/stream"] {
        let response = common::authenticated_request(
            &f.app,
            Method::POST,
            &format!("/api/v1/chats/sessions/{}/{}", session.session_code, suffix),
            Some(json!({"content":"$independent rejected","requestedSkills":["independent"]})),
        )
        .await;
        // The active run is rejected before any new message can be admitted.
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
    drop(_active);
    for suffix in ["messages", "messages/stream"] {
        let response = common::authenticated_request(
            &f.app,
            Method::POST,
            &format!("/api/v1/chats/sessions/{}/{}", session.session_code, suffix),
            Some(json!({"content":"$independent rejected","requestedSkills":["independent"]})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(common::response_json(response).await["code"], "101032002");
    }
}

#[tokio::test]
async fn composer_goal_steer_is_fenced_and_persists_the_goal_and_original_message() {
    let f = Fixture::new();
    let session = composer_session(&f);
    let _active = f
        .state
        .active_runs
        .register(&session.session_code, "goal-run", "REACT", true)
        .unwrap();
    let path = "/api/v1/chats/sessions/steer-message";
    let stale = common::authenticated_request(&f.app, Method::POST, path, Some(json!({"sessionCode":session.session_code,"expectedRunCode":"stale","content":"/goal should not persist","requestedSkills":[]}))).await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    let empty = common::authenticated_request(&f.app, Method::POST, path, Some(json!({"sessionCode":session.session_code,"expectedRunCode":"goal-run","content":"/goal","requestedSkills":[]}))).await;
    assert_eq!(empty.status(), StatusCode::BAD_REQUEST);
    assert_eq!(common::response_json(empty).await["code"], "101032003");
    let accepted = f.api(Method::POST, path, Some(json!({"sessionCode":session.session_code,"expectedRunCode":"goal-run","content":"/goal finish composer acceptance","requestedSkills":[]}))).await;
    assert_eq!(
        accepted["data"]["message"]["content"],
        "/goal finish composer acceptance"
    );
    let goal = f
        .api(
            Method::GET,
            &format!(
                "/api/v1/chats/sessions/get-latest-goal?sessionCode={}",
                session.session_code
            ),
            None,
        )
        .await;
    assert_eq!(
        goal["data"]["goal"]["objectiveMarkdown"], "finish composer acceptance",
        "{goal}"
    );
    f.state
        .short_term_memory
        .pause_goal(
            tron_backend::application::short_term_memory::GoalSessionCommand {
                session_code: session.session_code.clone(),
                goal_code: None,
            },
        )
        .unwrap();
    f.api(Method::POST, path, Some(json!({"sessionCode":session.session_code,"expectedRunCode":"goal-run","content":"/goal revised acceptance","requestedSkills":[]}))).await;
    let revised = f
        .api(
            Method::GET,
            &format!(
                "/api/v1/chats/sessions/get-latest-goal?sessionCode={}",
                session.session_code
            ),
            None,
        )
        .await;
    assert_eq!(
        revised["data"]["goal"]["goalCode"],
        goal["data"]["goal"]["goalCode"]
    );
    assert_eq!(
        revised["data"]["goal"]["objectiveMarkdown"],
        "revised acceptance"
    );
    assert_eq!(revised["data"]["goal"]["status"], "ACTIVE");
    assert_eq!(
        f.state
            .chats
            .list_messages(&session.session_code)
            .unwrap()
            .len(),
        2
    );
}
