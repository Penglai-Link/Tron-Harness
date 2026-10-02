use super::*;
use std::{future::Future, pin::Pin, process::Command, time::Instant};
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
        "variables":[],"skills":[],"tools":[{"toolName":"git_switch_branch","description":"test","inputSchema":{},"skillName":null,"requiredVariableKeys":[]}],
        "executionScope":{"localDirectory":root,"allowedLocalDirectories":[root]},"limits":{"maxSteps":4,"maxToolCalls":2,"maxParallelToolCalls":1},"messages":[],"observations":[],"startedAt":"test"})).unwrap()
}
fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
fn repository() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    git(root, &["init", "-b", "main"]);
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "initial",
        ],
    );
    git(root, &["branch", "feature"]);
    let hook = root.join(".git/hooks/post-checkout");
    std::fs::write(&hook, "#!/bin/sh\ntouch started\nsleep 0.3\ntouch late\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    temp
}
#[tokio::test]
async fn git_native_owner_cancellation_stops_hook_descendants_without_late_sideeffects() {
    let temp = repository();
    let context = context(temp.path());
    let (tx, rx) = tokio::sync::watch::channel(false);
    let control = Control {
        rx,
        deadline: Instant::now() + Duration::from_secs(3),
    };
    let input = json!({"branch":"feature"});
    let operation = git::execute("git_switch_branch", &input, &context, Some(&control));
    let cancel = async {
        tokio::time::timeout(Duration::from_secs(2), async {
            while !temp.path().join("started").exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        tx.send(true).unwrap();
    };
    let (result, _) = tokio::join!(operation, cancel);
    assert!(result.unwrap_err().to_string().contains("cancelled"));
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!temp.path().join("late").exists());
}
#[tokio::test]
async fn git_native_owner_deadline_stops_hook_descendants_without_late_sideeffects() {
    let temp = repository();
    let context = context(temp.path());
    let (_tx, rx) = tokio::sync::watch::channel(false);
    let control = Control {
        rx,
        deadline: Instant::now() + Duration::from_millis(100),
    };
    assert!(
        git::execute(
            "git_switch_branch",
            &json!({"branch":"feature"}),
            &context,
            Some(&control)
        )
        .await
        .is_err()
    );
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!temp.path().join("late").exists());
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
                "git_switch_branch",
                move |_| {
                    Ok(ToolObservation {
                        tool_name: "git_switch_branch".into(),
                        output: value.into(),
                    })
                },
            )))
            .unwrap();
    }
    let request = ToolRequest {
        tool_name: "git_switch_branch".into(),
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
