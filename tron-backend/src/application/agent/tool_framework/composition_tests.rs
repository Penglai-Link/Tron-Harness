use super::*;
use crate::{
    application::agent::{FrameworkHost, ModelClient, ModelStreamSink, ReactAgentBuilder},
    domain::agent::{AgentModelConfig, ModelRequest, ModelResponse},
    shared::{
        event_bus::{EventBusConfig, EventBusRuntimeBuilder},
        id::FixedSnowflakeIdProvider,
    },
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
fn content() -> fw::Content {
    fw::Content(vec![
        fw::ContentBlock::Media {
            media: fw::MediaContent {
                mime_type: "image/png".into(),
                source: fw::MediaSource::Base64("AA==".into()),
                name: Some("native-image".into()),
                estimated_tokens: Some(37),
            },
        },
        fw::ContentBlock::Resource {
            resource: fw::ResourceReference {
                uri: "mcp://demo/document".into(),
                mime_type: Some("application/json".into()),
                name: Some("native-resource".into()),
                size_bytes: Some(9),
            },
        },
        fw::ContentBlock::Json {
            value: json!({"native":true}),
        },
    ])
}
struct NativeTool;
#[async_trait]
impl fw::Tool for NativeTool {
    async fn call(
        &self,
        _: serde_json::Value,
        _: &fw::RunContext,
    ) -> Result<fw::ToolOutput, fw::ToolError> {
        let mut output = fw::ToolOutput::text(content());
        output.structured = Some(json!({"native":true}));
        output.result_reference = Some(Box::new(fw::ResourceReference {
            uri: "mcp://demo/result".into(),
            mime_type: None,
            name: None,
            size_bytes: None,
        }));
        Ok(output)
    }
}
struct NativeClient {
    step: AtomicUsize,
    actual: Arc<Mutex<Vec<fw::ChatRequest>>>,
}
impl NativeClient {
    fn response(&self) -> ModelResponse {
        if self.step.fetch_add(1, Ordering::SeqCst) == 0 {
            ModelResponse::tool_call(
                "tool_execute",
                json!({"tool":"mcp_demo","arguments":{"items":[{}]}}),
            )
        } else {
            ModelResponse::final_output("done")
        }
    }
}
impl ModelClient for NativeClient {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move { Ok(self.response()) })
    }
    fn generate_once<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        self.generate(c, r)
    }
    fn generate_native_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.actual.lock().unwrap().push(request.clone());
            Ok(self.response())
        })
    }
    fn close<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
    fn close_observed<'a>(
        &'a self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}
