use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EncryptedCredentialsPayload {
    pub(super) token: String,
    pub(super) refresh_token: String,
    pub(super) client_id: String,
    pub(super) public_key: String,
    pub(super) server_base_url: String,
    pub(super) authorization_expires_at: Option<i64>,
}

pub(super) fn encrypt_credentials(
    cipher: &(impl VariableCipher + ?Sized),
    credentials: &ProviderCredentials,
) -> Result<String, ModelProviderError> {
    let payload = serde_json::to_string(&EncryptedCredentialsPayload {
        token: credentials.token.clone(),
        refresh_token: credentials.refresh_token.clone(),
        client_id: credentials.client_id.clone(),
        public_key: credentials.public_key.clone(),
        server_base_url: credentials.server_base_url.clone(),
        authorization_expires_at: credentials.authorization_expires_at,
    })
    .map_err(|_| ModelProviderError::Storage)?;
    let encrypted = cipher
        .encrypt(&payload, b"model_provider.credentials")
        .map_err(|_| ModelProviderError::Storage)?;
    serde_json::to_string(&encrypted)
        .map(|value| format!("enc:v1:{value}"))
        .map_err(|_| ModelProviderError::Storage)
}

pub(super) fn decrypt_credentials(
    cipher: &(impl VariableCipher + ?Sized),
    encrypted: &str,
) -> Result<ProviderCredentials, ModelProviderError> {
    let payload = encrypted
        .trim()
        .strip_prefix("enc:v1:")
        .ok_or(ModelProviderError::AuthenticationRequired)?;
    let encrypted = serde_json::from_str(payload).map_err(|_| ModelProviderError::Storage)?;
    let plain = cipher
        .decrypt(&encrypted, b"model_provider.credentials")
        .map_err(|_| ModelProviderError::Storage)?;
    let payload: EncryptedCredentialsPayload =
        serde_json::from_str(&plain).map_err(|_| ModelProviderError::Storage)?;
    Ok(ProviderCredentials {
        token: payload.token,
        refresh_token: payload.refresh_token,
        client_id: payload.client_id,
        public_key: payload.public_key,
        server_base_url: payload.server_base_url,
        authorization_expires_at: payload.authorization_expires_at,
    })
}

pub(super) fn required_remote_value(
    value: Option<String>,
    field: &str,
) -> Result<String, ModelProviderError> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ModelProviderError::Remote(format!("Person-AI authorization response omitted {field}"))
        })
}

pub(super) fn remote_error(error: impl std::fmt::Display) -> ModelProviderError {
    ModelProviderError::Remote(format!("Person-AI request failed: {error}"))
}

pub(super) fn server_api_error(error: ServerApiError) -> ModelProviderError {
    match error {
        ServerApiError::DeviceUnbound => ModelProviderError::DeviceUnbound,
        error => remote_error(error),
    }
}

pub(super) fn is_person_ai_authorization_key_error(error: &ModelProviderError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    [
        "decrypt_error",
        "decryptfailed",
        "服务端解密失败",
        "授权密钥已失效",
    ]
    .iter()
    .any(|marker| message.contains(marker))
}

pub(super) fn account_info_error(error: ServerApiError) -> ModelProviderError {
    match error {
        ServerApiError::DecryptFailed => ModelProviderError::AuthenticationRequired,
        ServerApiError::DeviceUnbound => ModelProviderError::DeviceUnbound,
        ServerApiError::Business { code, .. } => ModelProviderError::Remote(format!(
            "Person-AI account information request was rejected (code {code})"
        )),
        _ => ModelProviderError::Remote("Person-AI account information request failed".into()),
    }
}

