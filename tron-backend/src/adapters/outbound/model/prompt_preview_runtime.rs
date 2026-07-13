use std::{future::Future, pin::Pin};

use crate::{
    application::{
        llm::LlmRuntimeClient,
        model::{ModelService, RuntimeModelConfig},
        prompt::{PromptPreviewCommand, PromptPreviewOutput, PromptPreviewRuntime},
    },
    domain::{
        model::{ModelType, ModelView},
        prompt::PromptError,
    },
};

use super::HttpLlmRuntimeClient;

pub struct PromptLlmPreviewRuntime {
    models: ModelService,
    client: HttpLlmRuntimeClient,
}

impl PromptLlmPreviewRuntime {
    pub fn new(models: ModelService, client: HttpLlmRuntimeClient) -> Self {
        Self { models, client }
    }
}

impl PromptPreviewRuntime for PromptLlmPreviewRuntime {
    fn validate_model(
        &self,
        workspace_code: Option<&str>,
        model_code: &str,
    ) -> Result<(), PromptError> {
        let model = match workspace_code {
            Some(workspace_code) => self.models.get_workspace_model(workspace_code, model_code),
            None => self.models.get_platform_model(model_code),
        }
        .map_err(|_| invalid_model_reference())?;
        if !model.enabled || model.model_type != ModelType::Text {
            return Err(invalid_model_reference());
        }
        Ok(())
    }

    fn generate<'a>(
        &'a self,
        command: PromptPreviewCommand,
    ) -> Pin<Box<dyn Future<Output = Result<PromptPreviewOutput, PromptError>> + Send + 'a>> {
        Box::pin(async move {
            let mut config = match command.workspace_code.as_deref() {
                Some(workspace_code) => self
                    .models
                    .text_runtime_model_config(workspace_code, command.model_code.as_deref())
                    .map_err(|error| PromptError::PreviewFailed(error.to_string()))?
                    .ok_or_else(|| {
                        PromptError::PreviewFailed("text model is unavailable".into())
                    })?,
                None => platform_text_runtime_config(&self.models, command.model_code.as_deref())?,
            };
            if command.temperature.is_some() {
                config.temperature = command.temperature;
            }
            if command.max_output_tokens.is_some() {
                config.max_tokens = command.max_output_tokens;
            }
            let output = self
                .client
                .generate(&config, &command.rendered_prompt)
                .await
                .map_err(PromptError::PreviewFailed)?;
            if output.trim().is_empty() {
                return Err(PromptError::PreviewFailed(
                    "provider returned empty output".into(),
                ));
            }
            Ok(PromptPreviewOutput {
                model_code: config.model_code,
                output,
            })
        })
    }
}

fn invalid_model_reference() -> PromptError {
    PromptError::InvalidInput(
        "modelCode must reference an enabled text model visible in prompt scope".into(),
    )
}

fn platform_text_runtime_config(
    models: &ModelService,
    model_code: Option<&str>,
) -> Result<RuntimeModelConfig, PromptError> {
    let requested = model_code.map(str::trim).filter(|value| !value.is_empty());
    let model = if let Some(model_code) = requested {
        models
            .get_platform_model(model_code)
            .map_err(|error| PromptError::PreviewFailed(error.to_string()))?
    } else {
        models
            .list_platform_models()
            .map_err(|error| PromptError::PreviewFailed(error.to_string()))?
            .into_iter()
            .find(|model| model.is_default && model.model_type == ModelType::Text)
            .ok_or_else(|| PromptError::PreviewFailed("text model is unavailable".into()))?
    };
    if !model.enabled || model.model_type != ModelType::Text {
        return Err(PromptError::PreviewFailed(
            "text model is unavailable".into(),
        ));
    }
    Ok(runtime_config_from_platform_model(model))
}

fn runtime_config_from_platform_model(model: ModelView) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: model.model_code,
        protocol: model.protocol,
        model_type: model.model_type,
        base_url: model.base_url,
        api_path: model.api_path,
        api_key: model.api_key_ref,
        model_name: model.model_name,
        thinking_enabled: model.thinking_enabled,
        responses_stateful_enabled: model.responses_stateful_enabled,
        temperature: model.temperature,
        max_tokens: model.max_tokens,
        context_window_tokens: model.context_window_tokens,
        top_p: model.top_p,
        max_retries: model.max_retries,
    }
}
