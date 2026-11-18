use std::collections::{BTreeMap, BTreeSet, HashMap};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::shared::locale::SupportedLocale;
use crate::{
    app::AppState,
    application::{
        short_term_memory::{ContextBudgetConfig, ContextProvidersConfig},
        system_config::SystemConfigChange,
    },
    response::ApiResponse,
};

const NATIVE_IDS: &[&str] = &[
    "seed_prompt_prefix",
    "conversation_history_summary",
    "current_session_history",
    "current_session_working_state",
    "runtime_identity",
    "assistant_onboarding",
    "tool_domains",
    "agent_directory",
    "skills.catalog",
    "page_context",
    "application_context",
    "topic_memo",
    "long_term_memory",
    "knowledge_base",
    "assistant_environment",
    "assistant_wakeup",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContextLayoutBlockVo {
    id: String,
    display_name: String,
    description: String,
    update_mode: eliteforge_agent_core::ContextBlockUpdateMode,
    compression: Option<ContextLayoutCompressionVo>,
    plugin: bool,
    unavailable: bool,
    provider_id: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContextLayoutCompressionVo {
    id: String,
    display_name: String,
    description: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContextLayoutVo {
    revision: String,
    order: Vec<String>,
    shares: BTreeMap<String, u32>,
    blocks: Vec<ContextLayoutBlockVo>,
    defaults: ContextLayoutDefaultsVo,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContextLayoutDefaultsVo {
    order: Vec<String>,
    shares: BTreeMap<String, u32>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateContextLayoutRequest {
    revision: String,
    order: Vec<String>,
    shares: BTreeMap<String, u32>,
}

pub(super) struct LayoutError {
    status: StatusCode,
    locale: SupportedLocale,
    key: &'static str,
}

impl LayoutError {
    fn new(status: StatusCode, key: &'static str, headers: &HeaderMap) -> Self {
        Self {
            status,
            locale: request_locale(headers),
            key,
        }
    }

    fn invalid(headers: &HeaderMap) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "agent_runtime.http.context_layout.invalid",
            headers,
        )
    }

    fn invalid_total(headers: &HeaderMap) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "agent_runtime.http.context_layout.invalid_total",
            headers,
        )
    }

    fn unavailable(headers: &HeaderMap) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "agent_runtime.http.context_layout.unavailable",
            headers,
        )
    }

    fn conflict(headers: &HeaderMap) -> Self {
        Self::new(
            StatusCode::CONFLICT,
            "agent_runtime.http.context_layout.conflict",
            headers,
        )
    }
}

impl IntoResponse for LayoutError {
    fn into_response(self) -> Response {
        let message: String = tron_i18n::translate(self.locale.as_str(), self.key).into_owned();
        (
            self.status,
            Json(ApiResponse::<crate::response::EmptyData>::protocol_error(
                "CONTEXT_LAYOUT_ERROR",
                message,
            )),
        )
            .into_response()
    }
}

pub(super) async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<ContextLayoutVo>>, LayoutError> {
    let config = state.short_term_memory.framework_context_config();
    let revision = state
        .system_config
        .get()
        .map_err(|_| LayoutError::unavailable(&headers))?
        .revision;
    let contributors = state
        .agent_framework
        .context_layout_catalog()
        .await
        .map_err(|_| LayoutError::unavailable(&headers))?;
    Ok(Json(ApiResponse::protocol_success(build_layout(
        revision,
        &config.context,
        contributors,
        request_locale(&headers),
    ))))
}

pub(super) async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateContextLayoutRequest>,
) -> Result<Json<ApiResponse<ContextLayoutVo>>, LayoutError> {
    // Validate the unmodified request. Never normalize invalid totals while saving.
    eliteforge_agent_memory::ContextShareAllocation::new(request.shares.clone())
        .map_err(|_| LayoutError::invalid_total(&headers))?;
    let current = state.short_term_memory.framework_context_config();
    let contributors = state
        .agent_framework
        .context_layout_catalog()
        .await
        .map_err(|_| LayoutError::unavailable(&headers))?;
    let active = contributors
        .iter()
        .map(|item| item.block_id.as_str())
        .collect::<BTreeSet<_>>();
    let known = current
        .context
        .providers
        .order
        .iter()
        .map(String::as_str)
        .chain(active.iter().copied())
        .collect::<BTreeSet<_>>();
    let ordered = request
        .order
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if ordered.len() != request.order.len()
        || ordered.len() != request.shares.len()
        || request
            .order
            .iter()
            .any(|id| !known.contains(id.as_str()) || !request.shares.contains_key(id))
        || active.iter().any(|id| !ordered.contains(id))
    {
        return Err(LayoutError::invalid(&headers));
    }
    let mut providers = current.context.providers;
    providers
        .data_blocks
        .retain(|id, _| NATIVE_IDS.contains(&id.as_str()) || active.contains(id.as_str()));
    providers.order = request.order.clone();
    providers.shares = request.shares.clone();
    providers
        .validate()
        .map_err(|_| LayoutError::invalid(&headers))?;
    let saved = state
        .system_config
        .update(
            &request.revision,
            vec![
                SystemConfigChange {
                    path: "shortTermMemory.context.providers.order".into(),
                    value: serde_json::json!(request.order),
                },
                SystemConfigChange {
                    path: "shortTermMemory.context.providers.shares".into(),
                    value: serde_json::json!(request.shares),
                },
            ],
            false,
        )
        .map_err(|error| match error {
            crate::application::system_config::SystemConfigError::RevisionConflict => {
                LayoutError::conflict(&headers)
            }
            crate::application::system_config::SystemConfigError::ReadFailed
            | crate::application::system_config::SystemConfigError::WriteFailed => {
                LayoutError::unavailable(&headers)
            }
            _ => LayoutError::invalid(&headers),
        })?;
    let updated = state.short_term_memory.framework_context_config();
    Ok(Json(ApiResponse::protocol_success(build_layout(
        saved.revision,
        &updated.context,
        contributors,
        request_locale(&headers),
    ))))
}