pub(super) fn account_info_error_kind(error: &ServerApiError) -> &'static str {
    match error {
        ServerApiError::Business { .. } => "business-response",
        ServerApiError::Validation(_) => "request-validation",
        ServerApiError::Http(_) => "http-transport",
        ServerApiError::Json(_) | ServerApiError::Other(_) => "invalid-response",
        ServerApiError::Crypto(_) => "crypto",
        ServerApiError::DecryptFailed => "authorization-key",
        ServerApiError::Gateway { .. } => "gateway",
        ServerApiError::DeviceUnbound => "device-unbound",
        ServerApiError::WebSocket(_) => "websocket",
        ServerApiError::Io(_) => "io",
        ServerApiError::Url(_) => "url",
    }
}

pub(super) fn parse_person_ai_account_info(
    response: Value,
) -> Result<ProviderAccountBalanceView, ModelProviderError> {
    if let Some(code) = response.get("code") {
        let successful = matches!(code, Value::Number(value) if value.as_i64() == Some(0))
            || matches!(code, Value::String(value) if value == "0" || value.eq_ignore_ascii_case("success"));
        if !successful {
            return Err(ModelProviderError::Remote(
                "Person-AI account information request was rejected".into(),
            ));
        }
    }

    let data = response.get("data").unwrap_or(&response);
    let fields = data.as_object().ok_or_else(|| {
        ModelProviderError::Remote("Person-AI account information response has invalid data".into())
    })?;
    Ok(ProviderAccountBalanceView {
        nickname: optional_account_string(fields, &["nickname"]),
        username: optional_account_string(fields, &["username"]),
        vip_level: optional_account_number(fields, &["vip_level", "vipLevel"])
            .and_then(|value| i32::try_from(value).ok()),
        // Person-AI defines remaining_tokens as the account's remaining points balance.
        remaining_points: ["remaining_tokens", "remainingTokens"].iter().find_map(|name| fields.get(*name).and_then(|value| match value { Value::Number(n) => n.as_f64(), Value::String(s) => s.trim().parse::<f64>().ok(), _ => None }).filter(|n| n.is_finite())),
    })
}

pub(super) fn optional_account_string(
    fields: &serde_json::Map<String, Value>,
    names: &[&str],
) -> Option<String> {
    names.iter().find_map(|name| {
        fields
            .get(*name)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

pub(super) fn optional_account_number(
    fields: &serde_json::Map<String, Value>,
    names: &[&str],
) -> Option<i64> {
    names.iter().find_map(|name| {
        fields.get(*name).and_then(|value| match value {
            Value::Number(value) => value
                .as_i64()
                .or_else(|| value.as_f64().and_then(account_number_from_float)),
            Value::String(value) => value.trim().parse::<i64>().ok().or_else(|| {
                value
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .and_then(account_number_from_float)
            }),
            _ => None,
        })
    })
}

pub(super) fn account_number_from_float(value: f64) -> Option<i64> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < i64::MIN as f64
        || value > i64::MAX as f64
    {
        return None;
    }
    Some(value as i64)
}

pub(super) fn safe_error_summary(error: &ModelProviderError) -> String {
    match error {
        ModelProviderError::AuthenticationRequired => "Person-AI authorization is required".into(),
        ModelProviderError::Remote(_) => "Person-AI remote request failed".into(),
        other => other.to_string(),
    }
}

pub(super) fn parse_remote_timestamp(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

pub(super) fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub(super) fn sync_run_code(now: i64) -> String {
    format!("PROVIDER-SYNC-{now}-{:08x}", rand::random::<u32>())
}

pub(super) fn model_type_storage(model_type: ModelType) -> &'static str {
    match model_type {
        ModelType::Text => "TEXT",
        ModelType::TextEmbedding => "TEXT_EMBEDDING",
        ModelType::MultimodalEmbedding => "MULTIMODAL_EMBEDDING",
        ModelType::TextRerank => "TEXT_RERANK",
        ModelType::MultimodalRerank => "MULTIMODAL_RERANK",
        ModelType::Image => "IMAGE",
        ModelType::Video => "VIDEO",
        ModelType::SpeechSynthesis => "SPEECH_SYNTHESIS",
        ModelType::SpeechRecognition => "SPEECH_RECOGNITION",
        ModelType::RealtimeSpeech => "REALTIME_SPEECH",
    }
}
