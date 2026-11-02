mod common;

use tron_backend::{
    adapters::outbound::persistence::{
        SqliteAgentManagementRepository, SqliteMcpRepository, SqliteWorkspaceRepository,
    },
    application::{
        agent::{
            AgentManagementService, CreateWorkspaceInternalAgentCommand,
            UpdateWorkspaceInternalAgentDefinitionCommand,
        },
        workspace::WorkspaceService,
    },
    config::AppConfig,
    domain::agent::{AgentManagementError, PlatformInternalAgentConfig},
};

fn service(database_url: &str) -> AgentManagementService {
    // Agent policy deletion touches MCP foreign keys even without assignments.
    SqliteMcpRepository::connect(database_url).unwrap();
    let agents = AgentManagementService::new(
        SqliteAgentManagementRepository::connect(database_url).unwrap(),
        WorkspaceService::new(SqliteWorkspaceRepository::connect(database_url).unwrap()),
        &AppConfig::default().system_agents,
    );

    agents
}

#[test]
fn avatars_persist_for_platform_and_workspace_with_inheritance_and_validation() {
    let config = common::prepared_config(AppConfig::default());
    let agents = service(&config.database_url);
    assert_eq!(
        agents
            .list_platform_internal_agents()
            .unwrap()
            .into_iter()
            .find(|agent| agent.agent_code == "main-agent")
            .unwrap()
            .avatar_id,
        "red-blue-pills"
    );
    for avatar in [
        "robot",
        "fox",
        "cat",
        "owl",
        "astronaut",
        "wizard",
        "panda",
        "frog",
        "penguin",
        "raccoon",
        "rabbit",
        "dragon",
        "neo",
        "kucha",
    ] {
        let created = agents
            .create_platform_internal_agent(CreateWorkspaceInternalAgentCommand {
                description: String::new(),
                candidate_model_codes: Vec::new(),
                agent_code: format!("avatar-{avatar}"),
                name: avatar.into(),
                role_prompt: String::new(),

                avatar_id: Some(avatar.into()),
                group_name: None,
            })
            .unwrap();
        assert_eq!(created.avatar_id, avatar);
    }
    agents
        .assign_agent_workspaces("avatar-fox", vec!["TRON_CORE".into()])
        .unwrap();
    assert_eq!(
        agents
            .workspace_internal_agent_policy("TRON_CORE", "avatar-fox")
            .unwrap()
            .avatar_id,
        "fox"
    );
    agents
        .update_internal_agent_definition(
            "TRON_CORE",
            "avatar-fox",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                avatar_id: Some("owl".into()),
                ..Default::default()
            },
        )
        .unwrap();
    agents
        .update_platform_internal_agent_definition(
            "avatar-fox",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                avatar_id: Some("dragon".into()),
                ..Default::default()
            },
        )
        .unwrap();
    agents
        .create_internal_agent(
            "TRON_CORE",
            CreateWorkspaceInternalAgentCommand {
                description: String::new(),
                candidate_model_codes: Vec::new(),
                agent_code: "local-avatar".into(),
                name: "Local".into(),
                role_prompt: String::new(),

                avatar_id: Some("panda".into()),
                group_name: None,
            },
        )
        .unwrap();
    drop(agents);
    let agents = service(&config.database_url);
    assert_eq!(
        agents
            .workspace_internal_agent_policy("TRON_CORE", "avatar-fox")
            .unwrap()
            .avatar_id,
        "owl"
    );
    assert_eq!(
        agents
            .workspace_internal_agent_policy("TRON_CORE", "local-avatar")
            .unwrap()
            .avatar_id,
        "panda"
    );
    assert_eq!(
        agents
            .list_platform_internal_agents()
            .unwrap()
            .into_iter()
            .find(|agent| agent.agent_code == "avatar-fox")
            .unwrap()
            .avatar_id,
        "dragon"
    );
    agents
        .reset_internal_agent_policy("TRON_CORE", "avatar-fox")
        .unwrap();
    assert_eq!(
        agents
            .workspace_internal_agent_policy("TRON_CORE", "avatar-fox")
            .unwrap()
            .avatar_id,
        "dragon"
    );
    for invalid in [
        "",
        "unknown",
        "../../outside.png",
        "https://example.com/avatar.png",
    ] {
        let command = UpdateWorkspaceInternalAgentDefinitionCommand {
            avatar_id: Some(invalid.into()),
            ..Default::default()
        };
        assert!(matches!(
            agents.update_platform_internal_agent_definition("avatar-fox", command.clone()),
            Err(AgentManagementError::InternalAgentDefinitionInvalid)
        ));
        assert!(matches!(
            agents.update_internal_agent_definition("TRON_CORE", "avatar-fox", command),
            Err(AgentManagementError::InternalAgentDefinitionInvalid)
        ));
    }
    // Legacy JSON configs have no avatarId. Their other settings must survive.
    let config: PlatformInternalAgentConfig = serde_json::from_value(serde_json::json!({
        "agentCode":"legacy", "name":"Legacy", "rolePrompt":"Keep this role", "toolAssignmentMode":"ALL_PLATFORM_TOOLS", "mcpAccessMode":"DISABLED", "maxSteps":10, "maxToolCalls":10, "maxParallelToolCalls":1, "selectedToolCodes":[], "selectedServerCodes":[]
    })).unwrap();
    assert_eq!(config.avatar_id, "robot");
    assert_eq!(config.role_prompt, "Keep this role");
}

