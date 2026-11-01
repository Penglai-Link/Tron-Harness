mod common;
use tron_backend::{
    adapters::outbound::persistence::{
        SqliteAgentManagementRepository, SqliteMcpRepository, SqliteWorkspaceRepository,
    },
    application::{
        agent::{
            AgentManagementService, CreateWorkspaceInternalAgentCommand,
            UpdateWorkspaceInternalAgentDefinitionCommand,
            UpdateWorkspaceInternalAgentPolicyCommand,
        },
        workspace::WorkspaceService,
    },
    config::AppConfig,
    domain::agent::AgentManagementError,
};
fn service(database: &str) -> AgentManagementService {
    SqliteMcpRepository::connect(database).unwrap();
    let agents = AgentManagementService::new(
        SqliteAgentManagementRepository::connect(database).unwrap(),
        WorkspaceService::new(SqliteWorkspaceRepository::connect(database).unwrap()),
        &AppConfig::default().system_agents,
    );

    agents.initialize_default_main_agent().unwrap();
    agents
}
fn create(code: &str, group: Option<&str>) -> CreateWorkspaceInternalAgentCommand {
    CreateWorkspaceInternalAgentCommand {
        description: String::new(),
        candidate_model_codes: Vec::new(),
        agent_code: code.into(),
        name: code.into(),
        role_prompt: "Preserve this role".into(),

        avatar_id: None,
        group_name: group.map(str::to_owned),
    }
}
fn regroup(group: &str) -> UpdateWorkspaceInternalAgentDefinitionCommand {
    UpdateWorkspaceInternalAgentDefinitionCommand {
        group_name: Some(group.into()),
        ..Default::default()
    }
}
#[test]
fn internal_agent_groups_persist_and_follow_definition_inheritance_without_changing_grants() {
    let config = common::prepared_config(AppConfig::default());
    let agents = service(&config.database_url);
    assert!(agents.list_platform_internal_agents().unwrap()[0].built_in);
    assert_eq!(
        agents.list_platform_internal_agents().unwrap()[0]
            .group_name
            .as_deref(),
        Some("主智能体")
    );
    assert_eq!(
        agents
            .create_internal_agent_group("", "  Engineering  ")
            .unwrap(),
        "Engineering"
    );
    agents
        .create_internal_agent_group("", "Engineering")
        .unwrap();
    agents
        .create_internal_agent_group("", "Empty group")
        .unwrap();
    agents
        .create_internal_agent_group("TRON_CORE", "Finance")
        .unwrap();
    agents
        .create_internal_agent_group("COMPLIANCE", "Legal")
        .unwrap();
    let platform = agents
        .create_platform_internal_agent(create("group-reviewer", Some("Engineering")))
        .unwrap();
    assert!(!platform.built_in);
    assert!(
        agents
            .agent_workspace_grants("group-reviewer")
            .unwrap()
            .is_empty()
    );
    assert!(
        !agents
            .list_internal_agent_groups("TRON_CORE")
            .unwrap()
            .contains(&"Engineering".into())
    );
    agents
        .assign_agent_workspaces("group-reviewer", vec!["TRON_CORE".into()])
        .unwrap();
    assert_eq!(
        agents
            .workspace_internal_agent_policy("TRON_CORE", "group-reviewer")
            .unwrap()
            .group_name
            .as_deref(),
        Some("Engineering")
    );
    agents
        .update_internal_agent_definition("TRON_CORE", "group-reviewer", regroup("Finance"))
        .unwrap();
    assert_eq!(
        agents
            .list_platform_internal_agents()
            .unwrap()
            .into_iter()
            .find(|a| a.agent_code == "group-reviewer")
            .unwrap()
            .group_name
            .as_deref(),
        Some("Engineering")
    );
    assert_eq!(
        agents
            .update_internal_agent_definition("TRON_CORE", "group-reviewer", regroup(""))
            .unwrap()
            .group_name,
        None
    );
    agents
        .reset_internal_agent_policy("TRON_CORE", "group-reviewer")
        .unwrap();
    assert_eq!(
        agents
            .workspace_internal_agent_policy("TRON_CORE", "group-reviewer")
            .unwrap()
            .group_name
            .as_deref(),
        Some("Engineering")
    );
    agents
        .create_internal_agent("TRON_CORE", create("local-grouped", Some("Finance")))
        .unwrap();
    assert!(matches!(
        agents.update_internal_agent_definition("TRON_CORE", "local-grouped", regroup("Legal")),
        Err(AgentManagementError::InternalAgentGroupInvalid)
    ));
    assert!(matches!(
        agents.create_platform_internal_agent(create("invalid-group", Some("Finance"))),
        Err(AgentManagementError::InternalAgentGroupInvalid)
    ));
    drop(agents);
    let agents = service(&config.database_url);
    assert_eq!(
        agents.list_internal_agent_groups("").unwrap(),
        vec![
            "Empty group",
            "Engineering",
            "主智能体",
            "开发",
            "探索",
            "设计"
        ]
    );
    let local = agents
        .workspace_internal_agent_policy("TRON_CORE", "local-grouped")
        .unwrap();
    assert_eq!(local.group_name.as_deref(), Some("Finance"));
    assert_eq!(local.role_prompt, "Preserve this role");
    assert_eq!(local.max_steps, platform.max_steps);
    assert_eq!(
        agents.agent_workspace_grants("group-reviewer").unwrap(),
        vec!["TRON_CORE"]
    );
}
#[test]
fn internal_agent_groups_validate_names_and_builtin_definitions_are_platform_editable() {
    let config = common::prepared_config(AppConfig::default());
    let agents = service(&config.database_url);
    for name in ["".to_owned(), " ".into(), "x".repeat(61), "a\nb".into()] {
        assert!(matches!(
            agents.create_internal_agent_group("", &name),
            Err(AgentManagementError::InternalAgentGroupInvalid)
        ));
    }
    agents
        .create_internal_agent_group("", "Engineering")
        .unwrap();
    for command in [
        regroup("Engineering"),
        UpdateWorkspaceInternalAgentDefinitionCommand {
            name: Some("Changed".into()),
            ..Default::default()
        },
        UpdateWorkspaceInternalAgentDefinitionCommand::default(),
    ] {
        agents
            .update_platform_internal_agent_definition("main-agent", command.clone())
            .unwrap();
        assert!(matches!(
            agents.update_internal_agent_definition("TRON_CORE", "MAIN-AGENT", command),
            Err(AgentManagementError::BuiltinAgentReadOnly)
        ));
    }
    let builtin_grants = agents.agent_workspace_grants("main-agent").unwrap();
    agents
        .update_platform_internal_agent_policy(
            "main-agent",
            UpdateWorkspaceInternalAgentPolicyCommand {
                max_steps: Some(25),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        agents
            .workspace_internal_agent_policy("TRON_CORE", "main-agent")
            .unwrap()
            .max_steps,
        25
    );
    assert_eq!(
        agents.agent_workspace_grants("main-agent").unwrap(),
        builtin_grants
    );
    assert!(agents.list_platform_internal_agents().unwrap()[0].built_in);
}
