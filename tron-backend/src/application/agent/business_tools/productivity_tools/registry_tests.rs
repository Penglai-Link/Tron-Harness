use super::*;
use std::{future::Future, pin::Pin, time::Instant};
struct Control {
    rx: tokio::sync::watch::Receiver<bool>,
    deadline: Instant,
}
impl ToolExecutionControl for Control {
    fn check_active(&self) -> Result<(), ToolError> {
        if *self.rx.borrow() || self.remaining().is_zero() {
            Err(error("cancelled or deadline exceeded"))
        } else {
            Ok(())
        }
    }
    fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
    fn cancelled(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let mut rx = self.rx.clone();
        Box::pin(async move {
            if *rx.borrow() {
                return;
            }
            let _ = rx.changed().await;
        })
    }
}
fn context(root: &Path) -> AgentRunContext {
    serde_json::from_value(json!({"conversationCode":null,"runCode":"test","workspaceCode":"test","userCode":null,"locale":"en-US","requestId":null,"streamKey":null,"agentKind":"REACT","systemAgentSessionId":null,
        "model":{"modelCode":"unused","protocol":"CHAT_COMPLETIONS","baseUrl":"","apiKey":"","modelName":"unused"},
        "variables":[],"skills":[],"tools":[{"toolName":"registry_test_tool","description":"test","inputSchema":{},"skillName":null,"requiredVariableKeys":[]}],
        "executionScope":{"localDirectory":root,"allowedLocalDirectories":[root]},"limits":{"maxSteps":4,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[],"observations":[],"startedAt":"test"})).unwrap()
}
#[test]
fn registry_replacement_keeps_the_selected_product_registration_authoritative() {
    use crate::domain::agent::{InProcessTool, ToolObservation, ToolRegistry};
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let registry = ToolRegistry::default();
    for value in ["first", "override"] {
        registry
            .register(Arc::new(InProcessTool::new(
                "registry_test_tool",
                move |_| {
                    Ok(ToolObservation {
                        tool_name: "registry_test_tool".into(),
                        output: value.into(),
                    })
                },
            )))
            .unwrap();
    }
    let request = ToolRequest {
        tool_name: "registry_test_tool".into(),
        input: json!({}),
        invocation_id: None,
    };
    let tool = registry.resolve_allowed(&request, &context).unwrap();
    let (_tx, rx) = tokio::sync::watch::channel(false);
    let control = Control {
        rx,
        deadline: Instant::now() + Duration::from_secs(1),
    };
    assert!(
        tool.exec_async(request.clone(), &context, &control)
            .is_none()
    );
    assert_eq!(
        tool.exec(request, &context).unwrap().observation.output,
        "override"
    );
}
