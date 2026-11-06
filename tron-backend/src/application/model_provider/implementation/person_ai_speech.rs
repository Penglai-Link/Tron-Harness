//! Person-AI encrypted realtime STT. The SDK supplies URL/auth/encryption;
//! its listener API cannot interleave sending and receiving, so this adapter
//! owns the duplex transport and waits for the final transcription event.
use base64::{Engine, engine::general_purpose::STANDARD};
use crab_serverapi::{
    AuthorizedClientApi, AuthorizedClientContext, crypto::ClientEnvelopeEncryptor,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_tungstenite::{Connector, connect_async_tls_with_config, tungstenite::Message};

pub(super) async fn transcribe(
    base: &str,
    context: &AuthorizedClientContext,
    wav: &[u8],
) -> Result<String, String> {
    let mut selected_model = None;
    match run(base, context, wav, &mut selected_model).await {
        Ok(text) => Ok(text),
        Err(detail) => {
            let detail = redact(&detail, context);
            tracing::error!(provider = "PersonAi", protocol = "websocket",
                model = selected_model.as_deref().unwrap_or("unresolved"), error = %detail,
                "Speech transcription failed");
            Err(detail)
        }
    }
}

async fn run(
    base: &str,
    context: &AuthorizedClientContext,
    wav: &[u8],
    selected_model: &mut Option<String>,
) -> Result<String, String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(110);
    let (rate, pcm) = pcm_from_wav(wav)?;
    context
        .validate_gateway_access()
        .map_err(|e| format!("credentials: {e}"))?;
    let required = |value: &Option<String>, name: &str| {
        value
            .as_ref()
            .filter(|v| !v.trim().is_empty())
            .cloned()
            .ok_or_else(|| format!("credentials: missing {name}"))
    };
    let query = HashMap::from([
        ("token".into(), required(&context.token, "token")?),
        (
            "client_id".into(),
            required(&context.client_id, "client_id")?,
        ),
        (
            "product_code".into(),
            required(&context.product_code, "product_code")?,
        ),
        (
            "product_version".into(),
            required(&context.product_version, "product_version")?,
        ),
    ]);
    let public_key = required(&context.public_key, "public_key")?;
    let api = AuthorizedClientApi::new(base).map_err(|e| format!("configuration: {e}"))?;
    let url = api
        .build_websocket_url("/ws/audio/transcriptions", &query)
        .map_err(|e| format!("configuration: {e}"))?;
    // Never log the URL: the handshake carries the access token in its query.
    // Other clients enable aws-lc as well as ring. Use an explicit provider for
    // this connector instead of Rustls's ambiguous process-wide default.
    let certificates = rustls_native_certs::load_native_certs();
    let mut roots = rustls::RootCertStore::empty();
    let (trusted, _) = roots.add_parsable_certificates(certificates.certs);
    if trusted == 0 {
        return Err(format!(
            "TLS trust store: no usable system certificates; {:?}",
            certificates.errors
        ));
    }
    let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| format!("TLS configuration: {e}"))?
    .with_root_certificates(roots)
    .with_no_client_auth();
    let connect =
        connect_async_tls_with_config(&url, None, false, Some(Connector::Rustls(Arc::new(tls))));
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(20), connect)
        .await
        .map_err(|_| "handshake: timed out after 20 seconds".to_owned())?
        .map_err(|e| format!("handshake: {e}"))?;
    socket
        .send(encrypted(
            &public_key,
            json!({
                "type": "start", "language": "", "vad": false, "sample_rate": rate,
            }),
        )?)
        .await
        .map_err(|e| format!("start send: {e}"))?;
    let (mut writer, mut reader) = socket.split();
    let model = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(message) = reader.next().await {
            let message = message.map_err(|e| format!("start receive: {e}"))?;
            if let Some(event) = event_json(message)? {
                check_error(&event, "start")?;
                if event["type"] == "started" {
                    return event["model"]
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "start: server omitted selected model".to_owned());
                }
            }
        }
        Err("start: socket closed before session was ready".into())
    })
    .await
    .map_err(|_| "start: no started event within 30 seconds".to_owned())??;
    *selected_model = Some(model.clone());
    tracing::info!(provider = "PersonAi", protocol = "websocket", model = %model,
        sample_rate = rate, audio_bytes = pcm.len(), "Speech transcription session started");

    let send = async {
        // 100 ms PCM chunks stay below the server's encrypted message size limit.
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        for chunk in pcm.chunks(rate as usize / 10 * 2) {
            interval.tick().await;
            writer
                .send(encrypted(
                    &public_key,
                    json!({"type":"audio", "audio":STANDARD.encode(chunk)}),
                )?)
                .await
                .map_err(|e| format!("audio send: {e}"))?;
        }
        writer
            .send(encrypted(&public_key, json!({"type":"commit"}))?)
            .await
            .map_err(|e| format!("commit: {e}"))?;
        Ok::<_, String>(())
    };
    let receive = async {
        while let Some(message) = reader.next().await {
            let message = message.map_err(|e| format!("transcript receive: {e}"))?;
            if let Some(event) = event_json(message)? {
                check_error(&event, "transcript")?;
                // Some gateway versions wrap upstream events; others forward them.
                let event = event
                    .get("event")
                    .filter(|_| event["type"] == "realtime_event")
                    .unwrap_or(&event);
                if event["type"] == "conversation.item.input_audio_transcription.completed" {
                    return event["transcript"]
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "transcript: completed event has no transcript".to_owned());
                }
                if event["type"] == "session.finished" || event["type"] == "stopped" {
                    return Err("transcript: session ended without a completed transcript".into());
                }
            }
        }
        Err("transcript: socket closed before completed event".into())
    };
    // No detached sender task: error, timeout or cancellation drops both halves.
    let (_, text) = tokio::time::timeout_at(deadline, async { tokio::try_join!(send, receive) })
        .await
        .map_err(|_| "transcript: exceeded 110 second session deadline".to_owned())??;
    // 'stopped' acknowledges cleanup/billing, not transcription completion.
    let cleanup = tokio::time::timeout(Duration::from_secs(5), async {
        writer
            .send(encrypted(&public_key, json!({"type":"stop"}))?)
            .await
            .map_err(|e| format!("stop: {e}"))?;
        while let Some(message) = reader.next().await {
            let message = message.map_err(|e| format!("stop receive: {e}"))?;
            if let Some(event) = event_json(message)? {
                check_error(&event, "stop")?;
                if event["type"] == "stopped" {
                    return Ok::<_, String>(());
                }
            }
        }
        Err("stop: socket ended without acknowledgement".into())
    })
    .await;
    let cleanup_error = match cleanup {
        Ok(Ok(())) => None,
        Ok(Err(detail)) => Some(detail),
        Err(_) => Some("stop: acknowledgement timed out after 5 seconds".into()),
    };
    if let Some(detail) = cleanup_error {
        tracing::warn!(provider = "PersonAi", protocol = "websocket", model = %model,
            error = %redact(&detail, context), "Speech session cleanup was not acknowledged");
    }
    // Cleanup gets its own bounded budget after a final result, so a late
    // acknowledgement cannot turn a successful transcription into a timeout.
    let close = tokio::time::timeout(Duration::from_secs(2), writer.close()).await;
    match close {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::warn!(provider = "PersonAi", model = %model,
            error = %redact(&format!("close: {error}"), context), "Speech socket close failed"),
        Err(_) => tracing::warn!(provider = "PersonAi", model = %model,
            error = "close: timed out after 2 seconds", "Speech socket close failed"),
    }
    tracing::info!(provider = "PersonAi", protocol = "websocket", model = %model,
        transcript_chars = text.chars().count(), "Speech transcription completed");
    Ok(text)
}

