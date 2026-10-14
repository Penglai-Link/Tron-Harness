use super::*;
use crate::adapters::outbound::persistence::{
    SqliteAgentManagementRepository, SqliteWorkspaceRepository,
};
use crate::application::workspace::WorkspaceRepository;
use std::sync::atomic::{AtomicBool, Ordering};

struct Probe(Arc<AtomicBool>);

#[test]
fn system_profiles_require_qualified_opencode_model_ids() {
    assert!(validate_profile("OpenCode", "provider/model", AgentRuntimeType::OpenCodeRun).is_ok());
    for invalid in ["model", "/model", "provider/"] {
        assert!(validate_profile("OpenCode", invalid, AgentRuntimeType::OpenCodeRun).is_err());
    }
    assert!(validate_profile("Codex", "model", AgentRuntimeType::CodexExec).is_ok());
}
impl SystemCommandProbe for Probe {
    fn is_available(&self, command: &str) -> Result<bool, String> {
        Ok(command == "fixture-codex" && self.0.load(Ordering::SeqCst))
    }
    fn list_models(&self, _: &str, _: &str) -> Result<Vec<SystemAgentModelRecord>, String> {
        Ok(["model-fast", "model-deep"]
            .into_iter()
            .map(|model| SystemAgentModelRecord {
                supported_reasoning_efforts: Vec::new(),
                model_id: model.into(),
                display_name: model.into(),
                context_window_tokens: Some(128_000),
            })
            .collect())
    }
}
fn service(path: &str, available: Arc<AtomicBool>) -> AgentManagementService {
    AgentManagementService::with_probe(
        SqliteAgentManagementRepository::connect(path).unwrap(),
        WorkspaceService::new(SqliteWorkspaceRepository::connect(path).unwrap()),
        system_agents::supported_system_agents_from_config(&SystemAgentConfig {
            codex_command: "fixture-codex".into(),
            opencode_command: String::new(),
        }),
        Arc::new(Probe(available)),
    )
}
fn workspace(path: &str) {
    SqliteWorkspaceRepository::connect(path)
        .unwrap()
        .create_workspace(&crate::domain::workspace::Workspace {
            id: crate::shared::id::SnowflakeId::new(1),
            workspace_code: "SPACE".into(),
            name: "Space".into(),
            description: None,
            parent_workspace_code: None,
            status: WorkspaceStatus::Active,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            archived_at: None,
        })
        .unwrap();
}

#[test]
fn system_profiles_survive_restart_rescan_and_cli_removal_without_losing_model_or_policy() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("profiles.sqlite");
    let path = path.to_str().unwrap();
    let available = Arc::new(AtomicBool::new(true));
    let agents = service(path, available.clone());
    workspace(path);
    agents.rescan_system_agents().unwrap();
    agents
        .create_system_agent_profile("codex-fast", "Fast", "codex", "model-fast")
        .unwrap();
    agents
        .create_system_agent_profile("codex-deep", "Deep", "codex", "model-deep")
        .unwrap();
    agents
        .assign_agent_workspaces("codex-fast", vec!["SPACE".into()])
        .unwrap();
    agents.disable_platform_system_agent("codex-deep").unwrap();
    let agents = service(path, available.clone());
    let records = agents.rescan_system_agents().unwrap().records;
    let fast = records
        .iter()
        .find(|record| record.agent_code == "codex-fast")
        .unwrap();
    assert_eq!(fast.name, "Fast");
    assert_eq!(fast.models.len(), 1);
    assert_eq!(fast.models[0].model_id, "model-fast");
    assert_eq!(fast.models[0].context_window_tokens, Some(128_000));
    assert_eq!(
        agents.agent_workspace_grants("codex-fast").unwrap(),
        ["SPACE"]
    );
    assert!(
        !records
            .iter()
            .find(|record| record.agent_code == "codex-deep")
            .unwrap()
            .enabled
    );
    available.store(false, Ordering::SeqCst);
    let records = agents.rescan_system_agents().unwrap().records;
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|record| !record.available));
    assert!(matches!(
        agents.resolve_workspace_agent_runtime("SPACE", Some("codex-fast")),
        Err(AgentManagementError::SystemAgentUnavailable)
    ));
    available.store(true, Ordering::SeqCst);
    agents.rescan_system_agents().unwrap();
    assert_eq!(
        agents
            .resolve_workspace_agent_runtime("SPACE", Some("codex-fast"))
            .unwrap()
            .system_model_id
            .as_deref(),
        Some("model-fast")
    );
    assert!(
        !agents
            .list_platform_system_agents()
            .unwrap()
            .iter()
            .find(|record| record.agent_code == "codex-deep")
            .unwrap()
            .enabled
    );
}

