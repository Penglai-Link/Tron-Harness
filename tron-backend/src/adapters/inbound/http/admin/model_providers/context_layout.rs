use std::collections::{BTreeMap, BTreeSet, HashMap};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    application::{short_term_memory::ContextProvidersConfig, system_config::SystemConfigChange},
    response::ApiResponse,
};

const NATIVE_IDS: &[&str] = &[
    "seed_prompt_prefix",
    "current_session_checkpoint",
    "current_session_history",
    "current_session_working_state",
    "runtime_identity",
    "dude_onboarding",
    "tool_domains",
    "agent_directory",
    "skills.catalog",
    "page_context",
    "application_context",
    "knowledge_base",
    "dude_environment",
    "dude_wakeup",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContextLayoutBlockVo {
    id: String,
    display_name: String,
    description: String,
    plugin: bool,
    unavailable: bool,
    provider_id: Option<String>,
    default_max_tokens: Option<usize>,
    default_max_ratio_bps: Option<u16>,
    default_max_messages: Option<usize>,
    override_max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContextLayoutVo {
    revision: String,
    order: Vec<String>,
    plugin_overrides: BTreeMap<String, u32>,
    blocks: Vec<ContextLayoutBlockVo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateContextLayoutRequest {
    revision: String,
    order: Vec<String>,
    plugin_overrides: BTreeMap<String, u32>,
}

pub(super) struct LayoutError {
    status: StatusCode,
    english: bool,
    message: &'static str,
}

impl LayoutError {
    fn invalid(headers: &HeaderMap) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            english: is_english(headers),
            message: "The context layout is invalid.",
        }
    }

    fn unavailable(headers: &HeaderMap) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            english: is_english(headers),
            message: "Could not load or save context layout settings.",
        }
    }

    fn conflict(headers: &HeaderMap) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            english: is_english(headers),
            message: "Context layout changed elsewhere. Refresh and try again.",
        }
    }
}

impl IntoResponse for LayoutError {
    fn into_response(self) -> Response {
        let message = if self.english {
            self.message
        } else {
            match self.message {
                "The context layout is invalid." => "上下文布局无效。",
                "Context layout changed elsewhere. Refresh and try again." => {
                    "上下文布局已被其他操作修改，请刷新后重试。"
                }
                _ => "无法读取或保存上下文布局配置。",
            }
        };
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

fn is_english(headers: &HeaderMap) -> bool {
    crate::shared::locale::SupportedLocale::from_request(
        headers.get("accept-language").and_then(|value| value.to_str().ok()),
    ) == crate::shared::locale::SupportedLocale::EnUs
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
        &config.context.providers,
        contributors,
    ))))
}

