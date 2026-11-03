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
            assert!(!config.thinking_enabled);
            assert!(config.reasoning_effort.is_none());
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
        route: None,
        model_code: code.into(),
        protocol: ModelProtocol::OpenAiCompatible,
        model_type: ModelType::Text,
        base_url: "http://model.test".into(),
        api_path: None,
        api_key: "test".into(),
        request_headers: Default::default(),
        model_name: code.into(),
        thinking_enabled: true,
        supports_developer_role: false,
        reasoning_effort: Some(crate::domain::reasoning::ReasoningEffort::High),
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: Some(128_000),
        top_p: None,
        max_retries: None,
    }
}
#[tokio::test]
async fn unavailable_or_invalid_text_generation_never_uses_the_chat_model() {
    for primary in [
        Err("No active provider key configured".into()),
        Ok("New chat".into()),
        Ok("<think>The user message is in Chinese".into()),
        Ok("<think>Only reasoning</think>".into()),
    ] {
        let models = Arc::new(TitleModels {
            primary,
            fallback: Ok("文档会诊".into()),
            calls: Mutex::new(vec![]),
        });
        let runtime = LlmRuntimeService::new(models.clone(), models.clone());
        let error = generate_title(&runtime, vec![config("auxiliary")], "Generate a title")
            .await
            .unwrap_err();
        assert!(matches!(
            error.0,
            "LLM_UPSTREAM_FAILED" | "TITLE_FORMAT_INVALID"
        ));
        assert_eq!(*models.calls.lock().unwrap(), vec!["auxiliary"]);
    }
}

#[test]
fn title_reasoning_blocks_are_removed_before_selecting_the_first_line() {
    for output in [
        "<think>The user message is in Chinese\nChoose a short title.</think>\nTron 架构分析",
        "<think>Analysis</think>Tron 架构分析",
        "\n<THINK>分析 <think>嵌套思考</think></THINK>\nTron 架构分析",
        "<analysis>Analysis</analysis><reasoning>More</reasoning>Tron 架构分析",
        "<thinking>Analysis</thinking>\nTron 架构分析",
        "Tron <think>private reasoning</think>架构分析",
    ] {
        assert_eq!(
            normalize_generated_session_title(output).as_deref(),
            Some("Tron 架构分析"),
            "{output}"
        );
    }
}

#[test]
fn title_reasoning_only_or_malformed_output_is_rejected() {
    for output in [
        "<think>The user message is in Chinese",
        "<think>Reasoning only</think>",
        "<think>Outer<think>Inner</think>",
        "Reasoning without opening tag</think>Title",
        "<think>Analysis</analysis>Title",
        "Title<think>truncated",
        "<think",
    ] {
        assert_eq!(normalize_generated_session_title(output), None, "{output}");
    }
}

#[test]
fn plain_titles_keep_existing_formatting_and_length_rules() {
    assert_eq!(
        normalize_generated_session_title("\nTitle: Tron 架构分析\nextra line").as_deref(),
        Some("Tron 架构分析")
    );
    assert_eq!(
        normalize_generated_session_title(&"架".repeat(80)).unwrap(),
        "架".repeat(64)
    );
}

#[tokio::test]
async fn reasoning_only_from_text_generation_is_a_title_format_error() {
    let models = Arc::new(TitleModels {
        primary: Ok("<think>Only reasoning</think>".into()),
        fallback: Ok("<think>Truncated reasoning".into()),
        calls: Mutex::new(vec![]),
    });
    let runtime = LlmRuntimeService::new(models.clone(), models.clone());
    let error = generate_title(&runtime, vec![config("auxiliary")], "Generate a title")
        .await
        .unwrap_err();
    assert_eq!(error.0, "TITLE_FORMAT_INVALID");
    assert_eq!(*models.calls.lock().unwrap(), vec!["auxiliary"]);
}

#[tokio::test]
async fn title_generation_does_not_force_provider_thinking_flags_on_the_http_wire() {
    for protocol in [ModelProtocol::OpenAiCompatible, ModelProtocol::Responses] {
        assert_title_generation_http_request(protocol).await;
    }
}

async fn assert_title_generation_http_request(protocol: ModelProtocol) {
    use crate::adapters::outbound::model::HttpLlmRuntimeClient;
    use axum::{Json, Router, routing::post};
    use eliteforge_agent_official_plugin_model_http::ModelHttpClient;
    use serde_json::json;

    let (path, reply) = if protocol == ModelProtocol::Responses {
        (
            "/responses",
            json!({"id":"title-response", "status":"completed", "output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"<think>Private analysis</think>Tron 架构分析"}]}]}),
        )
    } else {
        (
            "/chat/completions",
            json!({"choices":[{"message":{"role":"assistant","content":"<think>The user message is in Chinese.</think>Tron 架构分析"},"finish_reason":"stop"}]}),
        )
    };
    let (sent, received) = tokio::sync::oneshot::channel();
    let sent = Arc::new(Mutex::new(Some(sent)));
    let app = Router::new().route(
        path,
        post(move |Json(body): Json<Value>| {
            let sent = sent.clone();
            let reply = reply.clone();
            async move {
                sent.lock().unwrap().take().unwrap().send(body).unwrap();
                Json(reply)
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut model = config("selected");
    model.protocol = protocol;
    model.base_url = format!("http://{}", listener.local_addr().unwrap());
    model.api_path = Some(path.into());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let resolver = Arc::new(TitleModels {
        primary: Err("unused".into()),
        fallback: Err("unused".into()),
        calls: Mutex::new(vec![]),
    });
    let runtime = LlmRuntimeService::new(
        resolver,
        Arc::new(HttpLlmRuntimeClient::new(ModelHttpClient::new())),
    );
    let result = generate_title(&runtime, vec![model], "Generate a title").await;
    server.abort();
    assert_eq!(result.unwrap(), "Tron 架构分析");
    let body = received.await.unwrap();
    if protocol == ModelProtocol::Responses {
        assert!(body.get("reasoning").is_none());
    } else {
        assert!(body.get("enable_thinking").is_none());
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
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
        generate_title(&runtime, vec![config("auxiliary")], "Generate a title")
            .await
            .unwrap(),
        "辅助模型标题"
    );
    assert_eq!(*models.calls.lock().unwrap(), vec!["auxiliary"]);
}
#[tokio::test]
async fn missing_text_generation_route_makes_no_model_calls() {
    let models = Arc::new(TitleModels {
        primary: Err("unused".into()),
        fallback: Err("chat model unavailable".into()),
        calls: Mutex::new(vec![]),
    });
    let runtime = LlmRuntimeService::new(models.clone(), models.clone());
    let result = generate_title(&runtime, vec![], "Generate a title").await;
    assert_eq!(result.unwrap_err().0, "LLM_MODEL_UNAVAILABLE");
    assert!(models.calls.lock().unwrap().is_empty());
}