fn build_layout(
    revision: String,
    config: &ContextBudgetConfig,
    contributors: Vec<crate::application::agent::framework_runtime::ContextLayoutContributorInfo>,
    locale: SupportedLocale,
) -> ContextLayoutVo {
    let providers = &config.providers;
    let catalog = crate::application::short_term_memory::context_block_catalog(
        contributors.clone(),
        providers,
    );
    let contributors = contributors
        .into_iter()
        .map(|item| (item.block_id.clone(), item))
        .collect::<HashMap<_, _>>();
    let order = ordered_sources(providers, &contributors);
    let shares = providers.effective_shares(&order);
    // Use the same built-in configuration as new runtimes, never saved overrides.
    // Active plugin sources are included; stale unloaded sources are not restored.
    let built_in = ContextProvidersConfig::default();
    let default_order = ordered_sources(&built_in, &contributors);
    let defaults = ContextLayoutDefaultsVo {
        shares: built_in.effective_shares(&default_order),
        order: default_order,
    };
    let blocks = order
        .iter()
        .map(|id| {
            let (display_name, description) = match catalog.get(id) {
                Some(entry) => match entry.i18n {
                    // Host blocks carry stable resource keys; translate them here
                    // so persisted layouts never store localized text.
                    Some(keys) => (
                        tron_i18n::translate(locale.as_str(), keys.display_name).into_owned(),
                        tron_i18n::translate(locale.as_str(), keys.description).into_owned(),
                    ),
                    None => {
                        let text = entry.metadata.resolve(locale.as_str());
                        (text.display_name, text.description)
                    }
                },
                None => (id.clone(), String::new()),
            };
            let update_mode = catalog
                .get(id)
                .map(|entry| entry.metadata.update_mode)
                .unwrap_or_default();
            let compression = catalog
                .get(id)
                .map(|entry| &entry.metadata.compression)
                .filter(|metadata| !metadata.id.is_empty())
                .map(|metadata| {
                    let text = metadata.resolve(locale.as_str());
                    ContextLayoutCompressionVo {
                        id: metadata.id.clone(),
                        display_name: text.display_name,
                        description: text.description,
                    }
                });
            if let Some(item) = contributors.get(id) {
                ContextLayoutBlockVo {
                    id: id.clone(),
                    display_name,
                    description,
                    update_mode,
                    compression,
                    plugin: true,
                    unavailable: false,
                    provider_id: Some(item.provider_id.clone()),
                }
            } else {
                let native = NATIVE_IDS.contains(&id.as_str());
                ContextLayoutBlockVo {
                    id: id.clone(),
                    display_name,
                    description,
                    update_mode,
                    compression,
                    plugin: !native,
                    unavailable: !native,
                    provider_id: None,
                }
            }
        })
        .collect();
    ContextLayoutVo {
        revision,
        order,
        shares,
        blocks,
        defaults,
    }
}

fn ordered_sources(
    providers: &ContextProvidersConfig,
    contributors: &HashMap<
        String,
        crate::application::agent::framework_runtime::ContextLayoutContributorInfo,
    >,
) -> Vec<String> {
    let mut order = providers.order.clone();
    let mut additions = contributors
        .values()
        .filter(|item| !order.contains(&item.block_id))
        .map(|item| (item.metadata.default_order, item.block_id.clone()))
        .collect::<Vec<_>>();
    additions.sort();
    order.extend(additions.into_iter().map(|(_, id)| id));
    order
}

fn request_locale(headers: &HeaderMap) -> SupportedLocale {
    SupportedLocale::from_request(
        headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
    )
}
