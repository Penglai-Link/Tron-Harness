mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use eliteforge_agent_core as fw;
use fixture::*;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, atomic::Ordering},
};
use tron_backend::{
    application::agent::{FrameworkHost, ModelClient, ModelStreamSink, ReactAgentBuilder},
    domain::agent::{AgentModelConfig, AgentRunContext, ModelRequest, ModelResponse, ToolRegistry},
    shared::event_bus::EventBusHandle,
};
#[derive(Default)]
struct Capture(Mutex<Vec<fw::ChatRequest>>);
impl ModelClient for Capture {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Ok(ModelResponse::final_output("done")) })
    }
    fn generate_once<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate(c, r)
    }
    fn generate_native_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.0.lock().unwrap().push(request.clone());
            Ok(ModelResponse::final_output("done"))
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
fn context(f: &Fixture, run: &str, identity: bool) -> AgentRunContext {
    serde_json::from_value(serde_json::json!({
        "agentCode": if identity {Some(&f.agent)} else {None}, "conversationCode":run, "runCode":run, "workspaceCode":WORKSPACE,"userCode":USER,"locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"fixture-chat","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture","maxTokens":128,"contextWindowTokens":8000},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":2,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":"Tell me about orchid"}]}],"observations":[],"startedAt":"fixture"
    })).unwrap()
}
#[tokio::test]
async fn native_framework_loop_reads_current_agent_grants_and_revocation_without_prompt_acl() {
    {
        let f = Fixture::new();
        f.client.dimensions.store(1024, Ordering::SeqCst);
        let kb = f
            .service
            .create(USER, WORKSPACE, "orchid", "source")
            .unwrap();
        let d = f
            .service
            .upload(
                USER,
                WORKSPACE,
                &kb.id,
                "orchid.md",
                b"# Orchid\nPRIVATE_ORCHID_FACT orchid grows in cool weather.",
            )
            .unwrap();
        assert_eq!(
            f.service
                .index(USER, WORKSPACE, &kb.id, &d.id)
                .await
                .unwrap()
                .status,
            "READY"
        );
        let host = FrameworkHost::temporary(Some(f.state.resources.clone())).unwrap();
        host.bind_knowledge_bases(f.service.clone()).unwrap();
        let capture = Arc::new(Capture::default());
        host.bind_model_client(capture.clone()).unwrap();
        let harness = ReactAgentBuilder::new(
            capture.clone(),
            Arc::new(ToolRegistry::default()),
            EventBusHandle::disabled(),
        )
        .with_framework_host(Some(host.clone()))
        .build()
        .unwrap();
        let before = f.client.calls.load(Ordering::SeqCst);
        harness.run(context(&f, "no-grants", true)).await.unwrap();
        assert_eq!(f.client.calls.load(Ordering::SeqCst), before);
        f.service
            .set_grants(USER, WORKSPACE, &kb.id, vec![f.agent.clone()])
            .unwrap();
        harness.run(context(&f, "with-grants", true)).await.unwrap();
        assert_eq!(f.client.calls.load(Ordering::SeqCst), before + 1);
        f.service
            .set_grants(USER, WORKSPACE, &kb.id, vec![])
            .unwrap();
        harness.run(context(&f, "revoked", true)).await.unwrap();
        f.service
            .set_grants(USER, WORKSPACE, &kb.id, vec![f.agent.clone()])
            .unwrap();
        harness
            .run(context(&f, "legacy-no-identity", false))
            .await
            .unwrap();
        assert_eq!(f.client.calls.load(Ordering::SeqCst), before + 1);
        let requests = capture.0.lock().unwrap();
        assert_eq!(requests.len(), 4);
        let texts: Vec<String> = requests
            .iter()
            .map(|r| {
                r.messages
                    .iter()
                    .map(|m| m.content.text())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect();
        assert!(!texts[0].contains("PRIVATE_ORCHID_FACT"));
        assert!(texts[1].contains("PRIVATE_ORCHID_FACT"), "{}", texts[1]);
        assert!(!texts[2].contains("PRIVATE_ORCHID_FACT"));
        assert!(!texts[3].contains("PRIVATE_ORCHID_FACT"));
        drop(requests);
        assert!(host.close().await.is_success());
    }
}
