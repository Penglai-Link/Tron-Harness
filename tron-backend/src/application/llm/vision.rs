use super::{LlmRuntimeError, LlmRuntimeService};
use crate::application::model::RuntimeModelConfig;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct VisionModelBinding {
    pub model_code: String,
    pub fingerprint: String,
    config: RuntimeModelConfig,
    authorization_scope: String,
}
impl LlmRuntimeService {
    pub fn embedding_batch_input_limit(
        &self,
        workspace: &str,
        model: &str,
    ) -> Result<Option<usize>, LlmRuntimeError> {
        self.model_resolver
            .embedding_batch_input_limit(workspace, model)
            .map_err(|error| LlmRuntimeError::model_resolution(error.to_string()))
    }
    pub fn embedding_accepts_images(
        &self,
        workspace: &str,
        model: &str,
    ) -> Result<bool, LlmRuntimeError> {
        self.model_resolver
            .embedding_accepts_images(workspace, model)
            .map_err(|error| LlmRuntimeError::model_resolution(error.to_string()))
    }
    pub async fn auto_vision_model_binding(
        &self,
        workspace: &str,
    ) -> Result<VisionModelBinding, LlmRuntimeError> {
        let model = self
            .model_resolver
            .auto_vision_runtime_model_config(workspace)
            .await
            .map_err(|e| LlmRuntimeError::model_resolution(e.to_string()))?
            .ok_or_else(|| {
                LlmRuntimeError::localized_model_resolution("llm.vision.default_model_unconfigured")
            })?;
        self.vision_model_binding(workspace, &model.model_code)
    }
    pub fn vision_model_binding(
        &self,
        workspace: &str,
        model: &str,
    ) -> Result<VisionModelBinding, LlmRuntimeError> {
        if !model.starts_with("PERSON_AI:FIXED:High:") {
            return Err(LlmRuntimeError::model_resolution(
                "llm.vision.fixed_quality_model_required",
            ));
        }
        let mut config = self
            .model_resolver
            .vision_runtime_model_config(workspace, model)
            .map_err(|_| {
                LlmRuntimeError::localized_model_resolution("llm.vision.model_unavailable")
            })?
            .ok_or_else(|| {
                LlmRuntimeError::localized_model_resolution("llm.vision.model_unavailable")
            })?;
        if config.model_code != model {
            return Err(LlmRuntimeError::localized_model_resolution(
                "llm.vision.model_changed",
            ));
        }
        if config.model_type != crate::domain::model::ModelType::Text
            || !matches!(
                config.protocol,
                crate::domain::model::ModelProtocol::OpenAiCompatible
                    | crate::domain::model::ModelProtocol::Responses
            )
        {
            return Err(LlmRuntimeError::model_resolution(
                "llm.vision.protocol_unsupported",
            ));
        }
        config.temperature = Some(0.0);
        config.max_tokens = Some(2048);
        config.max_retries = Some(0);
        config.thinking_enabled = false;
        config.reasoning_effort = None;
        let scope = self
            .client
            .model_authorization_scope(&config)
            .map_err(|_| {
                LlmRuntimeError::localized_model_resolution(
                    "llm.vision.model_authorization_unavailable",
                )
            })?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                LlmRuntimeError::localized_model_resolution(
                    "llm.vision.account_authorization_required",
                )
            })?;
        let cache_identity = self
            .model_resolver
            .vision_cache_identity()
            .map_err(|_| {
                LlmRuntimeError::localized_model_resolution("llm.vision.account_unavailable")
            })?
            .unwrap_or_else(|| scope.clone());
        let identity = serde_json::to_vec(&(
            &config.model_code,
            &config.model_name,
            config.protocol,
            &config.base_url,
            &config.api_path,
            &cache_identity,
            "kb-image-description-v1",
        ))
        .map_err(|_| LlmRuntimeError::localized_model_resolution("llm.vision.config_invalid"))?;
        Ok(VisionModelBinding {
            model_code: config.model_code.clone(),
            fingerprint: format!("{:x}", Sha256::digest(identity)),
            config,
            authorization_scope: scope,
        })
    }
    pub fn check_vision_binding(
        &self,
        binding: &VisionModelBinding,
    ) -> Result<(), LlmRuntimeError> {
        let scope = self
            .client
            .model_authorization_scope(&binding.config)
            .map_err(|_| {
                LlmRuntimeError::localized_model_resolution(
                    "llm.vision.model_authorization_unavailable",
                )
            })?;
        if scope.as_deref() != Some(&binding.authorization_scope) {
            return Err(LlmRuntimeError::localized_model_resolution(
                "llm.vision.account_changed",
            ));
        }
        Ok(())
    }
    pub async fn vision_generation_call(
        &self,
        binding: &VisionModelBinding,
        prompt: &str,
        mime: &str,
        bytes: &[u8],
    ) -> Result<String, LlmRuntimeError> {
        self.check_vision_binding(binding)?;
        if bytes.is_empty()
            || bytes.len() > 20 * 1024 * 1024
            || !matches!(mime, "image/jpeg" | "image/png" | "image/webp")
        {
            return Err(LlmRuntimeError::invalid_provider_response(
                "llm.vision.image_unsupported",
            ));
        }
        let result = self
            .client
            .generate_vision(&binding.config, prompt, mime, bytes)
            .await
            .map_err(|_| {
                LlmRuntimeError::localized_provider_failed("llm.vision.generation_failed")
            })?;
        self.check_vision_binding(binding)?;
        Ok(result)
    }
}
