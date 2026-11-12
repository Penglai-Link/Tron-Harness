use crate::{
    app::AppState,
    domain::reasoning::ReasoningEffort,
    domain::{
        model::{
            ConfiguredProviderError, ModelProtocol, ModelSelectOption, ModelType,
            ModelViewSourceKind,
        },
        model_provider::{ModelProviderCode, ModelSelection, ProviderCatalogModelStatus},
    },
    response::ApiResponse,
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelOptionsQuery {
    #[serde(default)]
    pub workspace_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelOptionVo {
    pub option_code: String,
    pub model_code: String,
    pub display_name: String,
    pub model_name: String,
    pub source_kind: ModelViewSourceKind,
    pub source_label: String,
    pub vendor: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub enabled: bool,
    pub model_selection: ModelSelection,
    pub provider_code: Option<ModelProviderCode>,
    pub access_platform: Option<String>,
    pub upstream_vendor_name: Option<String>,
    pub remote_status: Option<ProviderCatalogModelStatus>,
    pub context_window_tokens: Option<u32>,
    pub quality_type: Option<crate::domain::model_provider::ProviderModelQuality>,
    pub input_coefficient: Option<f64>,
    pub cache_hit_coefficient: Option<f64>,
    pub output_coefficient: Option<f64>,
    pub supported_reasoning_efforts: Vec<ReasoningEffort>,
    pub default_reasoning_effort: Option<ReasoningEffort>,
    pub is_default: bool,
}
impl From<ModelSelectOption> for ProviderModelOptionVo {
    fn from(value: ModelSelectOption) -> Self {
        Self {
            option_code: value.option_code,
            model_code: value.model_code,
            display_name: value.display_name,
            model_name: value.model_name,
            source_kind: value.source_kind,
            source_label: value.source_label,
            vendor: value.vendor,
            protocol: value.protocol,
            model_type: value.model_type,
            enabled: value.enabled,
            model_selection: value.model_selection,
            provider_code: value.provider_code,
            access_platform: value.access_platform,
            upstream_vendor_name: value.upstream_vendor_name,
            remote_status: value.remote_status,
            context_window_tokens: value.context_window_tokens,
            quality_type: value.quality_type,
            input_coefficient: value.input_coefficient,
            cache_hit_coefficient: value.cache_hit_coefficient,
            output_coefficient: value.output_coefficient,
            supported_reasoning_efforts: value.supported_reasoning_efforts,
            default_reasoning_effort: value.default_reasoning_effort,
            is_default: value.is_default,
        }
    }
}
pub struct ProviderModelOptionsError {
    locale: SupportedLocale,
}
impl IntoResponse for ProviderModelOptionsError {
    fn into_response(self) -> Response {
        let descriptor = ConfiguredProviderError::Storage.descriptor();
        let message = match self.locale {
            SupportedLocale::EnUs => "Could not load provider models.",
            _ => "无法加载供应商模型。",
        };
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::protocol_error(descriptor.code, message)),
        )
            .into_response()
    }
}

pub async fn list_select_options(
    State(state): State<AppState>,
    Query(query): Query<ProviderModelOptionsQuery>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<ProviderModelOptionVo>>>, ProviderModelOptionsError> {
    let locale = SupportedLocale::from_request(
        headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
    );
    let failure = || ProviderModelOptionsError { locale };
    let workspace_code = query.workspace_code;
    let mut records = state.models.list_select_options().map_err(|_| failure())?;
    let provider_records = state
        .model_providers
        .list_text_select_options(&workspace_code)
        .map_err(|_| failure())?;
    records.extend(provider_records.into_iter().map(|record| {
        ModelSelectOption {
            option_code: record.option_code,
            model_code: match (
                record.selection.model_code.as_deref(),
                record.selection.route_code,
            ) {
                (Some(code), _) => format!(
                    "PERSON_AI:FIXED:{}:{code}",
                    record
                        .selection
                        .quality_type
                        .map(|quality| quality.storage_value())
                        .unwrap_or("High")
                ),
                (_, Some(route)) => format!("PERSON_AI:ROUTE:{}", route.storage_value()),
                _ => String::new(),
            },
            display_name: record.display_name,
            model_name: record
                .selection
                .model_code
                .clone()
                .unwrap_or_else(|| "automatic".into()),
            source_kind: ModelViewSourceKind::Provider,
            source_label: record.access_platform.clone(),
            vendor: record
                .upstream_vendor_name
                .clone()
                .unwrap_or_else(|| record.access_platform.clone()),
            protocol: match record.model_type {
                crate::domain::model::ModelType::TextEmbedding => {
                    ModelProtocol::OpenAiCompatibleEmbedding
                }
                crate::domain::model::ModelType::TextRerank => ModelProtocol::Rerank,
                _ => ModelProtocol::OpenAiCompatible,
            },
            model_type: record.model_type,
            enabled: record.enabled,
            model_selection: record.selection,
            provider_code: Some(record.provider_code),
            access_platform: Some(record.access_platform),
            upstream_vendor_name: record.upstream_vendor_name,
            remote_status: Some(record.remote_status),
            context_window_tokens: record.context_window_tokens,
            quality_type: record.quality_type,
            input_coefficient: record.input_coefficient,
            cache_hit_coefficient: record.cache_hit_coefficient,
            output_coefficient: record.output_coefficient,
            supported_reasoning_efforts: Vec::new(),
            default_reasoning_effort: None,
            is_default: record.is_default,
        }
    }));

    for kind in [
        crate::domain::model::AutoRouterKind::Embedding,
        crate::domain::model::AutoRouterKind::Rerank,
    ] {
        let code = format!("AUTO:{}", kind.code());
        let label = match (locale, kind) {
            (SupportedLocale::EnUs, crate::domain::model::AutoRouterKind::Embedding) => {
                "Embedding Automatic Routing"
            }
            (SupportedLocale::EnUs, _) => "Rerank Automatic Routing",
            (_, crate::domain::model::AutoRouterKind::Embedding) => "向量自动路由",
            _ => "重排自动路由",
        };
        records.push(ModelSelectOption {
            option_code: code.clone(),
            model_code: code.clone(),
            display_name: label.into(),
            model_name: label.into(),
            source_kind: ModelViewSourceKind::Provider,
            source_label: if locale == SupportedLocale::EnUs {
                "Automatic router"
            } else {
                "自动路由器"
            }
            .into(),
            vendor: String::new(),
            protocol: if matches!(kind, crate::domain::model::AutoRouterKind::Embedding) {
                ModelProtocol::OpenAiCompatibleEmbedding
            } else {
                ModelProtocol::Rerank
            },
            model_type: kind.model_type(),
            enabled: true,
            model_selection: ModelSelection::fixed(code),
            provider_code: None,
            access_platform: None,
            upstream_vendor_name: None,
            remote_status: None,
            context_window_tokens: None,
            quality_type: None,
            input_coefficient: None,
            cache_hit_coefficient: None,
            output_coefficient: None,
            supported_reasoning_efforts: vec![],
            default_reasoning_effort: None,
            is_default: false,
        });
    }
    Ok(Json(ApiResponse::protocol_success(
        records
            .into_iter()
            .map(ProviderModelOptionVo::from)
            .collect(),
    )))
}
