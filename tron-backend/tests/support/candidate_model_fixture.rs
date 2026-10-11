use tron_backend::{
    app::AppState,
    application::model::ModelCommand,
    domain::model::{ModelProtocol, ModelType, default_model_capabilities_for_type},
};

pub fn add_text_model(state: &AppState, name: &str) -> String {
    state
        .models
        .create_platform_model(ModelCommand {
            display_name: name.into(),
            vendor: "fixture".into(),
            protocol: ModelProtocol::OpenAiCompatible,
            model_type: ModelType::Text,
            model_capabilities: default_model_capabilities_for_type(ModelType::Text),
            base_url: "http://127.0.0.1:1/v1".into(),
            api_path: None,
            api_key_ref: "synthetic".into(),
            model_name: name.into(),
            enabled: true,
            description: None,
            thinking_enabled: false,
            supported_reasoning_efforts: vec![],
            default_reasoning_effort: None,
            structured_output_enabled: false,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: Some(2048),
            context_window_tokens: Some(32000),
            top_p: None,
            max_retries: None,
            allow_inheritance: Some(true),
        })
        .unwrap()
        .model_code
}

pub fn seed_loop(state: &AppState) {
    let agents = &state.agent_management;
    if !agents
        .list_loop_designs()
        .unwrap()
        .iter()
        .any(|design| design.loop_code == "test-react")
    {
        agents.save_loop_design(serde_json::from_value(serde_json::json!({"loopCode":"test-react","name":"Test React","kind":"React","version":0,"updatedAt":0,"nodes":[],"edges":[],"maxParallelNodes":1})).unwrap(), 0).unwrap();
    }
    for agent in agents.list_platform_internal_agents().unwrap() {
        if agent.loop_code.is_none() {
            agents.update_platform_internal_agent_definition(&agent.agent_code, tron_backend::application::agent::UpdateWorkspaceInternalAgentDefinitionCommand { loop_code: Some("test-react".into()), ..Default::default() }).unwrap();
        }
    }
}
