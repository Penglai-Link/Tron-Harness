use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    domain::{
        agent::{
            AgentRunContext, EffectiveTool, ExpandedDomainPersistenceMode, Tool, ToolContextDelta,
            ToolError, ToolExecutionResult, ToolObservation, ToolRequest,
        },
        prompt::PromptRuntimeProvider,
    },
    shared::locale::SupportedLocale,
};

use crate::application::agent::tool_catalog::{materialize_schema, translate};

pub const DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL: &str = "discovery_expand_tool_domain";

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
                "x-i18n-description-key": "agent.toolCatalog.discovery.expand.description",
                "required": ["domainCode"],
                "examples": [{
                    "domainCode": "workspace"
                }],
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
    let persistence_label = domain_persistence_label(locale, persistence_mode);
    let catalog = serde_json::to_string_pretty(visible_domains).unwrap_or_else(|_| "[]".into());
    let template =
        crate::domain::prompt::builtin_prompt_text("agent.domain-discovery", locale.as_str())
            .expect("domain discovery prompt must be bundled");
    template
        .replace("{{visible_domains}}", &catalog)
        .replace("{{persistence_mode}}", persistence_label)
        .replace(
            "{DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL}",
            DISCOVERY_EXPAND_TOOL_DOMAIN_TOOL,
        )
}

pub fn domain_discovery_runtime_values(
    visible_domains: &[crate::domain::agent::ToolDomainRecord],
    persistence_mode: ExpandedDomainPersistenceMode,
    locale: &str,
) -> HashMap<PromptRuntimeProvider, Value> {
    let locale = SupportedLocale::from_raw(locale);
    HashMap::from([
        (
            PromptRuntimeProvider::AgentVisibleDomains,
            serde_json::to_value(visible_domains).unwrap_or_else(|_| json!([])),
        ),
        (
            PromptRuntimeProvider::AgentDomainPersistenceMode,
            Value::String(domain_persistence_label(locale, persistence_mode).to_string()),
        ),
    ])
}

fn domain_persistence_label(
    locale: SupportedLocale,
    persistence_mode: ExpandedDomainPersistenceMode,
) -> &'static str {
    match (locale, persistence_mode) {
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
                "domainCode": domain.domain_code,
                "status": "expanded",
                "toolCount": tools.len(),
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
