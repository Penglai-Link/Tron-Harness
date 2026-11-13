//! Read-only localized presentation metadata, independent of persisted layout overrides.
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Serialize;

use crate::{
    app::AppState,
    response::{ApiResponse, EmptyData},
    shared::locale::SupportedLocale,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContextBlockMetadataVo {
    id: String,
    display_name: String,
    description: String,
    update_mode: eliteforge_agent_core::ContextBlockUpdateMode,
    compression: Option<ContextBlockCompressionVo>,
    provider_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContextBlockCompressionVo {
    id: String,
    display_name: String,
    description: String,
}

pub(super) async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<
    Json<ApiResponse<Vec<ContextBlockMetadataVo>>>,
    (StatusCode, Json<ApiResponse<EmptyData>>),
> {
    let locale = SupportedLocale::from_request(
        headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
    );
    let contributors = state
        .agent_framework
        .context_block_catalog()
        .await
        .map_err(|error| {
            tracing::error!(%error, "could not load context block metadata");
            let message = match locale {
                SupportedLocale::EnUs => "Could not load context block descriptions.",
                SupportedLocale::ZhCn => "无法读取上下文块描述。",
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::protocol_error("-1", message)),
            )
        })?;
    let config = state.short_term_memory.framework_context_config();
    let blocks = crate::application::short_term_memory::context_block_catalog(
        contributors,
        &config.context.providers,
    )
    .into_iter()
    .map(|(id, entry)| {
        let text = entry.metadata.resolve(locale.as_str());
        let compression = &entry.metadata.compression;
        let compression = (!compression.id.is_empty()).then(|| {
            let text = compression.resolve(locale.as_str());
            ContextBlockCompressionVo {
                id: compression.id.clone(),
                display_name: text.display_name,
                description: text.description,
            }
        });
        ContextBlockMetadataVo {
            id,
            display_name: text.display_name,
            description: text.description,
            update_mode: entry.metadata.update_mode,
            compression,
            provider_id: entry.provider_id,
        }
    })
    .collect();
    Ok(Json(ApiResponse::protocol_success(blocks)))
}
