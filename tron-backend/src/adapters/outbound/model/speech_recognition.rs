//! 将产品配置投影到 Framework 语音能力；协议实现归 Framework 供应商适配。
use super::*;
use base64::Engine;
use fw::ModelOperations;

pub(super) async fn transcribe(config: &RuntimeModelConfig, audio: Vec<u8>, mime: &str, file_name: &str) -> Result<String, String> {
    let wire = sdk_config(&runtime_to_agent_model_config(config))?;
    let model = fw::ModelRef::new("speech", &config.model_code);
    let mut update = eliteforge_agent_framework::ModelRuntimeUpdate::new("speech", 0, vec![]);
    update.recognition.push(fw::SpeechRecognitionModelService::new(model.clone(), Arc::new(sdk::HttpSpeechRecognitionModel { config: wire })));
    let client = eliteforge_agent_framework::ModelClient::new(update).map_err(|e| e.to_string())?
        .with_options(fw::ModelInvocationOptions {
            timeout: std::time::Duration::from_secs(120), max_input_bytes: 12 * 1024 * 1024,
            ..Default::default()
        });
    client.transcribe(fw::SpeechRecognitionRequest {
        choice: fw::ModelChoice::model(model),
        audio: fw::MediaContent {
            mime_type: mime.into(), name: Some(file_name.into()), estimated_tokens: None,
            source: fw::MediaSource::Base64(base64::engine::general_purpose::STANDARD.encode(audio)),
        },
        language: None,
    }).await.map(|response| response.text).map_err(|e| {
        tracing::error!(model = %config.model_name, error = %e, "Speech transcription failed");
        e.to_string()
    })
}
