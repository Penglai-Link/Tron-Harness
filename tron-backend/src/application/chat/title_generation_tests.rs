use super::*;
use crate::application::llm::{LlmRuntimeClient, LlmRuntimeModelResolver, LlmRuntimeRerankHit};
use crate::application::model::RuntimeModelConfig;
use crate::domain::model::{ModelError, ModelProtocol, ModelType};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

struct TitleModels {
    primary: Result<String, String>,
    fallback: Result<String, String>,
    calls: Mutex<Vec<String>>,
}
impl LlmRuntimeModelResolver for TitleModels {
    fn embedding_runtime_model_config(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        Ok(None)
    }
    fn rerank_runtime_model_config(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        Ok(None)
    }
}
impl LlmRuntimeClient for TitleModels {
    fn generate<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push(config.model_code.clone());
            if config.model_code == "auxiliary" {
                self.primary.clone()
            } else {
                self.fallback.clone()
            }
        })
    }
    fn embed<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async { unreachable!() })
    }
    fn rerank<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
        _: &'a [String],
        _: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        Box::pin(async { unreachable!() })
    }
}
fn config(code: &str) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: code.into(),
        protocol: ModelProtocol::OpenAiCompatible,
        model_type: ModelType::Text,
        base_url: "http://model.test".into(),
        api_path: None,
        api_key: "test".into(),
        model_name: code.into(),
        thinking_enabled: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: Some(128_000),
        top_p: None,
        max_retries: None,
    }
}
#[tokio::test]
async fn unavailable_auxiliary_key_and_invalid_title_fall_back_to_the_selected_chat_model() {
    for primary in [
        Err("No active provider key configured".into()),
        Ok("New chat".into()),
    ] {
        let models = Arc::new(TitleModels {
            primary,
            fallback: Ok("文档会诊".into()),
            calls: Mutex::new(vec![]),
        });
        let runtime = LlmRuntimeService::new(models.clone(), models.clone());
        let title = generate_title(
            &runtime,
            title_model_candidates(Some(config("auxiliary")), config("selected")),
            "Generate a title",
        )
        .await
        .unwrap();
        assert_eq!(title, "文档会诊");
        assert_eq!(*models.calls.lock().unwrap(), vec!["auxiliary", "selected"]);
    }
}
#[tokio::test]
async fn valid_auxiliary_title_does_not_spend_a_second_model_call() {
    let models = Arc::new(TitleModels {
        primary: Ok("辅助模型标题".into()),
        fallback: Err("unused".into()),
        calls: Mutex::new(vec![]),
    });
    let runtime = LlmRuntimeService::new(models.clone(), models.clone());
    assert_eq!(
        generate_title(
            &runtime,
            title_model_candidates(Some(config("auxiliary")), config("selected")),
            "Generate a title"
        )
        .await
        .unwrap(),
        "辅助模型标题"
    );
    assert_eq!(*models.calls.lock().unwrap(), vec!["auxiliary"]);
}
#[tokio::test]
async fn missing_auxiliary_route_uses_the_chat_model_and_total_failure_remains_an_error() {
    let models = Arc::new(TitleModels {
        primary: Err("unused".into()),
        fallback: Err("chat model unavailable".into()),
        calls: Mutex::new(vec![]),
    });
    let runtime = LlmRuntimeService::new(models.clone(), models.clone());
    let result = generate_title(
        &runtime,
        title_model_candidates(None, config("selected")),
        "Generate a title",
    )
    .await;
    assert_eq!(result.unwrap_err().0, "LLM_UPSTREAM_FAILED");
    assert_eq!(*models.calls.lock().unwrap(), vec!["selected"]);
    assert_eq!(
        title_model_candidates(Some(config("selected")), config("selected")).len(),
        1
    );
}