fn encrypted(key: &str, payload: Value) -> Result<Message, String> {
    let envelope = ClientEnvelopeEncryptor::encrypt(key, &payload.to_string())
        .map_err(|e| format!("encrypt: {e}"))?;
    serde_json::to_string(&envelope)
        .map(Message::Text)
        .map_err(|e| format!("encode envelope: {e}"))
}

fn event_json(message: Message) -> Result<Option<Value>, String> {
    match message {
        Message::Text(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| format!("decode event: {e}")),
        Message::Close(frame) => Err(format!(
            "socket closed: {}",
            frame
                .map(|f| format!("{} {}", f.code, f.reason))
                .unwrap_or_default()
        )),
        _ => Ok(None),
    }
}

fn check_error(event: &Value, phase: &str) -> Result<(), String> {
    let event = event
        .get("event")
        .filter(|_| event["type"] == "realtime_event")
        .unwrap_or(event);
    let kind = event["type"].as_str().unwrap_or_default();
    if kind == "error" || kind.ends_with(".failed") {
        let code = event
            .pointer("/error/code")
            .or_else(|| event.get("code"))
            .unwrap_or(&Value::Null);
        let message = event
            .pointer("/error/message")
            .or_else(|| event.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("upstream returned an error without a message");
        return Err(format!("{phase}: {kind}, code={code}, message={message}"));
    }
    Ok(())
}

fn redact(detail: &str, context: &AuthorizedClientContext) -> String {
    let mut text = detail.to_owned();
    for secret in [
        &context.token,
        &context.refresh_token,
        &context.client_id,
        &context.public_key,
    ]
    .into_iter()
    .flatten()
    {
        if !secret.is_empty() {
            text = text.replace(secret, "[REDACTED]");
        }
    }
    // Upstream errors may include URL queries or authorization values.
    let pattern = regex::Regex::new(
        r#"(?i)(bearer\s+|(?:token|api[_-]?key|authorization|client_id)=)[^\s&"<>]+"#,
    )
    .expect("redaction pattern");
    text = pattern.replace_all(&text, "${1}[REDACTED]").into_owned();
    text.chars()
        .filter(|c| !c.is_control())
        .take(2000)
        .collect()
}

fn pcm_from_wav(wav: &[u8]) -> Result<(u32, &[u8]), String> {
    let invalid = || {
        "audio format: expected mono PCM16 WAV at 16, 24 or 48 kHz (maximum 65 seconds)".to_owned()
    };
    if wav.len() < 12 || &wav[..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return Err(invalid());
    }
    let mut rate = None;
    let mut pcm = None;
    let mut offset = 12usize;
    while offset + 8 <= wav.len() {
        let size = u32::from_le_bytes(wav[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= wav.len())
            .ok_or_else(invalid)?;
        let data = &wav[start..end];
        match &wav[offset..offset + 4] {
            b"fmt " if data.len() >= 16 => {
                let u16_at = |i| u16::from_le_bytes([data[i], data[i + 1]]);
                let sample_rate = u32::from_le_bytes(data[4..8].try_into().unwrap());
                if u16_at(0) != 1
                    || u16_at(2) != 1
                    || u16_at(14) != 16
                    || u16_at(12) != 2
                    || !matches!(sample_rate, 16000 | 24000 | 48000)
                {
                    return Err(invalid());
                }
                rate = Some(sample_rate);
            }
            b"data" => pcm = Some(data),
            _ => {}
        }
        offset = end + size % 2;
    }
    let rate = rate.ok_or_else(invalid)?;
    let pcm = pcm
        .filter(|data| {
            !data.is_empty() && data.len() % 2 == 0 && data.len() <= rate as usize * 2 * 65
        })
        .ok_or_else(invalid)?;
    Ok((rate, pcm))
}
