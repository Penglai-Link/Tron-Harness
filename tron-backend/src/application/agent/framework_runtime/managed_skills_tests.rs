//! Real installed managed pack, production HarnessTools and native SkillsPlugin.
use super::*;
use crate::{
    application::agent::{FrameworkHost, ModelClient, ModelStreamSink, ReactAgentBuilder},
    domain::agent::{
        AgentModelConfig, AgentRunContext, EffectiveTool, InProcessTool, ModelRequest,
        ModelResponse, ToolObservation, ToolRegistry,
    },
    shared::event_bus::EventBusHandle,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    future::Future,
    io::Write,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

fn archive() -> Vec<u8> {
    let mut files=BTreeMap::from([
        ("manifest.json",serde_json::to_vec(&json!({"manifest_version":1,"id":"fixture.managed-skills","name":"Managed skills","version":"1.0.0","framework":"^0.1","skills":["skills"]})).unwrap()),
        ("skills/review/SKILL.md",b"---\nname: review\ndescription: A managed review skill.\n---\nNATIVE_MANAGED_INSTRUCTIONS\n".to_vec()),
        ("skills/review/guide.md",b"NATIVE_MANAGED_RESOURCE".to_vec()),
    ]);
    let checksums: BTreeMap<_, _> = files
        .iter()
        .map(|(path, data)| (*path, format!("{:x}", Sha256::digest(data))))
        .collect();
    files.insert("checksums.json", serde_json::to_vec(&checksums).unwrap());
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in files {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
/// Gateway mode: every request exposes exactly the stable pair.
fn assert_stable_tools(request: &fw::ChatRequest) {
    assert_eq!(request.tools.len(), 2);
    assert!(
        request
            .tools
            .iter()
            .any(|t| t.id == "framework.tools.search" && t.call_name == "tool_search")
    );
    assert!(
        request
            .tools
            .iter()
            .any(|t| t.id == "framework.tools.execute" && t.call_name == "tool_execute")
    );
}

/// Gateway-mode models address every wrapped tool through `tool_execute`.
fn gateway_call(id: &str, tool: &str, arguments: serde_json::Value) -> ModelResponse {
    ModelResponse::tool_call_with_id(
        id,
        "tool_execute",
        json!({"tool": tool, "arguments": if matches!(tool,"a2ui_publish"|"a2ui_request_input"|"chat_ask_user") { arguments } else { json!({"items":[arguments]}) }}),
    )
}

struct Client {
    mode: u8,
    step: AtomicUsize,
    requests: Mutex<Vec<fw::ChatRequest>>,
}
impl ModelClient for Client {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Err("native single-attempt dispatch required".into()) })
    }
    fn generate_native_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let step = self.step.fetch_add(1, Ordering::SeqCst);
            self.requests.lock().unwrap().push(request.clone());
            Ok(match (self.mode, step) {
                (0, 0) | (1, 0) => gateway_call("load", "skill_load", json!({"name":"review"})),
                (0, 1) => gateway_call(
                    "read",
                    "skill_read_resource",
                    json!({"name":"review","path":"guide.md"}),
                ),
                (6, 0) => gateway_call(
                    "tron-theme",
                    "skill_load",
                    json!({"name":"tron-theme-spec"}),
                ),
                (6, 1) => gateway_call(
                    "tron-resource",
                    "skill_read_resource",
                    json!({"name":"tron-theme-spec","path":"references/agent-ui.md"}),
                ),
                (4, 0) => {
                    gateway_call("a2ui-skill", "skill_load", json!({"name":"a2ui-interface"}))
                }
                (4, 1) => ModelResponse::tool_call_with_id(
                    "a2ui-search",
                    "tool_search",
                    json!({"items":[{"domainCode":"a2ui"}]}),
                ),
                (4, 2) => gateway_call(
                    "a2ui-report",
                    "a2ui_publish",
                    json!({"mode":"Display","messages":[{"version":"v0.9.1","createSurface":{"surfaceId":"report","catalogId":"https://eliteforge.dev/a2ui/v0.9.1/agent"}},{"version":"v0.9.1","updateComponents":{"surfaceId":"report","components":[{"id":"root","component":"Text","text":"Report ready"}]}}]}),
                ),
                (5, 0) => {
                    gateway_call("unauthorized-skill", "skill_load", json!({"name":"review"}))
                }
                (2, 0) | (3, 0) => gateway_call("registered", "skill_load", json!({})),
                _ => ModelResponse::final_output("done"),
            })
        })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}
