use tron_backend::{
    app::AppState,
    application::model::ConfiguredProviderCommand,
    domain::model::{ConfiguredProviderModel, ConfiguredProviderProtocol, ModelInputCapability},
};

pub fn add_text_model(state: &AppState, name: &str) -> String {
    let provider = state
        .models
        .create_configured_provider(ConfiguredProviderCommand {
            provider_type: None,
            display_name: "Fixture provider".into(),
            protocol: ConfiguredProviderProtocol::OpenAiChatCompletions,
            base_url: "http://127.0.0.1:1/v1".into(),
            api_key: Some("synthetic".into()),
            request_headers: Default::default(),
            enabled: true,
            models: vec![ConfiguredProviderModel {
                dimension: None,
                model_type: Default::default(),
                model_code: None,
                model_id: name.into(),
                name: name.into(),
                context_window_tokens: Some(32000),
                max_output_tokens: Some(2048),
                reasoning: false,
                supports_developer_role: false,
                input_capabilities: vec![ModelInputCapability::Text],
                enabled: true,
            }],
        })
        .unwrap();
    provider.models[0].model_code.clone().unwrap()
}
