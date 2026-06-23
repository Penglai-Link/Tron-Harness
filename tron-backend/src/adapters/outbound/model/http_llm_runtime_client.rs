use std::{future::Future, pin::Pin};

use serde_json::{Value, json};

use crate::{
    application::{
        llm::{LlmRuntimeClient, LlmRuntimeRerankHit},
        model::RuntimeModelConfig,
    },
    domain::{
        agent::{AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest, ModelResponse},
        model::{ModelProtocol, ModelType},
    },
};

use super::protocol_adapters::adapter_for_protocol;

#[derive(Debug, Clone, Default)]
pub struct HttpLlmRuntimeClient {
    client: reqwest::Client,
}

impl HttpLlmRuntimeClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl LlmRuntimeClient for HttpLlmRuntimeClient {
    fn generate<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move { self.generate_inner(config, prompt).await })
    }

    fn embed<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move { self.embed_inner(config, input).await })
    }

    fn rerank<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        query: &'a str,
        documents: &'a [String],
        top_n: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        Box::pin(async move { self.rerank_inner(config, query, documents, top_n).await })
    }
}

impl HttpLlmRuntimeClient {
    async fn generate_inner(
        &self,
        config: &RuntimeModelConfig,
        prompt: &str,
    ) -> Result<String, String> {
        if config.model_type != ModelType::Text {
            return Err(format!(
                "text generation call requires TEXT modelType, got {:?}",
                config.model_type
            ));
        }
        let model_config = runtime_to_agent_model_config(config)?;
        let adapter = adapter_for_protocol(model_config.protocol)
            .ok_or_else(|| "model protocol was unsupported".to_string())?;
        let path = config.api_path.as_deref().unwrap_or(adapter.default_path());
        let url = endpoint_url(config, path)?;
        let request = ModelRequest {
            messages: vec![AgentMessage::user(prompt)],
            tools: Vec::new(),
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        };
        let body = adapter.request_body(&model_config, &request)?;
        let response_body = self
            .post_json_with_retry(config, url.as_str(), &body)
            .await?;
        match adapter.parse_response(&response_body)? {
            ModelResponse::FinalOutput { output, .. } => Ok(output),
            ModelResponse::ToolCalls { .. } => {
                Err("text generation response unexpectedly requested tool calls".into())
            }
        }
    }

    async fn embed_inner(
        &self,
        config: &RuntimeModelConfig,
        input: &str,
    ) -> Result<Vec<f32>, String> {
        if config.model_type != ModelType::Embedding {
            return Err(format!(
                "embedding call requires EMBEDDING modelType, got {:?}",
                config.model_type
            ));
        }
        if config.protocol != ModelProtocol::OpenAiCompatible {
            return Err(format!(
                "embedding call requires OPENAI_COMPATIBLE protocol, got {:?}",
                config.protocol
            ));
        }

        let path = config.api_path.as_deref().unwrap_or("/embeddings");
        let url = endpoint_url(config, path)?;
        let body = json!({
            "model": config.model_name,
            "input": input,
        });
        let response_body = self
            .post_json_with_retry(config, url.as_str(), &body)
            .await?;
        parse_embedding_response(&response_body)
    }

    async fn rerank_inner(
        &self,
        config: &RuntimeModelConfig,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<LlmRuntimeRerankHit>, String> {
        if config.model_type != ModelType::Rerank {
            return Err(format!(
                "rerank call requires RERANK modelType, got {:?}",
                config.model_type
            ));
        }
        if config.protocol != ModelProtocol::Rerank {
            return Err(format!(
                "rerank call requires RERANK protocol, got {:?}",
                config.protocol
            ));
        }

        let path = config.api_path.as_deref().unwrap_or("/rerank");
        let url = endpoint_url(config, path)?;
        let mut body = json!({
            "model": config.model_name,
            "query": query,
            "documents": documents,
        });
        if let Some(top_n) = top_n
            && top_n > 0
        {
            body["top_n"] = json!(top_n);
        }
        let response_body = self
            .post_json_with_retry(config, url.as_str(), &body)
            .await?;
        parse_rerank_response(&response_body)
    }

    async fn post_json_with_retry(
        &self,
        config: &RuntimeModelConfig,
        url: &str,
        body: &Value,
    ) -> Result<Value, String> {
        let attempts = config.max_retries.unwrap_or(0).saturating_add(1);
        let mut last_error = None;
        for _ in 0..attempts {
            let response = self.request_builder(config, url, body).send().await;
            match response {
                Ok(response) if response.status().is_success() => {
                    return response.json().await.map_err(provider_error);
                }
                Ok(response) => {
                    let status = response.status();
                    let body: Value = response.json().await.unwrap_or(Value::Null);
                    last_error = Some(provider_status_error(status, &body));
                }
                Err(error) => {
                    last_error = Some(provider_error(error));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| "llm runtime request failed".into()))
    }

    fn request_builder(
        &self,
        config: &RuntimeModelConfig,
        url: &str,
        body: &Value,
    ) -> reqwest::RequestBuilder {
        self.client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(body)
    }
}

fn parse_embedding_response(body: &Value) -> Result<Vec<f32>, String> {
    let Some(data) = body.get("data").and_then(Value::as_array) else {
        return Err("embedding response missing data array".into());
    };
    let Some(first) = data.first() else {
        return Err("embedding response data array is empty".into());
    };
    let Some(raw_vector) = first.get("embedding").and_then(Value::as_array) else {
        return Err("embedding response missing embedding array".into());
    };
    raw_vector
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_f64()
                .map(|number| number as f32)
                .ok_or_else(|| format!("embedding value at index {index} was not numeric"))
        })
        .collect()
}

