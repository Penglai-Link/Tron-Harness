//! Read-only provider suggestions. A catalogue entry never creates a connection,
//! reads its environment variable, or makes a network request.
use std::sync::OnceLock;

use axum::{
    Json,
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::response::{ApiResponse, EmptyData};
use crate::shared::locale::{SupportedLocale, resolve_request_locale};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalogue {
    providers: Vec<Preset>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Preset {
    id: String,
    name: String,
    protocol: Option<String>,
    base_url: Option<String>,
    env_key: Option<String>,
    guessed: bool,
    needs_base_url: bool,
    supported: bool,
    unsupported_reason: Option<String>,
    models: Vec<PresetModel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetModel {
    model_id: String,
    name: String,
    context_window_tokens: Option<u32>,
    reasoning: bool,
    input_capabilities: Vec<String>,
    tool_use: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListPresetResponse<'a> {
    id: &'a str,
    name: &'a str,
    protocol: Option<&'a str>,
    base_url: Option<&'a str>,
    env_key: Option<&'a str>,
    guessed: bool,
    needs_base_url: bool,
    supported: bool,
    unsupported_reason: Option<&'a str>,
    model_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListPresetModelsResponse<'a> {
    model_id: &'a str,
    name: &'a str,
    context_window_tokens: Option<u32>,
    reasoning: bool,
    input_capabilities: &'a [String],
    tool_use: bool,
}

fn catalogue() -> &'static Catalogue {
    static CATALOGUE: OnceLock<Catalogue> = OnceLock::new();
    CATALOGUE.get_or_init(|| {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/model-provider-presets.json"
        )))
        .expect("bundled provider catalogue must be valid")
    })
}

pub(super) async fn list() -> Json<ApiResponse<Vec<ListPresetResponse<'static>>>> {
    Json(ApiResponse::protocol_success(
        catalogue()
            .providers
            .iter()
            .map(|preset| ListPresetResponse {
                id: &preset.id,
                name: &preset.name,
                protocol: preset.protocol.as_deref(),
                base_url: preset.base_url.as_deref(),
                env_key: preset.env_key.as_deref(),
                guessed: preset.guessed,
                needs_base_url: preset.needs_base_url,
                supported: preset.supported,
                unsupported_reason: preset.unsupported_reason.as_deref(),
                model_count: preset.models.len(),
            })
            .collect(),
    ))
}

pub(super) async fn models(Path(preset_id): Path<String>, headers: HeaderMap) -> Response {
    let Some(preset) = catalogue()
        .providers
        .iter()
        .find(|preset| preset.id == preset_id)
    else {
        let descriptor = crate::application::model::configured_provider_preset_not_found_descriptor();
        let locale = resolve_request_locale(
            headers
                .get("accept-language")
                .and_then(|value| value.to_str().ok()),
        );
        let message = if SupportedLocale::from_raw(&locale) == SupportedLocale::EnUs {
            "Provider preset was not found"
        } else {
            "供应商预设不存在"
        };
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                message,
            )),
        )
            .into_response();
    };
    Json(ApiResponse::protocol_success(
        preset
            .models
            .iter()
            .map(|model| ListPresetModelsResponse {
                model_id: &model.model_id,
                name: &model.name,
                context_window_tokens: model.context_window_tokens,
                reasoning: model.reasoning,
                input_capabilities: &model.input_capabilities,
                tool_use: model.tool_use,
            })
            .collect::<Vec<_>>(),
    ))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn bundled_catalogue_preserves_uncertain_metadata_without_enabling_connections() {
        let data = catalogue();
        assert!(!data.providers.is_empty());
        let mut ids = HashSet::new();
        for provider in &data.providers {
            assert!(
                ids.insert(&provider.id),
                "duplicate provider {}",
                provider.id
            );
            if provider.base_url.is_none() {
                assert!(provider.needs_base_url);
            }
            assert_eq!(provider.supported, provider.unsupported_reason.is_none());
            if provider.supported {
                assert!(matches!(
                    provider.protocol.as_deref(),
                    Some("OpenAiChatCompletions" | "AnthropicMessages" | "GoogleGemini")
                ));
            }
            let mut models = HashSet::new();
            for model in &provider.models {
                assert!(models.insert(&model.model_id));
                assert!(model.input_capabilities.iter().any(|input| input == "text"));
            }
        }
        assert!(data.providers.iter().any(|provider| provider.guessed));
        assert!(data.providers.iter().any(|provider| !provider.supported));
    }

    #[tokio::test]
    async fn provider_summaries_omit_models_and_model_details_do_not_invent_output_limits() {
        let Json(response) = list().await;
        let summary = serde_json::to_value(response).unwrap();
        assert_eq!(summary["code"], "0");
        assert!(summary["data"][0].get("models").is_none());
        assert!(summary["data"][0].get("baseUrl").is_some());
        let response = models(Path(catalogue().providers[0].id.clone()), HeaderMap::new()).await;
        let body = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(body["data"][0].get("maxOutputTokens").is_none());
        let missing = models(Path("missing-preset".into()), HeaderMap::new()).await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }
}
