use super::*;
use tron_backend::{
    adapters::outbound::persistence::SqliteAgentManagementRepository,
    application::agent::AgentManagementRepository,
    domain::agent::{AgentRuntimeType, ManagedAgentSource, SystemAgentRegistryEntry},
};

fn seed_external(f: &Fixture) -> std::path::PathBuf {
    let log = f.directory.path().join("external-argv.txt");
    let events = [
        r#"{"type":"thread.started","thread_id":"external-session"}"#,
        r#"{"type":"item.completed","item":{"type":"agent_message","text":"external subscription reply"}}"#,
        r#"{"type":"turn.completed","usage":{"input_tokens":11,"output_tokens":17}}"#,
    ];
    #[cfg(windows)]
    let executable = {
        // A native fixture accepts multiline prompts as literal argv, like Codex.
        // Windows batch files reject those arguments before the fixture can run.
        let executable = f.directory.path().join("fake-codex.exe");
        let source = f.directory.path().join("fake_codex.rs");
        std::fs::write(&source, format!(r#"
            use std::io::Write;
            fn main() {{
                let mut log = std::fs::OpenOptions::new().append(true).create(true).open({:?}).unwrap();
                writeln!(log, "{{}}", std::env::args().skip(1).collect::<Vec<_>>().join(" ")).unwrap();
                for event in {:?} {{ println!("{{event}}"); }}
            }}
        "#, log.to_string_lossy(), events)).unwrap();
        let compiled = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        executable
    };
    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        let executable = f.directory.path().join("fake-codex");
        let mut script = format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n", log.display());
        for event in events {
            script.push_str(&format!("echo '{event}'\n"));
        }
        std::fs::write(&executable, script).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        executable
    };
    SqliteAgentManagementRepository::connect(&f.state.config.database_url)
        .unwrap()
        .upsert_system_agent(&SystemAgentRegistryEntry {
            agent_code: "codex".into(),
            base_agent_code: None,
            model_id: None,
            name: "Codex".into(),
            description: "Subscription CLI".into(),
            runtime_type: AgentRuntimeType::CodexExec,
            command_name: executable.to_string_lossy().into_owned(),
            source: ManagedAgentSource::System,
            enabled: true,
            available: true,
            last_scanned_at: None,
            last_seen_at: None,
            last_error: None,
        })
        .unwrap();
    for (code, model) in [("codex-fast", "model-fast"), ("codex-deep", "model-deep")] {
        f.state
            .agent_management
            .create_system_agent_profile(code, code, "codex", model)
            .unwrap();
        f.state
            .agent_management
            .assign_agent_workspaces(code, vec![WORKSPACE.into()])
            .unwrap();
    }
    log
}

// Match the server runtime: synchronous repository calls can wait for an
// independent Thread's async SQLite transaction to commit on another worker.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn named_system_agents_dispatch_to_same_cli_with_distinct_models_and_separate_history() {
    let f = Fixture::new();
    let log = seed_external(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    for (prompt, code) in [
        ("root:system-fast", "codex-fast"),
        ("root:system-deep", "codex-deep"),
    ] {
        let mut root = context(prompt);
        root.execution_scope.local_directory =
            Some(f.directory.path().to_string_lossy().into_owned());
        let parent = root.run_code.clone();
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), agent.run(root))
            .await
            .unwrap()
            .unwrap();
        assert!(
            result.final_output.contains("external subscription reply"),
            "{}",
            result.final_output
        );
        let children = host.subagents(&parent, USER).await.unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].record.agent_code, code);
        assert_eq!(children[0].run.status, fw::HistoryStatus::Completed);
    }
    let argv = std::fs::read_to_string(log).unwrap();
    let lines: Vec<_> = argv
        .lines()
        .filter(|line| line.contains("--model"))
        .collect();
    assert_eq!(lines.len(), 2, "{argv}");
    assert!(lines[0].contains("model-fast"), "{argv}");
    assert!(lines[1].contains("model-deep"), "{argv}");
    assert!(lines.iter().all(|line| !line.contains("resume")), "{argv}");
    assert!(script.requests.lock().unwrap().iter().any(|request| {
        request
            .messages
            .iter()
            .any(|message| message.content.text().contains("codex-deep"))
    }));
    host.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn named_system_agent_thread_resumes_native_session_with_its_bound_model() {
    let f = Fixture::new();
    let log = seed_external(&f);
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        Arc::new(Script::default()),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut root = context("root:system-thread");
    root.execution_scope.local_directory = Some(f.directory.path().to_string_lossy().into_owned());
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), agent.run(root))
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.final_output.contains("external subscription reply"),
        "{}",
        result.final_output
    );
    let argv = std::fs::read_to_string(log).unwrap();
    let lines: Vec<_> = argv
        .lines()
        .filter(|line| line.contains("--model"))
        .collect();
    assert_eq!(lines.len(), 2, "{argv}");
    assert!(
        lines.iter().all(|line| line.contains("model-fast")),
        "{argv}"
    );
    assert!(!lines[0].contains("resume"), "{argv}");
    assert!(
        lines[1].contains("resume") && lines[1].contains("external-session"),
        "{argv}"
    );
    host.close().await;
}

#[tokio::test]
async fn system_agent_profile_http_crud_keeps_model_identity_and_rejects_duplicate_codes() {
    use axum::http::{Method, StatusCode};
    let f = Fixture::new();
    seed_external(&f);
    let app = tron_backend::app::build_router_with_state(f.state.clone()).unwrap();
    let path = "/api/v1/agents/registry/system";
    let payload = json!({"agentCode":"codex-extra","name":"Extra","baseAgentCode":"codex","modelId":"model-extra"});
    let created =
        common::authenticated_request(&app, Method::POST, path, Some(payload.clone())).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    assert_eq!(
        common::response_json(created).await["data"]["agentCode"],
        "codex-extra"
    );
    assert_eq!(
        common::authenticated_request(&app, Method::POST, path, Some(payload))
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let detail = format!("{path}/codex-extra");
    assert_eq!(
        common::authenticated_request(
            &app,
            Method::PUT,
            &detail,
            Some(json!({"name":"Updated", "modelId":"new-model"}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    let listed =
        common::response_json(common::authenticated_request(&app, Method::GET, path, None).await)
            .await;
    let records = listed["data"].as_array().unwrap();
    let record = records
        .iter()
        .find(|record| record["agentCode"] == "codex-extra")
        .unwrap();
    assert_eq!(record["baseAgentCode"], "codex");
    assert_eq!(record["modelId"], "new-model");
    assert_eq!(record["name"], "Updated");
    assert_eq!(
        common::authenticated_request(&app, Method::DELETE, &format!("{path}/codex"), None)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        common::authenticated_request(&app, Method::DELETE, &detail, None)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        common::authenticated_request(&app, Method::DELETE, &detail, None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    f.state.agent_framework.close().await;
}
