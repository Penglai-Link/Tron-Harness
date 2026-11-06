use crate::{application::model::RuntimeModelConfig, domain::model::ModelProtocol};
use base64::Engine;
use serde_json::{Value, json};

pub(super) async fn transcribe(
    config: &RuntimeModelConfig,
    audio: Vec<u8>,
    mime: &str,
    file_name: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "could not create transcription client")?;
    let base = config.base_url.trim_end_matches('/');
    let mut request = match config.protocol {
        ModelProtocol::OpenAiCompatible | ModelProtocol::Responses => {
            let part = reqwest::multipart::Part::bytes(audio)
                .file_name(file_name.to_owned())
                .mime_str(mime)
                .map_err(|_| "invalid audio type")?;
            client
                .post(format!("{base}/audio/transcriptions"))
                .bearer_auth(&config.api_key)
                .multipart(
                    reqwest::multipart::Form::new()
                        .text("model", config.model_name.clone())
                        .text("response_format", "json")
                        .part("file", part),
                )
        }
        ModelProtocol::Gemini => {
            let model = config.model_name.trim_start_matches("models/");
            let base = if base.ends_with("/v1beta") || base.ends_with("/v1") {
                base.to_owned()
            } else {
                format!("{base}/v1beta")
            };
            let mut url =
                url::Url::parse(&format!("{base}/models/")).map_err(|_| "invalid provider URL")?;
            url.path_segments_mut()
                .map_err(|_| "invalid provider URL")?
                .pop_if_empty()
                .push(&format!("{model}:generateContent"));
            client.post(url).header("x-goog-api-key", &config.api_key).json(&json!({
                "contents": [{"parts": [
                    {"text": "Transcribe the speech verbatim in its original language. Return only the transcript. If there is no speech, return an empty string."},
                    {"inlineData": {"mimeType": mime, "data": base64::engine::general_purpose::STANDARD.encode(audio)}}
                ]}]
            }))
        }
        _ => return Err("unsupported transcription protocol".into()),
    };
    for (name, value) in &config.request_headers {
        request = request.header(name, value);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "transcription request failed")?;
    if !response.status().is_success() {
        return Err("transcription provider rejected the request".into());
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| "invalid transcription response")?;
    if config.protocol == ModelProtocol::Gemini {
        let parts = body
            .pointer("/candidates/0/content/parts")
            .and_then(Value::as_array)
            .ok_or("missing transcription")?;
        Ok(parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""))
    } else {
        body.get("text")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| "missing transcription".into())
    }
}
