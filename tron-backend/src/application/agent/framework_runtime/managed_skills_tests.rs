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
                (0, 0) | (1, 0) => {
                    ModelResponse::tool_call_with_id("load", "skill_load", json!({"name":"review"}))
                }
                (0, 1) => ModelResponse::tool_call_with_id(
                    "read",
                    "skill_read_resource",
                    json!({"name":"review","path":"guide.md"}),
                ),
                (4, 0) => ModelResponse::tool_call_with_id(
                    "a2ui-skill",
                    "skill_load",
                    json!({"name":"a2ui-interface"}),
                ),
                (4, 1) => ModelResponse::tool_call_with_id("a2ui-expand", "a2ui_expand", json!({})),
                (4, 2) => ModelResponse::tool_call_with_id(
                    "a2ui-report",
                    "a2ui_publish",
                    json!({"mode":"Display","messages":[{"version":"v0.9.1","createSurface":{"surfaceId":"report","catalogId":"https://eliteforge.dev/a2ui/v0.9.1/agent"}},{"version":"v0.9.1","updateComponents":{"surfaceId":"report","components":[{"id":"root","component":"Text","text":"Report ready"}]}}]}),
                ),
                (5, 0) => ModelResponse::tool_call_with_id(
                    "unauthorized-skill",
                    "skill_load",
                    json!({"name":"review"}),
                ),
                (2, 0) | (3, 0) => {
                    ModelResponse::tool_call_with_id("registered", "skill_load", json!({}))
                }
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
    } else if mode == 0 {
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
    let result = agent.run(context(mode)).await.unwrap();
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
        assert_eq!(request.tools.len(), 2);
        assert!(
            request
                .tools
                .iter()
                .any(|t| t.id == "skills.load" && t.call_name == "skill_load")
        );
        assert!(
            request
                .tools
                .iter()
                .any(|t| t.id == "skills.read_resource" && t.call_name == "skill_read_resource")
        );
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
    assert!(requests.iter().all(|r| r.tools.is_empty()));
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
    assert_eq!(requests[0].tools.len(), 1);
    assert_eq!(requests[0].tools[0].id, "tron.skill_load");
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
    assert_eq!(requests[0].tools.len(), 1);
    assert_eq!(requests[0].tools[0].id, "tron.skill_load");
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
    assert!(requests.iter().all(|request| request.tools.is_empty()));
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
    assert_eq!(requests[0].tools.len(), 2);
    assert!(
        requests[0]
            .tools
            .iter()
            .any(|tool| tool.id == "skills.load")
    );
}

#[tokio::test]
async fn disabling_tool_extension_denies_managed_tools_but_keeps_context_capability() {
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
        requests.iter().any(|request| request
            .messages
            .iter()
            .any(|message| message.content.text().contains("<available_skills>"))),
        "the managed context plugin must survive tool extension disablement"
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
async fn a2ui_plugin_owns_guidance_skills_and_expansion_without_product_schema_injection() {
    for (client_support, pack_enabled, tools_enabled, try_other_skill) in [
        (true, true, true, false),
        (false, true, true, false),
        (true, false, true, false),
        (true, true, false, false),
        (true, true, true, true),
    ] {
        let enabled = client_support && pack_enabled && tools_enabled;
        let host = FrameworkHost::temporary(None).unwrap();
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
            context.messages[0].parts.push(crate::domain::agent::AgentMessagePart::Context {kind:"tron_interaction_response".into(),text:String::new(),metadata:Some(json!({"a2uiClientCapabilities":{"version":"v0.9.1","catalogIds":["https://eliteforge.dev/a2ui/v0.9.1/agent"]}}))});
        }
        let result = agent.run(context).await.unwrap();
        assert_eq!(result.final_output, "done");
        let requests = client.requests.lock().unwrap();
        let first = &requests[0];
        assert_eq!(
            first
                .tools
                .iter()
                .any(|tool| tool.call_name == "a2ui_expand"),
            enabled
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
            assert_eq!(requests.len(), 4);
            assert!(
                result
                    .observations
                    .iter()
                    .any(|o| o.output.contains("# A2UI interface"))
            );
            assert!(requests[2].tools.iter().any(|t| t.id == "a2ui.publish"));
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
