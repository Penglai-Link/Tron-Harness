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
    pub fn vision_model_binding(
        &self,
        workspace: &str,
        model: &str,
    ) -> Result<VisionModelBinding, LlmRuntimeError> {
        if !model.starts_with("PERSON_AI:FIXED:High:") {
            return Err(LlmRuntimeError::model_resolution(
                "请选择固定的 PersonAI 高质量视觉模型",
            ));
        }
        let mut config = self
            .model_resolver
            .vision_runtime_model_config(workspace, model)
            .map_err(|_| LlmRuntimeError::model_resolution("图片描述模型不可用"))?
            .ok_or_else(|| LlmRuntimeError::model_resolution("图片描述模型不可用"))?;
        if config.model_code != model {
            return Err(LlmRuntimeError::model_resolution("图片描述模型发生变化"));
        }
        if config.model_type != crate::domain::model::ModelType::Text || !matches!(config.protocol, crate::domain::model::ModelProtocol::OpenAiCompatible | crate::domain::model::ModelProtocol::Responses) {
            return Err(LlmRuntimeError::model_resolution("图片描述模型协议不受支持"));
        }
        config.temperature = Some(0.0);
        config.max_tokens = Some(2048);
        config.max_retries = Some(0);
        config.thinking_enabled = false;
        config.reasoning_effort = None;
        let scope = self
            .client
            .model_authorization_scope(&config)
            .map_err(|_| LlmRuntimeError::model_resolution("图片描述模型授权不可用"))?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| LlmRuntimeError::model_resolution("图片描述需要账户绑定的模型授权"))?;
        let cache_identity = self.model_resolver.vision_cache_identity().map_err(|_| LlmRuntimeError::model_resolution("图片描述账户不可用"))?.unwrap_or_else(|| scope.clone());
        let identity = serde_json::to_vec(&(
            &config.model_code,
            &config.model_name,
            config.protocol,
            &config.base_url,
            &config.api_path,
            &cache_identity,
            "kb-image-description-v1",
        ))
        .map_err(|_| LlmRuntimeError::model_resolution("图片描述配置无效"))?;
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
            .map_err(|_| LlmRuntimeError::model_resolution("图片描述模型授权不可用"))?;
        if scope.as_deref() != Some(&binding.authorization_scope) {
            return Err(LlmRuntimeError::model_resolution("模型账户已改变，请重试"));
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
                "图片格式或大小不受支持",
            ));
        }
        let result = self
            .client
            .generate_vision(&binding.config, prompt, mime, bytes)
            .await
            .map_err(|_| LlmRuntimeError::provider_failed("图片描述生成失败，请重试"))?;
        self.check_vision_binding(binding)?;
        Ok(result)
    }
}