pub(super) async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateContextLayoutRequest>,
) -> Result<Json<ApiResponse<ContextLayoutVo>>, LayoutError> {
    let current = state.short_term_memory.framework_context_config();
    let contributors = state
        .agent_framework
        .context_layout_catalog()
        .await
        .map_err(|_| LayoutError::unavailable(&headers))?;
    let active_plugin_ids = contributors
        .iter()
        .map(|item| item.block_id.as_str())
        .collect::<BTreeSet<_>>();
    let current_order = &current.context.providers.order;
    let history_anchor = "current_session_history";
    let current_anchor_index = current_order
        .iter()
        .position(|id| id == history_anchor)
        .ok_or_else(|| LayoutError::invalid(&headers))?;
    if request.order.get(..=current_anchor_index) != current_order.get(..=current_anchor_index) {
        return Err(LayoutError::invalid(&headers));
    }
    let known_order_ids = current
        .context
        .providers
        .order
        .iter()
        .map(String::as_str)
        .chain(active_plugin_ids.iter().copied())
        .chain(current.context.providers.plugin_overrides.keys().map(String::as_str))
        .collect::<BTreeSet<_>>();
    if request.order.iter().any(|id| !known_order_ids.contains(id.as_str())) {
        return Err(LayoutError::invalid(&headers));
    }
    if active_plugin_ids
        .iter()
        .any(|id| !request.order.iter().any(|ordered| ordered.as_str() == *id))
    {
        return Err(LayoutError::invalid(&headers));
    }
    if request
        .plugin_overrides
        .keys()
        .any(|id| !active_plugin_ids.contains(id.as_str()) && !current.context.providers.plugin_overrides.contains_key(id))
    {
        return Err(LayoutError::invalid(&headers));
    }

    let mut providers = current.context.providers;
    // Older Tron versions stored plugin token caps in `dataBlocks`. Preserve
    // those caps when the UI loads the layout, then remove the legacy entries
    // as the new plugin-owned defaults and host overrides are saved.
    for id in &active_plugin_ids {
        providers.data_blocks.remove(*id);
    }
    // Entries left by uninstalled plugins are removable from this page. Prune
    // their legacy policies together with the order/override supplied by UI.
    providers.data_blocks.retain(|id, _| {
        NATIVE_IDS.contains(&id.as_str()) || active_plugin_ids.contains(id.as_str())
    });
    providers.order = request.order.clone();
    providers.plugin_overrides = request.plugin_overrides.clone();
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
                    path: "shortTermMemory.context.providers.pluginOverrides".into(),
                    value: serde_json::json!(request.plugin_overrides),
                },
            ],
            false,
        )
        .map_err(|error| match error {
            crate::application::system_config::SystemConfigError::RevisionConflict => {
                LayoutError::conflict(&headers)
            }
            crate::application::system_config::SystemConfigError::InvalidField(_)
            | crate::application::system_config::SystemConfigError::InvalidYaml(_)
            | crate::application::system_config::SystemConfigError::MigrationConfirmationRequired(_) => {
                LayoutError::invalid(&headers)
            }
            crate::application::system_config::SystemConfigError::ReadFailed
            | crate::application::system_config::SystemConfigError::WriteFailed => {
                LayoutError::unavailable(&headers)
            }
        })?;

    let updated = state.short_term_memory.framework_context_config();
    Ok(Json(ApiResponse::protocol_success(build_layout(
        saved.revision,
        &updated.context.providers,
        contributors,
    ))))
}

fn build_layout(
    revision: String,
    providers: &ContextProvidersConfig,
    contributors: Vec<crate::application::agent::framework_runtime::ContextLayoutContributorInfo>,
) -> ContextLayoutVo {
    let contributors = contributors
        .into_iter()
        .map(|item| (item.block_id.clone(), item))
        .collect::<HashMap<_, _>>();
    let mut order = providers.order.clone();
    let mut additions = contributors
        .values()
        .filter(|item| !order.contains(&item.block_id))
        .map(|item| (item.default_order, item.block_id.clone()))
        .collect::<Vec<_>>();
    additions.sort();
    order.extend(additions.into_iter().map(|(_, id)| id));
    for id in providers.plugin_overrides.keys() {
        if !order.contains(id) {
            order.push(id.clone());
        }
    }

    let mut plugin_overrides = providers.plugin_overrides.clone();
    for item in contributors.values() {
        if !plugin_overrides.contains_key(&item.block_id) {
            if let Some(tokens) = providers
                .data_blocks
                .get(&item.block_id)
                .and_then(|legacy| legacy.max_tokens)
            {
                plugin_overrides.insert(item.block_id.clone(), tokens);
            }
        }
    }

    let blocks = order
        .iter()
        .map(|id| {
            if let Some(item) = contributors.get(id) {
                return ContextLayoutBlockVo {
                    id: id.clone(),
                    display_name: item.display_name.clone(),
                    description: item.description.clone(),
                    plugin: true,
                    unavailable: false,
                    provider_id: Some(item.provider_id.clone()),
                    default_max_tokens: item.default_max_tokens,
                    default_max_ratio_bps: item.default_max_ratio_bps,
                    default_max_messages: item.default_max_messages,
                    override_max_tokens: plugin_overrides.get(id).copied(),
                };
            }
            let is_native = NATIVE_IDS.contains(&id.as_str());
            ContextLayoutBlockVo {
                id: id.clone(),
                display_name: id.clone(),
                description: String::new(),
                plugin: !is_native,
                unavailable: !is_native,
                provider_id: None,
                default_max_tokens: None,
                default_max_ratio_bps: None,
                default_max_messages: None,
                override_max_tokens: plugin_overrides.get(id).copied(),
            }
        })
        .collect();

    ContextLayoutVo {
        revision,
        order,
        plugin_overrides,
        blocks,
    }
}
