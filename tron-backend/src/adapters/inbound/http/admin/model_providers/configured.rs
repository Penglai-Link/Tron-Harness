use crate::{
    app::AppState,
    application::model::ConfiguredProviderCommand,
    domain::model::{
        ConfiguredModelProvider, ConfiguredProviderError, ConfiguredProviderModel,
        ConfiguredProviderModelType, ConfiguredProviderProtocol, ModelInputCapability,
    },
    response::{ApiResponse, EmptyData},
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CreateConfiguredProviderRequest {
    provider_type: Option<String>,
    display_name: String,
    protocol: ConfiguredProviderProtocol,
    base_url: String,
    api_key: Option<String>,
    request_headers: BTreeMap<String, String>,
    enabled: bool,
    models: Vec<ConfiguredProviderModel>,
}
impl CreateConfiguredProviderRequest {
    fn command(self) -> ConfiguredProviderCommand {
        ConfiguredProviderCommand {
            provider_type: self.provider_type,
            display_name: self.display_name,
            protocol: self.protocol,
            base_url: self.base_url,
            api_key: self.api_key,
            request_headers: self.request_headers,
            enabled: self.enabled,
            models: self.models,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateConfiguredProviderRequest {
    provider_type: Option<String>,
    display_name: String,
    protocol: ConfiguredProviderProtocol,
    base_url: String,
    api_key: Option<String>,
    request_headers: BTreeMap<String, String>,
    enabled: bool,
    models: Vec<ConfiguredProviderModel>,
}
impl UpdateConfiguredProviderRequest {
    fn command(self) -> ConfiguredProviderCommand {
        ConfiguredProviderCommand {
            provider_type: self.provider_type,
            display_name: self.display_name,
            protocol: self.protocol,
            base_url: self.base_url,
            api_key: self.api_key,
            request_headers: self.request_headers,
            enabled: self.enabled,
            models: self.models,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfiguredModelResponse {
    logical_model_code: Option<String>,
    model_type: ConfiguredProviderModelType,
    model_code: String,
    model_id: String,
    name: String,
    dimension: Option<u32>,
    context_window_tokens: Option<u32>,
    max_output_tokens: Option<u32>,
    reasoning: bool,
    supports_developer_role: bool,
    input_capabilities: Vec<ModelInputCapability>,
    enabled: bool,
}
impl From<ConfiguredProviderModel> for ConfiguredModelResponse {
    fn from(model: ConfiguredProviderModel) -> Self {
        Self {
            logical_model_code: model.logical_model_code,
            model_type: model.model_type,
            model_code: model.model_code.unwrap_or_default(),
            model_id: model.model_id,
            name: model.name,
            dimension: model.dimension,
            context_window_tokens: model.context_window_tokens,
            max_output_tokens: model.max_output_tokens,
            reasoning: model.reasoning,
            supports_developer_role: model.supports_developer_role,
            input_capabilities: model.input_capabilities,
            enabled: model.enabled,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListConfiguredProviderResponse {
    provider_id: String,
    provider_type: Option<String>,
    display_name: String,
    protocol: ConfiguredProviderProtocol,
    base_url: String,
    has_api_key: bool,
    api_key_summary: Option<String>,
    request_headers: BTreeMap<String, String>,
    enabled: bool,
    models: Vec<ConfiguredModelResponse>,
    created_at: i64,
    updated_at: i64,
}
impl From<ConfiguredModelProvider> for ListConfiguredProviderResponse {
    fn from(provider: ConfiguredModelProvider) -> Self {
        Self {
            provider_id: provider.provider_id,
            provider_type: provider.provider_type,
            display_name: provider.display_name,
            protocol: provider.protocol,
            base_url: provider.base_url,
            has_api_key: !provider.encrypted_api_key.is_empty(),
            api_key_summary: provider.api_key_summary,
            request_headers: provider.request_headers,
            enabled: provider.enabled,
            models: provider.models.into_iter().map(Into::into).collect(),
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetConfiguredProviderResponse {
    provider_id: String,
    provider_type: Option<String>,
    display_name: String,
    protocol: ConfiguredProviderProtocol,
    base_url: String,
    has_api_key: bool,
    api_key_summary: Option<String>,
    request_headers: BTreeMap<String, String>,
    enabled: bool,
    models: Vec<ConfiguredModelResponse>,
    created_at: i64,
    updated_at: i64,
}
impl From<ConfiguredModelProvider> for GetConfiguredProviderResponse {
    fn from(provider: ConfiguredModelProvider) -> Self {
        Self {
            provider_id: provider.provider_id,
            provider_type: provider.provider_type,
            display_name: provider.display_name,
            protocol: provider.protocol,
            base_url: provider.base_url,
            has_api_key: !provider.encrypted_api_key.is_empty(),
            api_key_summary: provider.api_key_summary,
            request_headers: provider.request_headers,
            enabled: provider.enabled,
            models: provider.models.into_iter().map(Into::into).collect(),
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateConfiguredProviderResponse {
    provider_id: String,
    provider_type: Option<String>,
    display_name: String,
    protocol: ConfiguredProviderProtocol,
    base_url: String,
    has_api_key: bool,
    api_key_summary: Option<String>,
    request_headers: BTreeMap<String, String>,
    enabled: bool,
    models: Vec<ConfiguredModelResponse>,
    created_at: i64,
    updated_at: i64,
}
impl From<ConfiguredModelProvider> for CreateConfiguredProviderResponse {
    fn from(provider: ConfiguredModelProvider) -> Self {
        Self {
            provider_id: provider.provider_id,
            provider_type: provider.provider_type,
            display_name: provider.display_name,
            protocol: provider.protocol,
            base_url: provider.base_url,
            has_api_key: !provider.encrypted_api_key.is_empty(),
            api_key_summary: provider.api_key_summary,
            request_headers: provider.request_headers,
            enabled: provider.enabled,
            models: provider.models.into_iter().map(Into::into).collect(),
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateConfiguredProviderResponse {
    provider_id: String,
    provider_type: Option<String>,
    display_name: String,
    protocol: ConfiguredProviderProtocol,
    base_url: String,
    has_api_key: bool,
    api_key_summary: Option<String>,
    request_headers: BTreeMap<String, String>,
    enabled: bool,
    models: Vec<ConfiguredModelResponse>,
    created_at: i64,
    updated_at: i64,
}
impl From<ConfiguredModelProvider> for UpdateConfiguredProviderResponse {
    fn from(provider: ConfiguredModelProvider) -> Self {
        Self {
            provider_id: provider.provider_id,
            provider_type: provider.provider_type,
            display_name: provider.display_name,
            protocol: provider.protocol,
            base_url: provider.base_url,
            has_api_key: !provider.encrypted_api_key.is_empty(),
            api_key_summary: provider.api_key_summary,
            request_headers: provider.request_headers,
            enabled: provider.enabled,
            models: provider.models.into_iter().map(Into::into).collect(),
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}
pub(super) async fn list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match state.models.list_configured_providers() {
        Ok(providers) => Json(ApiResponse::protocol_success(
            providers
                .into_iter()
                .map(ListConfiguredProviderResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => error_response(error, &headers),
    }
}
pub(super) async fn get(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    match state.models.get_configured_provider(&provider_id) {
        Ok(provider) => Json(ApiResponse::protocol_success(
            GetConfiguredProviderResponse::from(provider),
        ))
        .into_response(),
        Err(error) => error_response(error, &headers),
    }
}
pub(super) async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<CreateConfiguredProviderRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = request else {
        return error_response(ConfiguredProviderError::InvalidModels, &headers);
    };
    match state.models.create_configured_provider(request.command()) {
        Ok(provider) => Json(ApiResponse::protocol_success(
            CreateConfiguredProviderResponse::from(provider),
        ))
        .into_response(),
        Err(error) => error_response(error, &headers),
    }
}
pub(super) async fn update(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    headers: HeaderMap,
    request: Result<Json<UpdateConfiguredProviderRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = request else {
        return error_response(ConfiguredProviderError::InvalidModels, &headers);
    };
    match state
        .models
        .update_configured_provider(&provider_id, request.command())
    {
        Ok(provider) => Json(ApiResponse::protocol_success(
            UpdateConfiguredProviderResponse::from(provider),
        ))
        .into_response(),
        Err(error) => error_response(error, &headers),
    }
}
pub(super) async fn delete(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    match state.models.delete_configured_provider(&provider_id) {
        Ok(()) => Json(ApiResponse::protocol_success(EmptyData {})).into_response(),
        Err(error) => error_response(error, &headers),
    }
}

pub(super) fn error_response(error: ConfiguredProviderError, headers: &HeaderMap) -> Response {
    let descriptor = error.descriptor();
    let locale = SupportedLocale::from_request(
        headers
            .get("accept-language")
            .and_then(|header| header.to_str().ok()),
    );
    let (status, en, zh) = match descriptor.message_key {
        "model_provider.not_found" => (
            StatusCode::NOT_FOUND,
            "Provider not found.",
            "未找到供应商。",
        ),
        "model_provider.already_exists" => (
            StatusCode::CONFLICT,
            "Provider ID already exists.",
            "供应商标识已存在。",
        ),
        "model_provider.invalid_id" => (
            StatusCode::BAD_REQUEST,
            "Use 1–80 lowercase letters, numbers, dots, underscores or hyphens for the provider ID.",
            "供应商标识需为1至80位小写字母、数字、点、下划线或连字符。",
        ),
        "model_provider.invalid_name" => (
            StatusCode::BAD_REQUEST,
            "Enter a provider name of at most 200 characters.",
            "请填写不超过200个字符的供应商名称。",
        ),
        "model_provider.invalid_url" => (
            StatusCode::BAD_REQUEST,
            "Enter an HTTP(S) API base URL without credentials, query parameters or fragments.",
            "请输入不含凭据、查询参数或片段的HTTP(S) API基础地址。",
        ),
        "model_provider.invalid_headers" => (
            StatusCode::BAD_REQUEST,
            "Request headers are invalid or contain reserved authentication or connection headers. Enter credentials in the API key field.",
            "请求头无效或含保留的认证、连接请求头；请在API密钥字段填写凭据。",
        ),
        "model_provider.invalid_models" => (
            StatusCode::BAD_REQUEST,
            "Add 1–500 models with unique IDs, names and valid token limits. Supported model inputs are text and images.",
            "请添加1至500个模型，使用唯一模型ID、名称及有效Token上限；当前支持的模型输入为文本和图像。",
        ),
        "model_provider.invalid_model_ownership" => (
            StatusCode::BAD_REQUEST,
            "A model does not belong to this provider.",
            "模型不属于当前供应商。",
        ),
        "model_provider.conflict" => (
            StatusCode::CONFLICT,
            "The provider changed. Reload before saving.",
            "供应商配置已变化，请重新加载后保存。",
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not save provider configuration.",
            "供应商配置存储失败。",
        ),
    };
    (
        status,
        Json(ApiResponse::error(
            descriptor.code,
            if locale == SupportedLocale::EnUs {
                en
            } else {
                zh
            },
        )),
    )
        .into_response()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderListResponse {
    provider_code: String,
    source: &'static str,
    provider_type: Option<String>,
    person_ai_user_id: Option<String>,
    person_ai_username: Option<String>,
    display_name: String,
    status: crate::domain::model_provider::ProviderConnectionStatus,
    server_base_url: String,
    authorized_at: Option<i64>,
    authorization_expires_at: Option<i64>,
    last_sync_at: Option<i64>,
    last_error: Option<String>,
    catalog_model_count: usize,
    updated_at: i64,
}
impl From<crate::domain::model_provider::ProviderConnectionView> for ProviderListResponse {
    fn from(provider: crate::domain::model_provider::ProviderConnectionView) -> Self {
        Self {
            provider_code: provider.provider_code.storage_value().into(),
            source: "PersonAi",
            provider_type: Some("PersonAi".into()),
            person_ai_user_id: provider.person_ai_user_id,
            person_ai_username: provider.person_ai_username,
            display_name: provider.display_name,
            status: provider.status,
            server_base_url: provider.server_base_url,
            authorized_at: provider.authorized_at,
            authorization_expires_at: provider.authorization_expires_at,
            last_sync_at: provider.last_sync_at,
            last_error: provider.last_error,
            catalog_model_count: provider.catalog_model_count,
            updated_at: provider.updated_at,
        }
    }
}
impl From<ConfiguredModelProvider> for ProviderListResponse {
    fn from(provider: ConfiguredModelProvider) -> Self {
        Self {
            provider_code: provider.provider_id,
            source: "Custom",
            provider_type: provider.provider_type,
            person_ai_user_id: None,
            person_ai_username: None,
            display_name: provider.display_name,
            status: if provider.enabled {
                crate::domain::model_provider::ProviderConnectionStatus::Connected
            } else {
                crate::domain::model_provider::ProviderConnectionStatus::Disconnected
            },
            server_base_url: provider.base_url,
            authorized_at: None,
            authorization_expires_at: None,
            last_sync_at: None,
            last_error: None,
            catalog_model_count: provider.models.len(),
            updated_at: provider.updated_at,
        }
    }
}
