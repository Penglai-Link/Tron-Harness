//! Explicitly opted-in probe of the real, selected PersonAI account. No secrets
//! or reasoning content are printed; ordinary test runs never make paid calls.
use serde_json::json;
use std::sync::Arc;
use tron_backend::{
    adapters::outbound::crypto::Sm4GcmVariableCipher,
    application::model_provider::ModelProviderService,
    config::AppConfig,
    domain::{model_provider::ProviderModelQuality, reasoning::ReasoningEffort},
};

#[tokio::test]
#[ignore = "requires TRON_LIVE_PERSONAI_PROBE=1 and an explicitly selected local configuration"]
async fn high_quality_personai_models_accept_deep_reasoning_and_explicit_output_budget() {
    assert_eq!(
        std::env::var("TRON_LIVE_PERSONAI_PROBE").as_deref(),
        Ok("1")
    );
    assert!(std::env::var_os("TRON_BACKEND_APPLICATION_CONFIG_PATH").is_some());
    assert!(std::env::var_os("TRON_BACKEND_DATABASE_URL").is_some());
    let config = AppConfig::from_application_yaml().unwrap();
    let providers = ModelProviderService::connect(
        &config.database_url,
        Arc::new(Sm4GcmVariableCipher::from_config(config.variable_crypto).unwrap()),
    )
    .unwrap();
    let candidates = providers
        .auto_chat_candidates(&[], Some(ReasoningEffort::High))
        .unwrap();
    let mut receipts = vec![];
    for (_, selection, mut model) in candidates {
        if selection.quality_type != Some(ProviderModelQuality::High) {
            continue;
        }
        // Probe only protocols actually advertised by this account's catalog.
        if model.protocol != tron_backend::domain::agent::AgentModelProtocol::ChatCompletions {
            continue;
        }
        model.max_tokens = Some(32_768);
        let transport = providers.sdk_model_transport(&model).await.unwrap();
        let request = json!({
            "model":model.model_name,
            "messages":[{"role":"user","content":"Return exactly the JSON object {\"ok\":true}. Do not call tools."}],
            "thinking":{"type":"enabled"},"reasoning_effort":"high","max_tokens":32768,
        });
        let response = transport.json("chat/completions", &request).await.unwrap();
        let choice = &response["choices"][0];
        let answer = choice["message"]["content"].as_str().unwrap_or("");
        assert!(
            !answer.trim().is_empty(),
            "model returned reasoning without an answer"
        );
        assert_eq!(choice["finish_reason"], "stop");
        let receipt = json!({
            "requestedModel":model.model_name,"resolvedModel":response["model"],
            "thinkingEnabled":true,"reasoningEffort":"high","maxTokens":32768,
            "answerPresent":true,"finishReason":choice["finish_reason"],
            "usage":response["usage"],
        });
        println!("PERSONAI_PARAMETER_RECEIPT {receipt}");
        receipts.push(receipt);
    }
    assert!(
        !receipts.is_empty(),
        "no advertised High-quality Chat model was probed"
    );
    if let Some(path) = std::env::var_os("TRON_LIVE_PERSONAI_REPORT_PATH") {
        std::fs::write(path, serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
    }
}
