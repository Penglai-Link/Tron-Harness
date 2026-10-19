mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
#[path = "support/candidate_model_fixture.rs"]
mod models;
use fixture::*;
use models::add_text_model;
use tron_backend::application::{
    agent::{CreateWorkspaceInternalAgentCommand, UpdateWorkspaceInternalAgentDefinitionCommand},
    model_router::AgentModelRouter,
};

struct HostRouter(String);
impl eliteforge_agent_core::ChatAutoRouter for HostRouter {
    fn select(
        &self,
        context: &eliteforge_agent_core::AutoRouteContext<'_>,
        candidates: &[eliteforge_agent_core::AutoRouteCandidate],
    ) -> Result<eliteforge_agent_core::ModelRef, eliteforge_agent_core::AgentError> {
        assert_eq!(
            context.operation,
            eliteforge_agent_core::ModelOperation::Chat
        );
        assert!(context.affinity.contains("custom-session"));
        candidates
            .iter()
            .find(|candidate| candidate.model.model.contains(&self.0))
            .map(|candidate| candidate.model.clone())
            .ok_or_else(|| eliteforge_agent_core::AgentError::Run("host candidate missing".into()))
    }
}

#[tokio::test]
async fn host_can_replace_the_pack_router_using_its_authorized_catalog() {
    let f = Fixture::new();
    let a = add_text_model(&f.state, "Custom A");
    let b = add_text_model(&f.state, "Custom B");
    let router = AgentModelRouter::new(f.state.models.clone(), f.state.model_providers.clone())
        .with_auto_router(std::sync::Arc::new(HostRouter(b.clone())));
    let (model, route) = router
        .resolve_initial(WORKSPACE, "custom-session", "custom-agent", None)
        .unwrap();
    assert_eq!(model.model_code, b);
    assert_eq!(route.authorized_candidates.len(), 2);
    assert!(
        route
            .authorized_candidates
            .iter()
            .any(|c| c.config.model_code == a)
    );
    assert_eq!(
        router
            .resolve_initial(WORKSPACE, "custom-session", "custom-agent", None)
            .unwrap()
            .0
            .model_code,
        b
    );
    assert!(f.state.agent_framework.close().await.is_success());
}

#[tokio::test]
async fn candidate_models_persist_inherit_and_bound_initial_and_refreshed_auto_selection() {
    let f = Fixture::new();
    models::seed_loop(&f.state);
    let a = add_text_model(&f.state, "Model A");
    let b = add_text_model(&f.state, "Model B");
    let management = &f.state.agent_management;
    let command = |code: &str| CreateWorkspaceInternalAgentCommand {
        description: String::new(),
        candidate_model_codes: vec![a.clone(), a.clone()],
        agent_code: code.into(),
        name: code.into(),
        role_prompt: String::new(),
        loop_code: Some("test-react".into()),
        avatar_id: None,
        group_name: None,
    };
    let created = management
        .create_platform_internal_agent(command("restricted"))
        .unwrap();
    assert_eq!(created.candidate_model_codes, vec![a.clone()]);
    management
        .assign_agent_workspaces("restricted", vec![WORKSPACE.into()])
        .unwrap();
    assert_eq!(
        management
            .workspace_internal_agent_policy(WORKSPACE, "restricted")
            .unwrap()
            .candidate_model_codes,
        vec![a.clone()]
    );
    let workspace = management
        .create_internal_agent(WORKSPACE, command("local-agent"))
        .unwrap();
    assert_eq!(workspace.candidate_model_codes, vec![a.clone()]);
    management
        .update_internal_agent_definition(
            WORKSPACE,
            "local-agent",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                candidate_model_codes: Some(vec![b.clone()]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        management
            .workspace_internal_agent_policy(WORKSPACE, "local-agent")
            .unwrap()
            .candidate_model_codes,
        vec![b.clone()]
    );
    let router = AgentModelRouter::new(f.state.models.clone(), f.state.model_providers.clone())
        .with_agent_management(management.clone());
    let (model, mut route) = router
        .resolve_initial(WORKSPACE, "session", "restricted", None)
        .unwrap();
    assert_eq!(model.model_code, a);
    assert_eq!(route.authorized_candidates.len(), 1);
    assert_eq!(
        router
            .authorized_models_for_route(WORKSPACE, &route, &model)
            .unwrap()
            .iter()
            .map(|m| m.model_code.clone())
            .collect::<Vec<_>>(),
        vec![a.clone()]
    );
    let (outside, _) = router
        .resolve_initial(WORKSPACE, "session", "local-agent", None)
        .unwrap();
    assert_eq!(outside.model_code, b);
    assert!(
        router
            .mark_success(WORKSPACE, &mut route, &outside)
            .is_err()
    );
    // Editing another workspace field must preserve model inheritance.
    management
        .update_internal_agent_definition(
            WORKSPACE,
            "restricted",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                name: Some("Workspace name".into()),
                ..Default::default()
            },
        )
        .unwrap();
    // A stale sticky assignment must not override an edited candidate list.
    management
        .update_platform_internal_agent_definition(
            "restricted",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                candidate_model_codes: Some(vec![b.clone()]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        router
            .resolve_initial(WORKSPACE, "session", "restricted", None)
            .unwrap()
            .0
            .model_code,
        b
    );
    assert!(
        router
            .authorized_models_for_route(WORKSPACE, &route, &model)
            .unwrap()
            .is_empty()
    );
    // An explicitly empty workspace override means all models, not platform inheritance.
    management
        .update_internal_agent_definition(
            WORKSPACE,
            "restricted",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                candidate_model_codes: Some(vec![]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        router
            .resolve_initial(WORKSPACE, "workspace-all", "restricted", None)
            .unwrap()
            .1
            .authorized_candidates
            .len(),
        2
    );
    management
        .reset_internal_agent_policy(WORKSPACE, "restricted")
        .unwrap();
    // Invalid or unavailable configured candidates must fail closed.
    management
        .update_platform_internal_agent_definition(
            "restricted",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                candidate_model_codes: Some(vec!["missing".into()]),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        router
            .resolve_initial(WORKSPACE, "session", "restricted", None)
            .is_err()
    );
    management
        .update_platform_internal_agent_definition(
            "restricted",
            UpdateWorkspaceInternalAgentDefinitionCommand {
                candidate_model_codes: Some(vec![]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        router
            .resolve_initial(WORKSPACE, "session", "restricted", None)
            .unwrap()
            .1
            .authorized_candidates
            .len(),
        2
    );
    assert!(f.state.agent_framework.close().await.is_success());
}
