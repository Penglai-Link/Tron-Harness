use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;

use crate::{
    domain::agent::{
        AgentRunContext, EffectiveTool, ExpandedDomainPersistenceMode, Tool, ToolContextDelta,
        ToolError, ToolExecutionResult, ToolObservation, ToolRequest,
    },
    shared::locale::SupportedLocale,
};

use crate::application::agent::tool_catalog::{materialize_schema, translate};

pub const DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL: &str = "discovery.expand_tool_domain";

pub fn discovery_expand_tool_definition(locale: &str) -> EffectiveTool {
    let locale = SupportedLocale::from_raw(locale);
    EffectiveTool {
        tool_name: DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL.to_string(),
        domain_code: Some("discovery".to_string()),
        description: translate(locale, "agent.toolCatalog.discovery.expand.description")
            .to_string(),
        input_schema: materialize_schema(
            locale,
            &json!({
                "type": "object",
                "required": ["domainCode"],
                "properties": {
                    "domainCode": {
                        "type": "string",
                        "x-i18n-description-key": "agent.toolCatalog.discovery.expand.params.domainCode"
                    }
                },
                "additionalProperties": false
            }),
        ),
        skill_name: None,
        required_variable_keys: Vec::new(),
    }
}

pub fn build_domain_discovery_guidance(
    visible_domains: &[crate::domain::agent::ToolDomainRecord],
    persistence_mode: ExpandedDomainPersistenceMode,
    locale: &str,
) -> String {
    let locale = SupportedLocale::from_raw(locale);
    let persistence_label = match (locale, persistence_mode) {
        (SupportedLocale::ZhCn, ExpandedDomainPersistenceMode::StickyExpanded) => {
            "sticky_expanded（持续保留）"
        }
        (SupportedLocale::ZhCn, ExpandedDomainPersistenceMode::EphemeralExpanded) => {
            "ephemeral_expanded（执行后收起）"
        }
        (SupportedLocale::EnUs, ExpandedDomainPersistenceMode::StickyExpanded) => "sticky_expanded",
        (SupportedLocale::EnUs, ExpandedDomainPersistenceMode::EphemeralExpanded) => {
            "ephemeral_expanded"
        }
    };
    let catalog = serde_json::to_string_pretty(visible_domains).unwrap_or_else(|_| "[]".into());
    match locale {
        SupportedLocale::ZhCn => format!(
            "当前 tool exposure mode 为 domain_discovery。\n\
下面以 JSON 给出当前可见的全部 tool domain 列表。当你需要某个 domain 里的具体工具时，请调用 `{DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL}` 并传入 `domainCode`。\n\
expanded domain persistence mode：{persistence_label}。\n\
Visible domains:\n{catalog}"
        ),
        SupportedLocale::EnUs => format!(
            "Tool exposure mode is domain_discovery.\n\
Visible tool domains are listed below as JSON. When you need concrete tools from a domain, call `{DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL}` with `domainCode`.\n\
Expanded domain persistence mode: {persistence_label}.\n\
Visible domains:\n{catalog}"
        ),
    }
}

pub(super) fn discovery_tool() -> Arc<dyn Tool> {
    Arc::new(DiscoveryExpandToolDomainTool)
}

struct DiscoveryExpandToolDomainTool;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpandToolDomainInput {
    domain_code: String,
}

impl Tool for DiscoveryExpandToolDomainTool {
    fn name(&self) -> &str {
        DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: ExpandToolDomainInput =
            serde_json::from_value(request.input).map_err(|error| {
                ToolError::ExecutionFailed(format!("tool input is invalid: {error}"))
            })?;
        let requested_domain = input.domain_code.trim();
        if requested_domain.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "domainCode is required".to_string(),
            ));
        }
        let Some(domain) = context
            .tool_exposure
            .visible_domains
            .iter()
            .find(|domain| domain.domain_code.eq_ignore_ascii_case(requested_domain))
            .cloned()
        else {
            return Err(ToolError::ExecutionFailed(format!(
                "tool domain `{requested_domain}` is not visible to this agent"
            )));
        };
        let tools = context
            .tool_exposure
            .hidden_tools
            .iter()
            .filter(|tool| {
                tool.domain_code
                    .as_deref()
                    .is_some_and(|domain_code| domain_code.eq_ignore_ascii_case(requested_domain))
            })
            .cloned()
            .collect::<Vec<_>>();
        if tools.is_empty() {
            return Err(ToolError::ExecutionFailed(format!(
                "tool domain `{requested_domain}` has no callable tools"
            )));
        }
        let observation = ToolObservation {
            tool_name: self.name().to_string(),
            output: serde_json::to_string(&json!({
                "domain": domain,
                "tools": tools,
            }))
            .map_err(service_tool_error)?,
        };
        Ok(ToolExecutionResult {
            observation,
            context_delta: ToolContextDelta {
                add_tools: tools,
                expanded_domain_codes: vec![requested_domain.to_string()],
            },
        })
    }
}

fn service_tool_error(error: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