async fn run(
    failing_close: bool,
) -> (
    Result<ToolExecutionResult, crate::domain::agent::AgentError>,
    Arc<Mutex<Vec<fw::ChatRequest>>>,
    usize,
    Vec<fw::HistoryToolInvocation>,
) {
    let registry = Arc::new(ToolRegistry::default());
    let closed = Arc::new(AtomicUsize::new(0));
    let close = closed.clone();
    let fallbacks = closed.clone();
    registry.register_framework_lease(
        "c3b6f2d9-421a-4ab9-8721-69271cc18a41",
        fw::ToolLease::new(vec![fw::ToolBinding::new(
            fw::ToolDescriptor::new(
                "tron.mcp_demo",
                "mcp_demo",
                "native MCP-shaped binding",
                json!({"type":"object","additionalProperties":false}),
            ),
            NativeTool,
        )])
        .with_cleanup(move || {
            fallbacks.fetch_add(1, Ordering::SeqCst);
        })
        .with_shutdown(move || async move {
            close.fetch_add(1, Ordering::SeqCst);
            if failing_close {
                Err("MCP close failed".into())
            } else {
                Ok(())
            }
        }),
    );
    let actual = Arc::new(Mutex::new(Vec::new()));
    let client = Arc::new(NativeClient {
        step: AtomicUsize::new(0),
        actual: actual.clone(),
    });
    let context:AgentRunContext=serde_json::from_value(json!({"conversationCode":"native-content","runCode":"c3b6f2d9-421a-4ab9-8721-69271cc18a41","workspaceCode":"test","locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"MODEL-1","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"unused","modelName":"test","maxTokens":128,"contextWindowTokens":131072},
        "variables":[],"skills":[],"tools":[{"toolName":"mcp_demo","description":"native MCP-shaped binding","inputSchema":{"type":"object","additionalProperties":false},"requiredVariableKeys":[]}],"executionScope":{},"limits":{"maxSteps":4,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":"Read the native MCP-shaped result."}]}],"observations":[],"startedAt":"test"})).unwrap();
    let bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(804_300_000_000_000_000)),
    )
    .build();
    let host = FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(client, registry, bus)
        .with_framework_host(Some(host.clone()))
        .build()
        .unwrap();
    let result = agent.run(context).await.map(|r| {
        ToolExecutionResult::observation(crate::domain::agent::ToolObservation {
            tool_name: "mcp_demo".into(),
            output: r.observations.last().unwrap().output.clone(),
        })
    });
    let history = host
        .storage_framework()
        .await
        .unwrap()
        .history()
        .unwrap()
        .read_session("native-content", fw::HistoryQuery::default())
        .await
        .unwrap();
    assert!(host.close().await.is_success());
    (
        result,
        actual,
        closed.load(Ordering::SeqCst),
        history.records.tool_invocations,
    )
}

#[tokio::test]
async fn blank_command_activity_does_not_mark_presentation_incomplete() {
    let bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(804_300_000_000_001_000)),
    )
    .without_workers_for_testing()
    .build();
    let host = FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
        Arc::new(NativeClient {
            step: AtomicUsize::new(0),
            actual: Default::default(),
        }),
        Arc::new(ToolRegistry::default()),
        bus.clone(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let context: AgentRunContext = serde_json::from_value(json!({
        "runCode":"command-progress", "workspaceCode":"test", "locale":"en-US", "agentKind":"REACT",
        "model":{"modelCode":"test","protocol":"CHAT_COMPLETIONS","baseUrl":"http://unused.invalid","apiKey":"","modelName":"test"},
        "variables":[],"skills":[],"tools":[],"executionScope":{},
        "limits":{"maxSteps":2,"maxToolCalls":1,"maxParallelToolCalls":1},
        "messages":[],"observations":[],"startedAt":"test"
    })).unwrap();
    agent.framework_tool_activity(&context, "CALL-1", "bash_exec", "");
    assert_eq!(bus.stats_snapshot().rejected_invalid, 0);
    assert_eq!(bus.stats_snapshot().accepted, 0);
    agent.framework_tool_activity(&context, "CALL-1", "bash_exec", "line");
    assert_eq!(bus.stats_snapshot().accepted, 1);
    assert_eq!(bus.stats_snapshot().rejected_invalid, 0);
    assert!(host.close().await.is_success());
}
#[tokio::test]
async fn native_tool_content_and_metadata_survive_product_audit_and_context_to_actual_model_dispatch()
 {
    let (result, actual, closed, history) = run(false).await;
    let result = result.unwrap();
    assert_eq!(closed, 2);
    assert_eq!(result.observation.output, content().protocol_text());
    let requests = actual.lock().unwrap();
    assert_eq!(requests.len(), 2);
    let message = requests[1]
        .messages
        .iter()
        .find(|m| m.role == fw::Role::Tool)
        .unwrap();
    let metadata = message.tool_result.as_ref().unwrap();
    assert!(!metadata.is_error);
    assert!(
        metadata.structured.is_none(),
        "model messages must not duplicate the full audit payload"
    );
    assert!(metadata.result_reference.is_none());
    let parent = history
        .iter()
        .find(|tool| {
            tool.batch.is_none()
                && tool
                    .structured
                    .as_ref()
                    .is_some_and(|value| value["kind"] == "toolBatch")
        })
        .expect("batch parent must retain full persisted output");
    let batch: fw::ToolBatchResult =
        serde_json::from_value(parent.structured.clone().unwrap()).unwrap();
    assert_eq!(batch.results.len(), 1);
    let item = &batch.results[0];
    assert_eq!(item.identity.index, 0);
    assert_eq!(item.status, fw::ToolBatchStatus::Completed);
    assert_eq!(item.is_error, Some(false));
    assert_eq!(item.content, content());
    assert_eq!(item.structured, Some(json!({"native":true})));
    assert_eq!(item.result, Some(json!({"native":true})));
    assert_eq!(
        item.result_reference.as_ref().unwrap().uri,
        "mcp://demo/result"
    );
    let attachments = message
        .content
        .0
        .iter()
        .filter(|block| {
            matches!(
                block,
                fw::ContentBlock::Media { .. } | fw::ContentBlock::Resource { .. }
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(attachments, content().0[..2]);
    let fw::ContentBlock::Text { text } = &message.content.0[0] else {
        panic!("model result must start with the batch summary")
    };
    let summary: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(summary["results"].as_array().unwrap().len(), 1);
    assert_eq!(summary["results"][0]["result"], json!({"native":true}));
    assert_eq!(summary["results"][0]["attachmentCount"], 2);
    assert_eq!(
        summary["results"][0]["resultReference"]["uri"],
        "mcp://demo/result"
    );
    assert!(message.content.estimated_tokens() >= content().estimated_tokens());
}
#[tokio::test]
async fn original_mcp_lease_shutdown_failure_reaches_the_product_run_outcome() {
    let (result, _, closed, _) = run(true).await;
    assert_eq!(closed, 2);
    assert!(result.unwrap_err().to_string().contains("MCP close failed"));
}
