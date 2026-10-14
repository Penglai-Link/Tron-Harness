//! Saved-policy tool authority and dynamic MCP bindings for a single invocation.
use super::super::{
    agui_warning::publish_mcp_runtime_warnings,
    helpers::{map_agent_error, map_mcp_error},
};
use super::{AgentInvocationError, AgentRunContextBuilder, McpWarningPublishContext};
use crate::{
    application::agent::{
        BusinessToolCatalog, discovery_expand_tool_definition, tool_record_to_effective_tool,
    },
    domain::agent::{
        AgentToolExposureState, EffectiveTool, InternalAgentToolPolicy, ToolAvailabilityState,
        ToolExposureMode,
    },
};
use std::collections::{HashMap, HashSet};

pub(super) struct PreparedToolAccess {
    pub(super) policy: Option<InternalAgentToolPolicy>,
    pub(super) tools: Vec<EffectiveTool>,
    pub(super) effective_tools: Vec<EffectiveTool>,
    pub(super) tool_exposure: AgentToolExposureState,
    pub(super) requested_tool_names: HashSet<String>,
}
impl AgentRunContextBuilder {
    pub(super) fn resolve_tool_access(
        &self,
        workspace_code: &str,
        agent_code: &str,
        locale: &str,
        requested_tools: Vec<EffectiveTool>,
    ) -> Result<PreparedToolAccess, AgentInvocationError> {
        let requested_tool_names = requested_tools
            .iter()
            .map(|tool| tool.tool_name.clone())
            .collect::<HashSet<_>>();
        let mut tools = requested_tools;
        let mut effective_tools = tools.clone();
        let mut tool_exposure = AgentToolExposureState::default();
        let internal_agents = if self.platform_scope || workspace_code.is_empty() {
            self.management.all_platform_internal_agents()
        } else {
            self.management
                .list_workspace_internal_agents(workspace_code)
        }
        .map_err(|error| map_agent_error(agent_code, error))?;
        let internal_agent_policy = internal_agents
            .into_iter()
            .find(|policy| policy.agent_code.eq_ignore_ascii_case(agent_code));

        if let Some(policy) = internal_agent_policy.as_ref() {
            // Saved policy owns internal-agent authority. Request DTOs only narrow
            // that authority by name; their descriptions/schemas are never trusted.
            // An empty request keeps the legacy contract: use the saved policy.
            let effective_tool_records = self
                .management
                .tools_by_codes(&policy.selected_tool_codes, locale)
                .into_iter()
                .filter(|tool| tool.availability_state == ToolAvailabilityState::Available)
                .filter(|tool| {
                    requested_tool_names.is_empty()
                        || requested_tool_names.contains(&tool.tool_name)
                })
                .collect::<Vec<_>>();
            match policy.tool_exposure_mode {
                ToolExposureMode::DirectList => {
                    tools = effective_tool_records
                        .into_iter()
                        .map(tool_record_to_effective_tool)
                        .collect();
                    effective_tools = tools.clone();
                }
                ToolExposureMode::DomainDiscovery => {
                    let visible_domains =
                        BusinessToolCatalog::domains_for_records(&effective_tool_records, locale);
                    let hidden_tools = effective_tool_records
                        .into_iter()
                        .map(tool_record_to_effective_tool)
                        .collect::<Vec<_>>();
                    let discovery_tool = discovery_expand_tool_definition(locale);
                    tools = vec![discovery_tool.clone()];
                    effective_tools = Vec::with_capacity(hidden_tools.len() + 1);
                    effective_tools.push(discovery_tool);
                    effective_tools.extend(hidden_tools.iter().cloned());
                    tool_exposure = AgentToolExposureState {
                        tool_exposure_mode: Some(policy.tool_exposure_mode),
                        expanded_domain_persistence_mode: Some(
                            policy.expanded_domain_persistence_mode,
                        ),
                        visible_domains,
                        hidden_tools,
                        expanded_domain_codes: Vec::new(),
                    };
                }
            }
        }

        Ok(PreparedToolAccess {
            policy: internal_agent_policy,
            tools,
            effective_tools,
            tool_exposure,
            requested_tool_names,
        })
    }

    pub(super) async fn materialize_mcp_tools(
        &self,
        access: &mut PreparedToolAccess,
        locale: &str,
        variable_values: &HashMap<String, String>,
        warning_context: &McpWarningPublishContext<'_>,
    ) -> Result<(), AgentInvocationError> {
        if let Some(policy) = access.policy.as_ref() {
            let mut materialization = self
                .mcp
                .materialize_runtime_tools(
                    warning_context.run_code,
                    warning_context.workspace_code,
                    policy.mcp_access_mode,
                    &policy.selected_server_codes,
                    variable_values,
                    self.tool_registry.as_ref(),
                )
                .await
                .map_err(map_mcp_error)?;
            publish_mcp_runtime_warnings(
                &self.event_bus,
                warning_context,
                &materialization.warnings,
            );
            // Only discovered bindings from policy-selected enabled servers can
            // add MCP authority. A same-name request projection creates no tool.
            let mut permitted = HashSet::new();
            let mut records = HashMap::new();
            let discovered = self
                .management
                .register_tool_descriptors(&materialization.descriptors)
                .map_err(AgentInvocationError::RuntimeFailed)?;
            for (descriptor, record) in materialization.descriptors.iter().zip(discovered) {
                if self
                    .management
                    .platform_tool_allowed(&descriptor.call_name)
                    .map_err(|error| map_agent_error(&policy.agent_code, error))?
                {
                    permitted.insert(descriptor.call_name.clone());
                }
                records.insert(descriptor.call_name.clone(), record);
            }
            // SelectedTools keeps the existing MCP server assignment semantics;
            // the selected enabled servers above already grant these tool codes.
            let server_tool_codes = records
                .values()
                .map(|record| record.tool_code.clone())
                .collect::<Vec<_>>();
            materialization.tools.retain(|tool| {
                records.get(&tool.tool_name).is_some_and(|record| {
                    policy
                        .tool_assignment_mode
                        .selects(record, &server_tool_codes)
                }) && permitted.contains(&tool.tool_name)
                    && (access.requested_tool_names.is_empty()
                        || access.requested_tool_names.contains(&tool.tool_name))
            });
            access
                .effective_tools
                .extend(materialization.tools.iter().cloned());
            if policy.tool_exposure_mode == ToolExposureMode::DomainDiscovery {
                access
                    .tool_exposure
                    .hidden_tools
                    .extend(materialization.tools);
                let hidden = &access.tool_exposure.hidden_tools;
                let records = self
                    .management
                    .list_tools(None, None, locale)
                    .into_iter()
                    .filter_map(|mut record| {
                        let tool = hidden
                            .iter()
                            .find(|tool| tool.tool_name == record.tool_name)?;
                        if let Some(domain) = &tool.domain_code {
                            record.domain_code = domain.clone();
                        }
                        Some(record)
                    })
                    .collect::<Vec<_>>();
                access.tool_exposure.visible_domains =
                    BusinessToolCatalog::domains_for_records(&records, locale);
            } else {
                access.tools.extend(materialization.tools);
            }
        }

        Ok(())
    }
}