struct Leased;
#[async_trait]
impl fw::Tool for Leased {
    async fn call(&self, _: Value, _: &fw::RunContext) -> Result<fw::ToolOutput, fw::ToolError> {
        Ok(fw::ToolOutput::text("ORIGINAL_REGISTERED_SDK_LEASE"))
    }
}
fn context(mode: u8) -> AgentRunContext {
    let mut context:AgentRunContext=serde_json::from_value(json!({
        "conversationCode":null,"runCode":"c3b6f2d9-421a-4ab9-8721-69271cc18a41","workspaceCode":"authorized-workspace","userCode":"user","locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"MODEL-1","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"fixture","modelName":"test","maxTokens":128,"contextWindowTokens":131072},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":4,"maxToolCalls":4,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":"review"}]}],"observations":[],"startedAt":"fixture"
    })).unwrap();
    // This product DTO grants authority; it supplies no implementation.
    let names: Vec<&str> = if mode == 1 {
        vec![]
    } else if mode == 0 || mode == 6 {
        vec!["skill_load", "skill_read_resource"]
    } else {
        vec!["skill_load"]
    };
    context.tools = names
        .into_iter()
        .map(|name| EffectiveTool {
            tool_name: name.into(),
            domain_code: Some("workspace.skills".into()),
            description: "product ACL metadata".into(),
            input_schema: json!({"type":"object","additionalProperties":false}),
            skill_name: None,
            required_variable_keys: vec![],
        })
        .collect();
    context
}
async fn run(mode: u8) -> (crate::domain::agent::AgentRunResult, Arc<Client>, usize) {
    run_with_profile(mode, true, true).await
}
async fn run_with_profile(
    mode: u8,
    harness_enabled: bool,
    tools_enabled: bool,
) -> (crate::domain::agent::AgentRunResult, Arc<Client>, usize) {
    run_with_profile_and_locale(mode, harness_enabled, tools_enabled, "en-US").await
}
async fn run_with_profile_and_locale(
    mode: u8,
    harness_enabled: bool,
    tools_enabled: bool,
    locale: &str,
) -> (crate::domain::agent::AgentRunResult, Arc<Client>, usize) {
    let host = FrameworkHost::temporary(None).unwrap();
    host.set_pack_enabled("tron.harness", harness_enabled)
        .await
        .unwrap();
    host.set_extension_enabled("tool", tools_enabled)
        .await
        .unwrap();
    super::super::capability_catalog::install_capability_pack(host.pack_root(), &archive())
        .unwrap();
    host.set_pack_enabled("fixture.managed-skills", true)
        .await
        .unwrap();
    let registry = Arc::new(ToolRegistry::default());
    let selected = Arc::new(AtomicUsize::new(0));
    if mode == 2 {
        let calls = selected.clone();
        registry
            .register(Arc::new(InProcessTool::new("skill_load", move |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: "skill_load".into(),
                    output: "ACTUAL_REGISTERED_PRODUCT".into(),
                })
            })))
            .unwrap();
    }
    if mode == 3 {
        let shutdown = selected.clone();
        let cleanup = selected.clone();
        registry.register_framework_lease(
            "c3b6f2d9-421a-4ab9-8721-69271cc18a41",
            fw::ToolLease::new(vec![fw::ToolBinding::new(
                fw::ToolDescriptor::new(
                    "tron.skill_load",
                    "skill_load",
                    "original leased SDK",
                    json!({"type":"object","additionalProperties":false}),
                ),
                Leased,
            )])
            .with_cleanup(move || {
                cleanup.fetch_add(1, Ordering::SeqCst);
            })
            .with_shutdown(move || async move {
                shutdown.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        );
    }
    let client = Arc::new(Client {
        mode,
        step: AtomicUsize::new(0),
        requests: Mutex::new(vec![]),
    });
    let agent = ReactAgentBuilder::new(client.clone(), registry, EventBusHandle::disabled())
        .with_framework_host(Some(host.clone()))
        .build()
        .unwrap();
    let mut context = context(mode);
    context.locale = locale.into();
    let result = agent.run(context).await.unwrap();
    assert!(host.close().await.is_success());
    (result, client, selected.load(Ordering::SeqCst))
}
#[tokio::test]
async fn managed_skills_use_original_native_tools_with_product_acl_without_duplicate_wrappers() {
    let (result, client, _) = run(0).await;
    assert_eq!(result.final_output, "done");
    assert_eq!(result.observations.len(), 2);
    assert!(
        result.observations[0]
            .output
            .contains("NATIVE_MANAGED_INSTRUCTIONS")
    );
    assert!(
        result.observations[1]
            .output
            .contains("NATIVE_MANAGED_RESOURCE")
    );
    let requests = client.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    for request in requests.iter() {
        // The stable gateway pair replaces direct skill-tool exposure.
        assert_stable_tools(request);
        assert!(
            request
                .messages
                .iter()
                .any(|m| m.content.text().contains("<available_skills>"))
        );
    }
}
#[tokio::test]
async fn managed_skills_without_workspace_tool_acl_are_filtered_and_cannot_execute() {
    let (result, client, _) = run(1).await;
    assert_eq!(result.final_output, "done");
    assert_eq!(result.observations.len(), 1);
    assert!(
        !result.observations[0]
            .output
            .contains("NATIVE_MANAGED_INSTRUCTIONS")
    );
    let requests = client.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    // No wrapped tools remain, but the gateway pair stays as infrastructure.
    for r in requests.iter() {
        assert_stable_tools(r);
    }
    let tool = requests[1]
        .messages
        .iter()
        .find(|m| m.role == fw::Role::Tool)
        .unwrap();
    assert!(tool.tool_result.as_ref().unwrap().is_error);
}
#[tokio::test]
async fn actual_registered_product_skill_override_remains_authoritative() {
    let (result, client, selected) = run(2).await;
    assert_eq!(selected, 1);
    assert_eq!(result.observations[0].output, "ACTUAL_REGISTERED_PRODUCT");
    let requests = client.requests.lock().unwrap();
    assert_stable_tools(&requests[0]);
}
#[tokio::test]
async fn original_framework_backed_skill_registration_retains_executor_and_lease_close() {
    let (result, client, closed) = run(3).await;
    assert_eq!(closed, 2);
    assert_eq!(
        result.observations[0].output,
        "ORIGINAL_REGISTERED_SDK_LEASE"
    );
    let requests = client.requests.lock().unwrap();
    assert_stable_tools(&requests[0]);
}

#[tokio::test]
async fn disabling_harness_keeps_managed_tool_acl_and_original_authorized_executor() {
    let (denied, denied_client, _) = run_with_profile(1, false, true).await;
    assert_eq!(denied.final_output, "done");
    assert!(
        !denied.observations[0]
            .output
            .contains("NATIVE_MANAGED_INSTRUCTIONS")
    );
    let requests = denied_client.requests.lock().unwrap();
    for request in requests.iter() {
        assert_stable_tools(request);
    }
    assert!(
        requests[1]
            .messages
            .iter()
            .find(|message| message.role == fw::Role::Tool)
            .unwrap()
            .tool_result
            .as_ref()
            .unwrap()
            .is_error
    );
    drop(requests);

    let (allowed, allowed_client, _) = run_with_profile(0, false, true).await;
    assert_eq!(allowed.final_output, "done");
    assert!(
        allowed.observations[0]
            .output
            .contains("NATIVE_MANAGED_INSTRUCTIONS")
    );
    assert!(
        allowed.observations[1]
            .output
            .contains("NATIVE_MANAGED_RESOURCE")
    );
    let requests = allowed_client.requests.lock().unwrap();
    // The gateway pair is the visible catalog; the skill tools execute as
    // envelope arguments and stay out of request.tools.
    assert_stable_tools(&requests[0]);
}

#[tokio::test]
async fn disabling_tool_extension_denies_managed_tools_and_hides_tool_guidance() {
    let (result, client, _) = run_with_profile(0, true, false).await;
    assert_eq!(result.final_output, "done");
    assert_eq!(result.observations.len(), 2);
    assert!(
        !result.observations[0]
            .output
            .contains("NATIVE_MANAGED_INSTRUCTIONS")
    );
    assert!(
        !result.observations[1]
            .output
            .contains("NATIVE_MANAGED_RESOURCE")
    );
    let requests = client.requests.lock().unwrap();
    assert!(requests.iter().all(|request| request.tools.is_empty()));
    assert!(
        !requests.iter().any(|request| request
            .messages
            .iter()
            .any(|message| message.content.text().contains("<available_skills>"))),
        "disabled tools must not leave skill-loading guidance in model context"
    );
    for request in requests.iter().skip(1) {
        assert!(
            request
                .messages
                .iter()
                .filter(|message| message.role == fw::Role::Tool)
                .all(|message| message.tool_result.as_ref().unwrap().is_error)
        );
    }
}

#[tokio::test]
async fn a2ui_plugin_owns_guidance_skills_and_gateway_discovery_without_product_schema_injection() {
    for (
        client_support,
        pack_enabled,
        tools_enabled,
        try_other_skill,
        registry_bound,
        revoke_a2ui,
    ) in [
        (true, true, true, false, false, false),
        (false, true, true, false, false, false),
        (true, false, true, false, false, false),
        (true, true, false, false, false, false),
        (true, true, true, true, false, false),
        (true, true, true, false, true, false),
        (false, true, true, false, true, false),
        (true, false, true, false, true, false),
        (true, true, false, false, true, false),
        (true, true, true, true, true, false),
        (true, true, true, false, true, true),
    ] {
        let enabled = client_support && pack_enabled && tools_enabled && !revoke_a2ui;
        let host = FrameworkHost::temporary(None).unwrap();
        let bound_skills = if registry_bound {
            use crate::adapters::outbound::persistence::{
                SqliteAgentManagementRepository, SqliteSkillRegistryRepository,
                SqliteWorkspaceRepository,
            };
            use crate::application::{
                agent::AgentManagementService, skill::SkillRegistryService,
                workspace::WorkspaceService,
            };
            let database = std::env::temp_dir()
                .join(format!("tron-a2ui-skills-{}.sqlite", fw::RunId::new_v4()));
            let path = database.to_str().unwrap();
            let agents = AgentManagementService::new(
                SqliteAgentManagementRepository::connect(path).unwrap(),
                WorkspaceService::new(SqliteWorkspaceRepository::connect(path).unwrap()),
                &crate::config::AppConfig::default().system_agents,
            );
            agents.initialize_default_main_agent().unwrap();
            let skills = SkillRegistryService::new(
                SqliteSkillRegistryRepository::connect(path).unwrap(),
                std::env::temp_dir().join(format!("tron-a2ui-skills-{}", fw::RunId::new_v4())),
            );
            skills.bind_agents(agents);
            host.bind_test_skill_registry(skills.clone());
            host.sync_capability_resources().unwrap();
            let builtin = skills.get_registry_skill("a2ui-interface").unwrap();
            assert_eq!(builtin.provider_pack_id.as_deref(), Some("framework.a2ui"));
            Some(skills)
        } else {
            None
        };
        host.set_pack_enabled(
            eliteforge_agent_official_plugin_a2ui::A2UI_PACK_ID,
            pack_enabled,
        )
        .await
        .unwrap();
        host.set_extension_enabled("tool", tools_enabled)
            .await
            .unwrap();
        // Another selected skill exists, but A2UI must not grant product ACL to load it.
        super::super::capability_catalog::install_capability_pack(host.pack_root(), &archive())
            .unwrap();
        host.set_pack_enabled("fixture.managed-skills", true)
            .await
            .unwrap();
        if registry_bound {
            host.sync_capability_resources().unwrap();
        }
        if try_other_skill {
            if let Some(skills) = &bound_skills {
                skills.replace_skill_agents("review", &[]).unwrap();
            }
        }
        if revoke_a2ui {
            bound_skills
                .as_ref()
                .unwrap()
                .replace_skill_agents("a2ui-interface", &[])
                .unwrap();
        }
        let client = Arc::new(Client {
            mode: if try_other_skill {
                5
            } else if enabled {
                4
            } else {
                6
            },
            step: AtomicUsize::new(0),
            requests: Mutex::new(vec![]),
        });
        let agent = ReactAgentBuilder::new(
            client.clone(),
            Arc::new(ToolRegistry::default()),
            EventBusHandle::disabled(),
        )
        .with_framework_host(Some(host.clone()))
        .build()
        .unwrap();
        let mut context = context(1);
        context.tools.clear();
        if client_support {
            std::sync::Arc::make_mut(&mut context.messages)[0].parts.push(crate::domain::agent::AgentMessagePart::Context {kind:"tron_interaction_response".into(),text:String::new(),metadata:Some(json!({"a2uiClientCapabilities":{"version":"v0.9.1","catalogIds":["https://eliteforge.dev/a2ui/v0.9.1/agent"]}}))});
        }
        let result = agent.run(context).await.unwrap();
        assert_eq!(result.final_output, "done");
        let requests = client.requests.lock().unwrap();
        let first = &requests[0];
        // Gateway mode: a2ui tools are wrapped inventory, never directly
        // exposed; the stable pair is present exactly when tools are enabled.
        assert_eq!(
            first
                .tools
                .iter()
                .any(|tool| tool.call_name == "tool_execute"),
            tools_enabled
        );
        assert!(
            !first
                .tools
                .iter()
                .any(|tool| tool.call_name == "a2ui_publish")
        );
        assert_eq!(
            serde_json::to_string(&first.messages)
                .unwrap()
                .contains("A2UI interactive interfaces are available"),
            enabled
        );
        if enabled && !try_other_skill {
            let catalog = first
                .messages
                .iter()
                .map(|message| message.content.text())
                .find(|text| text.contains("<available_skills>"))
                .expect("the model should receive a skill catalog");
            assert!(
                catalog.contains("a2ui-interface"),
                "A2UI must appear in the skill catalog"
            );
            assert_eq!(requests.len(), 4);
            assert!(
                result
                    .observations
                    .iter()
                    .any(|o| o.output.contains("# A2UI interface"))
            );
            let discovery = result
                .observations
                .iter()
                .find(|o| o.output.contains("a2ui_publish"))
                .expect("domain search must discover A2UI tools");
            assert!(discovery.output.contains("a2ui_request_input"));
            assert!(!discovery.output.contains("a2ui_expand"));
            assert!(
                result
                    .observations
                    .iter()
                    .any(|o| o.output.contains("A2UI surface published")),
                "publishing through the gateway succeeds"
            );
            assert!(
                requests[2]
                    .tools
                    .iter()
                    .any(|t| t.id == "framework.tools.execute")
            );
            let f = host.storage_framework().await.unwrap();
            assert!(
                f.services()
                    .unwrap()
                    .get::<eliteforge_agent_official_plugin_a2ui::A2uiService>()
                    .is_some()
            );
        }
        if try_other_skill {
            assert!(
                !result
                    .observations
                    .iter()
                    .any(|o| o.output.contains("NATIVE_MANAGED_INSTRUCTIONS"))
            );
        }
        drop(requests);
        assert!(host.close().await.is_success());
    }
}

#[tokio::test]
async fn builtin_tron_skills_load_embedded_instructions_and_resources() {
    let (result, client, _) = run(6).await;
    assert_eq!(result.final_output, "done");
    assert_eq!(result.observations.len(), 2);
    assert!(
        result.observations[0]
            .output
            .contains("Tron 主题规范与制作")
    );
    assert!(
        result.observations[1]
            .output
            .contains("共享聊天、A2UI 与动效")
    );
    let requests = client.requests.lock().unwrap();
    assert!(
        requests[0]
            .messages
            .iter()
            .any(|message| message.content.text().contains("eliteforge-memory-spec"))
    );
}

#[tokio::test]
async fn disabling_tron_pack_hides_embedded_skills() {
    let (_, client, _) = run_with_profile(6, false, true).await;
    let requests = client.requests.lock().unwrap();
    assert!(
        !requests[0]
            .messages
            .iter()
            .any(|message| message.content.text().contains("tron-theme-spec"))
    );
    let results: Vec<_> = requests
        .last()
        .unwrap()
        .messages
        .iter()
        .filter(|message| message.role == fw::Role::Tool)
        .collect();
    assert_eq!(results.len(), 2);
    assert!(
        results
            .iter()
            .all(|message| message.tool_result.as_ref().unwrap().is_error)
    );
    assert!(
        results
            .iter()
            .all(|message| !message.content.text().contains("Tron 主题规范与制作"))
    );
}

#[tokio::test]
async fn host_passes_normalized_request_locale_to_native_tool_policy() {
    use tron_harness_capability::TronToolPolicyLocale;
    for (locale, expected) in [
        ("en-US", TronToolPolicyLocale::EnUs),
        ("zh-CN", TronToolPolicyLocale::ZhCn),
        ("en-GB,en;q=0.9", TronToolPolicyLocale::EnUs),
        ("fr-FR", TronToolPolicyLocale::ZhCn),
    ] {
        let (_, client, _) = run_with_profile_and_locale(0, true, true, locale).await;
        let requests = client.requests.lock().unwrap();
        assert!(!requests.is_empty());
        for request in requests.iter() {
            assert_eq!(
                request.extensions.get::<TronToolPolicyLocale>().as_deref(),
                Some(&expected)
            );
        }
    }
}