#[test]
fn built_in_dudes_have_independent_default_avatars_and_preserve_customization() {
    let config = common::prepared_config(AppConfig::default());
    let agents = service(&config.database_url);
    agents
        .update_platform_internal_agent_definition(
            "main-agent",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                ..Default::default()
            },
        )
        .unwrap();
    for (code, avatar) in [("main-dude", "neo"), ("memory-dude", "black-hole")] {
        agents.initialize_dude_definition(code, code, None).unwrap();
        let definition = agents
            .all_platform_internal_agents()
            .unwrap()
            .into_iter()
            .find(|agent| agent.agent_code == code)
            .unwrap();
        assert_eq!(definition.avatar_id, avatar);
    }
    agents
        .update_platform_internal_agent_definition(
            "main-dude",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                avatar_id: Some("fox".into()),
                ..Default::default()
            },
        )
        .unwrap();
    drop(agents);
    let agents = service(&config.database_url);
    agents
        .initialize_dude_definition("main-dude", "ignored", None)
        .unwrap();
    assert_eq!(
        agents
            .all_platform_internal_agents()
            .unwrap()
            .into_iter()
            .find(|agent| agent.agent_code == "main-dude")
            .unwrap()
            .avatar_id,
        "fox"
    );
}

#[test]
fn disabled_internal_agent_persists_and_blocks_platform_and_workspace_execution() {
    let config = common::prepared_config(AppConfig::default());
    let agents = service(&config.database_url);
    agents
        .assign_agent_workspaces("prd-agent", vec!["TRON_CORE".into()])
        .unwrap();
    agents
        .update_platform_internal_agent_definition(
            "prd-agent",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                enabled: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    drop(agents);
    let agents = service(&config.database_url);
    assert!(
        !agents
            .list_platform_internal_agents()
            .unwrap()
            .iter()
            .find(|a| a.agent_code == "prd-agent")
            .unwrap()
            .enabled
    );
    assert!(
        !agents
            .workspace_internal_agent_policy("TRON_CORE", "prd-agent")
            .unwrap()
            .enabled
    );
    assert!(
        agents
            .resolve_platform_agent_runtime(Some("prd-agent"))
            .is_err()
    );
    assert!(
        agents
            .resolve_workspace_agent_runtime("TRON_CORE", Some("prd-agent"))
            .is_err()
    );
    agents
        .update_platform_internal_agent_definition(
            "prd-agent",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                enabled: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        agents
            .resolve_platform_agent_runtime(Some("prd-agent"))
            .is_ok()
    );
    assert!(
        agents
            .resolve_workspace_agent_runtime("TRON_CORE", Some("prd-agent"))
            .is_ok()
    );
}