fn parse_rerank_response(body: &Value) -> Result<Vec<LlmRuntimeRerankHit>, String> {
    let items = body
        .get("results")
        .or_else(|| body.get("data"))
        .and_then(Value::as_array)
        .ok_or_else(|| "rerank response missing results/data array".to_string())?;
    let mut hits = Vec::with_capacity(items.len());
    for (array_index, item) in items.iter().enumerate() {
        let document_index = item
            .get("index")
            .or_else(|| item.get("document_index"))
            .or_else(|| item.pointer("/document/index"))
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("rerank result {array_index} missing index"))?
            as usize;
        let score = item
            .get("relevance_score")
            .or_else(|| item.get("score"))
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("rerank result {array_index} missing score"))?;
        hits.push(LlmRuntimeRerankHit {
            document_index,
            score,
        });
    }
    Ok(hits)
}

fn provider_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "llm runtime connection timed out".into()
    } else {
        error.to_string()
    }
}

fn provider_status_error(status: reqwest::StatusCode, body: &Value) -> String {
    let detail = body
        .pointer("/error/message")
        .or_else(|| body.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("llm runtime request failed");
    format!("llm runtime provider returned {status}: {detail}")
}

fn endpoint_url(config: &RuntimeModelConfig, path: &str) -> Result<String, String> {
    let base = config.base_url.trim().trim_end_matches('/');
    let path = path.trim().trim_start_matches('/');
    if base.is_empty() || path.is_empty() {
        return Err("model baseUrl and apiPath are required".into());
    }
    Ok(format!("{base}/{path}"))
}

fn runtime_to_agent_model_config(config: &RuntimeModelConfig) -> Result<AgentModelConfig, String> {
    let protocol = match (config.protocol, config.model_type) {
        (ModelProtocol::OpenAiCompatible, ModelType::Text) => AgentModelProtocol::ChatCompletions,
        (ModelProtocol::Responses, ModelType::Text) => AgentModelProtocol::Responses,
        _ => {
            return Err(format!(
                "text generation call requires text-capable protocol/modelType, got {:?}/{:?}",
                config.protocol, config.model_type
            ));
        }
    };
    Ok(AgentModelConfig {
        model_code: config.model_code.clone(),
        protocol,
        base_url: config.base_url.clone(),
        api_path: config.api_path.clone(),
        api_key: config.api_key.clone(),
        model_name: config.model_name.clone(),
        thinking_enabled: config.thinking_enabled,
        responses_stateful_enabled: config.responses_stateful_enabled,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        context_window_tokens: config.context_window_tokens,
        top_p: config.top_p,
        max_retries: config.max_retries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_compatible_embedding_response() {
        let vector = parse_embedding_response(&json!({
            "data": [
                { "embedding": [0.11, -0.02, 3.5] }
            ]
        }))
        .expect("embedding response should parse");

        assert_eq!(vector, vec![0.11, -0.02, 3.5]);
    }

    #[test]
    fn parses_rerank_results_from_results_field() {
        let hits = parse_rerank_response(&json!({
            "results": [
                { "index": 2, "relevance_score": 0.88 },
                { "index": 0, "relevance_score": 0.12 }
            ]
        }))
        .expect("rerank response should parse");

        assert_eq!(
            hits,
            vec![
                LlmRuntimeRerankHit {
                    document_index: 2,
                    score: 0.88
                },
                LlmRuntimeRerankHit {
                    document_index: 0,
                    score: 0.12
                }
            ]
        );
    }
}
