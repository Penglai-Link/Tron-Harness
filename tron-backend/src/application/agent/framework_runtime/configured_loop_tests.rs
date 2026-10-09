use super::configured_loop::*;
use crate::domain::agent::loop_design::*;
use eliteforge_agent_core as fw;
use eliteforge_agent_test_support::TestBuilder;
use serde_json::json;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
struct Ask(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl fw::Tool for Ask {
    async fn call(
        &self,
        _: serde_json::Value,
        _: &fw::RunContext,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(fw::ToolOutput::text("Choose").await_input())
    }
}
struct Script {
    calls: AtomicUsize,
    requests: Arc<Mutex<Vec<fw::ChatRequest>>>,
}
#[async_trait::async_trait]
impl fw::ChatModel for Script {
    async fn generate(
        &self,
        _: &str,
        request: fw::ChatRequest,
        _: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        self.requests.lock().unwrap().push(request);
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(if call == 0 {
            fw::ChatResponse::tool_calls(vec![fw::ToolCall::new("ask-1", "ask", json!({}))])
        } else {
            fw::ChatResponse {
                content: format!("done-{call}").into(),
                ..Default::default()
            }
        })
    }
}
#[tokio::test]
async fn configured_loop_resume_keeps_admitted_workflow_and_dag_definitions() {
    for kind in [LoopKind::Workflow, LoopKind::Dag] {
        let requests = Arc::new(Mutex::new(vec![]));
        let tool_calls = Arc::new(AtomicUsize::new(0));
        let agent = TestBuilder::default()
            .default_models(false)
            .plugin(fw::AgentLoopPlugin::named(
                "fixture.loop",
                ConfiguredLoop(None),
            ))
            .plugin(fw::WorkflowNodePlugin::new(
                "tron.model-stage",
                ModelStageNode,
            ))
            .plugin(eliteforge_agent_framework::ModelRuntimePlugin::new(
                eliteforge_agent_framework::ModelRuntime::new(
                    eliteforge_agent_framework::ModelRuntimeUpdate::new("", 0, vec![]),
                )
                .unwrap(),
            ))
            .plugin(fw::ModelConnectionPlugin::new(fw::ModelConnection::new(
                "fixture",
                Arc::new(Script {
                    calls: AtomicUsize::new(0),
                    requests: requests.clone(),
                }),
                vec![fw::ModelDescriptor::new("model")],
            )))
            .plugin(fw::ToolProviderPlugin::new(
                "fixture.tools",
                fw::StaticToolProvider::new(vec![fw::ToolBinding::new(
                    fw::ToolDescriptor::new(
                        "fixture.ask",
                        "ask",
                        "Ask",
                        json!({"type":"object","properties":{}}),
                    ),
                    Ask(tool_calls.clone()),
                )]),
            ))
            .build()
            .await
            .unwrap();
        let mut request = fw::RunRequest::new("resume-session", "work");
        request.model = Some(fw::ModelRef::new("fixture", "model"));
        let definition = LoopDesign {
            loop_code: "resume-loop".into(),
            name: "Resume".into(),
            kind,
            version: 1,
            nodes: ["first", "last"]
                .iter()
                .map(|id| LoopNode {
                    id: (*id).into(),
                    kind: LoopNodeKind::Model,
                    instructions: format!("ORIGINAL_{id}"),
                    join: LoopJoin::Selected,
                    condition: None,
                    position: None,
                })
                .collect(),
            edges: if kind == LoopKind::Dag {
                vec![LoopEdge {
                    from: "first".into(),
                    to: "last".into(),
                }]
            } else {
                vec![]
            },
            max_parallel_nodes: 1,
            updated_at: 0,
        };
        request.extensions.insert(definition.clone());
        let result = agent.run(request).await.unwrap();
        assert_eq!(result.status, fw::RunStatus::AwaitingInput);
        let mut response = agent
            .answer("resume-session", result.interactions[0].id, json!("yes"))
            .await
            .unwrap();
        let mut changed = definition;
        changed.version = 2;
        changed.nodes[1].instructions = "CHANGED_SHOULD_NOT_RUN".into();
        response.extensions.insert(changed);
        let resumed = agent.run(response).await.unwrap();
        assert_eq!(resumed.status, fw::RunStatus::Completed);
        assert_eq!(tool_calls.load(Ordering::SeqCst), 1);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        let prompts = requests
            .iter()
            .map(|r| serde_json::to_string(&r.messages).unwrap())
            .collect::<Vec<_>>()
            .join(
                "
",
            );
        assert!(prompts.contains("ORIGINAL_last"));
        assert!(!prompts.contains("CHANGED_SHOULD_NOT_RUN"));
        drop(requests);
        let framework = agent.framework();
        assert!(agent.close().await.is_success());
        assert!(framework.close().await.is_success());
    }
}
