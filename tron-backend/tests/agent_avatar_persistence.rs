mod common;

use tron_backend::{
    adapters::outbound::persistence::{SqliteAgentManagementRepository, SqliteWorkspaceRepository, SqliteMcpRepository},
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
    AgentManagementService::new(
        SqliteAgentManagementRepository::connect(database_url).unwrap(),
        WorkspaceService::new(SqliteWorkspaceRepository::connect(database_url).unwrap()),
        &AppConfig::default().system_agents,
    )
}

#[test]
fn avatars_persist_for_platform_and_workspace_with_inheritance_and_validation() {
    let config = common::prepared_config(AppConfig::default());
    let agents = service(&config.database_url);
    assert_eq!(
        agents.list_platform_internal_agents().unwrap()[0].avatar_id,
        "robot"
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
    ] {
        let created = agents
            .create_platform_internal_agent(CreateWorkspaceInternalAgentCommand {
                agent_code: format!("avatar-{avatar}"),
                name: avatar.into(),
                role_prompt: String::new(),
                loop_code: None,
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
                agent_code: "local-avatar".into(),
                name: "Local".into(),
                role_prompt: String::new(),
                loop_code: None,
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
        "agentCode":"legacy", "name":"Legacy", "rolePrompt":"Keep this role", "toolAssignmentMode":"ALL_PLATFORM_TOOLS", "mcpAccessMode":"DISABLED", "toolExposureMode":"DIRECT_LIST", "expandedDomainPersistenceMode":"STICKY_EXPANDED", "maxSteps":10, "maxToolCalls":10, "maxParallelToolCalls":1, "selectedToolCodes":[], "selectedServerCodes":[]
    })).unwrap();
    assert_eq!(config.avatar_id, "robot");
    assert_eq!(config.role_prompt, "Keep this role");
}
