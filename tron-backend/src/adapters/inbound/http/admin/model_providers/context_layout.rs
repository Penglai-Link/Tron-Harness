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
    application::{short_term_memory::ContextBudgetConfig, system_config::SystemConfigChange},
    response::ApiResponse,
};

const NATIVE_IDS: &[&str] = &[
    "seed_prompt_prefix",
    "conversation_history_summary",
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
    native_budgets: BTreeMap<String, ContextLayoutBudget>,
    blocks: Vec<ContextLayoutBlockVo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateContextLayoutRequest {
    revision: String,
    order: Vec<String>,
    plugin_overrides: BTreeMap<String, u32>,
    #[serde(default)]
    native_budgets: BTreeMap<String, ContextLayoutBudget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextLayoutBudget {
    max_tokens: Option<u32>,
    max_ratio_bps: Option<u32>,
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
            message: "The context configuration is invalid.",
        }
    }

    fn unavailable(headers: &HeaderMap) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            english: is_english(headers),
            message: "Could not load or save context configuration settings.",
        }
    }

    fn conflict(headers: &HeaderMap) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            english: is_english(headers),
            message: "Context configuration changed elsewhere. Refresh and try again.",
        }
    }
}

impl IntoResponse for LayoutError {
    fn into_response(self) -> Response {
        let message = if self.english {
            self.message
        } else {
            match self.message {
                "The context configuration is invalid." => "上下文配置无效。",
                "Context configuration changed elsewhere. Refresh and try again." => {
                    "上下文配置已被其他操作修改，请刷新后重试。"
                }
                _ => "无法读取或保存上下文配置配置。",
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
        headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
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
        &config.context,
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
        .chain(
            current
                .context
                .providers
                .plugin_overrides
                .keys()
                .map(String::as_str),
        )
        .collect::<BTreeSet<_>>();
    if request
        .order
        .iter()
        .any(|id| !known_order_ids.contains(id.as_str()))
    {
        return Err(LayoutError::invalid(&headers));
    }
    if active_plugin_ids
        .iter()
        .any(|id| !request.order.iter().any(|ordered| ordered.as_str() == *id))
    {
        return Err(LayoutError::invalid(&headers));
    }
    if request.plugin_overrides.keys().any(|id| {
        !active_plugin_ids.contains(id.as_str())
            && !current.context.providers.plugin_overrides.contains_key(id)
    }) {
        return Err(LayoutError::invalid(&headers));
    }

    let mut budget_changes = native_budget_changes(&current.context, &request.native_budgets)
        .map_err(|_| LayoutError::invalid(&headers))?;
    let mut providers = current.context.providers;
    // Keep required host policy entries while validating the order. Plugin
    // overrides take precedence over legacy dataBlocks caps at runtime.
    for id in &active_plugin_ids {
        if !NATIVE_IDS.contains(id) {
            providers.data_blocks.remove(*id);
        }
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
            {
                budget_changes.extend([
                    SystemConfigChange {
                        path: "shortTermMemory.context.providers.order".into(),
                        value: serde_json::json!(request.order),
                    },
                    SystemConfigChange {
                        path: "shortTermMemory.context.providers.pluginOverrides".into(),
                        value: serde_json::json!(request.plugin_overrides),
                    },
                ]);
                budget_changes
            },
            false,
        )
        .map_err(|error| match error {
            crate::application::system_config::SystemConfigError::RevisionConflict => {
                LayoutError::conflict(&headers)
            }
            crate::application::system_config::SystemConfigError::InvalidField(_)
            | crate::application::system_config::SystemConfigError::InvalidYaml(_)
            | crate::application::system_config::SystemConfigError::MigrationConfirmationRequired(
                _,
            ) => LayoutError::invalid(&headers),
            crate::application::system_config::SystemConfigError::ReadFailed
            | crate::application::system_config::SystemConfigError::WriteFailed => {
                LayoutError::unavailable(&headers)
            }
        })?;

    let updated = state.short_term_memory.framework_context_config();
    Ok(Json(ApiResponse::protocol_success(build_layout(
        saved.revision,
        &updated.context,
        contributors,
    ))))
}

fn build_layout(
    revision: String,
    config: &ContextBudgetConfig,
    contributors: Vec<crate::application::agent::framework_runtime::ContextLayoutContributorInfo>,
) -> ContextLayoutVo {
    let providers = &config.providers;
    let native_budgets = native_budgets(config);
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
        native_budgets,
        blocks,
    }
}

fn native_budgets(config: &ContextBudgetConfig) -> BTreeMap<String, ContextLayoutBudget> {
    let mut budgets = config
        .providers
        .data_blocks
        .iter()
        .filter(|(id, _)| NATIVE_IDS.contains(&id.as_str()) && id.as_str() != "skills.catalog")
        .map(|(id, block)| {
            (
                id.clone(),
                ContextLayoutBudget {
                    max_tokens: block.max_tokens,
                    max_ratio_bps: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (id, block) in [
        (
            "conversation_history_summary",
            &config.blocks.conversation_history_summary,
        ),
        (
            "current_session_working_state",
            &config.blocks.current_session_working_state,
        ),
    ] {
        budgets.insert(
            id.into(),
            ContextLayoutBudget {
                max_tokens: block.max_tokens,
                max_ratio_bps: block.max_ratio_bps,
            },
        );
    }
    budgets
}

fn native_budget_changes(
    config: &ContextBudgetConfig,
    requested: &BTreeMap<String, ContextLayoutBudget>,
) -> Result<Vec<SystemConfigChange>, ()> {
    let current = native_budgets(config);
    let mut changes = Vec::new();
    for (id, budget) in requested {
        let previous = current.get(id).ok_or(())?;
        if budget.max_tokens == Some(0) {
            return Err(());
        }
        let block = match id.as_str() {
            "conversation_history_summary" => Some("conversationHistorySummary"),
            "current_session_working_state" => Some("currentSessionWorkingState"),
            _ => None,
        };
        let prefix = if let Some(block) = block {
            if !budget
                .max_ratio_bps
                .is_some_and(|value| (1..=10_000).contains(&value))
            {
                return Err(());
            }
            format!("shortTermMemory.context.blocks.{block}")
        } else {
            if budget.max_ratio_bps.is_some() {
                return Err(());
            }
            if budget.max_tokens.is_none()
                && config.providers.data_blocks[id].policy.strategy
                    != crate::application::short_term_memory::ContextReductionStrategy::Keep
            {
                return Err(());
            }
            format!("shortTermMemory.context.providers.dataBlocks.{id}")
        };
        if budget.max_tokens != previous.max_tokens {
            changes.push(SystemConfigChange {
                path: format!("{prefix}.maxTokens"),
                value: serde_json::json!(budget.max_tokens),
            });
        }
        if budget.max_ratio_bps != previous.max_ratio_bps {
            changes.push(SystemConfigChange {
                path: format!("{prefix}.maxRatio"),
                value: serde_json::json!(format!(
                    "{}%",
                    f64::from(budget.max_ratio_bps.ok_or(())?) / 100.0
                )),
            });
        }
    }
    Ok(changes)
}