#[test]
fn system_profiles_resolve_their_own_models_and_enforce_workspace_and_platform_policy() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("profiles.sqlite");
    let path = path.to_str().unwrap();
    let agents = service(path, Arc::new(AtomicBool::new(true)));
    workspace(path);
    agents.rescan_system_agents().unwrap();
    for (code, model) in [("codex-fast", "model-fast"), ("codex-deep", "model-deep")] {
        agents
            .create_system_agent_profile(code, code, "codex", model)
            .unwrap();
        let resolved = agents
            .resolve_workspace_agent_runtime("SPACE", Some(code))
            .unwrap();
        assert_eq!(resolved.system_model_id.as_deref(), Some(model));
        assert_eq!(resolved.command_name.as_deref(), Some("fixture-codex"));
        assert_eq!(
            agents
                .resolve_platform_agent_runtime(Some(code))
                .unwrap()
                .system_model_id
                .as_deref(),
            Some(model)
        );
    }
    agents
        .assign_agent_workspaces("codex-fast", vec![])
        .unwrap();
    assert!(matches!(
        agents.resolve_workspace_agent_runtime("SPACE", Some("codex-fast")),
        Err(AgentManagementError::SystemAgentUnavailable)
    ));
    agents
        .disable_workspace_system_agent("SPACE", "codex-deep")
        .unwrap();
    assert!(matches!(
        agents.resolve_workspace_agent_runtime("SPACE", Some("codex-deep")),
        Err(AgentManagementError::SystemAgentUnavailable)
    ));
    agents
        .enable_workspace_system_agent("SPACE", "codex-deep")
        .unwrap();
    agents
        .update_system_agent_profile("codex-deep", "Renamed", "manual-model")
        .unwrap();
    let record = agents
        .list_platform_system_agents()
        .unwrap()
        .into_iter()
        .find(|record| record.agent_code == "codex-deep")
        .unwrap();
    assert_eq!(record.models[0].model_id, "manual-model");
    agents.disable_platform_system_agent("codex-deep").unwrap();
    assert!(matches!(
        agents.resolve_platform_agent_runtime(Some("codex-deep")),
        Err(AgentManagementError::SystemAgentUnavailable)
    ));
    agents.delete_system_agent_profile("codex-fast").unwrap();
    assert!(matches!(
        agents.resolve_platform_agent_runtime(Some("codex-fast")),
        Err(AgentManagementError::SystemAgentNotFound)
    ));
}

#[test]
fn system_profiles_validate_identity_and_configuration_and_keep_scanned_agents_read_only() {
    let temp = tempfile::tempdir().unwrap();
    let agents = service(
        temp.path().join("profiles.sqlite").to_str().unwrap(),
        Arc::new(AtomicBool::new(true)),
    );
    agents.rescan_system_agents().unwrap();
    agents
        .create_system_agent_profile("codex-fast", "Fast", "codex", "model-fast")
        .unwrap();
    assert!(matches!(
        agents.create_system_agent_profile("codex-fast", "Other", "codex", "model-deep"),
        Err(AgentManagementError::InternalAgentCodeConflict)
    ));
    assert!(matches!(
        agents.create_system_agent_profile("main-agent", "Other", "codex", "model-deep"),
        Err(AgentManagementError::InternalAgentCodeConflict)
    ));
    assert!(matches!(
        agents.create_system_agent_profile("invalid code", "Other", "codex", "model-deep"),
        Err(AgentManagementError::InternalAgentCodeInvalid)
    ));
    assert!(matches!(
        agents.create_system_agent_profile("nested", "Other", "codex-fast", "model-deep"),
        Err(AgentManagementError::InternalAgentDefinitionInvalid)
    ));
    assert!(matches!(
        agents.create_system_agent_profile("empty", "Other", "codex", " "),
        Err(AgentManagementError::InternalAgentDefinitionInvalid)
    ));
    assert!(matches!(
        agents.update_system_agent_profile("codex", "Other", "model-deep"),
        Err(AgentManagementError::BuiltinAgentReadOnly)
    ));
    assert!(matches!(
        agents.delete_system_agent_profile("codex"),
        Err(AgentManagementError::BuiltinAgentReadOnly)
    ));
}
